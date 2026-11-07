import AppKit
import ApplicationServices
import CoreGraphics
let args = CommandLine.arguments
func emit(_ x: Any) { print(String(data: try! JSONSerialization.data(withJSONObject:x,options:[.sortedKeys]),encoding:.utf8)!) }
func windows(_ pid: Int32) -> [[String:Any]] { (CGWindowListCopyWindowInfo(.optionOnScreenOnly,kCGNullWindowID) as? [[String:Any]] ?? []).filter { ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid && ($0[kCGWindowLayer as String] as? Int) == 0 } }
func axWindow(_ pid: Int32) -> AXUIElement {
 var raw: CFTypeRef?
 let app = AXUIElementCreateApplication(pid)
 let err = AXUIElementCopyAttributeValue(app,kAXWindowsAttribute as CFString,&raw)
 guard err == .success, let ws = raw as? [AXUIElement], let w = ws.first else { emit(["axError":err.rawValue]);exit(77) }
 return w
}
func attr(_ w: AXUIElement,_ key:String) -> CFTypeRef? { var r:CFTypeRef?; AXUIElementCopyAttributeValue(w,key as CFString,&r); return r }
final class Recorder: NSObject {
 let file:String; let view:NSTextField; var clicks=0
 init(_ file:String,_ view:NSTextField) { self.file=file;self.view=view }
 @objc func clicked() { clicks+=1;save() }
 @objc func save() { let x:[String:Any] = ["pid":ProcessInfo.processInfo.processIdentifier,"text":view.stringValue,"clicks":clicks,"active":NSApp.isActive];try! JSONSerialization.data(withJSONObject:x,options:[.sortedKeys]).write(to:URL(fileURLWithPath:file),options:.atomic) }
}
if args[1] == "fixture" {
 let tag=args[2], path=args[3];let app=NSApplication.shared;app.setActivationPolicy(.regular)
 let w=NSWindow(contentRect:NSRect(x:100,y:200,width:480,height:300),styleMask:[.titled,.closable,.miniaturizable,.resizable],backing:.buffered,defer:false)
 w.title="Issue86 Mac PoC \(tag)";w.isReleasedWhenClosed=false
 w.contentView!.wantsLayer=true; w.contentView!.layer!.backgroundColor=(tag == "A" ? NSColor(calibratedRed:0.3,green:0.6,blue:0.95,alpha:1) : NSColor(calibratedRed:0.95,green:0.6,blue:0.25,alpha:1)).cgColor
 let label=NSTextField(labelWithString:"External process \(tag) — NORMAL WINDOW");label.frame=NSRect(x:20,y:240,width:430,height:30);w.contentView!.addSubview(label)
 let text=NSTextField(frame:NSRect(x:20,y:170,width:400,height:40));text.stringValue="";w.contentView!.addSubview(text)
 let button=NSButton(title:"Record real click",target:nil,action:nil);button.frame=NSRect(x:20,y:100,width:200,height:40);w.contentView!.addSubview(button)
 let recorder=Recorder(path,text);button.target=recorder;button.action=#selector(Recorder.clicked)
 let timer=Timer.scheduledTimer(timeInterval:0.1,target:recorder,selector:#selector(Recorder.save),userInfo:nil,repeats:true)
 w.makeKeyAndOrderFront(nil);app.activate(ignoringOtherApps:true);recorder.save();withExtendedLifetime((w,recorder,timer)){app.run()};exit(0)
}
if args[1] == "environment" {
 emit(["trusted":AXIsProcessTrusted(),"postEvents":CGPreflightPostEventAccess(),"screenCapture":CGPreflightScreenCaptureAccess(),"frontPID":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,"pointer":[CGEvent(source:nil)!.location.x,CGEvent(source:nil)!.location.y],"displays":NSScreen.screens.map{["frame":NSStringFromRect($0.frame),"scale":$0.backingScaleFactor]}]);exit(0)
}
if args[1] == "restorePointer" { CGWarpMouseCursorPosition(CGPoint(x:Double(args[2])!,y:Double(args[3])!));exit(0) }
let pid=Int32(args[2])!
if args[1] == "activate" { let ok = NSRunningApplication(processIdentifier:pid)?.activate(options:[]) ?? false; RunLoop.current.run(until:Date(timeIntervalSinceNow:0.3));emit(["activate":ok]);exit(0) }
let w=axWindow(pid)
switch args[1] {
case "state":
 var attrs:CFArray?;let e=AXUIElementCopyAttributeNames(w,&attrs)
 var acts:CFArray?;AXUIElementCopyActionNames(w,&acts)
 var writable:[String:Bool]=[:]
 for key in (attrs as? [String] ?? []) { var b:DarwinBoolean=false; AXUIElementIsAttributeSettable(w,key as CFString,&b);writable[key]=b.boolValue }
 emit(["pid":pid,"title":attr(w,kAXTitleAttribute) as? String ?? "","windows":windows(pid),"attributeError":e.rawValue,"attributes":attrs as? [String] ?? [],"writable":writable,"actions":acts as? [String] ?? [],"frontPID":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0])
case "move":
 var p=CGPoint(x:Double(args[3])!,y:Double(args[4])!);var s=CGSize(width:Double(args[5])!,height:Double(args[6])!)
 let ep=AXUIElementSetAttributeValue(w,kAXPositionAttribute as CFString,AXValueCreate(.cgPoint,&p)!);let es=AXUIElementSetAttributeValue(w,kAXSizeAttribute as CFString,AXValueCreate(.cgSize,&s)!)
 emit(["positionResult":ep.rawValue,"sizeResult":es.rawValue])
case "raise":emit(["raiseResult":AXUIElementPerformAction(w,kAXRaiseAction as CFString).rawValue])
case "level":
 var b:DarwinBoolean=false
 let e=AXUIElementIsAttributeSettable(w,"AXWindowLevel" as CFString,&b)
 let r=AXUIElementSetAttributeValue(w,"AXWindowLevel" as CFString,NSNumber(value:3))
 emit(["attribute":"AXWindowLevel (capability probe; not an Apple public AX attribute)","settableResult":e.rawValue,"settable":b.boolValue,"setResult":r.rawValue])
case "order":
 let other=Int32(args[3])!;let all=CGWindowListCopyWindowInfo(.optionOnScreenOnly,kCGNullWindowID) as! [[String:Any]]
 emit(["frontPID":NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,"frontToBack":all.filter{ let p=($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value; return (p==pid || p==other) && ($0[kCGWindowLayer as String] as? Int)==0 }])
case "input":
 guard CGPreflightPostEventAccess(),NSWorkspace.shared.frontmostApplication?.processIdentifier == pid else { emit(["blocked":"not permitted or unexpected foreground PID"]);exit(77) }
 let p=CGPoint(x:Double(args[3])!,y:Double(args[4])!)
 for type:CGEventType in [.leftMouseDown,.leftMouseUp] { CGEvent(mouseEventSource:nil,mouseType:type,mouseCursorPosition:p,mouseButton:.left)!.post(tap:.cghidEventTap);usleep(80000) }
 if args.count>5 { let text=Array(args[5].utf16);for down in [true,false] { let e=CGEvent(keyboardEventSource:nil,virtualKey:0,keyDown:down)!;e.keyboardSetUnicodeString(stringLength:text.count,unicodeString:text);e.post(tap:.cghidEventTap);usleep(80000) } }
 emit(["posted":true,"pid":pid])
default:exit(64)
}
