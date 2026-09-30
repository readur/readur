//! Per-lane document arrival counts (`GET /api/sources/arrivals`).
//!
//! Every document belongs to exactly one lane:
//! - a configured source, when `documents.source_id` is set;
//! - the watch folder, when there is no source and `source_type` is
//!   `watch_folder` (set by `scheduling::watcher`);
//! - uploads otherwise: `web_upload`, `direct_upload`, `batch_ingest`, NULL,
//!   and documents whose source was deleted (`source_id` is `ON DELETE SET NULL`).
//!
//! Days are UTC calendar days.

use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use sqlx::{Postgres, QueryBuilder, Row};
use uuid::Uuid;

use super::documents::apply_role_based_filter;
use super::Database;
use crate::models::{DayCount, SourceArrivals, UserRole, UPLOAD_LANE_KEY, WATCH_LANE_KEY};

/// The lane a group of documents belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrivalLane {
    Source(Uuid),
    Watch,
    Upload,
}

impl ArrivalLane {
    fn from_row(source_id: Option<Uuid>, is_watch: bool) -> Self {
        match (source_id, is_watch) {
            (Some(id), _) => ArrivalLane::Source(id),
            (None, true) => ArrivalLane::Watch,
            (None, false) => ArrivalLane::Upload,
        }
    }
}

/// One grouped row: `count` documents of a lane on `day`, the newest at
/// `last_at`. A group with no `day` only carries the lane's newest arrival of
/// all time, which may predate the window.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrivalGroup {
    pub lane: ArrivalLane,
    pub day: Option<NaiveDate>,
    pub count: i64,
    pub last_at: DateTime<Utc>,
}

/// The source fields a lane needs.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrivalSource {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    pub status: String,
}

/// `(is_watch, predicate)` of the two sourceless lanes, as used in the
/// last-arrival lookups. Each predicate must match the WHERE clause of its
/// partial index in `migrations/20260930000003_add_documents_arrivals_indexes.sql`
/// (beyond `source_id IS NULL`), or the lookup cannot use that index.
const LANE_PREDICATES: [(&str, &str); 2] = [
    ("TRUE", "d.source_type = 'watch_folder'"),
    ("FALSE", "d.source_type IS DISTINCT FROM 'watch_folder'"),
];

/// First day of a `days`-long window ending on `today` (inclusive).
pub fn window_start(today: NaiveDate, days: i64) -> NaiveDate {
    today - Duration::days(days.max(1) - 1)
}

#[derive(Default)]
struct LaneTotals {
    per_day: HashMap<NaiveDate, i64>,
    last_at: Option<DateTime<Utc>>,
}

fn totals_by_lane(groups: &[ArrivalGroup]) -> HashMap<ArrivalLane, LaneTotals> {
    let mut lanes: HashMap<ArrivalLane, LaneTotals> = HashMap::new();
    for group in groups {
        let totals = lanes.entry(group.lane).or_default();
        if let Some(day) = group.day {
            *totals.per_day.entry(day).or_insert(0) += group.count;
        }
        totals.last_at = totals.last_at.max(Some(group.last_at));
    }
    lanes
}

struct LaneMeta {
    key: String,
    source_id: Option<Uuid>,
    kind: String,
    name: String,
    enabled: bool,
    status: Option<String>,
}

fn build_lane(
    meta: LaneMeta,
    totals: Option<&LaneTotals>,
    today: NaiveDate,
    days: i64,
) -> SourceArrivals {
    let start = window_start(today, days);
    let day_counts: Vec<DayCount> = start
        .iter_days()
        .take_while(|d| *d <= today)
        .map(|date| DayCount {
            date,
            count: totals
                .and_then(|t| t.per_day.get(&date))
                .copied()
                .unwrap_or(0),
        })
        .collect();
    let today_count = day_counts.last().map_or(0, |d| d.count);
    SourceArrivals {
        key: meta.key,
        source_id: meta.source_id,
        kind: meta.kind,
        name: meta.name,
        days: day_counts,
        today: today_count,
        last_arrival_at: totals.and_then(|t| t.last_at),
        enabled: meta.enabled,
        status: meta.status,
    }
}

