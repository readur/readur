use crate::ocr::error::{CpuFeatures, OcrDiagnostics, OcrError};
use std::process::Command;
use std::env;
use sysinfo::System;

pub struct OcrHealthChecker;

impl OcrHealthChecker {
    pub fn new() -> Self {
        Self
    }
    
    pub fn check_tesseract_installation(&self) -> Result<String, OcrError> {
        let output = Command::new("tesseract")
            .arg("--version")
            .output()
            .map_err(|_| OcrError::TesseractNotInstalled)?;
        
        if !output.status.success() {
            return Err(OcrError::TesseractNotInstalled);
        }
        
        let version_info = String::from_utf8_lossy(&output.stdout);
        let version = version_info
            .lines()
            .next()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        Ok(version)
    }
    
    pub fn check_language_data(&self, lang: &str) -> Result<(), OcrError> {
        // Use Tesseract's built-in language validation by attempting to list languages
        let output = Command::new("tesseract")
            .arg("--list-langs")
            .output()
            .map_err(|_| OcrError::TesseractNotInstalled)?;
        
        if !output.status.success() {
            return Err(OcrError::TesseractNotInstalled);
        }
        
        let langs_output = String::from_utf8_lossy(&output.stdout);
        let available_langs: Vec<&str> = langs_output.lines().skip(1).collect(); // Skip first line "List of available languages:"
        
        if !available_langs.contains(&lang) {
            return Err(OcrError::LanguageDataNotFound {
                lang: lang.to_string(),
            });
        }
        
        Ok(())
    }
    
    
    pub fn get_available_languages(&self) -> Result<Vec<String>, OcrError> {
        // Use Tesseract's built-in language listing
        let output = Command::new("tesseract")
            .arg("--list-langs")
            .output()
            .map_err(|_| OcrError::TesseractNotInstalled)?;
        
        if !output.status.success() {
            return Err(OcrError::TesseractNotInstalled);
        }
        
        let langs_output = String::from_utf8_lossy(&output.stdout);
        let mut languages: Vec<String> = langs_output
            .lines()
            .skip(1) // Skip first line "List of available languages:"
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        languages.sort();
        Ok(languages)
    }
    
    pub fn validate_language(&self, lang: &str) -> Result<(), OcrError> {
        // Check if language is supported
        let available_languages = self.get_available_languages()?;
        if !available_languages.contains(&lang.to_string()) {
            return Err(OcrError::LanguageDataNotFound {
                lang: lang.to_string(),
            });
        }
        Ok(())
    }

    /// Validate a language combination (e.g., "eng+spa")
    pub fn validate_language_combination(&self, lang_combination: &str) -> Result<(), OcrError> {
        if lang_combination.is_empty() {
            return Err(OcrError::LanguageDataNotFound {
                lang: "empty".to_string(),
            });
        }

        // Split by '+' to handle multi-language combinations
        let languages: Vec<&str> = lang_combination.split('+').collect();
        
        // Validate each language in the combination
        for lang in &languages {
            self.validate_language(lang.trim())?;
        }
        
        // Limit number of languages for performance (max 4)
        if languages.len() > 4 {
            return Err(OcrError::LanguageDataNotFound {
                lang: format!("Too many languages in combination: {}. Maximum is 4.", languages.len()),
            });
        }
        
        Ok(())
    }

    /// Validate a list of preferred languages
    pub fn validate_preferred_languages(&self, languages: &[String]) -> Result<(), OcrError> {
        if languages.is_empty() {
            return Err(OcrError::LanguageDataNotFound {
                lang: "No languages provided".to_string(),
            });
        }
        
        // Limit number of languages for performance
        if languages.len() > 4 {
            return Err(OcrError::LanguageDataNotFound {
                lang: format!("Too many preferred languages: {}. Maximum is 4.", languages.len()),
            });
        }
        
        // Validate each language
        for lang in languages {
            self.validate_language(lang)?;
        }
        
        Ok(())
    }
    
    pub fn get_language_display_name(&self, lang_code: &str) -> String {
        match lang_code {
            "eng" => "English".to_string(),
            "spa" => "Spanish".to_string(),
            "fra" => "French".to_string(),
            "deu" => "German".to_string(),
            "ita" => "Italian".to_string(),
            "por" => "Portuguese".to_string(),
            "rus" => "Russian".to_string(),
            "chi_sim" => "Chinese (Simplified)".to_string(),
            "chi_tra" => "Chinese (Traditional)".to_string(),
            "jpn" => "Japanese".to_string(),
            "kor" => "Korean".to_string(),
            "ara" => "Arabic".to_string(),
            "hin" => "Hindi".to_string(),
            "nld" => "Dutch".to_string(),
            "swe" => "Swedish".to_string(),
            "nor" => "Norwegian".to_string(),
            "dan" => "Danish".to_string(),
            "fin" => "Finnish".to_string(),
            "pol" => "Polish".to_string(),
            "ces" => "Czech".to_string(),
            "hun" => "Hungarian".to_string(),
            "tur" => "Turkish".to_string(),
            "tha" => "Thai".to_string(),
            "vie" => "Vietnamese".to_string(),
            _ => lang_code.to_string(), // Return the code itself for unknown languages
        }
    }
    
