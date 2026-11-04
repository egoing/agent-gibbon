import AppKit
import CoreGraphics
var n: UInt32=0
let s=CGGetActiveDisplayList(0,nil,&n)
var v:[String:Any] = ["session":CGSessionCopyCurrentDictionary() as? [String:Any] ?? [:],"active_displays":n,"active_status":s.rawValue,"front_pid":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,"pointer":[CGEvent(source:nil)!.location.x,CGEvent(source:nil)!.location.y]]
if CommandLine.arguments.count>1 { NSRunningApplication(processIdentifier:Int32(CommandLine.arguments[1])!)?.activate(options:[.activateIgnoringOtherApps]); CGWarpMouseCursorPosition(CGPoint(x:Double(CommandLine.arguments[2])!,y:Double(CommandLine.arguments[3])!)) }
print(String(data:try! JSONSerialization.data(withJSONObject:v),encoding:.utf8)!)
