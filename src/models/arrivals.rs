//! Per-source arrival counts for the Home "is it flowing?" lanes
//! (`GET /api/sources/arrivals`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

/// Default number of days returned by `GET /api/sources/arrivals`.
pub const DEFAULT_ARRIVAL_DAYS: i64 = 14;
/// Largest accepted `days` value.
pub const MAX_ARRIVAL_DAYS: i64 = 60;

/// Lane key and kind of documents uploaded through the web UI or API, plus any
/// other document without a source that did not come from the watch folder.
pub const UPLOAD_LANE_KEY: &str = "upload";
/// Lane key and kind of documents ingested by the watch folder.
pub const WATCH_LANE_KEY: &str = "watch";

/// Query parameters of `GET /api/sources/arrivals`.
#[derive(Debug, Default, Deserialize, IntoParams)]
pub struct ArrivalsQuery {
    /// Number of UTC days to return, ending today (1-60, default 14)
    pub days: Option<i64>,
}

impl ArrivalsQuery {
    /// The validated day count.
    pub fn days(&self) -> Result<i64, String> {
        let days = self.days.unwrap_or(DEFAULT_ARRIVAL_DAYS);
        if (1..=MAX_ARRIVAL_DAYS).contains(&days) {
            Ok(days)
        } else {
            Err(format!("days must be between 1 and {}", MAX_ARRIVAL_DAYS))
        }
    }
}

/// Documents that arrived on one UTC calendar day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct DayCount {
    /// UTC calendar day, `YYYY-MM-DD`
    #[schema(value_type = String, format = Date, example = "2026-09-30")]
    #[ts(type = "string")]
    pub date: NaiveDate,
    pub count: i64,
}

/// Arrivals for one ingestion lane: a configured source, the watch folder or
/// uploads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema, TS)]
#[ts(export)]
pub struct SourceArrivals {
    /// Stable lane key: the source id for sources, `"watch"` or `"upload"`
    pub key: String,
    /// The source id; null for the watch folder and uploads
    pub source_id: Option<Uuid>,
    /// `"upload"`, `"watch"`, `"webdav"`, `"s3"` or `"local_folder"`
    pub kind: String,
    /// Source name, or "Watch folder" / "Uploads"
    pub name: String,
    /// One entry per UTC day, oldest first, ending today; zero-filled
    pub days: Vec<DayCount>,
    /// Documents that arrived today (UTC); equals the last `days` entry
    pub today: i64,
    /// Newest arrival of all time in this lane
    pub last_arrival_at: Option<DateTime<Utc>>,
    /// Whether the source is enabled; always true for watch and upload
    pub enabled: bool,
    /// Source status (`idle`, `syncing`, `error`); null for watch and upload
    pub status: Option<String>,
}
