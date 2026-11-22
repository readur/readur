//! How a user's own settings apply to their documents beyond the Tesseract
//! options (see [`super::tesseract_params`]): the OCR timeout and the upload
//! size limit.
//!
//! The server-wide values (`OCR_TIMEOUT_SECONDS`, `MAX_FILE_SIZE_MB`) are the
//! defaults for a user who has never saved settings; `MAX_FILE_SIZE_MB` also
//! caps every user's own limit, because the request body limit in front of the
//! upload routes is set from it.

use std::future::Future;
use std::time::Duration;

use anyhow::{anyhow, Result};

use crate::models::Settings;

/// Longest OCR timeout that can take effect: the queue maintenance task
/// resets jobs that have been processing for 10 minutes (to recover from a
/// crashed worker), so a longer timeout would never be reached.
pub const MAX_OCR_TIMEOUT_SECONDS: i32 = 600;

/// The settings a document is processed with: its owner's, or the defaults
/// with the server's OCR timeout when the owner has none saved (or the
/// document has no owner).
pub fn settings_for_processing(stored: Option<Settings>, server_timeout_seconds: u64) -> Settings {
    stored.unwrap_or_else(|| Settings {
        ocr_timeout_seconds: i32::try_from(server_timeout_seconds).unwrap_or(i32::MAX),
        ..Settings::default()
    })
}

/// The longest a document processed with `settings` may spend in OCR, at
/// most [`MAX_OCR_TIMEOUT_SECONDS`].
pub fn ocr_timeout(settings: &Settings) -> Duration {
    Duration::from_secs(
        settings
            .ocr_timeout_seconds
            .clamp(1, MAX_OCR_TIMEOUT_SECONDS) as u64,
    )
}

/// Runs `work`, failing it with a timeout error (classified `ocr_timeout`)
/// once `timeout` has passed. Dropping `work` stops it: external tools are
/// spawned with `kill_on_drop`, and an in-process Tesseract call finishes in
/// the background with its result discarded.
pub async fn with_ocr_timeout<T, F>(timeout: Duration, work: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match tokio::time::timeout(timeout, work).await {
        Ok(result) => result,
        Err(_) => Err(anyhow!(
            "OCR timed out after {:?} (the document owner's OCR timeout setting)",
            timeout
        )),
    }
}

/// The largest upload, in bytes, a user with `user_settings` may make: their
/// own `max_file_size_mb`, capped by the server's `MAX_FILE_SIZE_MB`.
pub fn max_upload_bytes(server_max_mb: u64, user_settings: Option<&Settings>) -> u64 {
    let user_mb = user_settings
        .and_then(|s| s.max_file_size_mb)
        .filter(|mb| *mb > 0)
        .map(|mb| mb as u64)
        .unwrap_or(server_max_mb);
    server_max_mb.min(user_mb).saturating_mul(1024 * 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owners_timeout_is_used_when_saved() {
        let stored = Settings {
            ocr_timeout_seconds: 42,
            ..Settings::default()
        };
        let settings = settings_for_processing(Some(stored), 300);
        assert_eq!(ocr_timeout(&settings), Duration::from_secs(42));
    }

    #[test]
    fn server_timeout_is_used_without_saved_settings() {
        let settings = settings_for_processing(None, 450);
        assert_eq!(ocr_timeout(&settings), Duration::from_secs(450));
        // An out-of-range server value cannot wrap into a negative timeout.
        let settings = settings_for_processing(None, u64::MAX);
        assert_eq!(settings.ocr_timeout_seconds, i32::MAX);
    }

    #[test]
    fn timeouts_stop_where_stale_job_recovery_starts() {
        let long = Settings {
            ocr_timeout_seconds: 1800,
            ..Settings::default()
        };
        assert_eq!(ocr_timeout(&long), Duration::from_secs(600));
        assert_eq!(
            ocr_timeout(&settings_for_processing(None, 3600)),
            Duration::from_secs(600)
        );
    }

    #[tokio::test]
    async fn slow_work_times_out_as_an_ocr_timeout() {
        let err = with_ocr_timeout(Duration::from_millis(20), async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            Ok::<_, anyhow::Error>("never")
        })
        .await
        .expect_err("the work outlives its timeout");
        let (reason, _) = super::super::failure_reason::classify_ocr_error(&err.to_string());
        assert_eq!(reason, "ocr_timeout", "{err}");
    }

    #[tokio::test]
    async fn work_finishing_in_time_keeps_its_result() {
        let value = with_ocr_timeout(Duration::from_secs(5), async { Ok(7) })
            .await
            .unwrap();
        assert_eq!(value, 7);
        let err = with_ocr_timeout(Duration::from_secs(5), async {
            Err::<(), _>(anyhow!("tesseract failed"))
        })
        .await
        .unwrap_err();
        assert_eq!(err.to_string(), "tesseract failed");
    }

    #[test]
    fn upload_limit_is_the_users_own_capped_by_the_server() {
        let mb = 1024 * 1024;
        let user = |max_file_size_mb| Settings {
            max_file_size_mb,
            ..Settings::default()
        };
        assert_eq!(max_upload_bytes(50, Some(&user(Some(5)))), 5 * mb);
        assert_eq!(max_upload_bytes(50, Some(&user(Some(500)))), 50 * mb);
        assert_eq!(max_upload_bytes(50, Some(&user(Some(0)))), 50 * mb);
        // No value of their own (or no settings at all): the server's limit.
        assert_eq!(max_upload_bytes(500, Some(&user(None))), 500 * mb);
        assert_eq!(max_upload_bytes(500, None), 500 * mb);
    }
}
