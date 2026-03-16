import SwiftUI

@main
struct StatusShareMacApp: App {
    @StateObject private var model = StatusShareViewModel()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(model)
                .frame(minWidth: 860, minHeight: 640)
        }
    }
}

