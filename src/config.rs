use anyhow::Result;
use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub server_address: String,
    pub jwt_secret: String,
    pub upload_path: String,
    pub watch_folder: String,
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
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        
        let config = Config {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://readur:readur@localhost/readur".to_string()),
            server_address: {
                // Support both SERVER_ADDRESS (full address) and SERVER_PORT (just port)
                if let Ok(addr) = env::var("SERVER_ADDRESS") {
                    addr
                } else {
                    let host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
                    let port = env::var("SERVER_PORT").unwrap_or_else(|_| "8000".to_string());
                    format!("{}:{}", host, port)
                }
            },
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key".to_string()),
            upload_path: env::var("UPLOAD_PATH")
                .unwrap_or_else(|_| "./uploads".to_string()),
            watch_folder: env::var("WATCH_FOLDER")
                .unwrap_or_else(|_| "./watch".to_string()),
            allowed_file_types: env::var("ALLOWED_FILE_TYPES")
                .unwrap_or_else(|_| "pdf,txt,doc,docx,png,jpg,jpeg".to_string())
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect(),
            watch_interval_seconds: env::var("WATCH_INTERVAL_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok()),
            file_stability_check_ms: env::var("FILE_STABILITY_CHECK_MS")
                .ok()
                .and_then(|s| s.parse().ok()),
            max_file_age_hours: env::var("MAX_FILE_AGE_HOURS")
                .ok()
                .and_then(|s| s.parse().ok()),
                
            // OCR Configuration
            ocr_language: env::var("OCR_LANGUAGE")
                .unwrap_or_else(|_| "eng".to_string()),
            concurrent_ocr_jobs: env::var("CONCURRENT_OCR_JOBS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4),
            ocr_timeout_seconds: env::var("OCR_TIMEOUT_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
            max_file_size_mb: env::var("MAX_FILE_SIZE_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50),
                
            // Performance
            memory_limit_mb: env::var("MEMORY_LIMIT_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(512),
            cpu_priority: env::var("CPU_PRIORITY")
                .unwrap_or_else(|_| "normal".to_string()),
        };
        
        // Validate configuration to prevent recursion issues
        config.validate_paths()?;
        
        Ok(config)
    }
    
    fn validate_paths(&self) -> Result<()> {
        use std::path::Path;
        
        let upload_path = Path::new(&self.upload_path);
        let watch_path = Path::new(&self.watch_folder);
        
        // Normalize paths to handle relative paths and symlinks
        let upload_canonical = upload_path.canonicalize()
            .unwrap_or_else(|_| upload_path.to_path_buf());
        let watch_canonical = watch_path.canonicalize()
            .unwrap_or_else(|_| watch_path.to_path_buf());
        
        // Check if paths are the same
        if upload_canonical == watch_canonical {
            return Err(anyhow::anyhow!(
                "Configuration Error: UPLOAD_PATH and WATCH_FOLDER cannot be the same directory.\n\
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
            return Err(anyhow::anyhow!(
                "Configuration Error: WATCH_FOLDER cannot be inside UPLOAD_PATH.\n\
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
            return Err(anyhow::anyhow!(
                "Configuration Error: UPLOAD_PATH cannot be inside WATCH_FOLDER.\n\
                 This would cause recursion where files from the watch folder are \n\
                 copied to uploads (inside the watch folder) and reprocessed.\n\
                 Current config:\n\
                 - UPLOAD_PATH: {}\n\
                 - WATCH_FOLDER: {}\n\
                 Please move the upload directory outside the watch folder.",
                self.upload_path, self.watch_folder
            ));
        }
        
        Ok(())
    }
}