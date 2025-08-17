use anyhow::Result;
use std::env;

use crate::models::S3SourceConfig;
use crate::cpu_allocation::CpuAllocation;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub server_address: String,
    pub jwt_secret: String,
    pub upload_path: String,
    pub watch_folder: String,
    pub user_watch_base_dir: String,
    pub enable_per_user_watch: bool,
    pub allowed_file_types: Vec<String>,
    pub watch_interval_seconds: Option<u64>,
    pub file_stability_check_ms: Option<u64>,
    pub max_file_age_hours: Option<u64>,
    
    // OCR Configuration
    pub ocr_language: String,
    pub concurrent_ocr_jobs: usize,
    pub ocr_timeout_seconds: u64,
    pub max_file_size_mb: u64,
    
    // Performance
    pub memory_limit_mb: usize,
    pub cpu_priority: String,
    
    // OIDC Configuration
    pub oidc_enabled: bool,
    pub oidc_client_id: Option<String>,
    pub oidc_client_secret: Option<String>,
    pub oidc_issuer_url: Option<String>,
    pub oidc_redirect_uri: Option<String>,
    
    // S3 Configuration
    pub s3_enabled: bool,
    pub s3_config: Option<S3SourceConfig>,
    
    // CPU Core Allocation
    pub cpu_allocation: CpuAllocation,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load .env file if present
        match dotenvy::dotenv() {
            Ok(path) => println!("🔧 Loaded environment variables from: {}", path.display()),
            Err(_) => println!("🔧 No .env file found, using system environment variables"),
        }
        
        // Log all environment variable loading with detailed information
        println!("\n📋 CONFIGURATION LOADING:");
        println!("{}", "=".repeat(50));
        
        // Database Configuration
        let database_url = match env::var("DATABASE_URL") {
            Ok(val) => {
                println!("✅ DATABASE_URL: configured (loaded from env)");
                val
            }
            Err(_) => {
                let default_url = "postgresql://readur:readur@localhost/readur".to_string();
                println!("⚠️  DATABASE_URL: {} (using default - env var not set)", 
                         "postgresql://readur:***@localhost/readur");
                default_url
            }
        };
        
        let config = Config {
            database_url,
            server_address: {
                // Support both SERVER_ADDRESS (full address) and SERVER_PORT (just port)
                match env::var("SERVER_ADDRESS") {
                    Ok(addr) => {
                        println!("✅ SERVER_ADDRESS: {} (loaded from env)", addr);
                        addr
                    }
                    Err(_) => {
                        let host = match env::var("SERVER_HOST") {
                            Ok(h) => {
                                println!("✅ SERVER_HOST: {} (loaded from env)", h);
                                h
                            }
                            Err(_) => {
                                let default_host = "0.0.0.0".to_string();
                                println!("⚠️  SERVER_HOST: {} (using default - env var not set)", default_host);
                                default_host
                            }
                        };
                        
                        let port = match env::var("SERVER_PORT") {
                            Ok(p) => {
                                println!("✅ SERVER_PORT: {} (loaded from env)", p);
                                p
                            }
                            Err(_) => {
                                let default_port = "8000".to_string();
                                println!("⚠️  SERVER_PORT: {} (using default - env var not set)", default_port);
                                default_port
                            }
                        };
                        
                        let combined_address = format!("{}:{}", host, port);
                        println!("🔗 Combined server_address: {}", combined_address);
                        combined_address
                    }
                }
            },
            jwt_secret: match env::var("JWT_SECRET") {
                Ok(secret) => {
                    if secret == "your-secret-key" {
                        println!("⚠️  JWT_SECRET: Using default value (SECURITY RISK in production!)");
                    } else {
                        println!("✅ JWT_SECRET: ***hidden*** (loaded from env, {} chars)", secret.len());
                    }
                    secret
                }
                Err(_) => {
                    let default_secret = "your-secret-key".to_string();
                    println!("⚠️  JWT_SECRET: Using default value (SECURITY RISK - env var not set!)");
                    default_secret
                }
            },
            upload_path: match env::var("UPLOAD_PATH") {
                Ok(path) => {
                    println!("✅ UPLOAD_PATH: {} (loaded from env)", path);
                    path
                }
                Err(_) => {
                    let default_path = "./uploads".to_string();
                    println!("⚠️  UPLOAD_PATH: {} (using default - env var not set)", default_path);
                    default_path
                }
            },
            watch_folder: match env::var("WATCH_FOLDER") {
                Ok(folder) => {
                    println!("✅ WATCH_FOLDER: {} (loaded from env)", folder);
                    folder
                }
                Err(_) => {
                    let default_folder = "./watch".to_string();
                    println!("⚠️  WATCH_FOLDER: {} (using default - env var not set)", default_folder);
                    default_folder
                }
            },
            user_watch_base_dir: match env::var("USER_WATCH_BASE_DIR") {
                Ok(dir) => {
                    println!("✅ USER_WATCH_BASE_DIR: {} (loaded from env)", dir);
                    dir
                }
                Err(_) => {
                    let default_dir = "./user_watch".to_string();
                    println!("⚠️  USER_WATCH_BASE_DIR: {} (using default - env var not set)", default_dir);
                    default_dir
                }
            },
            enable_per_user_watch: match env::var("ENABLE_PER_USER_WATCH") {
                Ok(val) => match val.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => {
                        println!("✅ ENABLE_PER_USER_WATCH: true (loaded from env)");
                        true
                    }
                    _ => {
                        println!("✅ ENABLE_PER_USER_WATCH: false (loaded from env)");
                        false
                    }
                },
                Err(_) => {
                    println!("⚠️  ENABLE_PER_USER_WATCH: false (using default - env var not set)");
                    false
                }
            },
            allowed_file_types: {
                let file_types_str = match env::var("ALLOWED_FILE_TYPES") {
                    Ok(types) => {
                        println!("✅ ALLOWED_FILE_TYPES: {} (loaded from env)", types);
                        types
                    }
                    Err(_) => {
                        let default_types = "pdf,txt,doc,docx,png,jpg,jpeg".to_string();
                        println!("⚠️  ALLOWED_FILE_TYPES: {} (using default - env var not set)", default_types);
                        default_types
                    }
                };
                
                let types_vec: Vec<String> = file_types_str
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .collect();
                    
                println!("📄 Parsed file types: {:?}", types_vec);
                types_vec
            },
            // Watcher Configuration
            watch_interval_seconds: {
                match env::var("WATCH_INTERVAL_SECONDS") {
                    Ok(val) => match val.parse::<u64>() {
                        Ok(parsed) => {
                            println!("✅ WATCH_INTERVAL_SECONDS: {} (loaded from env)", parsed);
                            Some(parsed)
                        }
                        Err(e) => {
                            println!("❌ WATCH_INTERVAL_SECONDS: Invalid value '{}' - {}, using default", val, e);
                            None
                        }
                    },
                    Err(_) => {
                        println!("⚠️  WATCH_INTERVAL_SECONDS: Not set, using default behavior");
                        None
                    }
                }
            },
            file_stability_check_ms: {
                match env::var("FILE_STABILITY_CHECK_MS") {
                    Ok(val) => match val.parse::<u64>() {
                        Ok(parsed) => {
                            println!("✅ FILE_STABILITY_CHECK_MS: {} (loaded from env)", parsed);
                            Some(parsed)
                        }
                        Err(e) => {
                            println!("❌ FILE_STABILITY_CHECK_MS: Invalid value '{}' - {}, using default", val, e);
                            None
                        }
                    },
                    Err(_) => {
                        println!("⚠️  FILE_STABILITY_CHECK_MS: Not set, using default behavior");
                        None
                    }
                }
            },
            max_file_age_hours: {
                match env::var("MAX_FILE_AGE_HOURS") {
                    Ok(val) => match val.parse::<u64>() {
                        Ok(parsed) => {
                            println!("✅ MAX_FILE_AGE_HOURS: {} (loaded from env)", parsed);
                            Some(parsed)
                        }
                        Err(e) => {
                            println!("❌ MAX_FILE_AGE_HOURS: Invalid value '{}' - {}, using unlimited", val, e);
                            None
                        }
                    },
                    Err(_) => {
                        println!("⚠️  MAX_FILE_AGE_HOURS: Not set, files will not expire");
                        None
                    }
                }
            },
                
            // OCR Configuration
            ocr_language: match env::var("OCR_LANGUAGE") {
                Ok(lang) => {
                    println!("✅ OCR_LANGUAGE: {} (loaded from env)", lang);
                    lang
                }
                Err(_) => {
                    let default_lang = "eng".to_string();
                    println!("⚠️  OCR_LANGUAGE: {} (using default - env var not set)", default_lang);
                    default_lang
                }
            },
            concurrent_ocr_jobs: {
                match env::var("CONCURRENT_OCR_JOBS") {
                    Ok(val) => match val.parse::<usize>() {
                        Ok(parsed) => {
                            println!("✅ CONCURRENT_OCR_JOBS: {} (loaded from env)", parsed);
                            parsed
                        }
                        Err(e) => {
                            let default_jobs = 4;
                            println!("❌ CONCURRENT_OCR_JOBS: Invalid value '{}' - {}, using default {}", val, e, default_jobs);
                            default_jobs
                        }
                    },
                    Err(_) => {
                        let default_jobs = 4;
                        println!("⚠️  CONCURRENT_OCR_JOBS: {} (using default - env var not set)", default_jobs);
                        default_jobs
                    }
                }
            },
            ocr_timeout_seconds: {
                match env::var("OCR_TIMEOUT_SECONDS") {
                    Ok(val) => match val.parse::<u64>() {
                        Ok(parsed) => {
                            println!("✅ OCR_TIMEOUT_SECONDS: {} (loaded from env)", parsed);
                            parsed
                        }
                        Err(e) => {
                            let default_timeout = 300;
                            println!("❌ OCR_TIMEOUT_SECONDS: Invalid value '{}' - {}, using default {}", val, e, default_timeout);
                            default_timeout
                        }
                    },
                    Err(_) => {
                        let default_timeout = 300;
                        println!("⚠️  OCR_TIMEOUT_SECONDS: {} (using default - env var not set)", default_timeout);
                        default_timeout
                    }
                }
            },
            max_file_size_mb: {
                match env::var("MAX_FILE_SIZE_MB") {
                    Ok(val) => match val.parse::<u64>() {
                        Ok(parsed) => {
                            println!("✅ MAX_FILE_SIZE_MB: {} (loaded from env)", parsed);
                            parsed
                        }
                        Err(e) => {
                            let default_size = 50;
                            println!("❌ MAX_FILE_SIZE_MB: Invalid value '{}' - {}, using default {}", val, e, default_size);
                            default_size
                        }
                    },
                    Err(_) => {
                        let default_size = 50;
                        println!("⚠️  MAX_FILE_SIZE_MB: {} (using default - env var not set)", default_size);
                        default_size
                    }
                }
            },
                
            // Performance Configuration
            memory_limit_mb: {
                match env::var("MEMORY_LIMIT_MB") {
                    Ok(val) => match val.parse::<usize>() {
                        Ok(parsed) => {
                            println!("✅ MEMORY_LIMIT_MB: {} (loaded from env)", parsed);
                            parsed
                        }
                        Err(e) => {
                            let default_memory = 512;
                            println!("❌ MEMORY_LIMIT_MB: Invalid value '{}' - {}, using default {}", val, e, default_memory);
                            default_memory
                        }
                    },
                    Err(_) => {
                        let default_memory = 512;
                        println!("⚠️  MEMORY_LIMIT_MB: {} (using default - env var not set)", default_memory);
                        default_memory
                    }
                }
            },
            cpu_priority: match env::var("CPU_PRIORITY") {
                Ok(priority) => {
                    println!("✅ CPU_PRIORITY: {} (loaded from env)", priority);
                    priority
                }
                Err(_) => {
                    let default_priority = "normal".to_string();
                    println!("⚠️  CPU_PRIORITY: {} (using default - env var not set)", default_priority);
                    default_priority
                }
            },
            
            // OIDC Configuration
            oidc_enabled: match env::var("OIDC_ENABLED") {
                Ok(val) => match val.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => {
                        println!("✅ OIDC_ENABLED: true (loaded from env)");
                        true
                    }
                    _ => {
                        println!("✅ OIDC_ENABLED: false (loaded from env)");
                        false
                    }
                },
                Err(_) => {
                    println!("⚠️  OIDC_ENABLED: false (using default - env var not set)");
                    false
                }
            },
            oidc_client_id: match env::var("OIDC_CLIENT_ID") {
                Ok(client_id) => {
                    println!("✅ OIDC_CLIENT_ID: {} (loaded from env)", client_id);
                    Some(client_id)
                }
                Err(_) => {
                    println!("⚠️  OIDC_CLIENT_ID: Not set");
                    None
                }
            },
            oidc_client_secret: match env::var("OIDC_CLIENT_SECRET") {
                Ok(secret) => {
                    println!("✅ OIDC_CLIENT_SECRET: ***hidden*** (loaded from env, {} chars)", secret.len());
                    Some(secret)
                }
                Err(_) => {
                    println!("⚠️  OIDC_CLIENT_SECRET: Not set");
                    None
                }
            },
            oidc_issuer_url: match env::var("OIDC_ISSUER_URL") {
                Ok(url) => {
                    println!("✅ OIDC_ISSUER_URL: {} (loaded from env)", url);
                    Some(url)
                }
                Err(_) => {
                    println!("⚠️  OIDC_ISSUER_URL: Not set");
                    None
                }
            },
            oidc_redirect_uri: match env::var("OIDC_REDIRECT_URI") {
                Ok(uri) => {
                    println!("✅ OIDC_REDIRECT_URI: {} (loaded from env)", uri);
                    Some(uri)
                }
                Err(_) => {
                    println!("⚠️  OIDC_REDIRECT_URI: Not set");
                    None
                }
            },
            
            // S3 Configuration
            s3_enabled: match env::var("S3_ENABLED") {
                Ok(val) => {
                    let enabled = val.to_lowercase() == "true";
                    println!("✅ S3_ENABLED: {} (loaded from env)", enabled);
                    enabled
                }
                Err(_) => {
                    println!("⚠️  S3_ENABLED: false (using default - env var not set)");
                    false
                }
            },
            s3_config: if env::var("S3_ENABLED").unwrap_or_default().to_lowercase() == "true" {
                // Only load S3 config if S3 is enabled
                let bucket_name = env::var("S3_BUCKET_NAME").unwrap_or_default();
                let region = env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
                let access_key_id = env::var("S3_ACCESS_KEY_ID").unwrap_or_default();
                let secret_access_key = env::var("S3_SECRET_ACCESS_KEY").unwrap_or_default();
                let endpoint_url = env::var("S3_ENDPOINT_URL").ok();
                let prefix = env::var("S3_PREFIX").ok();
                
                if !bucket_name.is_empty() && !access_key_id.is_empty() && !secret_access_key.is_empty() {
                    println!("✅ S3_BUCKET_NAME: {} (loaded from env)", bucket_name);
                    println!("✅ S3_REGION: {} (loaded from env)", region);
                    println!("✅ S3_ACCESS_KEY_ID: configured (loaded from env)");
                    println!("✅ S3_SECRET_ACCESS_KEY: configured (loaded from env)");
                    if let Some(ref endpoint) = endpoint_url {
                        println!("✅ S3_ENDPOINT_URL: {} (loaded from env)", endpoint);
                    }
                    if let Some(ref pref) = prefix {
                        println!("✅ S3_PREFIX: {} (loaded from env)", pref);
                    }

                    Some(S3SourceConfig {
                        bucket_name,
                        region,
                        access_key_id,
                        secret_access_key,
                        endpoint_url,
                        prefix,
                        watch_folders: vec![], // Will be configured separately for sources
                        file_extensions: vec![], // Will be configured separately for sources
                        auto_sync: false, // Not used for general storage
                        sync_interval_minutes: 0, // Not used for general storage
                    })
                } else {
                    println!("❌ S3 enabled but missing required configuration (bucket_name, access_key_id, or secret_access_key)");
                    None
                }
            } else {
                None
            },
            
            // Placeholder CPU allocation - will be replaced after detection
            cpu_allocation: CpuAllocation::from_auto_allocation(4).unwrap(),
        };
        
        // Initialize CPU allocation
        println!("\n🧮 CPU CORE ALLOCATION:");
        println!("{}", "=".repeat(50));
        let cpu_allocation = match CpuAllocation::detect_and_allocate() {
            Ok(allocation) => {
                allocation.log_allocation();
                allocation.validate_allocation()?;
                allocation
            }
            Err(e) => {
                println!("❌ Failed to detect and allocate CPU cores: {}", e);
                return Err(e);
            }
        };
        
        // Update concurrent OCR jobs based on CPU allocation if not manually set
        let concurrent_ocr_jobs = if env::var("CONCURRENT_OCR_JOBS").is_ok() {
            config.concurrent_ocr_jobs  // Keep user-specified value
        } else {
            let recommended = cpu_allocation.recommended_concurrent_ocr_jobs();
            println!("🧠 Adjusting concurrent OCR jobs from {} to {} based on CPU allocation", 
                     config.concurrent_ocr_jobs, recommended);
            recommended
        };
        
        // Update the config with CPU allocation and adjusted OCR jobs
        let mut config = config;
        config.cpu_allocation = cpu_allocation;
        config.concurrent_ocr_jobs = concurrent_ocr_jobs;
        
        println!("\n🔍 CONFIGURATION VALIDATION:");
        println!("{}", "=".repeat(50));
        
        // Validate server address format
        if !config.server_address.contains(':') {
            println!("❌ SERVER_ADDRESS: Invalid format '{}' - missing port", config.server_address);
            return Err(anyhow::anyhow!(
                "Invalid server address format: '{}'. Expected format: 'host:port' (e.g., '0.0.0.0:8000')", 
                config.server_address
            ));
        }
        
        // Validate database URL format
        if !config.database_url.starts_with("postgresql://") && !config.database_url.starts_with("postgres://") {
            println!("❌ DATABASE_URL: Invalid format - must start with 'postgresql://' or 'postgres://'");
            return Err(anyhow::anyhow!(
                "Invalid database URL format. Must start with 'postgresql://' or 'postgres://'"
            ));
        }
        
        // Validate configuration to prevent recursion issues
        println!("🔍 Validating directory paths for conflicts...");
        config.validate_paths()?;
        
        println!("\n📊 CONFIGURATION SUMMARY:");
        println!("{}", "=".repeat(50));
        println!("🌐 Server will bind to: {}", config.server_address);
        println!("📁 Upload directory: {}", config.upload_path);
        println!("👁️  Watch directory: {}", config.watch_folder);
        println!("👥 Per-user watch enabled: {}", config.enable_per_user_watch);
        if config.enable_per_user_watch {
            println!("📂 User watch base directory: {}", config.user_watch_base_dir);
        }
        println!("📄 Allowed file types: {:?}", config.allowed_file_types);
        println!("🧠 OCR language: {}", config.ocr_language);
        println!("⚙️  Concurrent OCR jobs: {}", config.concurrent_ocr_jobs);
        println!("⏱️  OCR timeout: {}s", config.ocr_timeout_seconds);
        println!("📏 Max file size: {}MB", config.max_file_size_mb);
        println!("💾 Memory limit: {}MB", config.memory_limit_mb);
        
        // Warning checks
        println!("\n⚠️  CONFIGURATION WARNINGS:");
        println!("{}", "=".repeat(50));
        if config.jwt_secret == "your-secret-key" {
            println!("🚨 SECURITY WARNING: Using default JWT secret! Set JWT_SECRET environment variable in production!");
        }
        if config.server_address.starts_with("0.0.0.0") {
            println!("🌍 INFO: Server will listen on all interfaces (0.0.0.0)");
        }
        if config.max_file_size_mb > 100 {
            println!("📏 INFO: Large file size limit ({}MB) may impact performance", config.max_file_size_mb);
        }
        if config.concurrent_ocr_jobs > 8 {
            println!("⚙️  INFO: High OCR concurrency ({}) may use significant CPU/memory", config.concurrent_ocr_jobs);
        }
        
        // OIDC validation
        if config.oidc_enabled {
            println!("🔐 OIDC is enabled");
            if config.oidc_client_id.is_none() {
                println!("❌ OIDC_CLIENT_ID is required when OIDC is enabled");
            }
            if config.oidc_client_secret.is_none() {
                println!("❌ OIDC_CLIENT_SECRET is required when OIDC is enabled");
            }
            if config.oidc_issuer_url.is_none() {
                println!("❌ OIDC_ISSUER_URL is required when OIDC is enabled");
            }
            if config.oidc_redirect_uri.is_none() {
                println!("❌ OIDC_REDIRECT_URI is required when OIDC is enabled");
            }
        } else {
            println!("🔐 OIDC is disabled");
        }
        
        println!("✅ Configuration validation completed successfully!\n");
        
        Ok(config)
    }
    
    fn validate_paths(&self) -> Result<()> {
        use std::path::Path;
        
        let upload_path = Path::new(&self.upload_path);
        let watch_path = Path::new(&self.watch_folder);
        let user_watch_path = Path::new(&self.user_watch_base_dir);
        
        println!("📁 Checking upload directory: {}", self.upload_path);
        println!("👁️  Checking watch directory: {}", self.watch_folder);
        if self.enable_per_user_watch {
            println!("👥 Checking user watch base directory: {}", self.user_watch_base_dir);
        }
        
        // Check if paths exist and are accessible
        if !upload_path.exists() {
            println!("⚠️  Upload directory does not exist yet: {}", self.upload_path);
        } else if !upload_path.is_dir() {
            println!("❌ Upload path exists but is not a directory: {}", self.upload_path);
            return Err(anyhow::anyhow!(
                "Upload path '{}' exists but is not a directory", self.upload_path
            ));
        } else {
            println!("✅ Upload directory exists and is accessible");
        }
        
        if !watch_path.exists() {
            println!("⚠️  Watch directory does not exist yet: {}", self.watch_folder);
        } else if !watch_path.is_dir() {
            println!("❌ Watch path exists but is not a directory: {}", self.watch_folder);
            return Err(anyhow::anyhow!(
                "Watch folder '{}' exists but is not a directory", self.watch_folder
            ));
        } else {
            println!("✅ Watch directory exists and is accessible");
        }
        
        if self.enable_per_user_watch {
            if !user_watch_path.exists() {
                println!("⚠️  User watch base directory does not exist yet: {}", self.user_watch_base_dir);
            } else if !user_watch_path.is_dir() {
                println!("❌ User watch base path exists but is not a directory: {}", self.user_watch_base_dir);
                return Err(anyhow::anyhow!(
                    "User watch base directory '{}' exists but is not a directory", self.user_watch_base_dir
                ));
            } else {
                println!("✅ User watch base directory exists and is accessible");
            }
        }
        
        // Normalize paths to handle relative paths and symlinks
        let upload_canonical = upload_path.canonicalize()
            .unwrap_or_else(|_| {
                println!("⚠️  Could not canonicalize upload path, using as-is");
                upload_path.to_path_buf()
            });
        let watch_canonical = watch_path.canonicalize()
            .unwrap_or_else(|_| {
                println!("⚠️  Could not canonicalize watch path, using as-is");
                watch_path.to_path_buf()
            });
        let user_watch_canonical = if self.enable_per_user_watch {
            Some(user_watch_path.canonicalize()
                .unwrap_or_else(|_| {
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
                self.upload_path, self.watch_folder
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
                self.upload_path, self.watch_folder
            ));
        }
        
        // Additional validation for user watch directory if enabled
        if let Some(ref user_watch) = user_watch_canonical {
            // Check if user watch is same as upload or watch
            if user_watch == &upload_canonical {
                println!("❌ CRITICAL ERROR: User watch base directory is same as upload directory!");
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
                println!("❌ CRITICAL ERROR: User watch base directory is inside upload directory!");
                return Err(anyhow::anyhow!(
                    "❌ Configuration Error: USER_WATCH_BASE_DIR cannot be inside UPLOAD_PATH.\n\
                     This would cause recursion issues.\n\
                     Current config:\n\
                     - UPLOAD_PATH: {}\n\
                     - USER_WATCH_BASE_DIR: {}\n\
                     Please move the user watch directory outside the upload directory.",
                    self.upload_path, self.user_watch_base_dir
                ));
            }
            
            if upload_canonical.starts_with(user_watch) {
                println!("❌ CRITICAL ERROR: Upload directory is inside user watch base directory!");
                return Err(anyhow::anyhow!(
                    "❌ Configuration Error: UPLOAD_PATH cannot be inside USER_WATCH_BASE_DIR.\n\
                     This would cause recursion issues.\n\
                     Current config:\n\
                     - UPLOAD_PATH: {}\n\
                     - USER_WATCH_BASE_DIR: {}\n\
                     Please move the upload directory outside the user watch directory.",
                    self.upload_path, self.user_watch_base_dir
                ));
            }
        }
        
        println!("✅ Directory path validation passed - no conflicts detected");
        Ok(())
    }
}