//! OCR queue statistics as the API reports them.

use crate::ocr::queue::QueueStats;

/// Returned by `GET /api/queue/stats` and the MCP `get_queue_stats` tool.
#[derive(Debug, serde::Serialize)]
pub struct QueueStatsResponse {
    pub pending: i64,
    pub processing: i64,
    pub failed: i64,
    pub completed_today: i64,
    pub avg_wait_time_minutes: Option<f64>,
    pub oldest_pending_minutes: Option<f64>,
}

impl From<QueueStats> for QueueStatsResponse {
    fn from(stats: QueueStats) -> Self {
        Self {
            pending: stats.pending_count,
            processing: stats.processing_count,
            failed: stats.failed_count,
            completed_today: stats.completed_today,
            avg_wait_time_minutes: stats.avg_wait_time_minutes,
            oldest_pending_minutes: stats.oldest_pending_minutes,
        }
    }
}
