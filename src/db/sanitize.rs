//! The one place text and JSON are made storable in PostgreSQL.
//!
//! PostgreSQL cannot store U+0000 anywhere. In a TEXT/VARCHAR value it raises
//! `invalid byte sequence for encoding "UTF8": 0x00`, and inside JSON/JSONB the
//! `\u0000` escape serde emits for it raises `unsupported Unicode escape
//! sequence` (#901). OCR output, extracted document text, PDF/EXIF metadata,
//! filenames, WebDAV/S3 paths and request bodies can all carry one, so every
//! write of user- or file-derived text binds through this module rather than
//! through an ad-hoc filter at the call site:
//!
//! ```ignore
//! use crate::db::sanitize::ToDb;
//! sqlx::query("UPDATE documents SET ocr_text = $2, source_metadata = $3 WHERE id = $1")
//!     .bind(id)
//!     .bind(ocr_text.for_db())       // String / &str / Option<..> -> TEXT
//!     .bind(metadata.for_db())       // serde_json::Value          -> JSONB
//! ```
//!
//! [`DbText`] strips at encode time, so it costs nothing for the (normal) text
//! without a NUL and cannot be skipped by binding the wrong copy of a value.
//! The `sqlx::query!` macros type-check their arguments against `&str`, so they
//! take [`strip_nul`] instead.

use std::borrow::Cow;

use serde::Serialize;
use serde_json::Value;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgHasArrayType, PgTypeInfo};
use sqlx::{Encode, Postgres, Type};

/// `text` without U+0000, borrowed when there was nothing to remove.
pub fn strip_nul(text: &str) -> Cow<'_, str> {
    if text.contains('\0') {
        Cow::Owned(text.replace('\0', ""))
    } else {
        Cow::Borrowed(text)
    }
}

/// Owned, NUL-free copies of `values`, for `QueryBuilder::push_bind`, which
/// needs its arguments to outlive the builder.
pub fn strip_nul_vec(values: &[String]) -> Vec<String> {
    values.iter().map(|v| strip_nul(v).into_owned()).collect()
}

/// Removes U+0000 from every string and object key in `value`, in place.
pub fn strip_nul_json(value: &mut Value) {
    match value {
        Value::String(s) => {
            if let Cow::Owned(clean) = strip_nul(s) {
                *s = clean;
            }
        }
        Value::Array(items) => items.iter_mut().for_each(strip_nul_json),
        Value::Object(map) => {
            if map.keys().any(|k| k.contains('\0')) {
                *map = std::mem::take(map)
                    .into_iter()
                    .map(|(k, v)| (strip_nul(&k).into_owned(), v))
                    .collect();
            }
            map.values_mut().for_each(strip_nul_json);
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// A TEXT bind parameter that drops U+0000 as it is encoded.
#[derive(Debug, Clone, Copy)]
pub struct DbText<'a>(pub &'a str);

impl Type<Postgres> for DbText<'_> {
    fn type_info() -> PgTypeInfo {
        <str as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <str as Type<Postgres>>::compatible(ty)
    }
}

impl PgHasArrayType for DbText<'_> {
    fn array_type_info() -> PgTypeInfo {
        <&str as PgHasArrayType>::array_type_info()
    }

    fn array_compatible(ty: &PgTypeInfo) -> bool {
        <&str as PgHasArrayType>::array_compatible(ty)
    }
}

impl Encode<'_, Postgres> for DbText<'_> {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        // A 0x00 byte in UTF-8 is only ever U+0000, so filtering bytes is exact.
        let bytes = self.0.as_bytes();
        if bytes.contains(&0) {
            tracing::debug!("stripped U+0000 from a text value bound for PostgreSQL");
            buf.extend(bytes.iter().copied().filter(|&b| b != 0));
        } else {
            buf.extend_from_slice(bytes);
        }
        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        self.0.len()
    }
}

/// A JSON/JSONB bind parameter with U+0000 removed from its strings and keys.
#[derive(Debug, Clone, Copy)]
pub struct DbJson<'a, T: ?Sized = Value>(pub &'a T);

impl<T: ?Sized> Type<Postgres> for DbJson<'_, T> {
    fn type_info() -> PgTypeInfo {
        <sqlx::types::Json<Value> as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <sqlx::types::Json<Value> as Type<Postgres>>::compatible(ty)
    }
}

impl<T: Serialize + ?Sized> Encode<'_, Postgres> for DbJson<'_, T> {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let mut value = serde_json::to_value(self.0)?;
        strip_nul_json(&mut value);
        <sqlx::types::Json<Value> as Encode<'_, Postgres>>::encode_by_ref(
            &sqlx::types::Json(value),
            buf,
        )
    }
}

