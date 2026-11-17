//! Startup validation of the loaded configuration, and the summary it logs.

use anyhow::Result;

use super::Config;

impl Config {
    /// Reject unusable settings and log a summary and warnings for the rest.
    pub(super) fn validate_and_report(&self) -> Result<()> {
        println!("\n🔍 CONFIGURATION VALIDATION:");
        println!("{}", "=".repeat(50));

        // Validate server address format
        if !self.server_address.contains(':') {
            println!(
                "❌ SERVER_ADDRESS: Invalid format '{}' - missing port",
                self.server_address
            );
            return Err(anyhow::anyhow!(
                "Invalid server address format: '{}'. Expected format: 'host:port' (e.g., '0.0.0.0:8000')", 
                self.server_address
            ));
        }

        // Validate database URL format
        if !self.database_url.starts_with("postgresql://")
            && !self.database_url.starts_with("postgres://")
        {
            println!("❌ DATABASE_URL: Invalid format - must start with 'postgresql://' or 'postgres://'");
            return Err(anyhow::anyhow!(
                "Invalid database URL format. Must start with 'postgresql://' or 'postgres://'"
            ));
        }

        // Validate configuration to prevent recursion issues
        println!("🔍 Validating directory paths for conflicts...");
        self.validate_paths()?;

        println!("\n📊 CONFIGURATION SUMMARY:");
        println!("{}", "=".repeat(50));
        println!("🌐 Server will bind to: {}", self.server_address);
        println!("📁 Upload directory: {}", self.upload_path);
        println!("👁️  Watch directory: {}", self.watch_folder);
        println!("👥 Per-user watch enabled: {}", self.enable_per_user_watch);
        if self.enable_per_user_watch {
            println!("📂 User watch base directory: {}", self.user_watch_base_dir);
        }
        println!("📄 Allowed file types: {:?}", self.allowed_file_types);
        println!("🧠 OCR language: {}", self.ocr_language);
        println!("⚙️  Concurrent OCR jobs: {}", self.concurrent_ocr_jobs);
        println!("⏱️  OCR timeout: {}s", self.ocr_timeout_seconds);
        println!("📏 Max file size: {}MB", self.max_file_size_mb);
        println!("📄 Max PDF size: {}MB", self.max_pdf_size_mb);
        println!(
            "📑 Max Office document size: {}MB",
            self.max_office_document_size_mb
        );
        println!("💾 Memory limit: {}MB", self.memory_limit_mb);

        // Warning checks
        println!("\n⚠️  CONFIGURATION WARNINGS:");
        println!("{}", "=".repeat(50));
        if self.server_address.starts_with("0.0.0.0") {
            println!("🌍 INFO: Server will listen on all interfaces (0.0.0.0)");
        }
        if self.max_file_size_mb > 100 {
            println!(
                "📏 INFO: Large file size limit ({}MB) may impact performance",
                self.max_file_size_mb
            );
        }
        if self.concurrent_ocr_jobs > 8 {
            println!(
                "⚙️  INFO: High OCR concurrency ({}) may use significant CPU/memory",
                self.concurrent_ocr_jobs
            );
        }

        // OIDC validation
        if self.oidc_enabled {
            println!("🔐 OIDC is enabled");
            println!(
                "🔓 OIDC auto-registration: {}",
                self.oidc_auto_register.unwrap_or(false)
            );
            if self.oidc_client_id.is_none() {
                println!("❌ OIDC_CLIENT_ID is required when OIDC is enabled");
            }
            if self.oidc_client_secret.is_none() {
                println!("ℹ️  OIDC_CLIENT_SECRET not set: using public-client (PKCE-only) flow");
            }
            if self.oidc_issuer_url.is_none() {
                println!("❌ OIDC_ISSUER_URL is required when OIDC is enabled");
            }
            if self.oidc_redirect_uri.is_none() {
                println!("❌ OIDC_REDIRECT_URI is required when OIDC is enabled");
            }
        } else {
            println!("🔐 OIDC is disabled");
        }

        // Authentication method validation
        let allow_local_auth = self.allow_local_auth.unwrap_or(true);
        println!(
            "🔑 Local authentication (username/password): {}",
            if allow_local_auth {
                "enabled"
            } else {
                "disabled"
            }
        );

        if !self.oidc_enabled && !allow_local_auth {
            println!("❌ WARNING: Both OIDC and local authentication are disabled!");
            println!(
                "   You will not be able to log in. Enable at least one authentication method."
            );
            return Err(anyhow::anyhow!(
                "Invalid authentication configuration: Both OIDC and local auth are disabled. \
                Enable at least one authentication method (OIDC_ENABLED=true or ALLOW_LOCAL_AUTH=true)"
            ));
        }

        println!("✅ Configuration validation completed successfully!\n");
        Ok(())
    }

