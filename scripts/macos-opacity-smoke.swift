import Foundation
import CoreGraphics
let offset = CommandLine.arguments.lastIndex(of: "--").map { $0 + 1 } ?? 1
let title = CommandLine.arguments[offset]
func info() -> [String: Any] {
    let windows = CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID) as! [[String: Any]]
    return windows.first { $0[kCGWindowOwnerName as String] as? String == "agent-gibbon" && $0[kCGWindowName as String] as? String == title }!
}
let original = CGEvent(source: nil)!.location
func move(_ point: CGPoint) {
    CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: point, mouseButton: .left)!.post(tap: .cghidEventTap)
    Thread.sleep(forTimeInterval: 2.9)
}
defer { move(original) }
let bounds = info()[kCGWindowBounds as String] as! [String: CGFloat]
let inside = CGPoint(x: bounds["X"]! + bounds["Width"]! / 2, y: bounds["Y"]! + bounds["Height"]! / 2)
// Top-left desktop point, outside the right-aligned test note.
move(CGPoint(x: 5, y: 40))
let outside = (info()[kCGWindowAlpha as String] as! NSNumber).doubleValue
move(inside)
let hovered = (info()[kCGWindowAlpha as String] as! NSNumber).doubleValue
move(CGPoint(x: 5, y: 40))
let left = (info()[kCGWindowAlpha as String] as! NSNumber).doubleValue
assert(abs(outside - 0.5) < 0.01 && abs(hovered - 1) < 0.01 && abs(left - 0.5) < 0.01, "Unexpected alpha: \(outside), \(hovered), \(left)")
print("PASS: actual native window alpha outside/inside/outside = \(outside)/\(hovered)/\(left)")
