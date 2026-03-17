import SwiftUI

enum SidebarItem: String, CaseIterable, Identifiable {
    case home
    case config
    case rules
    case debug
    case logs

    var id: String { rawValue }

    var label: String {
        switch self {
        case .home:   return "首页"
        case .config: return "连接配置"
        case .rules:  return "匹配规则"
        case .debug:  return "调试信息"
        case .logs:   return "运行日志"
        }
    }

    var icon: String {
        switch self {
        case .home:   return "gauge.with.dots.needle.33percent"
        case .config: return "gearshape"
        case .rules:  return "list.bullet.rectangle"
        case .debug:  return "ladybug"
        case .logs:   return "doc.text"
        }
    }
}

struct SidebarView: View {
    @Bindable var viewModel: AppViewModel

    var body: some View {
        List(SidebarItem.allCases, selection: $viewModel.selectedSidebar) { item in
            Label(item.label, systemImage: item.icon)
                .tag(item)
        }
        .listStyle(.sidebar)
        .navigationSplitViewColumnWidth(min: 180, ideal: 200, max: 240)
    }
}
