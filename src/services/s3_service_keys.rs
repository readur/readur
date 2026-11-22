//! Object-key layout of the S3 storage backend, and the classification of
//! "object is missing" errors. Included from `s3_service.rs` as `keys`.
//!
//! Layout (every key below `S3_PREFIX` when one is configured):
//! - `documents/<user>/<yyyy>/<mm>/<document>.<ext>` -- the month is the
//!   upload month, so a document's key can only be known from what was
//!   recorded at upload (`documents.file_path`), never recomputed later.
//! - `thumbnails/<user>/<document>_thumb.jpg`
//! - `processed_images/<user>/<document>_processed.png`

use chrono::{DateTime, Datelike, Utc};
use uuid::Uuid;

/// Scheme of the storage paths the backend records (`s3://<object key>`).
pub(super) const STORAGE_PATH_SCHEME: &str = "s3://";

/// `S3_PREFIX` normalised: surrounding whitespace and slashes removed.
/// `None` when unset or empty, so "", "/" and None all mean "no prefix".
pub(super) fn normalized_prefix(prefix: Option<&str>) -> Option<&str> {
    prefix
        .map(|p| p.trim().trim_matches('/'))
        .filter(|p| !p.is_empty())
}

/// `relative` placed below the configured prefix.
pub(super) fn prefixed(prefix: Option<&str>, relative: &str) -> String {
    match normalized_prefix(prefix) {
        Some(p) => format!("{p}/{relative}"),
        None => relative.to_string(),
    }
}

/// Key for a newly stored document, dated by `now` (the upload time).
pub(super) fn document_key(
    prefix: Option<&str>,
    user_id: Uuid,
    document_id: Uuid,
    filename: &str,
    now: DateTime<Utc>,
) -> String {
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    let name = if extension.is_empty() {
        document_id.to_string()
    } else {
        format!("{document_id}.{extension}")
    };
    prefixed(
        prefix,
        &format!(
            "documents/{}/{:04}/{:02}/{}",
            user_id,
            now.year(),
            now.month(),
            name
        ),
    )
}

fn thumbnail_relative(user_id: Uuid, document_id: Uuid) -> String {
    format!("thumbnails/{user_id}/{document_id}_thumb.jpg")
}

fn processed_image_relative(user_id: Uuid, document_id: Uuid) -> String {
    format!("processed_images/{user_id}/{document_id}_processed.png")
}

pub(super) fn thumbnail_key(prefix: Option<&str>, user_id: Uuid, document_id: Uuid) -> String {
    prefixed(prefix, &thumbnail_relative(user_id, document_id))
}

pub(super) fn processed_image_key(
    prefix: Option<&str>,
    user_id: Uuid,
    document_id: Uuid,
) -> String {
    prefixed(prefix, &processed_image_relative(user_id, document_id))
}

/// Keys of a document's derived objects (thumbnail, processed image) to
/// delete, labelled for error reports. Their keys are not recorded, so both
/// the current layout and, when a prefix is set, the un-prefixed layout used
/// before the prefix was configured are covered: turning `S3_PREFIX` on never
/// strands an object written without it.
pub(super) fn derived_object_keys(
    prefix: Option<&str>,
    user_id: Uuid,
    document_id: Uuid,
) -> Vec<(&'static str, String)> {
    let relatives = [
        ("Thumbnail", thumbnail_relative(user_id, document_id)),
        (
            "Processed image",
            processed_image_relative(user_id, document_id),
        ),
    ];
    let mut keys = Vec::with_capacity(4);
    for (label, relative) in relatives {
        let current = prefixed(prefix, &relative);
        if current != relative {
            keys.push((label, relative));
        }
        keys.push((label, current));
    }
    keys
}

/// The object key named by a recorded storage path (`documents.file_path`),
/// which the backend writes as `s3://<object key>` -- the full key, prefix
/// included, exactly as uploaded. `None` for a path that is not an S3
/// storage path (a local file from before S3 storage was enabled) or that
/// names no key.
pub(super) fn key_from_stored_path(path: &str) -> Option<&str> {
    path.strip_prefix(STORAGE_PATH_SCHEME)
        .filter(|key| !key.is_empty())
}

/// Whether a failed request says the object does not exist, decided from
/// the typed service error, never from message text: S3 error code
/// `NoSuchKey`/`NotFound`, or a 404 that carries no error code (HEAD-style
/// responses have no body). A 404 with any other code -- `NoSuchBucket`
/// above all -- is a real failure.
#[cfg(feature = "s3")]
pub(super) fn is_missing_object<E>(
    err: &aws_sdk_s3::error::SdkError<E, aws_sdk_s3::config::http::HttpResponse>,
) -> bool
where
    E: aws_sdk_s3::error::ProvideErrorMetadata,
{
    let aws_sdk_s3::error::SdkError::ServiceError(ctx) = err else {
        return false;
    };
    match ctx.err().code() {
        Some(code) => code == "NoSuchKey" || code == "NotFound",
        None => ctx.raw().status().as_u16() == 404,
    }
}

