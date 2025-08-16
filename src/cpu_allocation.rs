use anyhow::Result;
use std::env;
use tracing::{info, warn, error};

/// CPU core allocation configuration for the Readur backend
#[derive(Debug, Clone)]
pub struct CpuAllocation {
    /// Total available CPU cores detected
    pub total_cores: usize,
    /// Cores allocated for web server (HTTP requests, API)
    pub web_cores: usize,
    /// Cores allocated for backend processing (OCR, file processing, sync)
    pub backend_cores: usize,
    /// Cores allocated specifically for OCR tasks
    pub ocr_cores: usize,
    /// Cores allocated for background tasks (WebDAV sync, maintenance)
    pub background_cores: usize,
    /// Cores allocated for database operations
    pub db_cores: usize,
}

impl CpuAllocation {
    /// Automatically detect CPU cores and create an optimal allocation
    pub fn detect_and_allocate() -> Result<Self> {
        let total_cores = Self::detect_total_cores()?;
        
        // Check for environment variable overrides
        let web_cores_override = env::var("READUR_WEB_CORES")
            .ok()
            .and_then(|s| s.parse::<usize>().ok());
        let backend_cores_override = env::var("READUR_BACKEND_CORES")
            .ok()
            .and_then(|s| s.parse::<usize>().ok());
        
        // If both are manually specified, use them
        if let (Some(web), Some(backend)) = (web_cores_override, backend_cores_override) {
            return Self::from_manual_allocation(total_cores, web, backend);
        }
        
        // If only one is specified, calculate the other
        if let Some(web) = web_cores_override {
            let backend = total_cores.saturating_sub(web).max(1);
            return Self::from_manual_allocation(total_cores, web, backend);
        }
        
        if let Some(backend) = backend_cores_override {
            let web = total_cores.saturating_sub(backend).max(1);
            return Self::from_manual_allocation(total_cores, web, backend);
        }
        
        // Auto-allocation: split evenly between web and backend
        Self::from_auto_allocation(total_cores)
    }
    
    /// Detect the total number of available CPU cores
    fn detect_total_cores() -> Result<usize> {
        // Try std::thread::available_parallelism first (Rust 1.59+)
        match std::thread::available_parallelism() {
            Ok(cores) => {
                let count = cores.get();
                info!("✅ Detected {} CPU cores using std::thread::available_parallelism", count);
                Ok(count)
            }
            Err(e) => {
                warn!("⚠️  Failed to detect CPU cores with std::thread::available_parallelism: {}", e);
                
                // Fallback to environment variable
                if let Ok(cores_str) = env::var("READUR_TOTAL_CORES") {
                    match cores_str.parse::<usize>() {
                        Ok(cores) if cores > 0 => {
                            info!("✅ Using {} CPU cores from READUR_TOTAL_CORES environment variable", cores);
                            return Ok(cores);
                        }
                        _ => {
                            error!("❌ Invalid READUR_TOTAL_CORES value: {}", cores_str);
                        }
                    }
                }
                
                // Final fallback to a reasonable default
                warn!("🔄 Falling back to default of 4 CPU cores");
                Ok(4)
            }
        }
    }
    
    /// Create allocation from automatic detection (50/50 split)
    pub fn from_auto_allocation(total_cores: usize) -> Result<Self> {
        // Ensure minimum of 1 core for each component
        if total_cores < 2 {
            warn!("⚠️  Only {} core(s) detected, using minimal allocation", total_cores);
            return Ok(Self {
                total_cores,
                web_cores: 1,
                backend_cores: 1,
                ocr_cores: 1,
                background_cores: 1,
                db_cores: 1,
            });
        }
        
        // Split cores evenly between web and backend
        let web_cores = total_cores / 2;
        let backend_cores = total_cores - web_cores;
        
        Self::from_manual_allocation(total_cores, web_cores, backend_cores)
    }
    
