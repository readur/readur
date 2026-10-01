//! Unit tests for `crate::config`, kept in a sibling file so the module
//! itself stays readable.

mod s3_env_tests {
    use crate::config::*;

    #[test]
    fn s3_enabled_by_s3_enabled_var() {
        assert!(s3_storage_enabled(Some("true"), None));
        assert!(s3_storage_enabled(Some("TRUE"), None));
        assert!(!s3_storage_enabled(Some("false"), None));
        assert!(!s3_storage_enabled(None, None));
    }

    #[test]
    fn s3_enabled_by_storage_backend_alias() {
        assert!(s3_storage_enabled(None, Some("s3")));
        assert!(s3_storage_enabled(None, Some("S3")));
        assert!(!s3_storage_enabled(None, Some("local")));
    }

    #[test]
    fn force_path_style_parsing() {
        assert_eq!(parse_force_path_style(Some("true"), None), Some(true));
        assert_eq!(parse_force_path_style(Some("false"), None), Some(false));
        // legacy documented alias S3_PATH_STYLE
        assert_eq!(parse_force_path_style(None, Some("true")), Some(true));
        // primary wins over legacy
        assert_eq!(parse_force_path_style(Some("false"), Some("true")), Some(false));
        // unset -> default for the endpoint type
        assert_eq!(parse_force_path_style(None, None), None);
    }
}

mod jwt_secret_tests {
    use crate::config::parsing::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn rejects_empty_short_placeholder_and_known_values() {
        assert!(validate_jwt_secret("").is_err());
        assert!(validate_jwt_secret("short").is_err());
        assert!(validate_jwt_secret("<output of: openssl rand -hex 32>").is_err());
        for weak in KNOWN_WEAK_JWT_SECRETS {
            assert!(validate_jwt_secret(weak).is_err(), "{weak} should be rejected");
        }
        assert!(validate_jwt_secret("0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a6978").is_ok());
    }

    #[test]
    fn weak_configured_secret_falls_back_to_stored_key() {
        // Shipped by the v2.9.x docker-compose.yml; upgrades must keep starting.
        assert_eq!(resolve_jwt_secret("your-secret-key-change-this-in-production", false), "");
        for weak in KNOWN_WEAK_JWT_SECRETS {
            assert_eq!(resolve_jwt_secret(weak, false), "", "{weak} should be ignored");
        }
        assert_eq!(resolve_jwt_secret("my-own-secret-123", false), "");
        assert_eq!(resolve_jwt_secret("<output of: openssl rand -hex 32>", false), "");
        assert_eq!(resolve_jwt_secret("   ", false), "");
    }

    #[test]
    fn strong_secret_is_used_and_dev_mode_keeps_weak_one() {
        let strong = "0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a6978";
        assert_eq!(resolve_jwt_secret(strong, false), strong);
        assert_eq!(resolve_jwt_secret("short", true), "short");
    }

    /// Extract the literal value assigned to a `JWT_SECRET`-like key on a
    /// line, if any. Returns `None` for lines without an assignment and for
    /// values computed at runtime (`$VAR`, `$(cmd)`, `${VAR}`, templates).
    /// For `${VAR:-default}` the default is returned.
    fn literal_jwt_secret(line: &str) -> Option<String> {
        let idx = line.find("JWT_SECRET")?;
        let rest = line[idx + "JWT_SECRET".len()..].trim_start();
        let rest = rest.strip_prefix('=').or_else(|| rest.strip_prefix(':'))?;
        let mut value = rest.trim();
        if let Some(pos) = value.find(" #") {
            value = value[..pos].trim();
        }
        let value = value.trim_matches(|c| c == '"' || c == '\'').trim();
        if value.is_empty() {
            return None;
        }
        if let Some(inner) = value.strip_prefix("${").and_then(|v| v.strip_suffix('}')) {
            let (_, default) = inner.split_once(":-")?;
            return Some(default.trim_matches(|c| c == '"' || c == '\'').to_string());
        }
        if value.starts_with('$') || value.starts_with("{{") {
            return None;
        }
        Some(value.to_string())
    }

    fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_entry(|e| e.file_name() != "node_modules")
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                out.push(entry.into_path());
            }
        }
    }

    #[test]
    fn literal_extraction() {
        assert_eq!(literal_jwt_secret("JWT_SECRET=abc"), Some("abc".into()));
        assert_eq!(literal_jwt_secret("  JWT_SECRET: \"abc\"  # note"), Some("abc".into()));
        assert_eq!(literal_jwt_secret("JWT_SECRET: ${JWT_SECRET:-dflt}"), Some("dflt".into()));
        assert_eq!(literal_jwt_secret("JWT_SECRET: ${JWT_SECRET:?required}"), None);
        assert_eq!(literal_jwt_secret("JWT_SECRET: ${{ env.CI_JWT_SECRET }}"), None);
        assert_eq!(literal_jwt_secret("JWT_SECRET=$(openssl rand -hex 32)"), None);
        assert_eq!(literal_jwt_secret("JWT_SECRET: {{ .Values.x | b64enc }}"), None);
        assert_eq!(literal_jwt_secret("`JWT_SECRET` is required"), None);
        assert_eq!(literal_jwt_secret("JWT_SECRET="), None);
    }

    /// Every JWT secret value committed to the repository (compose files,
    /// env files, docs, CI workflows, Helm chart) must be refused at startup.
    #[test]
    fn committed_jwt_secrets_are_rejected() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        for entry in std::fs::read_dir(root).expect("read repository root").flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_compose = name.starts_with("docker-compose")
                && (name.ends_with(".yml") || name.ends_with(".yaml"));
            let wanted = is_compose || name.starts_with(".env") || name.ends_with(".md");
            if wanted && entry.path().is_file() {
                files.push(entry.path());
            }
        }
        for dir in ["docs", ".github", "charts"] {
            collect_files(&root.join(dir), &mut files);
        }

        let mut checked = 0;
        let mut accepted = Vec::new();
        for file in &files {
            let Ok(content) = std::fs::read_to_string(file) else { continue };
            for (n, line) in content.lines().enumerate() {
                if let Some(value) = literal_jwt_secret(line) {
                    checked += 1;
                    if validate_jwt_secret(&value).is_ok() {
                        accepted.push(format!("{}:{}: {}", file.display(), n + 1, value));
                    }
                }
            }
        }
        assert!(checked > 0, "expected to find JWT_SECRET examples in the repository");
        assert!(
            accepted.is_empty(),
            "committed JWT_SECRET values must be listed in KNOWN_WEAK_JWT_SECRETS:\n{}",
            accepted.join("\n")
        );
    }
}

mod security_setting_tests {
    use crate::config::parsing::*;

    #[test]
    fn flag_values() {
        for v in ["true", "TRUE", " 1 ", "yes", "on"] {
            assert_eq!(parse_flag(v), Some(true), "{v}");
        }
        for v in ["false", "0", "No", "off"] {
            assert_eq!(parse_flag(v), Some(false), "{v}");
        }
        for v in ["ture", "enabled", "2", ""] {
            assert_eq!(parse_flag(v), None, "{v}");
        }
    }

    #[test]
    fn cors_origins_are_normalized() {
        assert_eq!(normalize_cors_origin("https://app.example.com/").unwrap(), "https://app.example.com");
        assert_eq!(normalize_cors_origin(" HTTPS://App.Example.com ").unwrap(), "https://app.example.com");
        assert_eq!(normalize_cors_origin("http://localhost:5173").unwrap(), "http://localhost:5173");
        assert_eq!(normalize_cors_origin("https://app.example.com:443").unwrap(), "https://app.example.com");
    }

    #[test]
    fn invalid_cors_origins_are_rejected() {
        for bad in [
            "*",
            "app.example.com",
            "ftp://app.example.com",
            "https://app.example.com/path",
            "https://user@app.example.com",
            "https://app.example.com?x=1",
            "null",
        ] {
            assert!(normalize_cors_origin(bad).is_err(), "{bad}");
        }
    }
}

mod watch_settings_tests {
    use crate::config::*;

    #[test]
    fn effective_watch_settings_fall_back_to_the_watcher_defaults() {
        let mut config = crate::test_utils::TestConfigBuilder::default().build(String::new());
        config.watch_interval_seconds = None;
        config.file_stability_check_ms = None;
        assert_eq!(
            config.effective_watch_interval_seconds(),
            DEFAULT_WATCH_INTERVAL_SECONDS
        );
        assert_eq!(
            config.effective_file_stability_check_ms(),
            DEFAULT_FILE_STABILITY_CHECK_MS
        );

        config.watch_interval_seconds = Some(5);
        config.file_stability_check_ms = Some(250);
        assert_eq!(config.effective_watch_interval_seconds(), 5);
        assert_eq!(config.effective_file_stability_check_ms(), 250);
    }
}