/// Whether a failed ranged GetObject was refused because the range cannot
/// be satisfied (`InvalidRange`, HTTP 416). Every range readur sends starts
/// at byte 0, so this means the object is empty.
#[cfg(feature = "s3")]
pub(super) fn is_unsatisfiable_range<E>(
    err: &aws_sdk_s3::error::SdkError<E, aws_sdk_s3::config::http::HttpResponse>,
) -> bool
where
    E: aws_sdk_s3::error::ProvideErrorMetadata,
{
    let aws_sdk_s3::error::SdkError::ServiceError(ctx) = err else {
        return false;
    };
    ctx.err().code() == Some("InvalidRange") || ctx.raw().status().as_u16() == 416
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ids() -> (Uuid, Uuid) {
        (
            Uuid::parse_str("00000000-0000-0000-0000-0000000000aa").unwrap(),
            Uuid::parse_str("00000000-0000-0000-0000-0000000000bb").unwrap(),
        )
    }

    #[test]
    fn prefix_is_normalised() {
        assert_eq!(normalized_prefix(None), None);
        assert_eq!(normalized_prefix(Some("")), None);
        assert_eq!(normalized_prefix(Some(" / ")), None);
        assert_eq!(normalized_prefix(Some("/tenant/a/")), Some("tenant/a"));
        assert_eq!(
            prefixed(Some("tenant/"), "documents/x"),
            "tenant/documents/x"
        );
        assert_eq!(prefixed(Some("/"), "documents/x"), "documents/x");
    }

    #[test]
    fn document_key_uses_upload_month_and_prefix() {
        let (user, doc) = ids();
        let when = Utc.with_ymd_and_hms(2024, 3, 9, 12, 0, 0).unwrap();
        assert_eq!(
            document_key(None, user, doc, "Scan.PDF", when),
            format!("documents/{user}/2024/03/{doc}.PDF")
        );
        assert_eq!(
            document_key(Some("tenant"), user, doc, "noext", when),
            format!("tenant/documents/{user}/2024/03/{doc}")
        );
    }

    #[test]
    fn derived_keys_cover_legacy_unprefixed_layout_only_when_prefixed() {
        let (user, doc) = ids();
        let plain: Vec<String> = derived_object_keys(None, user, doc)
            .into_iter()
            .map(|(_, k)| k)
            .collect();
        assert_eq!(
            plain,
            vec![
                thumbnail_key(None, user, doc),
                processed_image_key(None, user, doc)
            ]
        );

        let with_prefix: Vec<String> = derived_object_keys(Some("p"), user, doc)
            .into_iter()
            .map(|(_, k)| k)
            .collect();
        assert_eq!(
            with_prefix,
            vec![
                thumbnail_key(None, user, doc),
                thumbnail_key(Some("p"), user, doc),
                processed_image_key(None, user, doc),
                processed_image_key(Some("p"), user, doc),
            ]
        );
    }

    #[test]
    fn stored_path_yields_the_full_recorded_key() {
        assert_eq!(
            key_from_stored_path("s3://p/documents/u/2024/03/d.pdf"),
            Some("p/documents/u/2024/03/d.pdf")
        );
        assert_eq!(key_from_stored_path("s3://"), None);
        assert_eq!(key_from_stored_path("./uploads/documents/d.pdf"), None);
    }

    #[cfg(feature = "s3")]
    fn get_object_failure(
        status: u16,
        code: Option<&str>,
    ) -> aws_sdk_s3::error::SdkError<
        aws_sdk_s3::operation::get_object::GetObjectError,
        aws_sdk_s3::config::http::HttpResponse,
    > {
        use aws_sdk_s3::error::ErrorMetadata;
        use aws_sdk_s3::operation::get_object::GetObjectError;
        use aws_sdk_s3::primitives::SdkBody;
        use aws_smithy_runtime_api::http::{Response, StatusCode};

        let mut meta = ErrorMetadata::builder();
        if let Some(code) = code {
            meta = meta.code(code);
        }
        aws_sdk_s3::error::SdkError::service_error(
            GetObjectError::generic(meta.build()),
            Response::new(StatusCode::try_from(status).unwrap(), SdkBody::empty()),
        )
    }

    /// A ranged read of an empty object is refused as `InvalidRange` / 416;
    /// that, and only that, is classified as an unsatisfiable range.
    #[cfg(feature = "s3")]
    #[test]
    fn unsatisfiable_range_is_told_from_typed_errors() {
        assert!(is_unsatisfiable_range(&get_object_failure(
            416,
            Some("InvalidRange")
        )));
        assert!(is_unsatisfiable_range(&get_object_failure(416, None)));
        assert!(!is_unsatisfiable_range(&get_object_failure(
            404,
            Some("NoSuchKey")
        )));
        assert!(!is_unsatisfiable_range(&get_object_failure(
            403,
            Some("AccessDenied")
        )));
        // A missing object stays a missing object under a ranged read.
        assert!(is_missing_object(&get_object_failure(
            404,
            Some("NoSuchKey")
        )));
    }
}
