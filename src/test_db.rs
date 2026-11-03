//! Per-test PostgreSQL databases cloned from one migrated template.
//!
//! Each test process gets one PostgreSQL server and runs the migrations once,
//! into a template database. Every test then gets its own database with
//! `CREATE DATABASE … TEMPLATE …`, a file-level copy that takes milliseconds,
//! instead of a fresh container plus the whole migration history per test.
//!
//! The server is either
//! - `READUR_TEST_POSTGRES_URL`, any database URL on an existing server whose
//!   role may `CREATEDB` (a CI service container, a local PostgreSQL), or
//! - a `postgres:POSTGRES_TAG` testcontainer started on first use.
//!
//! At process exit the container is removed or, on an external server, the
//! databases this process created are dropped. The template is kept on an
//! external server and reused by later runs while the migrations are unchanged
//! (its name carries a hash of them).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Duration;

use sqlx::{Connection, Executor, PgConnection};

/// The image tag for the test server. Keep in step with docker-compose*.yml.
pub const POSTGRES_TAG: &str = "18";

/// Point tests at an existing server instead of starting a container.
// DDL here interpolates database identifiers, which PostgreSQL cannot bind.
// Every one is generated in this file (readur_test_<pid>_<n>,
// readur_template_<fingerprint>), never taken from input, so the dynamic SQL
// is asserted safe for sqlx 0.9.
pub const SERVER_URL_ENV: &str = "READUR_TEST_POSTGRES_URL";

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A database created for one test. Dropped with [`TestDatabase::drop_database`],
/// or at process exit.
#[derive(Debug, Clone)]
pub struct TestDatabase {
    name: String,
    url: String,
}

impl TestDatabase {
    /// A fresh database with every migration applied.
    pub async fn from_template() -> Self {
        let server = server();
        Self::create(server, Some(&server.template)).await
    }

    /// A fresh database with no migrations applied, for tests of the migrations.
    pub async fn empty() -> Self {
        Self::create(server(), None).await
    }

