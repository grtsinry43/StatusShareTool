import SwiftUI

struct HomeView: View {
    @Bindable var viewModel: AppViewModel

    private var monitor: MonitorService { viewModel.monitorService }
    private var media: DetectedMedia? { monitor.currentMedia }

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                statusCard
                controlButtons
                metricsRow
                musicCard
            }
            .padding(24)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    // MARK: - Status card

    private var statusCard: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(monitor.resolveResult?.process ?? "Idle")
                .font(.system(size: 28, weight: .semibold, design: .rounded))
                .foregroundStyle(.primary)

            Text(monitor.resolveResult?.extend ?? "等待监控启动...")
                .font(.body)
                .foregroundStyle(.secondary)
                .lineLimit(3)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(20)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
    }

    // MARK: - Control

    private var controlButtons: some View {
        HStack(spacing: 12) {
            Button {
                viewModel.toggleMonitor()
            } label: {
                Label(
                    monitor.isRunning ? "停止监控" : "开始监控",
                    systemImage: monitor.isRunning ? "stop.circle.fill" : "play.circle.fill"
                )
                .font(.headline)
                .foregroundStyle(monitor.isRunning ? .red : .green)
            }
            .buttonStyle(.bordered)
            .controlSize(.large)

            Spacer()
        }
    }

    // MARK: - Metrics

    private var metricsRow: some View {
        HStack(spacing: 12) {
            metricChip(
                title: "状态",
                value: monitor.isRunning ? "Running" : "Stopped",
                color: monitor.isRunning ? .green : .secondary
            )
            metricChip(
                title: "来源",
                value: monitor.currentWindow?.backend ?? "—"
            )
            metricChip(
                title: "原因",
                value: reasonString(monitor.lastDecision?.reason)
            )
            metricChip(
                title: "心跳",
                value: "\(viewModel.heartbeatInterval)s"
            )
        }
    }

    private func metricChip(title: String, value: String, color: Color = .accentColor) -> some View {
        VStack(spacing: 4) {
            Text(title)
                .font(.caption2)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.system(.caption, design: .monospaced))
                .fontWeight(.medium)
                .foregroundStyle(color)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 10)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
    }

    // MARK: - Media (always visible)

    private var musicCard: some View {
        HStack(spacing: 14) {
            artworkView
                .frame(width: 56, height: 56)
                .clipShape(RoundedRectangle(cornerRadius: 10))
                .shadow(color: .black.opacity(0.1), radius: 3, y: 1)

            VStack(alignment: .leading, spacing: 4) {
                Text(media?.title ?? "暂无媒体")
                    .font(.headline)
                    .lineLimit(1)
                    .foregroundStyle(media != nil ? .primary : .tertiary)

                Text(media?.artist ?? "当前没有正在播放的内容")
                    .font(.subheadline)
                    .foregroundStyle(media != nil ? .secondary : .quaternary)
                    .lineLimit(1)
            }

            Spacer()

            Image(systemName: media != nil ? "waveform" : "speaker.slash")
                .font(.title3)
                .foregroundStyle(.tertiary)
                .symbolEffect(.variableColor.iterative, isActive: media != nil)
        }
        .padding(14)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
    }

    @ViewBuilder
    private var artworkView: some View {
        if let m = media, !m.thumbnail.isEmpty, let nsImage = NSImage(contentsOfFile: m.thumbnail) {
            Image(nsImage: nsImage)
                .resizable()
                .aspectRatio(contentMode: .fill)
        } else {
            RoundedRectangle(cornerRadius: 10)
                .fill(.quaternary)
                .overlay {
                    Image(systemName: "music.note")
                        .font(.title2)
                        .foregroundStyle(.tertiary)
                }
        }
    }
}

func reasonString(_ reason: ReportReason?) -> String {
    guard let reason else { return "—" }
    switch reason {
    case .none: return "None"
    case .initial: return "Initial"
    case .changed: return "Changed"
    case .heartbeat: return "Heartbeat"
    }
}
