// Read-only, PID-bound focus/window probe for the manager opening regression.
import AppKit
import ApplicationServices
import Foundation

func attr(_ element: AXUIElement, _ key: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}
func text(_ element: AXUIElement, _ key: String) -> String { attr(element, key) as? String ?? "" }
let pid = pid_t(CommandLine.arguments[1])!
let app = AXUIElementCreateApplication(pid)
let windows = attr(app, kAXWindowsAttribute) as? [AXUIElement] ?? []
let rows = windows.map { window -> [String: Any] in
    ["title": text(window, kAXTitleAttribute),
     "main": attr(window, kAXMainAttribute) as? Bool ?? false,
     "minimized": attr(window, kAXMinimizedAttribute) as? Bool ?? false]
}
// The CG list is ordered front to back; retain only our PID's windows.
let onscreen = (CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID)
    as? [[String: Any]] ?? []).filter { ($0[kCGWindowOwnerPID as String] as? Int) == Int(pid) }
print(String(data: try! JSONSerialization.data(withJSONObject: [
    "frontmost": NSWorkspace.shared.frontmostApplication?.processIdentifier ?? -1,
    "windows": rows, "onscreen": onscreen
], options: [.sortedKeys]), encoding: .utf8)!)
