import SwiftUI

struct ContentView: View {
    @EnvironmentObject private var model: StatusShareViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Form {
                TextField("Base URL", text: $model.baseURL)
                SecureField("Token", text: $model.token)
                Stepper("Heartbeat \(model.heartbeatInterval) s", value: $model.heartbeatInterval, in: 5...600, step: 5)
                Toggle("在线 / OK = 1", isOn: $model.ok)
                TextField("Process", text: $model.process)
                TextField("Extend", text: $model.extend)
                TextField("Media Title", text: $model.mediaTitle)
                TextField("Media Artist", text: $model.mediaArtist)
                TextField("Media Thumbnail", text: $model.mediaThumbnail)
            }

            HStack(spacing: 12) {
                Button("Fetch") {
                    Task { await model.fetch() }
                }
                Button("Push Once") {
                    Task { await model.push() }
                }
                Button(model.heartbeatRunning ? "Stop Heartbeat" : "Start Heartbeat") {
                    model.toggleHeartbeat()
                }
            }

            TextEditor(text: $model.output)
                .font(.system(.body, design: .monospaced))
                .border(Color.gray.opacity(0.2))
        }
        .padding(24)
    }
}

