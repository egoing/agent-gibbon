// Read-only native focus probe for isolation regression tests.
import AppKit
print(NSWorkspace.shared.frontmostApplication?.processIdentifier ?? -1)
