use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OcrError {
    #[error("Tesseract is not installed on the system")]
    TesseractNotInstalled,
    
    #[error("Tesseract language data not found for '{lang}'. Please install tesseract-ocr-{lang}")]
    LanguageDataNotFound { lang: String },
    
    #[error("TESSDATA_PREFIX environment variable not set or invalid: {path}")]
    TessdataPathInvalid { path: String },
    
    #[error("Insufficient memory for OCR operation. Required: {required}MB, Available: {available}MB")]
    InsufficientMemory { required: u64, available: u64 },
    
    #[error("CPU instruction set missing: {instruction}. Tesseract requires {instruction} support")]
    MissingCpuInstruction { instruction: String },
    
    #[error("Image too large for OCR. Max dimensions: {max_width}x{max_height}, Actual: {width}x{height}")]
    ImageTooLarge {
        width: u32,
        height: u32,
        max_width: u32,
        max_height: u32,
    },
    
    #[error("Invalid image format or corrupted image: {details}")]
    InvalidImageFormat { details: String },
    
    #[error("OCR timeout after {seconds} seconds. Consider reducing image size or quality")]
    OcrTimeout { seconds: u64 },
    
    #[error("Permission denied accessing file: {path}")]
    PermissionDenied { path: String },
    
    #[error("Tesseract initialization failed: {details}")]
    InitializationFailed { details: String },
    
    #[error("OCR quality too low. Confidence score: {score}% (minimum: {threshold}%)")]
    LowConfidence { score: f32, threshold: f32 },
    
    #[error("Hardware acceleration not available: {details}")]
    HardwareAccelerationUnavailable { details: String },
    
    #[error(transparent)]
    Io(#[from] std::io::Error),
    
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl OcrError {
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            OcrError::InsufficientMemory { .. }
                | OcrError::OcrTimeout { .. }
                | OcrError::LowConfidence { .. }
        )
    }
    
    pub fn is_configuration_error(&self) -> bool {
        matches!(
            self,
            OcrError::TesseractNotInstalled
                | OcrError::LanguageDataNotFound { .. }
                | OcrError::TessdataPathInvalid { .. }
                | OcrError::MissingCpuInstruction { .. }
        )
    }
    
    pub fn error_code(&self) -> &'static str {
        match self {
            OcrError::TesseractNotInstalled => "OCR_NOT_INSTALLED",
            OcrError::LanguageDataNotFound { .. } => "OCR_LANG_MISSING",
            OcrError::TessdataPathInvalid { .. } => "OCR_DATA_PATH_INVALID",
            OcrError::InsufficientMemory { .. } => "OCR_OUT_OF_MEMORY",
            OcrError::MissingCpuInstruction { .. } => "OCR_CPU_UNSUPPORTED",
            OcrError::ImageTooLarge { .. } => "OCR_IMAGE_TOO_LARGE",
            OcrError::InvalidImageFormat { .. } => "OCR_INVALID_FORMAT",
            OcrError::OcrTimeout { .. } => "OCR_TIMEOUT",
            OcrError::PermissionDenied { .. } => "OCR_PERMISSION_DENIED",
            OcrError::InitializationFailed { .. } => "OCR_INIT_FAILED",
            OcrError::LowConfidence { .. } => "OCR_LOW_CONFIDENCE",
            OcrError::HardwareAccelerationUnavailable { .. } => "OCR_NO_HW_ACCEL",
            OcrError::Io(_) => "OCR_IO_ERROR",
            OcrError::Other(_) => "OCR_UNKNOWN_ERROR",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OcrDiagnostics {
    pub tesseract_version: Option<String>,
    pub available_languages: Vec<String>,
    pub tessdata_path: Option<String>,
    pub cpu_features: CpuFeatures,
    pub memory_available_mb: u64,
    pub temp_space_available_mb: u64,
}

#[derive(Debug, Clone)]
pub struct CpuFeatures {
    pub sse2: bool,
    pub sse3: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,
    pub avx: bool,
    pub avx2: bool,
}

impl fmt::Display for OcrDiagnostics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "OCR Diagnostics:")?;
        writeln!(f, "  Tesseract Version: {}", self.tesseract_version.as_deref().unwrap_or("Not installed"))?;
        writeln!(f, "  Tessdata Path: {}", self.tessdata_path.as_deref().unwrap_or("Not set"))?;
        writeln!(f, "  Available Languages: {}", self.available_languages.join(", "))?;
        writeln!(f, "  Memory Available: {} MB", self.memory_available_mb)?;
        writeln!(f, "  Temp Space: {} MB", self.temp_space_available_mb)?;
        writeln!(f, "  CPU Features:")?;
        writeln!(f, "    SSE2: {}", self.cpu_features.sse2)?;
        writeln!(f, "    SSE4.1: {}", self.cpu_features.sse4_1)?;
        writeln!(f, "    AVX: {}", self.cpu_features.avx)?;
        writeln!(f, "    AVX2: {}", self.cpu_features.avx2)?;
        Ok(())
    }
}