import AppKit
import ApplicationServices
let rows = CGWindowListCopyWindowInfo([.optionOnScreenOnly,.excludeDesktopElements],kCGNullWindowID) as! [[String:Any]]
let pid=Int(CommandLine.arguments[1])!
let windows=rows.filter { ($0[kCGWindowOwnerPID as String] as? Int)==pid }
let screens=NSScreen.screens.map { s -> [String:Any] in
    ["frame":[s.frame.origin.x,s.frame.origin.y,s.frame.width,s.frame.height],"visible":[s.visibleFrame.origin.x,s.visibleFrame.origin.y,s.visibleFrame.width,s.visibleFrame.height],"scale":s.backingScaleFactor]
}
print(String(data:try! JSONSerialization.data(withJSONObject:["windows":windows,"screens":screens]),encoding:.utf8)!)