    /// Create allocation from manual specification
    pub fn from_manual_allocation(total_cores: usize, web_cores: usize, backend_cores: usize) -> Result<Self> {
        // Validate inputs
        let web_cores = web_cores.max(1);
        let backend_cores = backend_cores.max(1);
        
        if web_cores + backend_cores > total_cores {
            warn!("⚠️  Allocated cores ({} + {} = {}) exceed total cores ({}), scaling down proportionally", 
                  web_cores, backend_cores, web_cores + backend_cores, total_cores);
            
            // Scale down proportionally
            let total_requested = web_cores + backend_cores;
            let web_scaled = ((web_cores as f64 / total_requested as f64) * total_cores as f64).ceil() as usize;
            let backend_scaled = total_cores - web_scaled;
            
            return Self::from_manual_allocation(total_cores, web_scaled.max(1), backend_scaled.max(1));
        }
        
        // Allocate backend cores among different workloads
        let (ocr_cores, background_cores, db_cores) = Self::allocate_backend_cores(backend_cores);
        
        Ok(Self {
            total_cores,
            web_cores,
            backend_cores,
            ocr_cores,
            background_cores,
            db_cores,
        })
    }
    
    /// Intelligently allocate backend cores among OCR, background tasks, and DB operations
    fn allocate_backend_cores(backend_cores: usize) -> (usize, usize, usize) {
        if backend_cores == 1 {
            // All background tasks share the single core
            return (1, 1, 1);
        }
        
        if backend_cores == 2 {
            // OCR gets priority, background and DB share
            return (1, 1, 1);
        }
        
        if backend_cores <= 4 {
            // Small allocation: OCR gets most cores, others get 1 each
            let ocr_cores = backend_cores - 2;
            return (ocr_cores.max(1), 1, 1);
        }
        
        // Larger allocation: distribute more evenly
        // OCR is usually the most CPU-intensive, so it gets the largest share
        let ocr_cores = (backend_cores as f64 * 0.5).ceil() as usize;
        let remaining = backend_cores - ocr_cores;
        let background_cores = (remaining / 2).max(1);
        let db_cores = remaining - background_cores;
        
        (ocr_cores, background_cores.max(1), db_cores.max(1))
    }
    
    /// Log the allocation decision with detailed information
    pub fn log_allocation(&self) {
        info!("🧮 CPU CORE ALLOCATION:");
        info!("=====================================");
        info!("🔍 Total cores detected: {}", self.total_cores);
        info!("🌐 Web server cores: {} ({:.1}%)", 
              self.web_cores, 
              (self.web_cores as f64 / self.total_cores as f64) * 100.0);
        info!("⚙️  Backend processing cores: {} ({:.1}%)", 
              self.backend_cores,
              (self.backend_cores as f64 / self.total_cores as f64) * 100.0);
        info!("  ├── 🧠 OCR processing: {} cores", self.ocr_cores);
        info!("  ├── 🔄 Background tasks: {} cores", self.background_cores);
        info!("  └── 🗄️  Database operations: {} cores", self.db_cores);
        
        // Log environment variable information
        if env::var("READUR_WEB_CORES").is_ok() {
            info!("🔧 Web cores overridden by READUR_WEB_CORES");
        }
        if env::var("READUR_BACKEND_CORES").is_ok() {
            info!("🔧 Backend cores overridden by READUR_BACKEND_CORES");
        }
        if env::var("READUR_TOTAL_CORES").is_ok() {
            info!("🔧 Total cores overridden by READUR_TOTAL_CORES");
        }
        
        // Warn about potential issues
        if self.total_cores <= 2 {
            warn!("⚠️  Low CPU core count may impact performance with concurrent operations");
        }
        
        if self.ocr_cores >= 6 {
            info!("💪 High OCR core allocation - excellent for batch processing");
        }
        
        info!("=====================================");
    }
    
    /// Get the recommended concurrent OCR jobs based on core allocation
    pub fn recommended_concurrent_ocr_jobs(&self) -> usize {
        // Generally, 1-2 OCR jobs per core is reasonable
        // OCR jobs can be I/O bound due to disk reads, so slight oversubscription is OK
        (self.ocr_cores * 2).max(1)
    }
    
