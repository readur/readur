//! OCR queue priorities. Workers claim the highest priority first, so the
//! whole policy for ordering OCR work lives here: smaller files first, since
//! they finish quickly and give users results sooner.

const MB: i64 = 1024 * 1024;

/// Inclusive upper bounds of the file-size bands: up to 1 MB, 5 MB, 10 MB
/// and 50 MB, then everything larger.
const SIZE_BAND_LIMITS: [i64; 4] = [MB, 5 * MB, 10 * MB, 50 * MB];

/// Priority of a document queued by a user action (upload, single-document
/// retry), whatever its size.
pub const USER_PRIORITY: i32 = 5;

/// Index of the size band `bytes` falls in: 0 for the smallest files up to
/// 4 for files over 50 MB. A (bogus) negative size counts as small.
fn size_band(bytes: i64) -> usize {
    SIZE_BAND_LIMITS
        .iter()
        .position(|&limit| bytes <= limit)
        .unwrap_or(SIZE_BAND_LIMITS.len())
}

/// Priority of a document queued automatically on ingestion (watch folder,
/// batch import, source sync): 10, 8, 6, 4 or 2 by size band.
pub fn priority_for_file_size(bytes: i64) -> i32 {
    const PRIORITIES: [i32; 5] = [10, 8, 6, 4, 2];
    PRIORITIES[size_band(bytes)]
}

/// Priority of a document queued by a bulk OCR retry: 15, 12, 10, 8 or 6 by
/// size band, so retried documents run ahead of fresh ingestion of a
/// similar size.
pub fn retry_priority_for_file_size(bytes: i64) -> i32 {
    const PRIORITIES: [i32; 5] = [15, 12, 10, 8, 6];
    PRIORITIES[size_band(bytes)]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (size, ingestion priority, retry priority) at and around each band
    /// edge.
    const EDGES: [(i64, i32, i32); 11] = [
        (-1, 10, 15),
        (0, 10, 15),
        (MB, 10, 15),
        (MB + 1, 8, 12),
        (5 * MB, 8, 12),
        (5 * MB + 1, 6, 10),
        (10 * MB, 6, 10),
        (10 * MB + 1, 4, 8),
        (50 * MB, 4, 8),
        (50 * MB + 1, 2, 6),
        (i64::MAX, 2, 6),
    ];

    #[test]
    fn priorities_follow_the_size_bands() {
        for (bytes, ingestion, retry) in EDGES {
            assert_eq!(priority_for_file_size(bytes), ingestion, "{bytes} bytes");
            assert_eq!(retry_priority_for_file_size(bytes), retry, "{bytes} bytes");
        }
    }
}
