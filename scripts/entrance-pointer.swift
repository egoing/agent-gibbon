// Test-only pointer control. The caller restores the saved cursor position.
import AppKit
import CoreGraphics
import Foundation
let args = Array(CommandLine.arguments.dropFirst())
if args.first == "move", args.count == 3 {
    let point = CGPoint(x: Double(args[1])!, y: Double(args[2])!)
    CGWarpMouseCursorPosition(point)
    CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: point,
            mouseButton: .left)?.post(tap: .cghidEventTap)
} else {
    let point = CGEvent(source: nil)!.location
    let display = CGMainDisplayID()
    let bounds = CGDisplayBounds(display)
    let scale = Double(NSScreen.screens[0].backingScaleFactor)
    let value: [String: Double] = ["x": point.x, "y": point.y,
        "width": bounds.width, "height": bounds.height, "scale": scale]
    print(String(data: try! JSONSerialization.data(withJSONObject: value), encoding: .utf8)!)
}
