use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::StatusUpdate;

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

    let fingerprint = status_fingerprint(&update);
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
        category: trim_non_empty(update.category),
        game: update.game.and_then(crate::clean_game),
        media: update.media.and_then(crate::clean_media),
        timestamp: update.timestamp,
    })
}

/// 计算一次状态更新的指纹，用于判断“内容是否变化”。
///
/// 播放进度 (media.position) 与 timestamp 属于高频字段：
/// 它们每次采样都会变化，但不构成“状态切换”，
/// 因此不参与指纹，避免进度条导致每秒都触发 Changed 推送；
/// 进度会搭心跳推送（heartbeat interval）的便车同步到前端。
fn status_fingerprint(update: &StatusUpdate) -> String {
    #[derive(Serialize)]
    struct FingerprintMedia<'a> {
        title: &'a str,
        artist: &'a str,
        thumbnail: &'a str,
        duration: f64,
        state: &'a str,
    }

    #[derive(Serialize)]
    struct Fingerprint<'a> {
        ok: Option<i32>,
        process: &'a str,
        extend: &'a str,
        category: &'a str,
        game: Option<&'a crate::GameMeta>,
        media: Option<FingerprintMedia<'a>>,
    }

    let fingerprint = Fingerprint {
        ok: update.ok,
        process: update.process.as_deref().unwrap_or_default(),
        extend: update.extend.as_deref().unwrap_or_default(),
        category: update.category.as_deref().unwrap_or_default(),
        game: update.game.as_ref(),
        media: update.media.as_ref().map(|media| FingerprintMedia {
            title: &media.title,
            artist: &media.artist,
            thumbnail: &media.thumbnail,
            duration: media.duration,
            state: &media.state,
        }),
    };

    serde_json::to_string(&fingerprint).unwrap_or_default()
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
                category: None,
                game: None,
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
                category: None,
                game: None,
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
                category: None,
                game: None,
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
                category: None,
                game: None,
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
                category: None,
                game: None,
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
                category: None,
                game: None,
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
                category: None,
                game: None,
                media: None,
                timestamp: Some(5),
            }),
            5,
        );
        assert_eq!(second.decision.reason, ReportReason::None);
    }

    #[test]
    fn media_progress_does_not_trigger_changed_push() {
        use crate::MediaInfo;

        let media_at = |position: f64| {
            Some(MediaInfo {
                title: "Song".into(),
                artist: "Artist".into(),
                thumbnail: String::new(),
                position,
                duration: 200.0,
                state: "playing".into(),
            })
        };

        let scheduler = PushScheduler::new(5);
        let first = scheduler.plan(
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Spotify".into()),
                extend: None,
                category: None,
                game: None,
                media: media_at(12.0),
                timestamp: Some(1),
            }),
            1,
        );
        assert_eq!(first.reason, ReportReason::Initial);
        scheduler.mark_pushed(first.fingerprint.clone(), 1);

        // 进度从 12s 走到 13s：不构成状态变化，应等待心跳。
        let progress_only = scheduler.plan(
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Spotify".into()),
                extend: None,
                category: None,
                game: None,
                media: media_at(13.0),
                timestamp: Some(2),
            }),
            2,
        );
        assert!(!progress_only.should_push);

        // 同一首歌切到 paused：属于状态变化，立即推送。
        let mut paused_media = media_at(13.0).expect("media");
        paused_media.state = "paused".into();
        let paused = scheduler.plan(
            Some(StatusUpdate {
                ok: Some(1),
                process: Some("Spotify".into()),
                extend: None,
                category: None,
                game: None,
                media: Some(paused_media),
                timestamp: Some(3),
            }),
            3,
        );
        assert_eq!(paused.reason, ReportReason::Changed);
    }
}
