// Documents database operations organized into focused modules

mod helpers;
mod crud;
mod search;
mod management;
mod operations;

// Re-export helper functions for use by other modules if needed
pub use helpers::*;