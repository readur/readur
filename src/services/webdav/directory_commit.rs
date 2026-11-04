//! Directory rows a WebDAV scan would record, held back until the files the
//! scan found have been processed.
//!
//! Smart sync decides what to rescan by comparing stored directory ETags
//! with the server's. Rows written during discovery would claim a folder is
//! up to date even when the download, ingest or OCR enqueue of one of its
//! files failed, or the run was stopped, and the unfinished files would never
//! be retried. So a scan only collects the rows it would write, and the
//! caller commits them once the files are done, after dropping every row a
//! failure invalidates: a failed file drops its directory and every ancestor
//! up to and including the watch folder's own row; a directory that could not
//! be listed drops itself, its descendants and its ancestors. What is left of
//! the previous state makes those directories look changed or new, so they
//! are listed again next time.

use anyhow::Result;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::models::source::CreateWebDAVDirectory;

#[derive(Debug, Clone, Default)]
pub struct PendingDirectoryCommit {
    pub user_id: Uuid,
    /// The normalised watch folder the rows belong to; pruning is scoped to it.
    pub folder_path: String,
    pub directory_rows: Vec<CreateWebDAVDirectory>,
    /// Whether rows under `folder_path` that are not in `directory_rows` are
    /// deleted on commit. Only a complete scan without failures may prune.
    pub prune_missing: bool,
}

impl PendingDirectoryCommit {
    pub fn new(user_id: Uuid, folder_path: &str) -> Self {
        Self { user_id, folder_path: folder_path.to_string(), directory_rows: Vec::new(), prune_missing: false }
    }

    /// True when committing would change nothing.
    pub fn is_empty(&self) -> bool {
        self.directory_rows.is_empty() && !self.prune_missing
    }

    /// Drops the rows that the given failures invalidate and returns how many
    /// were dropped. `failed_files` are paths of files that were not
    /// processed; `failed_directories` are directories that could not be
    /// listed. Dropping anything also disables pruning, since the scan can no
    /// longer vouch for the folder as a whole.
    pub fn exclude_failures(&mut self, failed_files: &[String], failed_directories: &[String]) -> usize {
        let before = self.directory_rows.len();
        self.directory_rows.retain(|row| {
            let row_path = trim(&row.directory_path);
            let under_failed_file = failed_files.iter().any(|file| is_ancestor_or_self(row_path, parent_dir(file)));
            let touches_failed_directory = failed_directories.iter().any(|dir| {
                let dir = trim(dir);
                is_ancestor_or_self(row_path, dir) || is_ancestor_or_self(dir, row_path)
            });
            !(under_failed_file || touches_failed_directory)
        });
        let dropped = before - self.directory_rows.len();
        if dropped > 0 {
            self.prune_missing = false;
        }
        dropped
    }

    /// Writes the rows (and prunes, when allowed) in one transaction. If that
    /// fails the rows are upserted one by one, so a transient error does not
    /// lose the whole folder's state; the first error is returned.
    pub async fn commit(&self, db: &Database) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        match db.sync_webdav_directories(self.user_id, &self.folder_path, &self.directory_rows, self.prune_missing).await {
            Ok((saved, deleted)) => {
                info!("Recorded {} directories under '{}' ({} stale rows pruned)", saved.len(), self.folder_path, deleted);
                Ok(())
            }
            Err(e) => {
                warn!("Atomic directory sync for '{}' failed, saving rows individually: {}", self.folder_path, e);
                self.save_individually(db).await
            }
        }
    }

    async fn save_individually(&self, db: &Database) -> Result<()> {
        let mut first_error = None;
        let mut saved = 0;
        for row in &self.directory_rows {
            match db.create_or_update_webdav_directory(row).await {
                Ok(_) => saved += 1,
                Err(e) => {
                    warn!("Failed to save directory ETag for {}: {}", row.directory_path, e);
                    first_error.get_or_insert(e);
                }
            }
        }
        info!("Saved ETags for {}/{} directories under '{}' individually", saved, self.directory_rows.len(), self.folder_path);
        first_error.map_or(Ok(()), Err)
    }

    /// Commits every pending folder after a run's files are done, minus the
    /// rows `failed_files` invalidate. A commit that fails is logged: the
    /// folder is simply rescanned next time, which is the safe direction.
    pub async fn commit_all(db: &Database, pending: Vec<PendingDirectoryCommit>, failed_files: &[String]) {
        for mut commit in pending {
            let dropped = commit.exclude_failures(failed_files, &[]);
            if dropped > 0 {
                info!(
                    "Not recording {} directories under '{}' because files in them failed; they are listed again next run",
                    dropped, commit.folder_path
                );
            }
            if let Err(e) = commit.commit(db).await {
                error!("Failed to record directory state for '{}': {}", commit.folder_path, e);
            }
        }
    }
}