    fn validate_paths(&self) -> Result<()> {
        use std::path::Path;

        let upload_path = Path::new(&self.upload_path);
        let watch_path = Path::new(&self.watch_folder);
        let user_watch_path = Path::new(&self.user_watch_base_dir);

        println!("📁 Checking upload directory: {}", self.upload_path);
        println!("👁️  Checking watch directory: {}", self.watch_folder);
        if self.enable_per_user_watch {
            println!(
                "👥 Checking user watch base directory: {}",
                self.user_watch_base_dir
            );
        }

        // Check if paths exist and are accessible
        if !upload_path.exists() {
            println!(
                "⚠️  Upload directory does not exist yet: {}",
                self.upload_path
            );
        } else if !upload_path.is_dir() {
            println!(
                "❌ Upload path exists but is not a directory: {}",
                self.upload_path
            );
            return Err(anyhow::anyhow!(
                "Upload path '{}' exists but is not a directory",
                self.upload_path
            ));
        } else {
            println!("✅ Upload directory exists and is accessible");
        }

        if !watch_path.exists() {
            println!(
                "⚠️  Watch directory does not exist yet: {}",
                self.watch_folder
            );
        } else if !watch_path.is_dir() {
            println!(
                "❌ Watch path exists but is not a directory: {}",
                self.watch_folder
            );
            return Err(anyhow::anyhow!(
                "Watch folder '{}' exists but is not a directory",
                self.watch_folder
            ));
        } else {
            println!("✅ Watch directory exists and is accessible");
        }

        if self.enable_per_user_watch {
            if !user_watch_path.exists() {
                println!(
                    "⚠️  User watch base directory does not exist yet: {}",
                    self.user_watch_base_dir
                );
            } else if !user_watch_path.is_dir() {
                println!(
                    "❌ User watch base path exists but is not a directory: {}",
                    self.user_watch_base_dir
                );
                return Err(anyhow::anyhow!(
                    "User watch base directory '{}' exists but is not a directory",
                    self.user_watch_base_dir
                ));
            } else {
                println!("✅ User watch base directory exists and is accessible");
            }
        }

        // Normalize paths to handle relative paths and symlinks
        let upload_canonical = upload_path.canonicalize().unwrap_or_else(|_| {
            println!("⚠️  Could not canonicalize upload path, using as-is");
            upload_path.to_path_buf()
        });
        let watch_canonical = watch_path.canonicalize().unwrap_or_else(|_| {
            println!("⚠️  Could not canonicalize watch path, using as-is");
            watch_path.to_path_buf()
        });
        let user_watch_canonical = if self.enable_per_user_watch {
            Some(user_watch_path.canonicalize().unwrap_or_else(|_| {
                println!("⚠️  Could not canonicalize user watch path, using as-is");
                user_watch_path.to_path_buf()
            }))
        } else {
            None
        };

        println!("📍 Canonical upload path: {}", upload_canonical.display());
        println!("📍 Canonical watch path: {}", watch_canonical.display());
        if let Some(ref user_watch) = user_watch_canonical {
            println!("📍 Canonical user watch path: {}", user_watch.display());
        }

        // Check if paths are the same
        if upload_canonical == watch_canonical {
            println!("❌ CRITICAL ERROR: Upload and watch directories are the same!");
            return Err(anyhow::anyhow!(
                "❌ Configuration Error: UPLOAD_PATH and WATCH_FOLDER cannot be the same directory.\n\
                 This would cause infinite recursion where WebDAV files are downloaded to the upload \n\
                 directory and then immediately reprocessed by the watcher.\n\
                 Current config:\n\
                 - UPLOAD_PATH: {}\n\
                 - WATCH_FOLDER: {}\n\
                 Please set them to different directories.",
                self.upload_path, self.watch_folder
            ));
        }

        // Check if watch folder is inside upload folder
        if watch_canonical.starts_with(&upload_canonical) {
            println!("❌ CRITICAL ERROR: Watch folder is inside upload directory!");
            return Err(anyhow::anyhow!(
                "❌ Configuration Error: WATCH_FOLDER cannot be inside UPLOAD_PATH.\n\
                 This would cause recursion where WebDAV files downloaded to uploads are \n\
                 detected by the watcher as new files.\n\
                 Current config:\n\
                 - UPLOAD_PATH: {}\n\
                 - WATCH_FOLDER: {}\n\
                 Please move the watch folder outside the upload directory.",
                self.upload_path,
                self.watch_folder
            ));
        }

        // Check if upload folder is inside watch folder
        if upload_canonical.starts_with(&watch_canonical) {
            println!("❌ CRITICAL ERROR: Upload directory is inside watch folder!");
            return Err(anyhow::anyhow!(
                "❌ Configuration Error: UPLOAD_PATH cannot be inside WATCH_FOLDER.\n\
                 This would cause recursion where files from the watch folder are \n\
                 copied to uploads (inside the watch folder) and reprocessed.\n\
                 Current config:\n\
                 - UPLOAD_PATH: {}\n\
                 - WATCH_FOLDER: {}\n\
                 Please move the upload directory outside the watch folder.",
                self.upload_path,
                self.watch_folder
            ));
        }

        // Additional validation for user watch directory if enabled
        if let Some(ref user_watch) = user_watch_canonical {
            // Check if user watch is same as upload or watch
            if user_watch == &upload_canonical {
                println!(
                    "❌ CRITICAL ERROR: User watch base directory is same as upload directory!"
                );
                return Err(anyhow::anyhow!(
                    "❌ Configuration Error: USER_WATCH_BASE_DIR cannot be the same as UPLOAD_PATH.\n\
                     Current config:\n\
                     - UPLOAD_PATH: {}\n\
                     - USER_WATCH_BASE_DIR: {}\n\
                     Please set them to different directories.",
                    self.upload_path, self.user_watch_base_dir
                ));
            }

            if user_watch == &watch_canonical {
                println!("❌ CRITICAL ERROR: User watch base directory is same as global watch directory!");
                return Err(anyhow::anyhow!(
                    "❌ Configuration Error: USER_WATCH_BASE_DIR cannot be the same as WATCH_FOLDER.\n\
                     Current config:\n\
                     - WATCH_FOLDER: {}\n\
                     - USER_WATCH_BASE_DIR: {}\n\
                     Please set them to different directories.",
                    self.watch_folder, self.user_watch_base_dir
                ));
            }

            // Check if user watch is inside upload or vice versa
            if user_watch.starts_with(&upload_canonical) {
                println!(
                    "❌ CRITICAL ERROR: User watch base directory is inside upload directory!"
                );
                return Err(anyhow::anyhow!(
                    "❌ Configuration Error: USER_WATCH_BASE_DIR cannot be inside UPLOAD_PATH.\n\
                     This would cause recursion issues.\n\
                     Current config:\n\
                     - UPLOAD_PATH: {}\n\
                     - USER_WATCH_BASE_DIR: {}\n\
                     Please move the user watch directory outside the upload directory.",
                    self.upload_path,
                    self.user_watch_base_dir
                ));
            }

            if upload_canonical.starts_with(user_watch) {
                println!(
                    "❌ CRITICAL ERROR: Upload directory is inside user watch base directory!"
                );
                return Err(anyhow::anyhow!(
                    "❌ Configuration Error: UPLOAD_PATH cannot be inside USER_WATCH_BASE_DIR.\n\
                     This would cause recursion issues.\n\
                     Current config:\n\
                     - UPLOAD_PATH: {}\n\
                     - USER_WATCH_BASE_DIR: {}\n\
                     Please move the upload directory outside the user watch directory.",
                    self.upload_path,
                    self.user_watch_base_dir
                ));
            }
        }

        println!("✅ Directory path validation passed - no conflicts detected");
        Ok(())
    }
}