/// Builds one zero-filled lane per source (in the given order), then the watch
/// folder and uploads lanes. Groups of sources not in `sources` are dropped.
pub fn build_arrivals(
    sources: &[ArrivalSource],
    groups: &[ArrivalGroup],
    today: NaiveDate,
    days: i64,
) -> Vec<SourceArrivals> {
    let lanes = totals_by_lane(groups);
    let source_metas = sources.iter().map(|s| {
        let meta = LaneMeta {
            key: s.id.to_string(),
            source_id: Some(s.id),
            kind: s.kind.clone(),
            name: s.name.clone(),
            enabled: s.enabled,
            status: Some(s.status.clone()),
        };
        (ArrivalLane::Source(s.id), meta)
    });
    let fixed = [
        (ArrivalLane::Watch, WATCH_LANE_KEY, "Watch folder"),
        (ArrivalLane::Upload, UPLOAD_LANE_KEY, "Uploads"),
    ]
    .into_iter()
    .map(|(lane, key, name)| {
        let meta = LaneMeta {
            key: key.to_string(),
            source_id: None,
            kind: key.to_string(),
            name: name.to_string(),
            enabled: true,
            status: None,
        };
        (lane, meta)
    });
    source_metas
        .chain(fixed)
        .map(|(lane, meta)| build_lane(meta, lanes.get(&lane), today, days))
        .collect()
}

