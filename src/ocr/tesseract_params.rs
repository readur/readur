//! The Tesseract options a document owner's OCR settings select, and how they
//! reach each OCR engine Readur runs: the tesseract library (images and PDF
//! pages rendered by pdftoppm), pdftoppm itself, and ocrmypdf.
//!
//! Everything here is a pure function of [`Settings`] so the mapping can be
//! tested without running an OCR engine.

use anyhow::Result;

use crate::models::Settings;

/// Lowest DPI pdftoppm renders at and Tesseract is told an image has.
pub const MIN_OCR_DPI: i32 = 70;
/// Highest DPI: a letter page at 1200 DPI is already a ~135-megapixel image.
pub const MAX_OCR_DPI: i32 = 1200;

/// Longest edge (pixels) preprocessing ever scales an image to, whatever the
/// user's maximum image size: larger images only cost memory and time.
const PREPROCESS_MAX_EDGE: u32 = 2048;
/// Images whose shortest edge is under this are scaled up before OCR.
const PREPROCESS_SMALL_EDGE: u32 = 300;
/// ...so that their shortest edge becomes this.
const PREPROCESS_SMALL_TARGET: f32 = 600.0;

/// The Tesseract options for one document, taken from its owner's settings.
#[derive(Debug, Clone, PartialEq)]
pub struct TesseractParams {
    /// Tesseract language string, e.g. `eng` or `deu+eng` (primary first).
    pub language: String,
    /// Page segmentation mode, 0-13 (`ocr_page_segmentation_mode`).
    pub page_segmentation_mode: i32,
    /// Engine mode, 0-3 (`ocr_engine_mode`).
    pub engine_mode: i32,
    /// Rendering / source resolution, clamped to `MIN_OCR_DPI..=MAX_OCR_DPI`.
    pub dpi: i32,
    /// Only these characters may be recognised (`tessedit_char_whitelist`).
    pub whitelist: Option<String>,
    /// These characters are never recognised (`tessedit_char_blacklist`).
    pub blacklist: Option<String>,
}

impl TesseractParams {
    pub fn from_settings(settings: &Settings) -> Self {
        let non_empty = |chars: &Option<String>| chars.clone().filter(|c| !c.is_empty());
        Self {
            language: language_combination(settings),
            page_segmentation_mode: clamp_or(settings.ocr_page_segmentation_mode, 0..=13, 3),
            engine_mode: clamp_or(settings.ocr_engine_mode, 0..=3, 3),
            dpi: settings.ocr_dpi.clamp(MIN_OCR_DPI, MAX_OCR_DPI),
            whitelist: non_empty(&settings.ocr_whitelist_chars),
            blacklist: non_empty(&settings.ocr_blacklist_chars),
        }
    }

    /// The `tessedit_char_*` variables to set, as `(name, value)`.
    pub fn character_variables(&self) -> Vec<(&'static str, &str)> {
        let mut vars = Vec::new();
        if let Some(whitelist) = &self.whitelist {
            vars.push(("tessedit_char_whitelist", whitelist.as_str()));
        }
        if let Some(blacklist) = &self.blacklist {
            vars.push(("tessedit_char_blacklist", blacklist.as_str()));
        }
        vars
    }

    /// The contents of a Tesseract config file setting the character
    /// variables, for ocrmypdf's `--tesseract-config`; `None` when there are none.
    pub fn tesseract_config_file(&self) -> Option<String> {
        let vars = self.character_variables();
        (!vars.is_empty()).then(|| {
            vars.iter()
                .map(|(name, value)| format!("{name} {value}\n"))
                .collect()
        })
    }

    /// The ocrmypdf options that carry these settings (the language replaces
    /// any `--language` a strategy names). `config_file` is the path of the
    /// written [`Self::tesseract_config_file`], when there is one.
    pub fn ocrmypdf_args(&self, config_file: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "--language".to_string(),
            self.language.clone(),
            "--tesseract-oem".to_string(),
            self.engine_mode.to_string(),
            "--tesseract-pagesegmode".to_string(),
            self.page_segmentation_mode.to_string(),
        ];
        if let Some(path) = config_file {
            args.push("--tesseract-config".to_string());
            args.push(path.to_string());
        }
        args
    }

    /// The pdftoppm options that render pages at these settings' DPI.
    pub fn pdftoppm_args(&self) -> Vec<String> {
        vec!["-png".to_string(), "-r".to_string(), self.dpi.to_string()]
    }
}

fn clamp_or(value: i32, range: std::ops::RangeInclusive<i32>, fallback: i32) -> i32 {
    if range.contains(&value) {
        value
    } else {
        fallback
    }
}

/// A strategy's ocrmypdf flags with its `--language <lang>` replaced by the
/// owner's options.
pub fn ocrmypdf_strategy_with_params(
    strategy_args: &[&str],
    params: &TesseractParams,
    config_file: Option<&str>,
) -> Vec<String> {
    let mut args = Vec::with_capacity(strategy_args.len() + 8);
    let mut iter = strategy_args.iter();
    while let Some(arg) = iter.next() {
        if *arg == "--language" || *arg == "-l" {
            iter.next();
            continue;
        }
        args.push(arg.to_string());
    }
    args.extend(params.ocrmypdf_args(config_file));
    args
}

/// Tesseract language string for the settings: every preferred language,
/// the primary one first, joined with `+`; `ocr_language` when none is set.
pub fn language_combination(settings: &Settings) -> String {
    if settings.preferred_languages.len() > 1 {
        let mut languages = settings.preferred_languages.clone();
        languages.retain(|lang| lang != &settings.primary_language);
        languages.insert(0, settings.primary_language.clone());
        languages.join("+")
    } else if let Some(only) = settings.preferred_languages.first() {
        only.clone()
    } else {
        settings.ocr_language.clone()
    }
}

