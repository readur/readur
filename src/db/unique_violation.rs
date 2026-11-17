//! Recognising unique-constraint violations by their structured fields.
//!
//! A clash is identified by SQLSTATE 23505 (`unique_violation`) plus the name of the constraint
//! or unique index that rejected the row, never by the error text: the text changes with the
//! server's `lc_messages`, and a substring test on it can match an unrelated error.

/// The unique constraint on `users.username` (the column's inline `UNIQUE` in the initial schema).
pub const USERS_USERNAME_UNIQUE: &str = "users_username_key";

/// The unique constraint on `users.email` (the column's inline `UNIQUE` in the initial schema).
pub const USERS_EMAIL_UNIQUE: &str = "users_email_key";

/// The unique index on `documents (user_id, file_hash)`: one document per content per user.
pub const DOCUMENTS_USER_FILE_HASH_UNIQUE: &str = "idx_documents_user_file_hash";

/// The constraint (or unique index) a unique violation was raised on. `None` for any other error,
/// including a unique violation whose constraint the server did not name.
pub fn violated_unique_constraint(error: &sqlx::Error) -> Option<&str> {
    let db = error.as_database_error()?;
    if db.is_unique_violation() {
        db.constraint()
    } else {
        None
    }
}

/// [`violated_unique_constraint`] for an `anyhow` error, looking through its whole source chain
/// for the underlying [`sqlx::Error`], so context added on the way up does not hide it.
pub fn violated_unique_constraint_in(error: &anyhow::Error) -> Option<&str> {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<sqlx::Error>())
        .and_then(violated_unique_constraint)
}

/// Builds `sqlx::Error`s with chosen fields, so tests can show classification reads the
/// structured fields and ignores the message.
#[cfg(test)]
pub(crate) mod fake {
    use sqlx::error::{DatabaseError, ErrorKind};
    use std::borrow::Cow;

    #[derive(Debug)]
    struct FakeDbError {
        message: &'static str,
        code: Option<&'static str>,
        constraint: Option<&'static str>,
    }

    impl std::fmt::Display for FakeDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.message)
        }
    }

    impl std::error::Error for FakeDbError {}

    impl DatabaseError for FakeDbError {
        fn message(&self) -> &str {
            self.message
        }
        fn code(&self) -> Option<Cow<'_, str>> {
            self.code.map(Cow::Borrowed)
        }
        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }
        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }
        fn constraint(&self) -> Option<&str> {
            self.constraint
        }
        fn kind(&self) -> ErrorKind {
            match self.code {
                Some("23505") => ErrorKind::UniqueViolation,
                Some("23503") => ErrorKind::ForeignKeyViolation,
                _ => ErrorKind::Other,
            }
        }
    }

    /// A database error with the given message, SQLSTATE and constraint name.
    pub(crate) fn db_error(
        message: &'static str,
        code: Option<&'static str>,
        constraint: Option<&'static str>,
    ) -> sqlx::Error {
        sqlx::Error::Database(Box::new(FakeDbError {
            message,
            code,
            constraint,
        }))
    }

    /// A unique violation (SQLSTATE 23505) on `constraint`.
    pub(crate) fn unique_violation(message: &'static str, constraint: &'static str) -> sqlx::Error {
        db_error(message, Some("23505"), Some(constraint))
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{db_error, unique_violation};
    use super::*;

    #[test]
    fn names_the_constraint_of_a_unique_violation_whatever_the_language() {
        let german = unique_violation(
            "doppelter Schlüsselwert verletzt Unique-Constraint »users_email_key«",
            USERS_EMAIL_UNIQUE,
        );
        assert_eq!(
            violated_unique_constraint(&german),
            Some(USERS_EMAIL_UNIQUE)
        );
    }

    #[test]
    fn other_sqlstates_are_not_unique_violations_even_with_matching_text() {
        let fk = db_error(
            "duplicate key value violates unique constraint \"users_username_key\"",
            Some("23503"),
            Some(USERS_USERNAME_UNIQUE),
        );
        assert_eq!(violated_unique_constraint(&fk), None);
    }

    #[test]
    fn unnamed_constraint_and_non_database_errors_yield_none() {
        let unnamed = db_error("duplicate key", Some("23505"), None);
        assert_eq!(violated_unique_constraint(&unnamed), None);
        assert_eq!(violated_unique_constraint(&sqlx::Error::RowNotFound), None);
    }

    #[test]
    fn anyhow_wrapping_and_context_do_not_hide_the_violation() {
        let wrapped = anyhow::Error::from(unique_violation("x", DOCUMENTS_USER_FILE_HASH_UNIQUE))
            .context("creating document")
            .context("ingesting upload");
        assert_eq!(
            violated_unique_constraint_in(&wrapped),
            Some(DOCUMENTS_USER_FILE_HASH_UNIQUE)
        );
        let plain = anyhow::anyhow!(
            "duplicate key value violates unique constraint \"idx_documents_user_file_hash\""
        );
        assert_eq!(violated_unique_constraint_in(&plain), None);
    }
}
