// Test-only bare modifier tap. Mixed input is delivered only to a supplied PID.
import CoreGraphics
import Foundation
guard CGPreflightPostEventAccess() else { exit(77) }
let args=Array(CommandLine.arguments.dropFirst())
func flags(_ code: CGKeyCode, _ value: CGEventFlags) {
    let event=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown: !value.isEmpty)!
    event.type = .flagsChanged;event.flags=value;event.post(tap:.cghidEventTap)
    usleep(args.first == "fast" ? 20000 : 50000)
}
if args.isEmpty || args.first == "fast" {
    flags(55,[.maskControl,.maskAlternate,.maskCommand]);usleep(20000);flags(55,[]);usleep(40000);exit(0)
}
flags(59,[.maskControl]);flags(58,[.maskControl,.maskAlternate]);flags(55,[.maskControl,.maskAlternate,.maskCommand])
if args.first == "mixed", args.count == 2, let pid=Int32(args[1]) {
    for down in [true,false] {
        let event=CGEvent(keyboardEventSource:nil,virtualKey:0,keyDown:down)!
        event.flags=[.maskControl,.maskAlternate,.maskCommand]
        event.postToPid(pid);usleep(50000)
    }
} else if args.first == "hold" { usleep(700000) }
else { usleep(args.first == "fast" ? 10000 : 60000) }
flags(55,[.maskControl,.maskAlternate]);flags(58,[.maskControl]);flags(59,[])
usleep(40000)
