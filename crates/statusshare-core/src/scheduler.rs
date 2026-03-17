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

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct SchedulerSnapshot {
    pub heartbeat_interval_secs: u64,
    pub last_fingerprint: String,
    pub last_report_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, uniffi::Record)]
pub struct SchedulerPlanResult {
    pub decision: ScheduleDecision,
    pub snapshot: SchedulerSnapshot,
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
        let interval = *self.heartbeat_interval_secs.lock().unwrap();
        let state = self.state.lock().unwrap();
        let snapshot = SchedulerSnapshot {
            heartbeat_interval_secs: interval,
            last_fingerprint: state.last_fingerprint.clone(),
            last_report_at: state.last_report_at,
        };
        drop(state);

        plan_status_update(snapshot, update, now_secs).decision
    }

    pub fn mark_pushed(&self, fingerprint: String, now_secs: i64) {
        let mut state = self.state.lock().unwrap();
        state.last_fingerprint = fingerprint;
        state.last_report_at = now_secs;
    }
}

#[uniffi::export]
pub fn plan_status_update(
    snapshot: SchedulerSnapshot,
    update: Option<StatusUpdate>,
    now_secs: i64,
) -> SchedulerPlanResult {
    let snapshot = normalize_snapshot(snapshot);
    let Some(update) = normalize_status_update(update) else {
        return SchedulerPlanResult {
            decision: ScheduleDecision::default(),
            snapshot,
        };
    };

    // Exclude timestamp from fingerprint so only content changes trigger a push
    let fingerprint_data = (&update.ok, &update.process, &update.extend, &update.media);
    let fingerprint = serde_json::to_string(&fingerprint_data).unwrap_or_default();
    let heartbeat_interval_secs = snapshot.heartbeat_interval_secs as i64;

    let decision = if snapshot.last_fingerprint.is_empty() {
        ScheduleDecision {
            should_push: true,
            reason: ReportReason::Initial,
            fingerprint,
        }
    } else if snapshot.last_fingerprint != fingerprint {
        ScheduleDecision {
            should_push: true,
            reason: ReportReason::Changed,
            fingerprint,
        }
    } else if snapshot.last_report_at <= 0
        || now_secs - snapshot.last_report_at >= heartbeat_interval_secs
    {
        ScheduleDecision {
            should_push: true,
            reason: ReportReason::Heartbeat,
            fingerprint,
        }
    } else {
        ScheduleDecision {
            should_push: false,
            reason: ReportReason::None,
            fingerprint,
        }
    };

    SchedulerPlanResult { decision, snapshot }
}

#[uniffi::export]
pub fn mark_status_pushed(
    snapshot: SchedulerSnapshot,
    fingerprint: String,
    now_secs: i64,
) -> SchedulerSnapshot {
    let mut snapshot = normalize_snapshot(snapshot);
    snapshot.last_fingerprint = fingerprint;
    snapshot.last_report_at = now_secs;
    snapshot
}

fn normalize_snapshot(snapshot: SchedulerSnapshot) -> SchedulerSnapshot {
    SchedulerSnapshot {
        heartbeat_interval_secs: snapshot.heartbeat_interval_secs.max(5),
        last_fingerprint: snapshot.last_fingerprint,
        last_report_at: snapshot.last_report_at,
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
    use super::{
        SchedulerSnapshot, PushScheduler, ReportReason, mark_status_pushed, plan_status_update,
    };
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

    #[test]
    fn stateless_snapshot_bridge_matches_scheduler_flow() {
        let snapshot = SchedulerSnapshot {
            heartbeat_interval_secs: 10,
            last_fingerprint: String::new(),
            last_report_at: 0,
        };

        let planned = plan_status_update(
            snapshot,
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Kitty".into()),
                extend: None,
                media: None,
                timestamp: Some(1),
            }),
            1,
        );
        assert_eq!(planned.decision.reason, ReportReason::Initial);

        let snapshot = mark_status_pushed(planned.snapshot, planned.decision.fingerprint.clone(), 1);
        let second = plan_status_update(
            snapshot,
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Kitty".into()),
                extend: None,
                media: None,
                timestamp: Some(5),
            }),
            5,
        );
        assert_eq!(second.decision.reason, ReportReason::None);
    }
}
