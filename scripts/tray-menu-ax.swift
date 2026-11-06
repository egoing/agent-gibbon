// Read or press the actual tray menu belonging to exactly one verified PID.
import ApplicationServices
import Foundation
func attr(_ e: AXUIElement, _ k: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(e, k as CFString, &value) == .success ? value : nil
}
func text(_ e: AXUIElement, _ k: String) -> String { attr(e,k) as? String ?? "" }
func walk(_ e: AXUIElement, _ depth: Int = 0) -> [AXUIElement] {
    if depth > 16 { return [] }
    return [e] + (attr(e,kAXChildrenAttribute) as? [AXUIElement] ?? []).flatMap { walk($0,depth+1) }
}
func normalized(_ s: String) -> String { s.replacingOccurrences(of: "[", with: " ").replacingOccurrences(of: "]", with: " ").split(whereSeparator: { $0.isWhitespace }).joined(separator: " ") }
let args=CommandLine.arguments
let app=AXUIElementCreateApplication(pid_t(args[1])!)
guard let bar=attr(app,kAXExtrasMenuBarAttribute) else { fputs("tray unavailable\n",stderr);exit(1) }
let nodes=walk(bar as! AXUIElement)
if args.count>2 {
    guard let target=nodes.first(where:{ text($0,kAXRoleAttribute)=="AXMenuItem" && normalized(text($0,kAXTitleAttribute))==normalized(args[2]) }) else { fputs("item unavailable\n",stderr);exit(1) }
    guard AXUIElementPerformAction(target,kAXPressAction as CFString) == .success else { exit(1) }
} else {
    let rows=nodes.filter { text($0,kAXRoleAttribute)=="AXMenuItem" }.map { e -> [String:Any] in
        ["title":text(e,kAXTitleAttribute),"enabled":attr(e,kAXEnabledAttribute) as? Bool ?? false]
    }
    print(String(data:try! JSONSerialization.data(withJSONObject:rows,options:[.sortedKeys]),encoding:.utf8)!)
}
