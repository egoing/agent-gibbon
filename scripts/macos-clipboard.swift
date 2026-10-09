// Test helper: keep every pasteboard representation out of logs and restore it.
import AppKit
import Foundation
let offset = CommandLine.arguments.lastIndex(of: "--").map { $0 + 1 } ?? 1
let args = Array(CommandLine.arguments.dropFirst(offset))
let url = URL(fileURLWithPath: args[1])
let board = NSPasteboard.general
if args[0] == "save" {
    let items = (board.pasteboardItems ?? []).map { item -> [String: Data] in
        var values = [String: Data]()
        for type in item.types { if let data = item.data(forType: type) { values[type.rawValue] = data } }
        return values
    }
    let bytes = try PropertyListSerialization.data(fromPropertyList: items, format: .binary, options: 0)
    try bytes.write(to: url)
    try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
} else {
    let values = try PropertyListSerialization.propertyList(from: Data(contentsOf: url), options: [], format: nil) as! [[String: Data]]
    let items = values.map { values -> NSPasteboardItem in
        let item = NSPasteboardItem()
        for (type, data) in values { item.setData(data, forType: NSPasteboard.PasteboardType(type)) }
        return item
    }
    board.clearContents()
    if !items.isEmpty { board.writeObjects(items) }
}
