use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use statusshare_core::{
    ApiCallResult, PersistedConfig, PushScheduler, ResolveStatusInput, ResolveStatusResult,
    ScheduleDecision, StatusShareClient, WindowInfo, resolve_status_update,
};

use crate::detection::{DetectedWindow, detect_active_window, detect_media};

#[derive(Debug)]
pub struct MonitorControl {
    stop_tx: Sender<()>,
    join_handle: JoinHandle<()>,
}

#[derive(Debug, Clone)]
pub struct MonitorTick {
    pub backend: String,
    pub window: WindowInfo,
    pub media: Option<statusshare_core::MediaInfo>,
    pub resolve: ResolveStatusResult,
    pub decision: ScheduleDecision,
    pub api_result: Option<ApiCallResult>,
}

impl MonitorControl {
    pub fn stop(self) {
        let _ = self.stop_tx.send(());
        let _ = self.join_handle.join();
    }
}

pub fn start_monitoring(
    config: PersistedConfig,
    ui_tx: Sender<Result<MonitorTick, String>>,
) -> MonitorControl {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    let join_handle = thread::spawn(move || {
        let client = StatusShareClient::new(config.core.clone());
        let scheduler = PushScheduler::new(config.core.heartbeat_interval_secs);

        loop {
            let result = monitor_once(&client, &scheduler, &config);
            let _ = ui_tx.send(result);

            match stop_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(_) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    MonitorControl {
        stop_tx,
        join_handle,
    }
}

fn monitor_once(
    client: &StatusShareClient,
    scheduler: &PushScheduler,
    config: &PersistedConfig,
) -> Result<MonitorTick, String> {
    let DetectedWindow { backend, window } = detect_active_window()?;
    let media = detect_media().unwrap_or(None);
    let now_secs = unix_timestamp_now();

    let resolve = resolve_status_update(
        config.matching.clone(),
        ResolveStatusInput {
            window: window.clone(),
            media: media.clone(),
            timestamp: Some(now_secs),
        },
    );

    let decision = scheduler.plan(resolve.update.clone(), now_secs);
    let mut api_result = None;

    if resolve.should_report && decision.should_push {
        if let Some(update) = resolve.update.clone() {
            let result = client.push_status(update);
            if result.success {
                scheduler.mark_pushed(decision.fingerprint.clone(), now_secs);
            }
            api_result = Some(result);
        }
    }

    Ok(MonitorTick {
        backend,
        window,
        media,
        resolve,
        decision,
        api_result,
    })
}

fn unix_timestamp_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
