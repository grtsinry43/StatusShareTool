import SwiftUI

struct DebugView: View {
    @Bindable var viewModel: AppViewModel

    private var monitor: MonitorService { viewModel.monitorService }

    var body: some View {
        ScrollView {
            HStack(alignment: .top, spacing: 20) {
                windowMetadataColumn
                resolveResultColumn
            }
            .padding(24)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    // MARK: - Window metadata

    private var windowMetadataColumn: some View {
        VStack(alignment: .leading, spacing: 0) {
            sectionHeader("窗口元数据")
            debugRow("检测来源", monitor.currentWindow?.backend ?? "—")
            debugRow("窗口标题", monitor.currentWindow?.windowTitle ?? "—")
            debugRow("应用名称", monitor.currentWindow?.appName ?? "—")
            debugRow("进程名", monitor.currentWindow?.processName ?? "—")
            debugRow("可执行路径", monitor.currentWindow?.executablePath ?? "—")
            debugRow("Bundle ID", monitor.currentWindow?.bundleId ?? "—")
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
    }

    // MARK: - Resolve result

    private var resolveResultColumn: some View {
        VStack(alignment: .leading, spacing: 0) {
            sectionHeader("解析结果")
            debugRow("命中规则", monitor.resolveResult?.matchedRuleId ?? "—")
            debugRow("解析名称", monitor.resolveResult?.process ?? "—")
            debugRow("解析文案", monitor.resolveResult?.extend ?? "—")
            debugRow("上报原因", reasonString(monitor.lastDecision?.reason))

            Divider().padding(.vertical, 8)

            sectionHeader("媒体信息")
            debugRow("标题", monitor.currentMedia?.title ?? "—")
            debugRow("作者", monitor.currentMedia?.artist ?? "—")
            debugRow("封面路径", monitor.currentMedia?.thumbnail ?? "—")

            Divider().padding(.vertical, 8)

            sectionHeader("最近 API 响应")
            debugRow("成功", monitor.lastApiResult.map { $0.success ? "true" : "false" } ?? "—")
            debugRow("HTTP", monitor.lastApiResult.map { "\($0.httpStatus)" } ?? "—")
            debugRow("消息", monitor.lastApiResult?.message ?? "—")
            debugRow("请求ID", monitor.lastApiResult?.requestId ?? "—")
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 12))
    }

    // MARK: - Helpers

    private func sectionHeader(_ title: String) -> some View {
        Text(title)
            .font(.headline)
            .padding(.bottom, 8)
    }

    private func debugRow(_ label: String, _ value: String) -> some View {
        HStack(alignment: .top, spacing: 8) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(width: 80, alignment: .trailing)
            Text(value)
                .font(.system(.caption, design: .monospaced))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 2)
    }
}
