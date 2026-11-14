// Move only a PID/title-selected test memo. Never activates the app or moves the pointer.
import ApplicationServices
import Foundation
let args = CommandLine.arguments
let app = AXUIElementCreateApplication(pid_t(args[1])!)
var value: CFTypeRef?
guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &value) == .success,
      let windows = value as? [AXUIElement] else { exit(1) }
let window = windows.first { w in
    var title: CFTypeRef?
    AXUIElementCopyAttributeValue(w, kAXTitleAttribute as CFString, &title)
    return title as? String == args[2]
}
guard let window else { exit(2) }
let duration = Double(args[3]) ?? 5
let start = ProcessInfo.processInfo.systemUptime
var frame = 0
while ProcessInfo.processInfo.systemUptime - start < duration {
    let t = ProcessInfo.processInfo.systemUptime - start
    var point = CGPoint(x: 700 + 80 * sin(t * 2), y: 300 + 100 * sin(t * 3))
    let v = AXValueCreate(.cgPoint, &point)!
    guard AXUIElementSetAttributeValue(window,kAXPositionAttribute as CFString,v) == .success else { exit(3) }
    frame += 1
    let deadline = start + Double(frame) / 60
    let remaining = deadline - ProcessInfo.processInfo.systemUptime
    if remaining > 0 { Thread.sleep(forTimeInterval: remaining) }
}
print(frame)
