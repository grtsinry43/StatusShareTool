import SwiftUI

@main
struct StatusShareMacApp: App {
    @State private var viewModel = AppViewModel()

    var body: some Scene {
        WindowGroup {
            ContentView(viewModel: viewModel)
                .background(WindowConfigurator(isExpanded: viewModel.isExpanded))
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentSize)
        .defaultSize(width: 360, height: 300)

        MenuBarExtra("StatusShare", systemImage: "antenna.radiowaves.left.and.right") {
            MenuBarPopover(viewModel: viewModel)
        }
        .menuBarExtraStyle(.window)
    }
}

// MARK: - NSWindow configurator

struct WindowConfigurator: NSViewRepresentable {
    let isExpanded: Bool

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        DispatchQueue.main.async {
            guard let window = view.window else { return }
            applyStyle(window, expanded: isExpanded)
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        guard context.coordinator.lastExpanded != isExpanded else { return }
        context.coordinator.lastExpanded = isExpanded

        DispatchQueue.main.async {
            guard let window = nsView.window else { return }

            let current = window.frame
            let target: NSSize
            if isExpanded {
                target = NSSize(width: 1080, height: 720)
            } else {
                target = NSSize(width: 360, height: 300)
            }
            let origin = NSPoint(
                x: current.midX - target.width / 2,
                y: current.midY - target.height / 2
            )
            // Apply style BEFORE resizing so the visual change is seamless
            applyStyle(window, expanded: isExpanded)
            window.setFrame(NSRect(origin: origin, size: target), display: true, animate: true)
        }
    }

    final class Coordinator {
        var lastExpanded: Bool?
    }
}

@MainActor
private func applyStyle(_ window: NSWindow, expanded: Bool) {
    window.titlebarAppearsTransparent = true
    window.titleVisibility = .hidden
    window.isMovableByWindowBackground = true
    window.hasShadow = true

    if expanded {
        window.styleMask = [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView]
        window.isOpaque = true
        window.backgroundColor = .windowBackgroundColor
    } else {
        window.styleMask = [.titled, .closable, .miniaturizable, .fullSizeContentView]
        window.isOpaque = false
        window.backgroundColor = .clear
    }
}
