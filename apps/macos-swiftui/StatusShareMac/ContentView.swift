import SwiftUI

struct ContentView: View {
    @Bindable var viewModel: AppViewModel

    var body: some View {
        if viewModel.isExpanded {
            expandedView
                .frame(minWidth: 960, minHeight: 680)
        } else {
            CompactCardView(viewModel: viewModel)
        }
    }

    private var expandedView: some View {
        NavigationSplitView {
            SidebarView(viewModel: viewModel)
        } detail: {
            detailView
        }
        .toolbar {
            ToolbarItem(placement: .navigation) {
                Button {
                    viewModel.isExpanded = false
                } label: {
                    Image(systemName: "rectangle.compress.vertical")
                }
                .help("收起为卡片")
            }
        }
    }

    @ViewBuilder
    private var detailView: some View {
        switch viewModel.selectedSidebar {
        case .home:
            HomeView(viewModel: viewModel)
        case .config:
            ConfigView(viewModel: viewModel)
        case .rules:
            RulesView(viewModel: viewModel)
        case .debug:
            DebugView(viewModel: viewModel)
        case .logs:
            LogView(viewModel: viewModel)
        }
    }
}
