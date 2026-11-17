//! Settings read at startup and logged as they are read.
//!
//! Each helper takes the setting's name and the result of reading it (callers
//! pass the `env::var` result themselves, so every variable the server reads is
//! named literally at its read site) and prints where the value came from.

use std::env::VarError;
use std::fmt::Display;
use std::str::FromStr;

/// A string setting, or `default` when unset.
pub(super) fn logged_string(name: &str, value: Result<String, VarError>, default: &str) -> String {
    match value {
        Ok(val) => {
            println!("✅ {}: {} (loaded from env)", name, val);
            val
        }
        Err(_) => {
            println!(
                "⚠️  {}: {} (using default - env var not set)",
                name, default
            );
            default.to_string()
        }
    }
}

/// A numeric setting, or `default` when unset or unparseable.
pub(super) fn logged_number<T>(name: &str, value: Result<String, VarError>, default: T) -> T
where
    T: FromStr + Display,
    T::Err: Display,
{
    match value {
        Ok(val) => match val.parse::<T>() {
            Ok(parsed) => {
                println!("✅ {}: {} (loaded from env)", name, parsed);
                parsed
            }
            Err(e) => {
                println!(
                    "❌ {}: Invalid value '{}' - {}, using default {}",
                    name, val, e, default
                );
                default
            }
        },
        Err(_) => {
            println!(
                "⚠️  {}: {} (using default - env var not set)",
                name, default
            );
            default
        }
    }
}

/// An optional numeric setting: `None` when unset or unparseable.
/// `invalid_fallback` and `unset_note` describe what `None` means.
pub(super) fn logged_optional_number(
    name: &str,
    value: Result<String, VarError>,
    invalid_fallback: &str,
    unset_note: &str,
) -> Option<u64> {
    match value {
        Ok(val) => match val.parse::<u64>() {
            Ok(parsed) => {
                println!("✅ {}: {} (loaded from env)", name, parsed);
                Some(parsed)
            }
            Err(e) => {
                println!(
                    "❌ {}: Invalid value '{}' - {}, {}",
                    name, val, e, invalid_fallback
                );
                None
            }
        },
        Err(_) => {
            println!("⚠️  {}: {}", name, unset_note);
            None
        }
    }
}

/// A switch that is on for true/1/yes/on and off otherwise (and when unset).
pub(super) fn logged_switch(name: &str, value: Result<String, VarError>) -> bool {
    match value {
        Ok(val) => {
            let on = matches!(val.to_lowercase().as_str(), "true" | "1" | "yes" | "on");
            println!("✅ {}: {} (loaded from env)", name, on);
            on
        }
        Err(_) => {
            println!("⚠️  {}: false (using default - env var not set)", name);
            false
        }
    }
}

/// A yes/no setting that stays `None` (the caller's default, described by
/// `default_note`) when unset or unrecognised.
pub(super) fn logged_tristate(
    name: &str,
    value: Result<String, VarError>,
    default_note: &str,
) -> Option<bool> {
    match value {
        Ok(val) => match val.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => {
                println!("✅ {}: true (loaded from env)", name);
                Some(true)
            }
            "false" | "0" | "no" | "off" => {
                println!("✅ {}: false (loaded from env)", name);
                Some(false)
            }
            _ => {
                println!(
                    "⚠️  {}: Invalid value '{}', using default ({})",
                    name, val, default_note
                );
                None
            }
        },
        Err(_) => {
            println!("⚠️  {}: Not set, will use default ({})", name, default_note);
            None
        }
    }
}

/// An optional string setting. A secret is logged by length only.
pub(super) fn logged_optional(
    name: &str,
    value: Result<String, VarError>,
    secret: bool,
) -> Option<String> {
    match value {
        Ok(val) if secret => {
            println!(
                "✅ {}: ***hidden*** (loaded from env, {} chars)",
                name,
                val.len()
            );
            Some(val)
        }
        Ok(val) => {
            println!("✅ {}: {} (loaded from env)", name, val);
            Some(val)
        }
        Err(_) => {
            println!("⚠️  {}: Not set", name);
            None
        }
    }
}