/// The size preprocessing scales a `width` x `height` image to: small images
/// are scaled up, large ones down to at most 2048 px on the long edge and
/// within the owner's `ocr_max_image_width` x `ocr_max_image_height`; then
/// `ocr_upscale_factor` is applied, still within those limits.
pub fn preprocess_dimensions(width: u32, height: u32, settings: &Settings) -> (u32, u32) {
    if width == 0 || height == 0 {
        return (width, height);
    }
    let max_w = (settings.ocr_max_image_width.max(1) as u32).min(PREPROCESS_MAX_EDGE);
    let max_h = (settings.ocr_max_image_height.max(1) as u32).min(PREPROCESS_MAX_EDGE);
    let (w, h) = (width as f32, height as f32);

    let base = if width.min(height) < PREPROCESS_SMALL_EDGE {
        PREPROCESS_SMALL_TARGET / w.min(h)
    } else {
        1.0
    };
    let upscale = if settings.ocr_upscale_factor > 0.0 {
        settings.ocr_upscale_factor
    } else {
        1.0
    };
    let fit = (max_w as f32 / w).min(max_h as f32 / h);
    let scale = (base * upscale).min(fit);
    let scaled = |v: f32| ((v * scale).round() as u32).max(1);
    (scaled(w), scaled(h))
}

/// The 3x3 sharpening kernel for `ocr_sharpening_strength`: the centre pixel
/// gains `4 * strength` times the difference from its four neighbours. A
/// strength of 0 or less (sharpening chosen automatically for a blurry image)
/// uses strength 1.
pub fn sharpening_kernel(strength: f32) -> [[f32; 3]; 3] {
    let k = if strength > 0.0 { strength } else { 1.0 };
    [[0.0, -k, 0.0], [-k, 1.0 + 4.0 * k, -k], [0.0, -k, 0.0]]
}

#[cfg(feature = "ocr")]
mod engine {
    use super::TesseractParams;
    use anyhow::Result;
    use tesseract::{OcrEngineMode, PageSegMode, Tesseract};

    pub fn page_seg_mode(mode: i32) -> PageSegMode {
        match mode {
            0 => PageSegMode::PsmOsdOnly,
            1 => PageSegMode::PsmAutoOsd,
            2 => PageSegMode::PsmAutoOnly,
            4 => PageSegMode::PsmSingleColumn,
            5 => PageSegMode::PsmSingleBlockVertText,
            6 => PageSegMode::PsmSingleBlock,
            7 => PageSegMode::PsmSingleLine,
            8 => PageSegMode::PsmSingleWord,
            9 => PageSegMode::PsmCircleWord,
            10 => PageSegMode::PsmSingleChar,
            11 => PageSegMode::PsmSparseText,
            12 => PageSegMode::PsmSparseTextOsd,
            13 => PageSegMode::PsmRawLine,
            _ => PageSegMode::PsmAuto,
        }
    }

    pub fn engine_mode(mode: i32) -> OcrEngineMode {
        match mode {
            0 => OcrEngineMode::TesseractOnly,
            1 => OcrEngineMode::LstmOnly,
            2 => OcrEngineMode::TesseractLstmCombined,
            _ => OcrEngineMode::Default,
        }
    }

    /// A Tesseract instance for `image_path` with every option in `params`.
    /// The legacy engine modes (0 and 2) need legacy `.traineddata`; without
    /// it initialisation fails and the error says which mode was asked for.
    pub fn configure_tesseract(image_path: &str, params: &TesseractParams) -> Result<Tesseract> {
        let tesseract = Tesseract::new_with_oem(
            None,
            Some(&params.language),
            engine_mode(params.engine_mode),
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to initialise Tesseract for '{}' with OCR engine mode {}: {}",
                params.language,
                params.engine_mode,
                e
            )
        })?;
        let mut tesseract = tesseract
            .set_image(image_path)?
            .set_source_resolution(params.dpi);
        for (name, value) in params.character_variables() {
            tesseract = tesseract.set_variable(name, value)?;
        }
        tesseract.set_page_seg_mode(page_seg_mode(params.page_segmentation_mode));
        Ok(tesseract)
    }

    /// Tesseract's mean word confidence, clamped to 0-100.
    pub fn mean_confidence(tesseract: &mut Tesseract) -> f32 {
        (tesseract.mean_text_conf() as f32).clamp(0.0, 100.0)
    }
}

#[cfg(feature = "ocr")]
pub use engine::{configure_tesseract, engine_mode, mean_confidence, page_seg_mode};

/// Writes `params`' Tesseract config file (if any) into `temp_dir` and
/// returns its path; the caller removes it. A write that fails or is dropped
/// part-way leaves no file behind.
pub async fn write_tesseract_config(
    params: &TesseractParams,
    temp_dir: &str,
) -> Result<Option<String>> {
    let Some(contents) = params.tesseract_config_file() else {
        return Ok(None);
    };
    let path = crate::utils::temp_files::TempPath::new(format!(
        "{}/tesseract_{}.cfg",
        temp_dir,
        uuid::Uuid::new_v4()
    ));
    tokio::fs::write(path.as_str(), contents).await?;
    Ok(Some(path.keep()))
}

#[cfg(test)]
#[path = "tesseract_params_tests.rs"]
mod tests;