    /// The connection URL for this database.
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Drops the database now. Close every pool on it first.
    pub async fn drop_database(&self) -> Result<(), sqlx::Error> {
        let mut admin = PgConnection::connect(&server().admin_url).await?;
        admin
            .execute(sqlx::AssertSqlSafe(format!(r#"DROP DATABASE IF EXISTS "{}" WITH (FORCE)"#, self.name)))
            .await?;
        server().created.lock().unwrap().retain(|n| n != &self.name);
        Ok(())
    }

    async fn create(server: &'static Server, template: Option<&str>) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let name = format!(
            "readur_test_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let sql = match template {
            Some(t) => format!(r#"CREATE DATABASE "{name}" TEMPLATE "{t}""#),
            None => format!(r#"CREATE DATABASE "{name}""#),
        };
        server.created.lock().unwrap().push(name.clone());

        let mut admin = PgConnection::connect(&server.admin_url)
            .await
            .expect("connect to test postgres");
        // A template cannot be copied while another session is connected to it.
        for attempt in 1.. {
            match admin.execute(sqlx::AssertSqlSafe(sql.clone())).await {
                Ok(_) => break,
                Err(sqlx::Error::Database(e))
                    if e.code().as_deref() == Some("55006") && attempt < 50 =>
                {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) => panic!("{sql}: {e}"),
            }
        }
        let url = with_database(&server.admin_url, &name);
        Self { name, url }
    }
}

struct Server {
    /// A URL on the server's maintenance database, for CREATE/DROP DATABASE.
    admin_url: String,
    template: String,
    created: Mutex<Vec<String>>,
    /// Tells the owner thread to tear down; it answers once done.
    shutdown: Mutex<Option<(mpsc::Sender<()>, mpsc::Receiver<()>)>>,
}

static SERVER: OnceLock<Server> = OnceLock::new();

fn server() -> &'static Server {
    SERVER.get_or_init(start_server)
}

/// Starts the owner thread, which keeps its own runtime (and the container,
/// if any) for the life of the process, independent of each test's runtime.
fn start_server() -> Server {
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(String, String), String>>();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (done_tx, done_rx) = mpsc::channel::<()>();

    std::thread::Builder::new()
        .name("test-postgres".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let started = rt.block_on(async {
                let (admin_url, container) = match std::env::var(SERVER_URL_ENV) {
                    Ok(url) => (with_database(&url, "postgres"), None),
                    Err(_) => {
                        let (url, container) = start_container().await?;
                        (url, Some(container))
                    }
                };
                let template = build_template(&admin_url).await?;
                Ok::<_, String>((admin_url, template, container))
            });
            let container = match started {
                Ok((admin_url, template, container)) => {
                    let _ = ready_tx.send(Ok((admin_url, template)));
                    container
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                    return;
                }
            };
            if stop_rx.recv().is_ok() {
                rt.block_on(async {
                    match container {
                        // Removing the container takes every database with it.
                        Some(container) => drop(container),
                        None => drop_created_databases().await,
                    }
                });
                let _ = done_tx.send(());
            }
        })
        .expect("spawn test-postgres thread");

    let (admin_url, template) = match ready_rx.recv() {
        Ok(Ok(started)) => started,
        Ok(Err(e)) => panic!("test postgres unavailable: {e}"),
        Err(_) => panic!("test postgres thread exited during startup"),
    };
    register_teardown();
    Server {
        admin_url,
        template,
        created: Mutex::new(Vec::new()),
        shutdown: Mutex::new(Some((stop_tx, done_rx))),
    }
}

type Container = testcontainers::ContainerAsync<testcontainers_modules::postgres::Postgres>;

async fn start_container() -> Result<(String, Container), String> {
    use testcontainers::{runners::AsyncRunner, ImageExt};

    let container = testcontainers_modules::postgres::Postgres::default()
        .with_tag(POSTGRES_TAG)
        .with_env_var("POSTGRES_USER", "readur")
        .with_env_var("POSTGRES_PASSWORD", "readur")
        .with_env_var("POSTGRES_DB", "postgres")
        // Durability is irrelevant for throwaway test data.
        .with_cmd(["-c", "fsync=off", "-c", "synchronous_commit=off", "-c", "full_page_writes=off", "-c", "max_connections=500"])
        .start()
        .await
        .map_err(|e| {
            format!("could not start postgres:{POSTGRES_TAG} ({e}); set {SERVER_URL_ENV} to use an existing server")
        })?;
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .map_err(|e| e.to_string())?;
    Ok((
        format!("postgresql://readur:readur@127.0.0.1:{port}/postgres"),
        container,
    ))
}

/// Creates the migrated template unless an up-to-date one already exists.
/// Serialised by an advisory lock, since test binaries may share a server.
async fn build_template(admin_url: &str) -> Result<String, String> {
    let template = format!("readur_template_{:016x}", migrations_fingerprint());
    let err = |e: sqlx::Error| format!("building {template}: {e}");

    let mut admin = PgConnection::connect(admin_url).await.map_err(err)?;
    admin
        .execute("SELECT pg_advisory_lock(7_901_000_001)")
        .await
        .map_err(err)?;
    let ready: Option<bool> =
        sqlx::query_scalar("SELECT datistemplate FROM pg_database WHERE datname = $1")
            .bind(&template)
            .fetch_optional(&mut admin)
            .await
            .map_err(err)?;
    if ready != Some(true) {
        // A database that is not yet flagged as a template is a half-built one.
        admin
            .execute(sqlx::AssertSqlSafe(format!(r#"DROP DATABASE IF EXISTS "{template}" WITH (FORCE)"#)))
            .await
            .map_err(err)?;
        admin
            .execute(sqlx::AssertSqlSafe(format!(r#"CREATE DATABASE "{template}""#)))
            .await
            .map_err(err)?;
        let mut conn = PgConnection::connect(&with_database(admin_url, &template))
            .await
            .map_err(err)?;
        MIGRATOR
            .run(&mut conn)
            .await
            .map_err(|e| format!("migrating {template}: {e}"))?;
        conn.close().await.map_err(err)?;
        admin
            .execute(sqlx::AssertSqlSafe(format!(r#"ALTER DATABASE "{template}" IS_TEMPLATE true"#)))
            .await
            .map_err(err)?;
    }
    admin
        .execute("SELECT pg_advisory_unlock(7_901_000_001)")
        .await
        .map_err(err)?;
    Ok(template)
}

async fn drop_created_databases() {
    let server = server();
    let names = std::mem::take(&mut *server.created.lock().unwrap());
    let Ok(mut admin) = PgConnection::connect(&server.admin_url).await else {
        return;
    };
    for name in names {
        let _ = admin
            .execute(sqlx::AssertSqlSafe(format!(r#"DROP DATABASE IF EXISTS "{name}" WITH (FORCE)"#)))
            .await;
    }
}

/// Process statics are never dropped, so tear down from an `atexit` hook. The
/// test harness ends with `process::exit`, which runs it whether tests passed
/// or failed; a killed process leaks the container, as a killed test always has.
fn register_teardown() {
    extern "C" fn teardown() {
        let Some(server) = SERVER.get() else { return };
        if let Some((stop, done)) = server.shutdown.lock().ok().and_then(|mut s| s.take()) {
            if stop.send(()).is_ok() {
                let _ = done.recv_timeout(Duration::from_secs(30));
            }
        }
    }
    unsafe {
        libc::atexit(teardown);
    }
}

/// Stable across runs: FNV-1a over every migration's version and checksum.
fn migrations_fingerprint() -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for migration in MIGRATOR.iter() {
        for byte in migration
            .version
            .to_le_bytes()
            .iter()
            .chain(migration.checksum.iter())
        {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

/// `url` with its database (path) replaced by `database`.
fn with_database(url: &str, database: &str) -> String {
    let mut parsed = url::Url::parse(url)
        .unwrap_or_else(|e| panic!("invalid postgres URL in {SERVER_URL_ENV}: {e}"));
    parsed.set_path(database);
    parsed.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_database_replaces_only_the_path() {
        assert_eq!(
            with_database(
                "postgresql://u:p@h:5432/readur?sslmode=disable",
                "readur_test_1_0"
            ),
            "postgresql://u:p@h:5432/readur_test_1_0?sslmode=disable"
        );
        assert_eq!(
            with_database("postgres://h", "postgres"),
            "postgres://h/postgres"
        );
    }

    #[test]
    fn fingerprint_is_stable_within_a_build() {
        assert_eq!(migrations_fingerprint(), migrations_fingerprint());
    }
}
