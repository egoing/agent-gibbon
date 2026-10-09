// Native test input; restore the pointer after targeting only Floatnote controls.
import Foundation
import CoreGraphics
let offset = CommandLine.arguments.lastIndex(of: "--").map { $0 + 1 } ?? 1
let args = Array(CommandLine.arguments.dropFirst(offset))
let original = CGEvent(source: nil)!.location
func point(_ offset: Int) -> CGPoint { CGPoint(x: Double(args[offset])!, y: Double(args[offset + 1])!) }
func post(_ type: CGEventType, _ point: CGPoint) {
    CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: point, mouseButton: .left)!.post(tap: .cghidEventTap)
    usleep(60000)
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