    /// Check if the current allocation is sensible and log warnings if not
    pub fn validate_allocation(&self) -> Result<()> {
        let mut warnings = Vec::new();
        
        if self.web_cores == 0 {
            return Err(anyhow::anyhow!("Web server must have at least 1 core"));
        }
        
        if self.backend_cores == 0 {
            return Err(anyhow::anyhow!("Backend processing must have at least 1 core"));
        }
        
        if self.web_cores > self.total_cores {
            return Err(anyhow::anyhow!("Web cores ({}) cannot exceed total cores ({})", 
                                      self.web_cores, self.total_cores));
        }
        
        if self.backend_cores > self.total_cores {
            return Err(anyhow::anyhow!("Backend cores ({}) cannot exceed total cores ({})", 
                                      self.backend_cores, self.total_cores));
        }
        
        // Warnings for suboptimal configurations
        if self.web_cores > self.backend_cores * 2 {
            warnings.push(format!("Web cores ({}) significantly exceed backend cores ({}) - may be suboptimal for processing-heavy workloads", 
                                 self.web_cores, self.backend_cores));
        }
        
        if self.backend_cores > self.web_cores * 3 {
            warnings.push(format!("Backend cores ({}) significantly exceed web cores ({}) - may cause slow API responses under load", 
                                 self.backend_cores, self.web_cores));
        }
        
        for warning in warnings {
            warn!("⚠️  {}", warning);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detect_total_cores() {
        let cores = CpuAllocation::detect_total_cores().unwrap();
        assert!(cores > 0, "Should detect at least 1 core");
        assert!(cores <= 256, "Should not detect unreasonably high core count");
    }
    
    #[test]
    fn test_auto_allocation_even_cores() {
        let allocation = CpuAllocation::from_auto_allocation(8).unwrap();
        assert_eq!(allocation.total_cores, 8);
        assert_eq!(allocation.web_cores, 4);
        assert_eq!(allocation.backend_cores, 4);
        assert!(allocation.ocr_cores >= 1);
        assert!(allocation.background_cores >= 1);
        assert!(allocation.db_cores >= 1);
    }
    
    #[test]
    fn test_auto_allocation_odd_cores() {
        let allocation = CpuAllocation::from_auto_allocation(7).unwrap();
        assert_eq!(allocation.total_cores, 7);
        assert_eq!(allocation.web_cores, 3);
        assert_eq!(allocation.backend_cores, 4);
    }
    
    #[test]
    fn test_minimal_allocation() {
        let allocation = CpuAllocation::from_auto_allocation(1).unwrap();
        assert_eq!(allocation.total_cores, 1);
        assert_eq!(allocation.web_cores, 1);
        assert_eq!(allocation.backend_cores, 1);
        assert_eq!(allocation.ocr_cores, 1);
        assert_eq!(allocation.background_cores, 1);
        assert_eq!(allocation.db_cores, 1);
    }
    
    #[test]
    fn test_manual_allocation() {
        let allocation = CpuAllocation::from_manual_allocation(8, 2, 6).unwrap();
        assert_eq!(allocation.total_cores, 8);
        assert_eq!(allocation.web_cores, 2);
        assert_eq!(allocation.backend_cores, 6);
    }
    
    #[test]
    fn test_backend_core_allocation() {
        let (ocr, bg, db) = CpuAllocation::allocate_backend_cores(6);
        assert_eq!(ocr + bg + db, 6);
        assert!(ocr >= 1);
        assert!(bg >= 1);
        assert!(db >= 1);
        assert!(ocr >= bg); // OCR should get priority
    }
    
    #[test]
    fn test_validation() {
        let allocation = CpuAllocation::from_auto_allocation(4).unwrap();
        allocation.validate_allocation().unwrap();
    }
    
    #[test]
    fn test_recommended_ocr_jobs() {
        let allocation = CpuAllocation::from_auto_allocation(8).unwrap();
        let jobs = allocation.recommended_concurrent_ocr_jobs();
        assert!(jobs >= 1);
        assert!(jobs <= allocation.ocr_cores * 3); // Should be reasonable
    }
}