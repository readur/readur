//! Where the server listens, which database it uses and which files it accepts.

use std::env;

/// `DATABASE_URL`, or a URL built from the `POSTGRES_*` variables (each with a default).
pub(super) fn database_url() -> String {
    if env::var("DATABASE_URL").is_ok() {
        println!("✅ DATABASE_URL: configured (loaded from env)");
        return env::var("DATABASE_URL").unwrap_or_default();
    }
    let db_host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = env::var("POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
    let db_name = env::var("POSTGRES_DB").unwrap_or_else(|_| "readur".to_string());
    let db_user = env::var("POSTGRES_USER").unwrap_or_else(|_| "readur".to_string());
    let db_password = env::var("POSTGRES_PASSWORD").unwrap_or_else(|_| "readur".to_string());

    // Check if any individual env vars were set
    let using_individual_vars = env::var("POSTGRES_HOST").is_ok()
        || env::var("POSTGRES_PORT").is_ok()
        || env::var("POSTGRES_DB").is_ok()
        || env::var("POSTGRES_USER").is_ok()
        || env::var("POSTGRES_PASSWORD").is_ok();

    if using_individual_vars {
        println!("📊 Constructing DATABASE_URL from individual environment variables:");
        for (name, value) in [
            ("POSTGRES_HOST", &db_host),
            ("POSTGRES_PORT", &db_port),
            ("POSTGRES_DB", &db_name),
            ("POSTGRES_USER", &db_user),
        ] {
            if env::var(name).is_ok() {
                println!("  ✅ {}: {}", name, value);
            } else {
                println!("  ⚠️  {}: {} (using default)", name, value);
            }
        }
        if env::var("POSTGRES_PASSWORD").is_ok() {
            println!(
                "  ✅ POSTGRES_PASSWORD: ***hidden*** ({} chars)",
                db_password.len()
            );
        } else {
            println!("  ⚠️  POSTGRES_PASSWORD: using default");
        }
        println!(
            "🔗 Constructed DATABASE_URL: postgresql://{}:***@{}:{}/{}",
            db_user, db_host, db_port, db_name
        );
    } else {
        println!("⚠️  DATABASE_URL: postgresql://readur:***@localhost:5432/readur (using all defaults - no env vars set)");
    }

    format!(
        "postgresql://{}:{}@{}:{}/{}",
        db_user, db_password, db_host, db_port, db_name
    )
}

/// `SERVER_ADDRESS` (host:port), or `SERVER_HOST` and `SERVER_PORT` combined.
pub(super) fn server_address() -> String {
    if let Ok(addr) = env::var("SERVER_ADDRESS") {
        println!("✅ SERVER_ADDRESS: {} (loaded from env)", addr);
        return addr;
    }
    let host = super::logged::logged_string("SERVER_HOST", env::var("SERVER_HOST"), "0.0.0.0");
    let port = super::logged::logged_string("SERVER_PORT", env::var("SERVER_PORT"), "8000");
    let combined_address = format!("{}:{}", host, port);
    println!("🔗 Combined server_address: {}", combined_address);
    combined_address
}

/// `ALLOWED_FILE_TYPES`: comma-separated extensions, lowercased.
pub(super) fn allowed_file_types() -> Vec<String> {
    let file_types_str = super::logged::logged_string(
        "ALLOWED_FILE_TYPES",
        env::var("ALLOWED_FILE_TYPES"),
        "pdf,txt,doc,docx,png,jpg,jpeg",
    );
    let types_vec: Vec<String> = file_types_str
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .collect();
    println!("📄 Parsed file types: {:?}", types_vec);
    types_vec
}
