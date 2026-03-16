import Foundation
import SwiftUI

#if canImport(StatusshareCore)
import StatusshareCore
#endif

@MainActor
final class StatusShareViewModel: ObservableObject {
    @Published var baseURL: String = "http://127.0.0.1:3000"
    @Published var token: String = ""
    @Published var heartbeatInterval: Int = 60
    @Published var ok: Bool = true
    @Published var process: String = ""
    @Published var extend: String = ""
    @Published var mediaTitle: String = ""
    @Published var mediaArtist: String = ""
    @Published var mediaThumbnail: String = ""
    @Published var output: String = "Generate UniFFI Swift bindings before building this target."
    @Published var heartbeatRunning: Bool = false

    private var heartbeatTask: Task<Void, Never>?

    func fetch() async {
        output = await StatusShareBridge.shared.fetchStatus(config: makeConfig())
    }

    func push() async {
        output = await StatusShareBridge.shared.pushStatus(config: makeConfig(), update: makeUpdate())
    }

    func toggleHeartbeat() {
        if heartbeatRunning {
            heartbeatTask?.cancel()
            heartbeatTask = nil
            heartbeatRunning = false
            return
        }

        heartbeatRunning = true
        heartbeatTask = Task { [weak self] in
            guard let self else { return }
            while !Task.isCancelled {
                self.output = await StatusShareBridge.shared.pushStatus(config: self.makeConfig(), update: self.makeUpdate())
                try? await Task.sleep(for: .seconds(self.heartbeatInterval))
            }
        }
    }

    private func makeConfig() -> StatusShareBridge.Config {
        StatusShareBridge.Config(
            baseURL: baseURL,
            token: token,
            heartbeatInterval: UInt64(heartbeatInterval),
            userAgent: "StatusShare macOS/0.1.0"
        )
    }

    private func makeUpdate() -> StatusShareBridge.Update {
        let media = mediaTitle.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty &&
            mediaArtist.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty &&
            mediaThumbnail.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
            ? nil
            : StatusShareBridge.Media(title: mediaTitle, artist: mediaArtist, thumbnail: mediaThumbnail)

        return StatusShareBridge.Update(
            ok: ok ? 1 : 0,
            process: blankToNil(process),
            extend: blankToNil(extend),
            media: media
        )
    }

    private func blankToNil(_ value: String) -> String? {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}

enum StatusShareBridge {
    struct Config {
        let baseURL: String
        let token: String
        let heartbeatInterval: UInt64
        let userAgent: String
    }

    struct Media {
        let title: String
        let artist: String
        let thumbnail: String
    }

    struct Update {
        let ok: Int32
        let process: String?
        let extend: String?
        let media: Media?
    }

    static let shared = Adapter()

    final class Adapter {
        func fetchStatus(config: Config) async -> String {
            #if canImport(StatusshareCore)
            let client = StatusShareClient(config: CoreConfig(baseUrl: config.baseURL, token: config.token, heartbeatIntervalSecs: config.heartbeatInterval, userAgent: config.userAgent))
            let result = client.fetchStatus()
            return prettyJSONString(from: result)
            #else
            return "Swift bindings are missing. Run the UniFFI generation step from docs/architecture.md."
            #endif
        }

        func pushStatus(config: Config, update: Update) async -> String {
            #if canImport(StatusshareCore)
            let client = StatusShareClient(config: CoreConfig(baseUrl: config.baseURL, token: config.token, heartbeatIntervalSecs: config.heartbeatInterval, userAgent: config.userAgent))
            let media = update.media.map { MediaInfo(title: $0.title, artist: $0.artist, thumbnail: $0.thumbnail) }
            let payload = StatusUpdate(ok: update.ok, process: update.process, extend: update.extend, media: media, timestamp: nil)
            let result = client.pushStatus(update: payload)
            return prettyJSONString(from: result)
            #else
            return "Swift bindings are missing. Run the UniFFI generation step from docs/architecture.md."
            #endif
        }

        private func prettyJSONString<T>(from value: T) -> String {
            let mirror = Mirror(reflecting: value)
            var dictionary: [String: Any] = [:]
            for child in mirror.children {
                if let label = child.label {
                    dictionary[label] = child.value
                }
            }
            guard JSONSerialization.isValidJSONObject(dictionary),
                  let data = try? JSONSerialization.data(withJSONObject: dictionary, options: [.prettyPrinted]),
                  let text = String(data: data, encoding: .utf8)
            else {
                return String(describing: value)
            }
            return text
        }
    }
}

