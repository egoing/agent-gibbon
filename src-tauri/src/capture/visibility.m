#import <Cocoa/Cocoa.h>

// Query WindowServer, not NSWindow's local visibility flag. Hidden windows must
// be absent from the same on-screen window list used by Quartz screen capture.
bool gibbon_capture_windows_hidden(const int64_t *ids, size_t count) {
    @autoreleasepool {
        NSArray *windows = CFBridgingRelease(CGWindowListCopyWindowInfo(
            kCGWindowListOptionOnScreenOnly, kCGNullWindowID));
        if (!windows) return false;
        for (NSDictionary *window in windows) {
            int64_t number = [window[(id)kCGWindowNumber] longLongValue];
            for (size_t i = 0; i < count; ++i) {
                if (ids[i] == number) return false;
            }
        }
        return true;
    }
}
