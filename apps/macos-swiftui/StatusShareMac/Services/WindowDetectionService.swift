import AppKit
import CoreGraphics

struct DetectedWindow {
    var backend: String = "nsworkspace"
    var windowTitle: String = ""
    var appName: String = ""
    var processName: String = ""
    var executablePath: String = ""
    var bundleId: String = ""
}

final class WindowDetectionService {

    func detectActiveWindow() -> DetectedWindow? {
        guard let app = NSWorkspace.shared.frontmostApplication else {
            return nil
        }

        let bundleId = app.bundleIdentifier ?? ""
        let appName = app.localizedName ?? ""
        let processName = app.executableURL?.lastPathComponent ?? ""
        let executablePath = app.executableURL?.path ?? ""

        let windowTitle = fetchWindowTitle(pid: app.processIdentifier)

        return DetectedWindow(
            backend: "nsworkspace",
            windowTitle: windowTitle,
            appName: appName,
            processName: processName,
            executablePath: executablePath,
            bundleId: bundleId
        )
    }

    private func fetchWindowTitle(pid: pid_t) -> String {
        guard let windowList = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] else {
            return ""
        }

        for entry in windowList {
            guard let ownerPID = entry[kCGWindowOwnerPID as String] as? pid_t,
                  ownerPID == pid,
                  let layer = entry[kCGWindowLayer as String] as? Int,
                  layer == 0 else {
                continue
            }

            if let name = entry[kCGWindowName as String] as? String, !name.isEmpty {
                return name
            }
        }

        return ""
    }
}
