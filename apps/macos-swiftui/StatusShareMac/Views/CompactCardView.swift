import SwiftUI

struct CompactCardView: View {
    @Bindable var viewModel: AppViewModel

    private var monitor: MonitorService { viewModel.monitorService }
    private var media: DetectedMedia? { monitor.currentMedia }

    var body: some View {
        VStack(spacing: 0) {
            VStack(spacing: 14) {
                headerRow
                statusSection
                musicCard
                metricsStrip
            }
            .padding(.top, 32)
            .padding(.horizontal, 18)
            .padding(.bottom, 14)

            separator

            bottomBar
                .padding(.horizontal, 18)
                .padding(.vertical, 10)
        }
        .frame(width: 360, height: 300)
        .background(.ultraThinMaterial)
    }

    // MARK: - Header

    private var headerRow: some View {
        HStack(spacing: 6) {
            Image(systemName: "antenna.radiowaves.left.and.right")
                .font(.system(size: 10, weight: .semibold))
                .foregroundStyle(.tertiary)

            Text("StatusShare")
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundStyle(.tertiary)

            Spacer()

            HStack(spacing: 4) {
                Circle()
                    .fill(monitor.isRunning ? Color.green : Color.secondary.opacity(0.4))
                    .frame(width: 5, height: 5)

                Text(monitor.isRunning ? "监控中" : "已停止")
                    .font(.system(size: 10, weight: .medium))
                    .foregroundStyle(monitor.isRunning ? .secondary : .tertiary)
            }
        }
    }

    // MARK: - Status

    private var statusSection: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(monitor.resolveResult?.process ?? "Idle")
                .font(.system(size: 22, weight: .bold, design: .rounded))
                .foregroundStyle(.primary)
                .lineLimit(1)

            Text(monitor.resolveResult?.extend ?? "点击展开配置面板")
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
                .lineLimit(2)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - Music (always visible)

    private var musicCard: some View {
        HStack(spacing: 10) {
            artworkView
                .frame(width: 40, height: 40)
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .shadow(color: .black.opacity(0.08), radius: 2, y: 1)

            VStack(alignment: .leading, spacing: 1) {
                Text(media?.title ?? "暂无媒体")
                    .font(.system(size: 12, weight: .medium))
                    .lineLimit(1)
                    .foregroundStyle(media != nil ? .primary : .tertiary)

                Text(media?.artist ?? "当前没有正在播放的内容")
                    .font(.system(size: 10))
                    .foregroundStyle(media != nil ? .secondary : .quaternary)
                    .lineLimit(1)
            }

            Spacer(minLength: 0)

            Image(systemName: media != nil ? "waveform" : "speaker.slash")
                .font(.system(size: 10))
                .foregroundStyle(.quaternary)
                .symbolEffect(.variableColor.iterative, isActive: media != nil)
        }
        .padding(10)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 10))
    }

    @ViewBuilder
    private var artworkView: some View {
        if let m = media, !m.thumbnail.isEmpty, let nsImage = NSImage(contentsOfFile: m.thumbnail) {
            Image(nsImage: nsImage)
                .resizable()
                .aspectRatio(contentMode: .fill)
        } else {
            RoundedRectangle(cornerRadius: 8)
                .fill(.quaternary.opacity(0.5))
                .overlay {
                    Image(systemName: "music.note")
                        .font(.system(size: 14))
                        .foregroundStyle(.quaternary)
                }
        }
    }

    // MARK: - Metrics

    private var metricsStrip: some View {
        HStack(spacing: 0) {
            metricItem("原因", reasonString(monitor.lastDecision?.reason))
            pill
            metricItem("心跳", "\(viewModel.heartbeatInterval)s")
            pill
            metricItem("来源", monitor.currentWindow?.backend ?? "—")
        }
        .padding(.vertical, 5)
        .padding(.horizontal, 6)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 8))
    }

    private func metricItem(_ title: String, _ value: String) -> some View {
        VStack(spacing: 1) {
            Text(title)
                .font(.system(size: 8, weight: .medium))
                .foregroundStyle(.quaternary)
            Text(value)
                .font(.system(size: 9, weight: .semibold, design: .monospaced))
                .foregroundStyle(.tertiary)
        }
        .frame(maxWidth: .infinity)
    }

    private var pill: some View {
        RoundedRectangle(cornerRadius: 0.5)
            .fill(.quaternary)
            .frame(width: 0.5, height: 16)
    }

    private var separator: some View {
        Rectangle()
            .fill(.quaternary.opacity(0.5))
            .frame(height: 0.5)
    }

    // MARK: - Bottom

    private var bottomBar: some View {
        HStack(spacing: 6) {
            Button {
                viewModel.toggleMonitor()
            } label: {
                Image(systemName: monitor.isRunning ? "stop.fill" : "play.fill")
                    .font(.system(size: 9))
                    .foregroundStyle(monitor.isRunning ? .red : .green)
                    .frame(width: 20, height: 20)
                    .background(.regularMaterial, in: Circle())
            }
            .buttonStyle(.plain)
            .help(monitor.isRunning ? "停止监控" : "开始监控")

            Spacer()

            Button {
                viewModel.isExpanded = true
            } label: {
                HStack(spacing: 3) {
                    Text("展开面板")
                        .font(.system(size: 10))
                    Image(systemName: "arrow.up.left.and.arrow.down.right")
                        .font(.system(size: 8))
                }
                .foregroundStyle(.tertiary)
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .background(.regularMaterial, in: Capsule())
            }
            .buttonStyle(.plain)
        }
    }
}
