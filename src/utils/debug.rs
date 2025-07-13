use std::env;
use tracing::{info, warn, error};

/// Check if DEBUG environment variable is set to enable verbose debug output
pub fn is_debug_enabled() -> bool {
    env::var("DEBUG")
        .map(|val| !val.is_empty() && val != "0" && val.to_lowercase() != "false")
        .unwrap_or(false)
}

/// Log debug message only if DEBUG environment variable is set
pub fn debug_log(message: &str) {
    if is_debug_enabled() {
        info!("🐛 DEBUG: {}", message);
    }
}

/// Log debug message with context only if DEBUG environment variable is set
pub fn debug_log_context(context: &str, message: &str) {
    if is_debug_enabled() {
        info!("🐛 DEBUG [{}]: {}", context, message);
    }
}

/// Log debug message with structured data only if DEBUG environment variable is set
pub fn debug_log_structured(context: &str, key_values: &[(&str, &dyn std::fmt::Display)]) {
    if is_debug_enabled() {
        let mut formatted = String::new();
        for (i, (key, value)) in key_values.iter().enumerate() {
            if i > 0 {
                formatted.push_str(", ");
            }
            formatted.push_str(&format!("{}={}", key, value));
        }
        info!("🐛 DEBUG [{}]: {}", context, formatted);
    }
}

/// Log error with debug context
pub fn debug_error(context: &str, error: &dyn std::fmt::Display) {
    if is_debug_enabled() {
        error!("🐛 DEBUG ERROR [{}]: {}", context, error);
    } else {
        error!("[{}]: {}", context, error);
    }
}

/// Log warning with debug context
pub fn debug_warn(context: &str, message: &str) {
    if is_debug_enabled() {
        warn!("🐛 DEBUG WARN [{}]: {}", context, message);
    } else {
        warn!("[{}]: {}", context, message);
    }
}

/// Macro for easier debug logging with automatic context
#[macro_export]
macro_rules! debug_log {
    ($msg:expr) => {
        crate::utils::debug::debug_log($msg)
    };
    // Structured logging pattern (must come before format pattern due to => token)
    ($context:expr, $($key:expr => $value:expr),+ $(,)?) => {
        crate::utils::debug::debug_log_structured($context, &[$(($key, &$value)),+])
    };
    // Format pattern with arguments
    ($context:expr, $msg:expr, $($args:expr),+ $(,)?) => {
        crate::utils::debug::debug_log_context($context, &format!($msg, $($args),+))
    };
    // Simple context + message pattern
    ($context:expr, $msg:expr) => {
        crate::utils::debug::debug_log_context($context, $msg)
    };
}

/// Macro for debug error logging
#[macro_export]
macro_rules! debug_error {
    ($context:expr, $error:expr) => {
        crate::utils::debug::debug_error($context, &$error)
    };
}

/// Macro for debug warning logging
#[macro_export]
macro_rules! debug_warn {
    ($context:expr, $msg:expr) => {
        crate::utils::debug::debug_warn($context, $msg)
    };
}