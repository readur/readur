// Documents database operations organized into focused modules

mod crud;
mod helpers;
mod management;
mod operations;
mod search;
mod search_query;
mod snippets;
mod trigram;

// Re-export helper functions for use by other modules if needed
pub use helpers::*;
pub use search_query::boolean_to_tsquery;
pub mod sync_identity;
