// Native input driver for the macOS end-to-end test; never linked into Floatnote.
import AppKit
import CoreGraphics
import Foundation

let args = Array(CommandLine.arguments.dropFirst())
if args.first == "move", args.count == 4,
   let x = Double(args[1]), let y = Double(args[2]) {
    let option = args[3] == "option"
    let event = CGEvent(mouseEventSource: nil, mouseType: .mouseMoved,
                        mouseCursorPosition: CGPoint(x: x, y: y), mouseButton: .left)!
    event.flags = option ? .maskAlternate : []
    event.post(tap: .cghidEventTap)
    usleep(150_000)
} else if args.first == "inspect" {
    let pointer = CGEvent(source: nil)!.location
    let windows = CGWindowListCopyWindowInfo([.optionAll], kCGNullWindowID) as! [[String: Any]]
    let candidates = windows.filter {
        ($0[kCGWindowOwnerName as String] as? String)?.lowercased() == "floatnote"
            && ($0[kCGWindowLayer as String] as? Int ?? -1) >= 0
            && ($0[kCGWindowBounds as String] as? [String: Any]) != nil
    }
    // Main note uses 380×460 points initially; exclude AppKit's helper window.
    guard let window = candidates.first(where: { ($0[kCGWindowName as String] as? String) == "Floatnote" })
        ?? candidates.first(where: { ($0[kCGWindowIsOnscreen as String] as? Bool) == true }) else { exit(3) }
    let bounds = window[kCGWindowBounds as String] as! [String: Double]
    let rect = CGRect(x: bounds["X"]!, y: bounds["Y"]!, width: bounds["Width"]!, height: bounds["Height"]!)
    let top = NSScreen.screens[0].frame.maxY
    let screen = NSScreen.screens.max { a, b in
        func area(_ screen: NSScreen) -> CGFloat {
            let f = screen.frame
            let r = CGRect(x: f.minX, y: top - f.maxY, width: f.width, height: f.height).intersection(rect)
            return r.isNull ? 0 : r.width * r.height
        }
        return area(a) < area(b)
    }!
    let work = screen.visibleFrame
    let data: [String: Any] = ["x":rect.minX,"y":rect.minY,"width":rect.width,"height":rect.height,
        "pointerX":pointer.x,"pointerY":pointer.y,"option":NSEvent.modifierFlags.contains(.option),
        "screenX":work.minX,"screenY":top-work.maxY,"screenWidth":work.width,"screenHeight":work.height,
        "pid":window[kCGWindowOwnerPID as String]!,"windowID":window[kCGWindowNumber as String]!]
    print(String(data: try! JSONSerialization.data(withJSONObject: data, options: [.sortedKeys]), encoding:.utf8)!)
} else { exit(2) }