/// `.for_db()`: the sanitized bind parameter for a value headed to PostgreSQL.
pub trait ToDb {
    type Bind<'a>
    where
        Self: 'a;

    fn for_db(&self) -> Self::Bind<'_>;
}

impl ToDb for str {
    type Bind<'a> = DbText<'a>;
    fn for_db(&self) -> DbText<'_> {
        DbText(self)
    }
}

impl ToDb for String {
    type Bind<'a> = DbText<'a>;
    fn for_db(&self) -> DbText<'_> {
        DbText(self)
    }
}

impl ToDb for Value {
    type Bind<'a> = DbJson<'a>;
    fn for_db(&self) -> DbJson<'_> {
        DbJson(self)
    }
}

impl<T: ToDb + ?Sized> ToDb for &T {
    type Bind<'a>
        = T::Bind<'a>
    where
        Self: 'a;
    fn for_db(&self) -> Self::Bind<'_> {
        (**self).for_db()
    }
}

impl<T: ToDb> ToDb for Option<T> {
    type Bind<'a>
        = Option<T::Bind<'a>>
    where
        Self: 'a;
    fn for_db(&self) -> Self::Bind<'_> {
        self.as_ref().map(ToDb::for_db)
    }
}

/// TEXT[] columns, e.g. `tags`.
impl ToDb for [String] {
    type Bind<'a> = Vec<DbText<'a>>;
    fn for_db(&self) -> Vec<DbText<'_>> {
        self.iter().map(|s| DbText(s)).collect()
    }
}

impl ToDb for Vec<String> {
    type Bind<'a> = Vec<DbText<'a>>;
    fn for_db(&self) -> Vec<DbText<'_>> {
        self.as_slice().for_db()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn encoded<'q, T: Encode<'q, Postgres>>(value: T) -> Vec<u8> {
        let mut buf = PgArgumentBuffer::default();
        assert!(matches!(value.encode_by_ref(&mut buf).unwrap(), IsNull::No));
        buf.to_vec()
    }

    #[test]
    fn strip_nul_borrows_clean_text_and_removes_every_nul() {
        assert!(matches!(strip_nul("clean"), Cow::Borrowed("clean")));
        assert_eq!(strip_nul("\0a\0\0b\0"), "ab");
        assert_eq!(strip_nul("\0"), "");
        assert_eq!(strip_nul_vec(&["a\0".into(), "b".into()]), ["a", "b"]);
    }

    #[test]
    fn text_bind_encodes_without_nul_and_keeps_multibyte_text() {
        assert_eq!(
            encoded(DbText("Zürich\0 – 東京\0")),
            "Zürich – 東京".as_bytes()
        );
        assert_eq!(encoded(DbText("untouched")), b"untouched");
    }

    #[test]
    fn json_strips_strings_keys_and_nested_values() {
        // What a UTF-16BE PDF date string looks like after from_utf8_lossy (#901).
        let mut value = json!({
            "pdf_creation_date": "\u{FFFD}\u{FFFD}\0D\0:\x002\x000\x002\x005",
            "ke\0y": ["a\0", {"inner\0": "b\0"}, 1, null, true],
        });
        strip_nul_json(&mut value);
        assert_eq!(
            value,
            json!({
                "pdf_creation_date": "\u{FFFD}\u{FFFD}D:2025",
                "key": ["a", {"inner": "b"}, 1, null, true],
            })
        );
    }

    #[test]
    fn json_bind_never_emits_a_nul_escape() {
        let bytes = encoded(DbJson(&json!({"title": "a\0b"})));
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("\\u0000"),
            "encoded JSONB still carries a NUL escape: {text}"
        );
        assert!(text.contains("\"ab\""));
    }

    #[test]
    fn for_db_covers_the_shapes_bound_in_this_crate() {
        let tags = vec!["in\0voice".to_string(), "2025".to_string()];
        let owned: Option<String> = Some("x\0".into());
        let borrowed: Option<&str> = None;
        let meta: Option<Value> = Some(json!({"k": "v\0"}));

        assert_eq!(encoded(tags.for_db()), encoded(vec!["invoice", "2025"]));
        assert_eq!(encoded(owned.for_db().unwrap()), b"x");
        assert!(borrowed.for_db().is_none());
        assert!(!String::from_utf8_lossy(&encoded(meta.for_db().unwrap())).contains("\\u0000"));
    }
}