fn trim(path: &str) -> &str {
    path.trim_end_matches('/')
}

/// The directory a file path lies in, without trailing slash (`""` for the root).
fn parent_dir(file_path: &str) -> &str {
    let trimmed = trim(file_path);
    match trimmed.rfind('/') {
        Some(index) => &trimmed[..index],
        None => "",
    }
}

/// True when `ancestor` is `path` itself or a directory above it. Both are
/// given without trailing slash; the root (`""`) is above everything.
fn is_ancestor_or_self(ancestor: &str, path: &str) -> bool {
    ancestor == path || path.starts_with(&format!("{ancestor}/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(path: &str) -> CreateWebDAVDirectory {
        CreateWebDAVDirectory {
            user_id: Uuid::nil(),
            directory_path: path.to_string(),
            directory_etag: "e".to_string(),
            file_count: 0,
            total_size_bytes: 0,
        }
    }

    fn commit_with(folder: &str, paths: &[&str]) -> PendingDirectoryCommit {
        let mut commit = PendingDirectoryCommit::new(Uuid::nil(), folder);
        commit.directory_rows = paths.iter().map(|p| row(p)).collect();
        commit.prune_missing = true;
        commit
    }

    fn remaining(commit: &PendingDirectoryCommit) -> Vec<&str> {
        commit.directory_rows.iter().map(|r| r.directory_path.as_str()).collect()
    }

    #[test]
    fn a_failed_file_drops_its_directory_and_ancestors_but_not_siblings() {
        let mut commit = commit_with("/Docs", &["/Docs", "/Docs/A/", "/Docs/A/Deep/", "/Docs/Other/"]);
        assert_eq!(commit.exclude_failures(&["/Docs/A/x.pdf".to_string()], &[]), 2);
        assert_eq!(remaining(&commit), vec!["/Docs/A/Deep/", "/Docs/Other/"]);
        assert!(!commit.prune_missing);
    }

    #[test]
    fn a_failed_directory_drops_itself_descendants_and_ancestors() {
        let mut commit = commit_with("/Docs", &["/Docs", "/Docs/A/", "/Docs/A/B/", "/Docs/A/B/C/", "/Docs/AB/", "/Docs/Other/"]);
        assert_eq!(commit.exclude_failures(&[], &["/Docs/A/B/".to_string()]), 4);
        assert_eq!(remaining(&commit), vec!["/Docs/AB/", "/Docs/Other/"]);
        assert!(!commit.prune_missing);
    }

    #[test]
    fn the_root_watch_folder_row_is_an_ancestor_of_everything() {
        let mut commit = commit_with("/", &["/", "/A/", "/B/"]);
        assert_eq!(commit.exclude_failures(&["/B/x.pdf".to_string()], &[]), 2);
        assert_eq!(remaining(&commit), vec!["/A/"]);
    }

    #[test]
    fn unrelated_failures_keep_every_row_and_pruning() {
        let mut commit = commit_with("/Docs", &["/Docs", "/Docs/A/"]);
        assert_eq!(commit.exclude_failures(&["/Elsewhere/x.pdf".to_string()], &["/Elsewhere/Sub".to_string()]), 0);
        assert_eq!(remaining(&commit), vec!["/Docs", "/Docs/A/"]);
        assert!(commit.prune_missing);
        assert!(!commit.is_empty());
    }

    #[test]
    fn parent_and_ancestor_helpers() {
        assert_eq!(parent_dir("/Docs/A/x.pdf"), "/Docs/A");
        assert_eq!(parent_dir("/x.pdf"), "");
        assert_eq!(parent_dir("x.pdf"), "");
        assert!(is_ancestor_or_self("/Docs", "/Docs"));
        assert!(is_ancestor_or_self("/Docs", "/Docs/A"));
        assert!(!is_ancestor_or_self("/Doc", "/Docs/A"));
        assert!(is_ancestor_or_self("", "/Docs"));
    }
}
