import AppKit
import ApplicationServices
import CoreGraphics
import Foundation
let args = CommandLine.arguments
func attr(_ e: AXUIElement, _ k: String) -> CFTypeRef? { var v:CFTypeRef?; return AXUIElementCopyAttributeValue(e,k as CFString,&v) == .success ? v:nil }
func str(_ e:AXUIElement,_ k:String)->String { attr(e,k) as? String ?? "" }
func xy(_ e:AXUIElement,_ k:String)->[String:Double] { guard let v=attr(e,k) else {return [:]}; let a=v as! AXValue; if k == kAXPositionAttribute { var p=CGPoint.zero; AXValueGetValue(a,.cgPoint,&p); return ["x":p.x,"y":p.y] }; var s=CGSize.zero; AXValueGetValue(a,.cgSize,&s); return ["width":s.width,"height":s.height] }
func node(_ e:AXUIElement,depth:Int=0)->[String:Any] { var o:[String:Any] = ["role":str(e,kAXRoleAttribute),"title":str(e,kAXTitleAttribute),"description":str(e,kAXDescriptionAttribute),"value":str(e,kAXValueAttribute),"position":xy(e,kAXPositionAttribute),"size":xy(e,kAXSizeAttribute)]; if depth<25 {o["children"]=(attr(e,kAXChildrenAttribute) as? [AXUIElement] ?? []).map{node($0,depth:depth+1)}};return o }
func output(_ o:Any) { print(String(data:try! JSONSerialization.data(withJSONObject:o,options:[.sortedKeys]),encoding:.utf8)!) }
switch args[1] {
case "access": output(["post":CGPreflightPostEventAccess(),"ax":AXIsProcessTrusted()])
case "pointer": let p=CGEvent(source:nil)!.location; output(["x":p.x,"y":p.y,"front":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0])
case "move": let p=CGPoint(x:Double(args[2])!,y:Double(args[3])!); CGWarpMouseCursorPosition(p); let e=CGEvent(mouseEventSource:nil,mouseType:.mouseMoved,mouseCursorPosition:p,mouseButton:.left)!; if args.count>4 {e.flags = .maskControl}; e.post(tap:.cghidEventTap)
case "ctrl": let down=args[2]=="down"; let e=CGEvent(keyboardEventSource:nil,virtualKey:59,keyDown:down)!; e.flags=down ? .maskControl:[]; e.post(tap:.cghidEventTap)
case "windows": let all=CGWindowListCopyWindowInfo(.optionOnScreenOnly,kCGNullWindowID) as! [[String:Any]];output(all.filter{($0[kCGWindowOwnerPID as String] as? NSNumber)?.intValue == Int(args[2])!})
case "ax": let app=AXUIElementCreateApplication(pid_t(args[2])!); let ws=attr(app,kAXWindowsAttribute) as? [AXUIElement] ?? [];output(ws.map{node($0)})
case "activate": NSRunningApplication(processIdentifier:pid_t(args[2])!)?.activate(options:[.activateIgnoringOtherApps])
case "position":
 let app=AXUIElementCreateApplication(pid_t(args[2])!); let ws=attr(app,kAXWindowsAttribute) as? [AXUIElement] ?? []
 guard let w=ws.first(where:{str($0,kAXTitleAttribute)==args[3]}) else {exit(2)}
 var point=CGPoint(x:Double(args[4])!,y:Double(args[5])!); let v=AXValueCreate(.cgPoint,&point)!
 print(AXUIElementSetAttributeValue(w,kAXPositionAttribute as CFString,v).rawValue)
case "press":
 let app=AXUIElementCreateApplication(pid_t(args[2])!); let ws=attr(app,kAXWindowsAttribute) as? [AXUIElement] ?? []
 guard let w=ws.first(where:{str($0,kAXTitleAttribute)==args[3]}) else {exit(2)}
 func find(_ e:AXUIElement)->AXUIElement? { if str(e,kAXRoleAttribute)=="AXButton" && str(e,kAXTitleAttribute)==args[4] {return e}; for c in attr(e,kAXChildrenAttribute) as? [AXUIElement] ?? [] {if let found=find(c) {return found}};return nil }
 guard let button=find(w) else {exit(3)}
 print(AXUIElementPerformAction(button,kAXPressAction as CFString).rawValue)
case "click": let p=CGPoint(x:Double(args[2])!,y:Double(args[3])!); for kind in [CGEventType.leftMouseDown,.leftMouseUp] {CGEvent(mouseEventSource:nil,mouseType:kind,mouseCursorPosition:p,mouseButton:.left)!.post(tap:.cghidEventTap)}
default: exit(1)
}
