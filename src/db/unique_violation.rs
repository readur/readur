//! Recognising unique-constraint violations.
//!
//! Always decided from the structured error Postgres sends, never from its message text: the
//! server translates messages per `lc_messages`, but the SQLSTATE (23505, `unique_violation`)
//! and the constraint name are fixed. Only the named constraint counts; any other unique
//! violation is a genuine failure and must not be reported to the user as a name clash.

/// Whether `error` is a unique violation of `constraint`.
pub fn violates_unique_constraint(error: &sqlx::Error, constraint: &str) -> bool {
    error
        .as_database_error()
        .is_some_and(|db| db.is_unique_violation() && db.constraint() == Some(constraint))
}

/// [`violates_unique_constraint`] for an `anyhow` error, looking through its whole chain (so a
/// `sqlx::Error` wrapped with context still counts).
pub fn chain_violates_unique_constraint(error: &anyhow::Error, constraint: &str) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<sqlx::Error>())
        .any(|db| violates_unique_constraint(db, constraint))
}

/// The constraint that keeps each user's source names unique: the unnamed `UNIQUE(user_id,
/// name)` in `migrations/20250620100011_add_sources_table.sql`, which Postgres names
/// `<table>_<columns>_key`.
pub const SOURCE_NAME_UNIQUE_CONSTRAINT: &str = "sources_user_id_name_key";

/// Whether a failed source insert/update collided with a name the user already has.
pub fn is_source_name_taken(error: &anyhow::Error) -> bool {
    chain_violates_unique_constraint(error, SOURCE_NAME_UNIQUE_CONSTRAINT)
}

#[cfg(test)]
pub(crate) use fake::fake_db_error;

#[cfg(test)]
mod fake {
    use sqlx::error::{DatabaseError, ErrorKind};
    use std::borrow::Cow;

    /// A database error with a chosen message, SQLSTATE class and constraint, to show
    /// constraint checks read the structured fields and ignore the text.
    #[derive(Debug)]
    struct FakeDbError {
        message: &'static str,
        unique_violation: bool,
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
            self.unique_violation.then_some(Cow::Borrowed("23505"))
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
            if self.unique_violation {
                ErrorKind::UniqueViolation
            } else {
                ErrorKind::Other
            }
        }
    }

    pub(crate) fn fake_db_error(
        message: &'static str,
        unique_violation: bool,
        constraint: Option<&'static str>,
    ) -> sqlx::Error {
        sqlx::Error::Database(Box::new(FakeDbError {
            message,
            unique_violation,
            constraint,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_name_clash_is_recognised_through_anyhow_context() {
        // What a server with lc_messages = 'fr_FR' sends, wrapped the way `db::sources` does.
        let french = fake_db_error(
            "la valeur d'une clé dupliquée rompt la contrainte unique « sources_user_id_name_key »",
            true,
            Some(SOURCE_NAME_UNIQUE_CONSTRAINT),
        );
        let wrapped = anyhow::Error::from(french).context("Failed to create source");
        assert!(is_source_name_taken(&wrapped));
    }

    #[test]
    fn source_name_text_without_the_constraint_is_not_a_clash() {
        // The old text match ("name" and "unique" in the message) said yes to all of these.
        let other_constraint = fake_db_error(
            "duplicate key value violates unique constraint \"sources_pkey\" (name unique)",
            true,
            Some("sources_pkey"),
        );
        assert!(!is_source_name_taken(&other_constraint.into()));

        let not_unique = fake_db_error(
            "null value in column \"name\" violates not-null constraint; unique",
            false,
            Some(SOURCE_NAME_UNIQUE_CONSTRAINT),
        );
        assert!(!is_source_name_taken(&not_unique.into()));

        assert!(!is_source_name_taken(&anyhow::anyhow!(
            "name must be unique"
        )));
    }
}
