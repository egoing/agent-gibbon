#import <Cocoa/Cocoa.h>
#import <ApplicationServices/ApplicationServices.h>
#import <unistd.h>

typedef void (*VisitRect)(void *, double, double, double, double, bool);

static id attribute(AXUIElementRef element, CFStringRef key) {
    CFTypeRef value = NULL;
    if (AXUIElementCopyAttributeValue(element, key, &value) != kAXErrorSuccess) return nil;
    return CFBridgingRelease(value);
}

static bool elementRect(AXUIElementRef element, CGRect *rect) {
    id pos = attribute(element, kAXPositionAttribute);
    id size = attribute(element, kAXSizeAttribute);
    return pos && size && CFGetTypeID((__bridge CFTypeRef)pos) == AXValueGetTypeID()
        && CFGetTypeID((__bridge CFTypeRef)size) == AXValueGetTypeID()
        && AXValueGetValue((__bridge AXValueRef)pos, kAXValueCGPointType, &rect->origin)
        && AXValueGetValue((__bridge AXValueRef)size, kAXValueCGSizeType, &rect->size);
}

// Inspect geometry and roles only. Editable and scrollable surfaces reserve
// their entire viewport, including blank pixels within them.
static bool contentBounds(AXUIElementRef container, CGRect body, CGRect *content,
                          int *budget, CFAbsoluteTime deadline) {
    NSArray *children = attribute(container, kAXChildrenAttribute);
    if (![children isKindOfClass:NSArray.class] || !children.count) return false;
    for (id object in children) {
        if (--*budget < 0 || CFAbsoluteTimeGetCurrent() > deadline) return false;
        AXUIElementRef child = (__bridge AXUIElementRef)object;
        CGRect frame;
        if (!elementRect(child, &frame)) return false;
        frame = CGRectIntersection(frame, body);
        if (CGRectIsEmpty(frame)) continue;
        NSString *role = attribute(child, kAXRoleAttribute);
        // Only split groups have a known layout-only role. Generic groups may
        // implement editors or canvases with unreported interactive children.
        if ([role isEqualToString:(__bridge NSString *)kAXSplitGroupRole]
            && contentBounds(child, body, content, budget, deadline)) continue;
        *content = CGRectUnion(*content, frame);
    }
    return true;
}

static void visitGutters(pid_t pid, CGRect bounds, NSArray<NSValue *> *above,
                         void *context, VisitRect visit) {
    if (!AXIsProcessTrusted()) return;
    AXUIElementRef app = AXUIElementCreateApplication(pid);
    AXUIElementSetMessagingTimeout(app, 0.08);
    id object = attribute(app, kAXFocusedWindowAttribute);
    CFRelease(app);
    if (!object) return;
    AXUIElementRef window = (__bridge AXUIElementRef)object;
    AXUIElementSetMessagingTimeout(window, 0.08);
    CGRect frame;
    if (!elementRect(window, &frame) || !CGRectEqualToRect(bounds, frame)) return;
    CGRect body = CGRectInset(bounds, 12, 12);
    body.origin.y += 48;
    body.size.height -= 48;
    if (CGRectIsEmpty(body)) return;
    CGRect content = CGRectNull;
    int budget = 96;
    if (!contentBounds(window, body, &content, &budget, CFAbsoluteTimeGetCurrent()+0.20)
        || CGRectIsNull(content)) return;
    CGFloat right = CGRectGetMaxX(content) + 12;
    CGRect margins[] = {
        CGRectMake(body.origin.x, body.origin.y, CGRectGetMinX(content)-12-body.origin.x, body.size.height),
        CGRectMake(right, body.origin.y, CGRectGetMaxX(body)-right, body.size.height)
    };
    for (int i=0; i<2; i++) {
        CGRect r = margins[i];
        if (r.size.width <= 0 || r.size.height <= 0) continue;
        bool visible = true;
        for (NSValue *v in above) if (CGRectIntersectsRect(r, v.rectValue)) { visible=false; break; }
        if (visible) visit(context, CGRectGetMinX(r), CGRectGetMinY(r), CGRectGetMaxX(r), CGRectGetMaxY(r), true);
    }
}

// Quartz coordinates use the same top-left desktop space as xcap on macOS.
bool jajan_visit_app_windows(void *context, VisitRect visit) {
    NSArray *windows = CFBridgingRelease(CGWindowListCopyWindowInfo(
        kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, kCGNullWindowID));
    if (!windows) return false;
    pid_t front = NSWorkspace.sharedWorkspace.frontmostApplication.processIdentifier;
    NSMutableArray<NSValue *> *above = [NSMutableArray array];
    bool inspected = false;
    for (NSDictionary *entry in windows) {
        pid_t pid = [entry[(id)kCGWindowOwnerPID] intValue];
        NSInteger layer = [entry[(id)kCGWindowLayer] integerValue];
        if ([entry[(id)kCGWindowAlpha] doubleValue] <= 0 || layer < 0) continue;
        CGRect bounds;
        if (!CGRectMakeWithDictionaryRepresentation((__bridge CFDictionaryRef)entry[(id)kCGWindowBounds], &bounds)) return false;
        if (CGRectIsEmpty(bounds) || pid==getpid()) continue;
        if (layer == 0) {
            visit(context, CGRectGetMinX(bounds), CGRectGetMinY(bounds), CGRectGetMaxX(bounds), CGRectGetMaxY(bounds), false);
            if (!inspected && pid==front) {
                visitGutters(pid,bounds,above,context,visit);
                inspected=true;
            }
        }
        [above addObject:[NSValue valueWithRect:bounds]];
    }
    return true;
}