    pub fn check_cpu_features(&self) -> CpuFeatures {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            use raw_cpuid::CpuId;
            let cpuid = CpuId::new();
            
            let features = cpuid.get_feature_info().map(|f| CpuFeatures {
                sse2: f.has_sse2(),
                sse3: f.has_sse3(),
                sse4_1: f.has_sse41(),
                sse4_2: f.has_sse42(),
                avx: f.has_avx(),
                avx2: cpuid.get_extended_feature_info()
                    .map(|ef| ef.has_avx2())
                    .unwrap_or(false),
            }).unwrap_or_else(|| CpuFeatures {
                sse2: false,
                sse3: false,
                sse4_1: false,
                sse4_2: false,
                avx: false,
                avx2: false,
            });
            
            features
        }
        
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        {
            CpuFeatures {
                sse2: false,
                sse3: false,
                sse4_1: false,
                sse4_2: false,
                avx: false,
                avx2: false,
            }
        }
    }
    
    pub fn check_memory_available(&self) -> u64 {
        let mut sys = System::new_all();
        sys.refresh_memory();
        sys.available_memory() / (1024 * 1024) // Convert to MB
    }
    
    pub fn check_temp_space(&self) -> u64 {
        use std::fs;
        
        let temp_dir = env::temp_dir();
        
        // Try to get actual available space using statvfs on Unix-like systems
        #[cfg(target_family = "unix")]
        {
            use std::mem;
            
            #[repr(C)]
            struct statvfs {
                f_bsize: u64,    // file system block size
                f_frsize: u64,   // fragment size
                f_blocks: u64,   // size of fs in f_frsize units
                f_bfree: u64,    // # free blocks
                f_bavail: u64,   // # free blocks for unprivileged users
                f_files: u64,    // # inodes
                f_ffree: u64,    // # free inodes
                f_favail: u64,   // # free inodes for unprivileged users
                f_fsid: u64,     // file system ID
                f_flag: u64,     // mount flags
                f_namemax: u64,  // maximum filename length
            }
            
            extern "C" {
                fn statvfs(path: *const i8, buf: *mut statvfs) -> i32;
            }
            
            unsafe {
                let mut buf: statvfs = mem::zeroed();
                let path_cstr = format!("{}\0", temp_dir.display());
                
                if statvfs(path_cstr.as_ptr() as *const i8, &mut buf) == 0 {
                    let available_bytes = buf.f_bavail * buf.f_frsize;
                    return available_bytes / (1024 * 1024); // Convert to MB
                }
            }
        }
        
        // Windows implementation
        #[cfg(target_family = "windows")]
        {
            // For Windows, we'd need to use GetDiskFreeSpaceEx from winapi
            // For now, try to estimate based on a test file write
        }
        
        // Fallback: Try to estimate available space by checking if we can create a test file
        let test_file = temp_dir.join(".ocr_space_test");
        let test_size = 100 * 1024 * 1024; // 100MB test
        
        match fs::write(&test_file, vec![0u8; test_size]) {
            Ok(_) => {
                let _ = fs::remove_file(&test_file);
                // If we can write 100MB, assume at least 1GB is available
                1000
            }
            Err(_) => {
                // If we can't write 100MB, report low space
                50
            }
        }
    }
    
    pub fn validate_cpu_requirements(&self) -> Result<(), OcrError> {
        let features = self.check_cpu_features();
        
        // Tesseract 4.x+ requires at least SSE2
        if !features.sse2 {
            return Err(OcrError::MissingCpuInstruction {
                instruction: "SSE2".to_string(),
            });
        }
        
        Ok(())
    }
    
    pub fn estimate_memory_requirement(&self, image_width: u32, image_height: u32) -> u64 {
        // Rough estimation: 4 bytes per pixel (RGBA) * 3 (for processing buffers)
        // Plus 100MB base overhead for Tesseract
        let pixels = (image_width as u64) * (image_height as u64);
        let image_memory = (pixels * 4 * 3) / (1024 * 1024); // Convert to MB
        image_memory + 100
    }
    
    pub fn validate_memory_for_image(&self, width: u32, height: u32) -> Result<(), OcrError> {
        let required = self.estimate_memory_requirement(width, height);
        let available = self.check_memory_available();
        
        if required > available {
            return Err(OcrError::InsufficientMemory { required, available });
        }
        
        Ok(())
    }
    
    pub fn get_full_diagnostics(&self) -> OcrDiagnostics {
        OcrDiagnostics {
            tesseract_version: self.check_tesseract_installation().ok(),
            available_languages: self.get_available_languages().unwrap_or_else(|_| vec![]),
            tessdata_path: None, // No longer managing tessdata paths
            cpu_features: self.check_cpu_features(),
            memory_available_mb: self.check_memory_available(),
            temp_space_available_mb: self.check_temp_space(),
        }
    }
    
    pub fn perform_full_health_check(&self) -> Result<OcrDiagnostics, Vec<OcrError>> {
        let mut errors = Vec::new();
        
        // Check Tesseract installation
        if let Err(e) = self.check_tesseract_installation() {
            errors.push(e);
        }
        
        // Check CPU requirements
        if let Err(e) = self.validate_cpu_requirements() {
            errors.push(e);
        }
        
        // Tessdata path no longer managed - Tesseract handles it automatically
        
        // Check for at least English language data
        if let Err(e) = self.check_language_data("eng") {
            errors.push(e);
        }
        
        let diagnostics = self.get_full_diagnostics();
        
        if errors.is_empty() {
            Ok(diagnostics)
        } else {
            Err(errors)
        }
    }
}