impl Database {
    /// Sources visible to the caller: admins see every source (matching the
    /// document list, where admins see every document), users their own.
    async fn arrival_sources(&self, user_id: Uuid, role: UserRole) -> Result<Vec<ArrivalSource>> {
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT id, name, source_type, enabled, status FROM sources WHERE 1=1",
        );
        if role != UserRole::Admin {
            query.push(" AND user_id = ");
            query.push_bind(user_id);
        }
        query.push(" ORDER BY lower(name), id");
        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| ArrivalSource {
                id: row.get("id"),
                name: row.get("name"),
                kind: row.get("source_type"),
                enabled: row.get("enabled"),
                status: row.get("status"),
            })
            .collect())
    }

    /// Visible documents inside the window, grouped by lane and UTC day. The
    /// `created_at >= since` range keeps this to the window's rows
    /// (`idx_documents_created_at_id`, or `idx_documents_user_created_at` for
    /// a user).
    async fn arrival_window_groups(
        &self,
        user_id: Uuid,
        role: UserRole,
        since: DateTime<Utc>,
    ) -> Result<Vec<ArrivalGroup>> {
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT source_id, \
             COALESCE(source_id IS NULL AND source_type = 'watch_folder', FALSE) AS is_watch, \
             (created_at AT TIME ZONE 'UTC')::date AS day, \
             COUNT(*) AS count, MAX(created_at) AS last_at \
             FROM documents WHERE created_at >= ",
        );
        query.push_bind(since);
        apply_role_based_filter(&mut query, user_id, role);
        query.push(" GROUP BY 1, 2, 3");
        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|row| ArrivalGroup {
                lane: ArrivalLane::from_row(row.get("source_id"), row.get("is_watch")),
                day: Some(row.get("day")),
                count: row.get("count"),
                last_at: row.get("last_at"),
            })
            .collect())
    }

    /// Newest arrival of all time per lane, as dayless groups. Each lookup is
    /// one probe of an expression index from
    /// `migrations/20260930000003_add_documents_arrivals_indexes.sql`: per
    /// source, and per user for the watch and upload lanes. The lookups order
    /// by `created_at AT TIME ZONE 'UTC'`, which only those indexes provide, so
    /// the planner can't walk `idx_documents_created_at_id` hunting for a quiet
    /// lane's newest row.
    async fn arrival_last_groups(
        &self,
        user_id: Uuid,
        role: UserRole,
    ) -> Result<Vec<ArrivalGroup>> {
        let is_user = role != UserRole::Admin;
        let mut query = QueryBuilder::<Postgres>::new(
            "SELECT s.id AS source_id, FALSE AS is_watch, la.utc AT TIME ZONE 'UTC' AS last_at \
             FROM sources s CROSS JOIN LATERAL (\
             SELECT d.created_at AT TIME ZONE 'UTC' AS utc FROM documents d \
             WHERE d.source_id = s.id",
        );
        if is_user {
            query.push(" AND d.user_id = ");
            query.push_bind(user_id);
        }
        query.push(" ORDER BY d.created_at AT TIME ZONE 'UTC' DESC LIMIT 1) la");
        if is_user {
            query.push(" WHERE s.user_id = ");
            query.push_bind(user_id);
        }
        for (is_watch, predicate) in LANE_PREDICATES {
            query.push(format!(
                " UNION ALL SELECT NULL::uuid, {}, MAX(la.utc) AT TIME ZONE 'UTC' \
                 FROM users u CROSS JOIN LATERAL (\
                 SELECT d.created_at AT TIME ZONE 'UTC' AS utc FROM documents d \
                 WHERE d.user_id = u.id AND d.source_id IS NULL AND {} \
                 ORDER BY d.created_at AT TIME ZONE 'UTC' DESC LIMIT 1) la",
                is_watch, predicate
            ));
            if is_user {
                query.push(" WHERE u.id = ");
                query.push_bind(user_id);
            }
        }
        let rows = query.build().fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .filter_map(|row| {
                let last_at: Option<DateTime<Utc>> = row.get("last_at");
                last_at.map(|last_at| ArrivalGroup {
                    lane: ArrivalLane::from_row(row.get("source_id"), row.get("is_watch")),
                    day: None,
                    count: 0,
                    last_at,
                })
            })
            .collect())
    }

    /// Arrival lanes for the last `days` UTC days, ending today.
    pub async fn get_source_arrivals(
        &self,
        user_id: Uuid,
        role: UserRole,
        days: i64,
    ) -> Result<Vec<SourceArrivals>> {
        let today = Utc::now().date_naive();
        let since = window_start(today, days)
            .and_hms_opt(0, 0, 0)
            .expect("midnight is a valid time")
            .and_utc();
        let sources = self.arrival_sources(user_id, role).await?;
        let mut groups = self.arrival_window_groups(user_id, role, since).await?;
        groups.extend(self.arrival_last_groups(user_id, role).await?);
        Ok(build_arrivals(&sources, &groups, today, days))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn date(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, d).unwrap()
    }

    fn at(d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, d, h, 0, 0).unwrap()
    }

    fn source(name: &str, kind: &str) -> ArrivalSource {
        ArrivalSource {
            id: Uuid::new_v4(),
            name: name.to_string(),
            kind: kind.to_string(),
            enabled: true,
            status: "idle".to_string(),
        }
    }

    #[test]
    fn window_start_is_inclusive_of_today() {
        assert_eq!(window_start(date(30), 1), date(30));
        assert_eq!(window_start(date(30), 14), date(17));
    }

    #[test]
    fn lanes_are_zero_filled_and_include_watch_and_upload() {
        let src = source("Scans", "local_folder");
        let lanes = build_arrivals(std::slice::from_ref(&src), &[], date(30), 3);
        let keys: Vec<&str> = lanes.iter().map(|l| l.key.as_str()).collect();
        assert_eq!(keys, vec![src.id.to_string().as_str(), "watch", "upload"]);
        for lane in &lanes {
            assert_eq!(lane.days.len(), 3);
            assert_eq!(lane.days[0].date, date(28));
            assert_eq!(lane.days[2].date, date(30));
            assert!(lane.days.iter().all(|d| d.count == 0));
            assert_eq!(lane.today, 0);
            assert_eq!(lane.last_arrival_at, None);
        }
        assert_eq!(lanes[0].kind, "local_folder");
        assert_eq!(lanes[0].status.as_deref(), Some("idle"));
        assert_eq!(lanes[1].kind, "watch");
        assert_eq!(lanes[1].name, "Watch folder");
        assert_eq!(lanes[2].kind, "upload");
        assert_eq!(lanes[2].status, None);
        assert!(lanes[2].enabled);
    }

    #[test]
    fn groups_fill_days_today_and_last_arrival() {
        let src = source("Nextcloud", "webdav");
        let groups = vec![
            ArrivalGroup {
                lane: ArrivalLane::Source(src.id),
                day: Some(date(29)),
                count: 2,
                last_at: at(29, 10),
            },
            ArrivalGroup {
                lane: ArrivalLane::Source(src.id),
                day: Some(date(30)),
                count: 5,
                last_at: at(30, 8),
            },
            ArrivalGroup {
                lane: ArrivalLane::Upload,
                day: None,
                count: 7,
                last_at: at(1, 9),
            },
            ArrivalGroup {
                lane: ArrivalLane::Watch,
                day: Some(date(28)),
                count: 1,
                last_at: at(28, 1),
            },
            // A source the caller cannot see is dropped.
            ArrivalGroup {
                lane: ArrivalLane::Source(Uuid::new_v4()),
                day: Some(date(30)),
                count: 9,
                last_at: at(30, 9),
            },
        ];
        let lanes = build_arrivals(std::slice::from_ref(&src), &groups, date(30), 3);
        assert_eq!(lanes.len(), 3);
        let counts: Vec<i64> = lanes[0].days.iter().map(|d| d.count).collect();
        assert_eq!(counts, vec![0, 2, 5]);
        assert_eq!(lanes[0].today, 5);
        assert_eq!(lanes[0].last_arrival_at, Some(at(30, 8)));
        assert_eq!(lanes[1].days[0].count, 1);
        assert_eq!(lanes[1].today, 0);
        assert!(lanes[2].days.iter().all(|d| d.count == 0));
        assert_eq!(lanes[2].last_arrival_at, Some(at(1, 9)));
    }

    #[test]
    fn lane_from_row_prefers_the_source() {
        let id = Uuid::new_v4();
        assert_eq!(
            ArrivalLane::from_row(Some(id), true),
            ArrivalLane::Source(id)
        );
        assert_eq!(ArrivalLane::from_row(None, true), ArrivalLane::Watch);
        assert_eq!(ArrivalLane::from_row(None, false), ArrivalLane::Upload);
    }
}
