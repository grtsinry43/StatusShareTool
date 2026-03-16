use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::{MediaInfo, StatusUpdate};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, uniffi::Enum, PartialEq, Eq)]
pub enum ReportReason {
    #[default]
    None,
    Initial,
    Changed,
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct ScheduleDecision {
    pub should_push: bool,
    pub reason: ReportReason,
    pub fingerprint: String,
}

#[derive(Debug, Default)]
struct SchedulerState {
    last_fingerprint: String,
    last_report_at: i64,
}

#[derive(Debug, uniffi::Object)]
pub struct PushScheduler {
    heartbeat_interval_secs: Mutex<u64>,
    state: Mutex<SchedulerState>,
}

#[uniffi::export]
impl PushScheduler {
    #[uniffi::constructor]
    pub fn new(heartbeat_interval_secs: u64) -> Self {
        Self {
            heartbeat_interval_secs: Mutex::new(heartbeat_interval_secs.max(5)),
            state: Mutex::new(SchedulerState::default()),
        }
    }

    pub fn update_interval(&self, heartbeat_interval_secs: u64) {
        *self.heartbeat_interval_secs.lock().unwrap() = heartbeat_interval_secs.max(5);
    }

    pub fn reset(&self) {
        *self.state.lock().unwrap() = SchedulerState::default();
    }

    pub fn plan(&self, update: Option<StatusUpdate>, now_secs: i64) -> ScheduleDecision {
        let Some(update) = normalize_status_update(update) else {
            return ScheduleDecision::default();
        };

        let fingerprint = serde_json::to_string(&update).unwrap_or_default();
        let heartbeat_interval_secs = *self.heartbeat_interval_secs.lock().unwrap() as i64;
        let state = self.state.lock().unwrap();

        if state.last_fingerprint.is_empty() {
            return ScheduleDecision {
                should_push: true,
                reason: ReportReason::Initial,
                fingerprint,
            };
        }

        if state.last_fingerprint != fingerprint {
            return ScheduleDecision {
                should_push: true,
                reason: ReportReason::Changed,
                fingerprint,
            };
        }

        if state.last_report_at <= 0 || now_secs - state.last_report_at >= heartbeat_interval_secs {
            return ScheduleDecision {
                should_push: true,
                reason: ReportReason::Heartbeat,
                fingerprint,
            };
        }

        ScheduleDecision {
            should_push: false,
            reason: ReportReason::None,
            fingerprint,
        }
    }

    pub fn mark_pushed(&self, fingerprint: String, now_secs: i64) {
        let mut state = self.state.lock().unwrap();
        state.last_fingerprint = fingerprint;
        state.last_report_at = now_secs;
    }
}

fn normalize_status_update(update: Option<StatusUpdate>) -> Option<StatusUpdate> {
    let update = update?;
    Some(StatusUpdate {
        ok: update.ok,
        process: trim_non_empty(update.process),
        extend: trim_non_empty(update.extend),
        media: update.media.and_then(clean_media),
        timestamp: update.timestamp,
    })
}

fn trim_non_empty(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn clean_media(media: MediaInfo) -> Option<MediaInfo> {
    let title = media.title.trim().to_string();
    let artist = media.artist.trim().to_string();
    let thumbnail = media.thumbnail.trim().to_string();
    if title.is_empty() && artist.is_empty() && thumbnail.is_empty() {
        None
    } else {
        Some(MediaInfo {
            title,
            artist,
            thumbnail,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{PushScheduler, ReportReason};
    use crate::StatusUpdate;

    #[test]
    fn changed_update_pushes_immediately() {
        let scheduler = PushScheduler::new(10);
        let first = scheduler.plan(
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Kitty".into()),
                extend: None,
                media: None,
                timestamp: Some(1),
            }),
            1,
        );
        assert_eq!(first.reason, ReportReason::Initial);
        scheduler.mark_pushed(first.fingerprint.clone(), 1);

        let changed = scheduler.plan(
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Firefox".into()),
                extend: None,
                media: None,
                timestamp: Some(2),
            }),
            2,
        );
        assert_eq!(changed.reason, ReportReason::Changed);
    }

    #[test]
    fn stable_update_becomes_heartbeat() {
        let scheduler = PushScheduler::new(10);
        let first = scheduler.plan(
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Kitty".into()),
                extend: None,
                media: None,
                timestamp: Some(1),
            }),
            1,
        );
        scheduler.mark_pushed(first.fingerprint.clone(), 1);

        let idle = scheduler.plan(
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Kitty".into()),
                extend: None,
                media: None,
                timestamp: Some(5),
            }),
            5,
        );
        assert!(!idle.should_push);

        let heartbeat = scheduler.plan(
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Kitty".into()),
                extend: None,
                media: None,
                timestamp: Some(11),
            }),
            11,
        );
        assert_eq!(heartbeat.reason, ReportReason::Heartbeat);
    }
}
