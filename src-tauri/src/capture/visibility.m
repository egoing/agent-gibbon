#import <Cocoa/Cocoa.h>
#import <unistd.h>
#import <Accelerate/Accelerate.h>

// Quartz expects window numbers as raw pointer-sized integers in front-to-back order.
CFArrayRef jajan_copy_background_windows(void) {
    @autoreleasepool {
        NSArray *windows = CFBridgingRelease(CGWindowListCopyWindowInfo(
            kCGWindowListOptionOnScreenOnly, kCGNullWindowID));
        if (!windows) return NULL;
        CFMutableArrayRef ids = CFArrayCreateMutable(NULL, windows.count, NULL);
        if (!ids) return NULL;
        for (NSDictionary *window in windows) {
            if ([window[(id)kCGWindowOwnerPID] intValue] == getpid()) continue;
            uintptr_t number = [window[(id)kCGWindowNumber] unsignedIntValue];
            CFArrayAppendValue(ids, (const void *)number);
        }
        return ids;
    }
}

// Copy padded Quartz BGRA rows into tightly packed RGBA in one pass.
bool jajan_copy_rgba(const uint8_t *source, uint8_t *destination,
                     size_t width, size_t height, size_t stride) {
    vImage_Buffer input = {(void *)source, height, width, stride};
    vImage_Buffer output = {destination, height, width, width * 4};
    const uint8_t channels[] = {2, 1, 0, 3};
    return vImagePermuteChannels_ARGB8888(&input, &output, channels, kvImageNoFlags) == kvImageNoError;
}

// Copy only the ordering metadata needed to restore windows across applications.
bool jajan_visit_window_order(void *context, void (*visit)(void *, uint32_t, int32_t)) {
    NSArray *windows = CFBridgingRelease(CGWindowListCopyWindowInfo(
        kCGWindowListOptionOnScreenOnly, kCGNullWindowID));
    if (!windows) return false;
    for (NSDictionary *window in windows)
        visit(context, [window[(id)kCGWindowNumber] unsignedIntValue],
              [window[(id)kCGWindowLayer] intValue]);
    return true;
}
