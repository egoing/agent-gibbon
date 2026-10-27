// PID-bound accessibility helper for the development Feedback smoke test.
import ApplicationServices
import Foundation

func attribute(_ element: AXUIElement, _ key: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}
func text(_ element: AXUIElement, _ key: String) -> String {
    attribute(element, key) as? String ?? ""
}
func descendants(_ element: AXUIElement, depth: Int = 0) -> [AXUIElement] {
    if depth > 30 { return [] }
    let children = attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []
    return [element] + children.flatMap { descendants($0, depth: depth + 1) }
}
let args = CommandLine.arguments
let app = AXUIElementCreateApplication(pid_t(args[1])!)
let windows = attribute(app, kAXWindowsAttribute) as? [AXUIElement] ?? []
guard let window = windows.first(where: { text($0, kAXTitleAttribute) == args[2] }) else {
    fputs("target window not ready\n", stderr); exit(1)
}
let nodes = descendants(window)
switch args[3] {
case "dump":
    for node in nodes {
        print([text(node, kAXRoleAttribute), text(node, kAXTitleAttribute), text(node, kAXDescriptionAttribute), text(node, kAXValueAttribute)].joined(separator: " | "))
    }
case "click":
    guard let node = nodes.first(where: {
        ["AXButton", "AXCheckBox"].contains(text($0, kAXRoleAttribute)) &&
        [text($0, kAXTitleAttribute), text($0, kAXDescriptionAttribute)].contains(args[4])
    }) else { fputs("button not ready\n", stderr); exit(1) }
    // A disappearing surface may return cannotComplete; caller verifies app state.
    _ = AXUIElementPerformAction(node, kAXPressAction as CFString)
case "draft":
    guard let node = nodes.first(where: { text($0, kAXRoleAttribute) == "AXTextArea" }) else { exit(1) }
    guard AXUIElementSetAttributeValue(node, kAXValueAttribute as CFString, args[4] as CFString) == .success else { exit(1) }
    print(text(node, kAXValueAttribute))
case "resize":
    var size = CGSize(width: Int(args[4])!, height: Int(args[5])!)
    let value = AXValueCreate(.cgSize, &size)!
    guard AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, value) == .success else { exit(1) }
default: exit(1)
}
