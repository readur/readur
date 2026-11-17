//! Figures about the running server process itself, for `/metrics` and
//! `/api/metrics`.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

static STARTED: OnceLock<Instant> = OnceLock::new();

/// Records the moment the process started. `main` calls this first thing;
/// later calls keep the first moment.
pub fn mark_process_start() {
    STARTED.get_or_init(Instant::now);
}

/// Time since [`mark_process_start`] (or since the first call here, in tests
/// and tools that never call it).
pub fn uptime() -> Duration {
    STARTED.get_or_init(Instant::now).elapsed()
}

/// Resident memory of this process in bytes, or `None` where the platform
/// does not report it.
pub fn resident_memory_bytes() -> Option<u64> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
    let pid = sysinfo::get_current_pid().ok()?;
    let mut system = System::new().ok()?;
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        false,
        ProcessRefreshKind::nothing().with_memory(),
    );
    let bytes = system.process(pid)?.memory();
    (bytes > 0).then_some(bytes)
}

/// Percentage of the filesystem holding `path` that is in use, or `None`
/// when no mounted filesystem can be matched to it.
pub fn filesystem_usage_percent(path: &std::path::Path) -> Option<f64> {
    let path = path.canonicalize().ok()?;
    let disks = sysinfo::Disks::new_with_refreshed_list().ok()?;
    let disk = disks
        .iter()
        .filter(|disk| path.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())?;
    let total = disk.total_space();
    (total > 0).then(|| {
        let used = total.saturating_sub(disk.available_space());
        used as f64 / total as f64 * 100.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_grows() {
        mark_process_start();
        let first = uptime();
        std::thread::sleep(Duration::from_millis(20));
        assert!(uptime() > first);
    }

    #[test]
    fn this_process_uses_memory() {
        let bytes = resident_memory_bytes().expect("RSS is reported on this platform");
        assert!(bytes > 1024 * 1024, "{bytes} bytes");
    }

    #[test]
    fn the_temp_directory_lives_on_a_filesystem() {
        let percent = filesystem_usage_percent(&std::env::temp_dir()).expect("a filesystem");
        assert!((0.0..=100.0).contains(&percent), "{percent}");
    }
}
