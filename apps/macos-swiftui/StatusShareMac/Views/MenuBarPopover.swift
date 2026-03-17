import SwiftUI

struct MenuBarPopover: View {
    @Bindable var viewModel: AppViewModel

    private var monitor: MonitorService { viewModel.monitorService }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Status summary
            HStack(spacing: 8) {
                Circle()
                    .fill(monitor.isRunning ? .green : .red)
                    .frame(width: 8, height: 8)
                Text(monitor.isRunning ? "监控中" : "已停止")
                    .font(.headline)
            }

            if let resolved = monitor.resolveResult {
                VStack(alignment: .leading, spacing: 4) {
                    Text(resolved.process)
                        .font(.body)
                        .fontWeight(.medium)
                    if !resolved.extend.isEmpty {
                        Text(resolved.extend)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .lineLimit(2)
                    }
                }
            }

            if let media = monitor.currentMedia {
                Divider()
                HStack(spacing: 8) {
                    Image(systemName: "music.note")
                        .foregroundStyle(.secondary)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(media.title)
                            .font(.caption)
                            .lineLimit(1)
                        Text(media.artist)
                            .font(.caption2)
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                }
            }

            Divider()

            Button {
                viewModel.toggleMonitor()
            } label: {
                Label(
                    monitor.isRunning ? "停止监控" : "开始监控",
                    systemImage: monitor.isRunning ? "stop.fill" : "play.fill"
                )
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.bordered)
            .controlSize(.small)

            Button {
                if let app = NSApp {
                    app.activate(ignoringOtherApps: true)
                    if let window = app.windows.first(where: { $0.canBecomeMain }) {
                        window.makeKeyAndOrderFront(nil)
                    }
                }
            } label: {
                Label("打开主窗口", systemImage: "macwindow")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.bordered)
            .controlSize(.small)

            Divider()

            Button {
                NSApp?.terminate(nil)
            } label: {
                Label("退出", systemImage: "power")
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.bordered)
            .controlSize(.small)
        }
        .padding(16)
        .frame(width: 260)
    }
}
