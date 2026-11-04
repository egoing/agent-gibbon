// Native test input; restore the pointer after targeting only Agent Gibbon controls.
import Foundation
import CoreGraphics
guard CGPreflightPostEventAccess() else {
    fputs("INPUT_UNAVAILABLE: macOS event posting permission denied; no native click/drag was sent.\n", stderr)
    exit(77)
}
let offset = CommandLine.arguments.lastIndex(of: "--").map { $0 + 1 } ?? 1
let args = Array(CommandLine.arguments.dropFirst(offset))
var delay: useconds_t = 60000
if let index = CommandLine.arguments.firstIndex(of: "--delay-ms"), index + 1 < offset {
    guard let ms = UInt32(CommandLine.arguments[index + 1]), (10...1000).contains(ms) else { exit(64) }
    delay = ms * 1000
}
let original = CGEvent(source: nil)!.location
func point(_ offset: Int) -> CGPoint { CGPoint(x: Double(args[offset])!, y: Double(args[offset + 1])!) }
func post(_ type: CGEventType, _ point: CGPoint) {
    CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)!.post(tap: .cghidEventTap)
    usleep(delay)
}
let start = point(1)
post(.leftMouseDown, start)
if args[0] == "drag" {
    let end = point(3)
    for step in 1...8 {
        let fraction = CGFloat(step) / 8
        post(.leftMouseDragged, CGPoint(x: start.x + (end.x - start.x) * fraction, y: start.y + (end.y - start.y) * fraction))
    }
    post(.leftMouseUp, end)
} else { post(.leftMouseUp, start) }
usleep(100000)
CGWarpMouseCursorPosition(original)
