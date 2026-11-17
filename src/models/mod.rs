// Re-export all model types for backward compatibility and ease of use

pub mod api_key;
pub mod arrivals;
pub mod comment;
pub mod document;
pub mod label;
pub mod ocr;
pub mod queue;
pub mod responses;
pub mod search;
pub mod settings;
pub mod shared_link;
pub mod source;
pub mod source_error;
pub mod user;

// Re-export commonly used types
pub use arrivals::*;
pub use document::*;
pub use ocr::*;
pub use queue::*;
pub use search::*;
pub use settings::*;
pub use source::*;
pub use source_error::*;
pub use user::*;

pub use responses::*;
