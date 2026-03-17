import Foundation

@Observable
@MainActor
final class MonitorService {

    var isRunning: Bool = false
    var currentWindow: DetectedWindow?
    var currentMedia: DetectedMedia?
    var resolveResult: ResolveStatusResult?
    var lastDecision: ScheduleDecision?
    var lastApiResult: ApiCallResult?
    var logs: [String] = []

    private var monitorTask: Task<Void, Never>?
    private let windowService = WindowDetectionService()
    private let mediaService = MediaDetectionService()
    private var schedulerSnapshot = SchedulerSnapshot(heartbeatIntervalSecs: 5, lastFingerprint: "", lastReportAt: 0)

    var config: CoreConfig = defaultConfig()
    var matchingConfig: MatchEngineConfig = defaultPersistedConfig().matching

    func start() {
        guard !isRunning else { return }
        isRunning = true
        schedulerSnapshot = SchedulerSnapshot(
            heartbeatIntervalSecs: config.heartbeatIntervalSecs,
            lastFingerprint: "",
            lastReportAt: 0
        )
        appendLog("Monitor started")

        monitorTask = Task { [weak self] in
            guard let self else { return }
            while !Task.isCancelled {
                await self.tick()
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }

    func stop() {
        monitorTask?.cancel()
        monitorTask = nil
        isRunning = false
        appendLog("Monitor stopped")
    }

    private func tick() async {
        // 1. Detect window
        let window = windowService.detectActiveWindow()
        self.currentWindow = window

        // 2. Detect media
        let media = await mediaService.detectMedia()
        self.currentMedia = media

        // 3. Build input
        let windowInfo = WindowInfo(
            windowTitle: window?.windowTitle ?? "",
            appName: window?.appName ?? "",
            processName: window?.processName ?? "",
            executablePath: window?.executablePath ?? "",
            bundleId: window?.bundleId ?? ""
        )
        let mediaInfo: MediaInfo? = media.map {
            MediaInfo(title: $0.title, artist: $0.artist, thumbnail: $0.thumbnail)
        }
        let now = Int64(Date().timeIntervalSince1970)

        let input = ResolveStatusInput(
            window: windowInfo,
            media: mediaInfo,
            timestamp: now
        )

        // 4. Resolve
        let resolved = resolveStatusUpdate(config: matchingConfig, input: input)
        self.resolveResult = resolved

        guard resolved.shouldReport, let update = resolved.update else {
            self.lastDecision = ScheduleDecision(shouldPush: false, reason: .none, fingerprint: "")
            return
        }

        // 5. Plan
        schedulerSnapshot = SchedulerSnapshot(
            heartbeatIntervalSecs: config.heartbeatIntervalSecs,
            lastFingerprint: schedulerSnapshot.lastFingerprint,
            lastReportAt: schedulerSnapshot.lastReportAt
        )
        let planResult = planStatusUpdate(snapshot: schedulerSnapshot, update: update, nowSecs: now)
        let decision = planResult.decision
        self.lastDecision = decision
        self.schedulerSnapshot = planResult.snapshot

        guard decision.shouldPush else { return }

        // 6. Push
        let client = StatusShareClient(config: config)
        let apiResult = client.pushStatus(update: update)
        self.lastApiResult = apiResult

        // 7. Mark pushed
        if apiResult.success {
            self.schedulerSnapshot = markStatusPushed(
                snapshot: schedulerSnapshot,
                fingerprint: decision.fingerprint,
                nowSecs: now
            )
        }

        // 8. Log
        let reasonStr: String
        switch decision.reason {
        case .none: reasonStr = "None"
        case .initial: reasonStr = "Initial"
        case .changed: reasonStr = "Changed"
        case .heartbeat: reasonStr = "Heartbeat"
        }
        appendLog("[\(reasonStr)] \(resolved.process) – \(apiResult.success ? "OK (\(apiResult.httpStatus))" : apiResult.errorMessage)")
    }

    private func appendLog(_ entry: String) {
        let ts = ISO8601DateFormatter().string(from: Date())
        logs.append("[\(ts)] \(entry)")
        if logs.count > 500 {
            logs.removeFirst(logs.count - 500)
        }
    }
}
