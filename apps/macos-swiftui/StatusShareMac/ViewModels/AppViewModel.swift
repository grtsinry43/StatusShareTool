import Foundation
import SwiftUI

@Observable
@MainActor
final class AppViewModel {

    // MARK: - Window mode

    var isExpanded: Bool = false

    // MARK: - Sidebar

    var selectedSidebar: SidebarItem = .home

    // MARK: - Config

    var configPath: String = ""
    var baseURL: String = "http://127.0.0.1:3000"
    var token: String = ""
    var heartbeatInterval: Int = 5
    var userAgent: String = "StatusShareMac/0.1.0"

    // MARK: - Matching config

    var matchingConfig: MatchEngineConfig

    var selectedRuleIndex: Int? = nil

    // MARK: - Monitor

    let monitorService = MonitorService()

    // MARK: - Output / feedback

    var output: String = ""

    // MARK: - Init

    init() {
        let persisted = defaultPersistedConfig()
        configPath = defaultConfigFilePath()
        baseURL = persisted.core.baseUrl
        token = persisted.core.token
        heartbeatInterval = Int(persisted.core.heartbeatIntervalSecs)
        userAgent = persisted.core.userAgent
        matchingConfig = persisted.matching
    }

    // MARK: - Config persistence

    func loadConfig() {
        let result = loadPersistedConfig(path: configPath)
        if result.success, let config = result.config {
            applyPersistedConfig(config)
            output = "Config loaded from \(result.path)"
        } else {
            output = "Load failed: \(result.errorMessage)"
        }
    }

    func saveConfig() {
        let persisted = buildPersistedConfig()
        let result = savePersistedConfig(path: configPath, config: persisted)
        if result.success {
            output = "Config saved to \(result.path)"
        } else {
            output = "Save failed: \(result.errorMessage)"
        }
    }

    // MARK: - Fetch

    func fetchStatus() {
        let client = StatusShareClient(config: buildCoreConfig())
        let result = client.fetchStatus()
        if result.success, let snap = result.snapshot {
            output = """
            Fetch OK (HTTP \(result.httpStatus))
            process: \(snap.process)
            extend: \(snap.extend)
            ok: \(snap.ok)
            timestamp: \(snap.timestamp)
            """
        } else {
            output = "Fetch failed: \(result.errorMessage)"
        }
    }

    // MARK: - Monitor control

    func startMonitor() {
        applyConfigToMonitor()
        monitorService.start()
    }

    func stopMonitor() {
        monitorService.stop()
    }

    func toggleMonitor() {
        if monitorService.isRunning {
            stopMonitor()
        } else {
            startMonitor()
        }
    }

    // MARK: - Rules

    func addRule() {
        let newId = "rule_\(Int(Date().timeIntervalSince1970))"
        let rule = WindowMatchRule(
            id: newId,
            enabled: true,
            field: .appName,
            kind: .contains,
            pattern: "",
            caseSensitive: false,
            reportPolicy: .allow,
            displayName: "",
            extend: ""
        )
        matchingConfig = MatchEngineConfig(
            defaultReport: matchingConfig.defaultReport,
            defaultDisplayName: matchingConfig.defaultDisplayName,
            defaultExtend: matchingConfig.defaultExtend,
            rules: matchingConfig.rules + [rule]
        )
        selectedRuleIndex = matchingConfig.rules.count - 1
    }

    func deleteRule(at index: Int) {
        guard matchingConfig.rules.indices.contains(index) else { return }
        var newRules = matchingConfig.rules
        newRules.remove(at: index)
        matchingConfig = MatchEngineConfig(
            defaultReport: matchingConfig.defaultReport,
            defaultDisplayName: matchingConfig.defaultDisplayName,
            defaultExtend: matchingConfig.defaultExtend,
            rules: newRules
        )
        if let selected = selectedRuleIndex {
            if selected >= matchingConfig.rules.count {
                selectedRuleIndex = matchingConfig.rules.isEmpty ? nil : matchingConfig.rules.count - 1
            } else if selected == index {
                selectedRuleIndex = nil
            }
        }
    }

    func updateRule(at index: Int, _ rule: WindowMatchRule) {
        guard matchingConfig.rules.indices.contains(index) else { return }
        var newRules = matchingConfig.rules
        newRules[index] = rule
        matchingConfig = MatchEngineConfig(
            defaultReport: matchingConfig.defaultReport,
            defaultDisplayName: matchingConfig.defaultDisplayName,
            defaultExtend: matchingConfig.defaultExtend,
            rules: newRules
        )
    }

    // MARK: - Private helpers

    func applyConfigToMonitor() {
        monitorService.config = buildCoreConfig()
        monitorService.matchingConfig = matchingConfig
    }

    private func buildCoreConfig() -> CoreConfig {
        CoreConfig(
            baseUrl: baseURL,
            token: token,
            heartbeatIntervalSecs: UInt64(max(heartbeatInterval, 5)),
            userAgent: userAgent
        )
    }

    private func buildPersistedConfig() -> PersistedConfig {
        PersistedConfig(
            schemaVersion: 1,
            core: buildCoreConfig(),
            matching: matchingConfig
        )
    }

    private func applyPersistedConfig(_ config: PersistedConfig) {
        baseURL = config.core.baseUrl
        token = config.core.token
        heartbeatInterval = Int(config.core.heartbeatIntervalSecs)
        userAgent = config.core.userAgent
        matchingConfig = config.matching
    }
}
