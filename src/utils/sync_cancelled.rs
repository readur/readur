//! The error every sync path returns when its cancellation token fires.
//!
//! A stop request and a real failure both surface as `Err` from the same
//! calls, and the token being set at the time an error is observed does not
//! say which one it was: a download that failed while the user pressed stop
//! is still a failed download. Every cancel path therefore returns a
//! [`SyncCancelled`], and callers ask [`is_cancelled`] instead of the token.

use anyhow::Result;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// The sync was stopped by its cancellation token; `what` names the
/// operation that was interrupted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncCancelled {
    pub what: String,
}

impl SyncCancelled {
    pub fn new(what: impl Into<String>) -> Self {
        Self { what: what.into() }
    }
}

impl std::fmt::Display for SyncCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} cancelled", self.what)
    }
}

impl std::error::Error for SyncCancelled {}

/// The error to return from a cancel check: `cancelled("WebDAV operation")`.
pub fn cancelled(what: impl Into<String>) -> anyhow::Error {
    SyncCancelled::new(what).into()
}

/// Whether `err`, or any error it wraps with context, is a [`SyncCancelled`].
pub fn is_cancelled(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| cause.is::<SyncCancelled>())
}

/// Sleeps for `wait`, unless `token` is cancelled first, in which case the
/// cancellation error for `what` is returned without waiting out the rest.
pub async fn sleep_unless_cancelled(wait: Duration, token: Option<&CancellationToken>, what: &str) -> Result<()> {
    match token {
        Some(token) => tokio::select! {
            _ = token.cancelled() => Err(cancelled(what)),
            _ = tokio::time::sleep(wait) => Ok(()),
        },
        None => {
            tokio::time::sleep(wait).await;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    #[test]
    fn the_marker_is_recognised_through_context_layers() {
        let bare = cancelled("WebDAV operation");
        assert!(is_cancelled(&bare));
        assert_eq!(bare.to_string(), "WebDAV operation cancelled");

        let wrapped = bare.context("Failed to download /a.pdf");
        assert!(is_cancelled(&wrapped));
        let twice: anyhow::Error = wrapped.context("file /a.pdf");
        assert!(is_cancelled(&twice));
    }

    #[test]
    fn other_errors_are_not_cancellations_even_when_they_mention_it() {
        assert!(!is_cancelled(&anyhow!("download refused")));
        assert!(!is_cancelled(&anyhow!("WebDAV operation cancelled")));
        let io: anyhow::Error = std::io::Error::other("disk full").into();
        assert!(!is_cancelled(&io.context("Failed to download /a.pdf")));
    }

    #[tokio::test]
    async fn a_cancelled_token_cuts_the_sleep_short() {
        let token = CancellationToken::new();
        let cancel = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel.cancel();
        });
        let started = std::time::Instant::now();
        let err = sleep_unless_cancelled(Duration::from_secs(60), Some(&token), "S3 download").await.unwrap_err();
        assert!(is_cancelled(&err), "{err}");
        assert_eq!(err.to_string(), "S3 download cancelled");
        assert!(started.elapsed() < Duration::from_secs(1), "slept {:?}", started.elapsed());
    }

    #[tokio::test]
    async fn without_a_token_the_sleep_runs_to_completion() {
        let started = std::time::Instant::now();
        sleep_unless_cancelled(Duration::from_millis(20), None, "S3 download").await.unwrap();
        assert!(started.elapsed() >= Duration::from_millis(20));
    }
}
