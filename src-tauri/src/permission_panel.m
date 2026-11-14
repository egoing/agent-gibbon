#import <Cocoa/Cocoa.h>

@interface GibbonFileRow : NSButton <NSDraggingSource>
@property(nonatomic, strong) NSURL *fileURL;
@end
@implementation GibbonFileRow
- (void)mouseDown:(NSEvent *)event {
    NSDraggingItem *item = [[NSDraggingItem alloc] initWithPasteboardWriter:self.fileURL];
    NSImage *icon = [[NSWorkspace sharedWorkspace] iconForFile:self.fileURL.path];
    NSPoint point = [self convertPoint:event.locationInWindow fromView:nil];
    [item setDraggingFrame:NSMakeRect(point.x-16, point.y-16, 32, 32) contents:icon];
    [self beginDraggingSessionWithItems:@[item] event:event source:self];
}
- (NSDragOperation)draggingSession:(NSDraggingSession *)session sourceOperationMaskForDraggingContext:(NSDraggingContext)context {
    return NSDragOperationCopy;
}
@end

// Called on the main thread. The panel retains only the currently running app's URL.
static NSPanel *guidePanel;
static NSTimer *positionTimer;

// Quartz uses a top-left origin on the primary screen, AppKit a bottom-left origin.
static BOOL alignWithSettings(void) {
    NSArray *settings = [NSRunningApplication runningApplicationsWithBundleIdentifier:@"com.apple.systempreferences"];
    if (!settings.count || !NSScreen.screens.count) return NO;
    NSArray *windows = CFBridgingRelease(CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly |
        kCGWindowListExcludeDesktopElements, kCGNullWindowID));
    for (NSDictionary *entry in windows) {
        if ([entry[(id)kCGWindowOwnerPID] intValue] != [settings.firstObject processIdentifier] ||
            [entry[(id)kCGWindowLayer] integerValue] != 0) continue;
        CGRect bounds;
        if (!CGRectMakeWithDictionaryRepresentation((__bridge CFDictionaryRef)entry[(id)kCGWindowBounds], &bounds)
            || bounds.size.width < 350 || bounds.size.height < 200) continue;
        NSRect frame = NSMakeRect(bounds.origin.x, NSMaxY(NSScreen.screens.firstObject.frame)-CGRectGetMaxY(bounds),
                                  bounds.size.width, bounds.size.height);
        NSScreen *best = nil;
        CGFloat largest = 0;
        for (NSScreen *screen in NSScreen.screens) {
            NSRect overlap = NSIntersectionRect(frame, screen.frame);
            CGFloat area = NSWidth(overlap)*NSHeight(overlap);
            if (area > largest) { best = screen; largest = area; }
        }
        if (!best) continue;
        NSRect visible = best.visibleFrame;
        NSSize size = guidePanel.frame.size;
        CGFloat x = NSMidX(frame) - size.width/2;
        CGFloat y = NSMinY(frame) + 12;
        NSPoint origin = NSMakePoint(round(MAX(NSMinX(visible), MIN(x, NSMaxX(visible)-size.width))),
                                     round(MAX(NSMinY(visible), MIN(y, NSMaxY(visible)-size.height))));
        if (!NSEqualPoints(guidePanel.frame.origin, origin)) [guidePanel setFrameOrigin:origin];
        return YES;
    }
    return NO;
}
void gibbon_hide_permission_panel(void) {
    [positionTimer invalidate];
    positionTimer = nil;
    [guidePanel orderOut:nil];
}
void gibbon_show_permission_panel(BOOL screenCapture, NSWindow *overview) {
    NSURL *target = NSBundle.mainBundle.bundleURL;
    BOOL bundled = [target.pathExtension.lowercaseString isEqualToString:@"app"];
    if (!bundled) target = NSBundle.mainBundle.executableURL;
    if (!target) return;
    if (!guidePanel) {
        guidePanel = [[NSPanel alloc] initWithContentRect:NSMakeRect(0,0,500,124)
            styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable|NSWindowStyleMaskNonactivatingPanel
            backing:NSBackingStoreBuffered defer:NO];
        guidePanel.releasedWhenClosed = NO;
        guidePanel.floatingPanel = YES;
        guidePanel.hidesOnDeactivate = NO;
        guidePanel.collectionBehavior = NSWindowCollectionBehaviorMoveToActiveSpace;
    }
    guidePanel.title = screenCapture ? @"화면 캡처 허용" : @"손쉬운 사용 허용";
    NSView *body = [[NSView alloc] initWithFrame:NSMakeRect(0,0,500,124)];
    NSTextField *label = [NSTextField labelWithString:@"↑  아래 앱을 시스템 설정의 허용 목록으로 드래그하세요."];
    label.frame = NSMakeRect(16,92,470,20);
    [body addSubview:label];
    GibbonFileRow *row = [[GibbonFileRow alloc] initWithFrame:NSMakeRect(16,44,468,44)];
    row.fileURL = target;
    row.title = bundled ? target.lastPathComponent : [target.lastPathComponent stringByAppendingString:@"  ·  개발 실행 파일"];
    row.image = [[NSWorkspace sharedWorkspace] iconForFile:target.path];
    row.image.size = NSMakeSize(26,26);
    row.imagePosition = NSImageLeft;
    row.toolTip = target.path;
    [body addSubview:row];
    NSTextField *hint = [NSTextField labelWithString:bundled ? @"목록에 추가되지 않으면 + 버튼으로 앱을 선택하세요." : @"개발 실행 파일은 macOS가 목록 추가를 제한할 수 있습니다."];
    hint.frame = NSMakeRect(16,12,470,22);
    [body addSubview:hint];
    guidePanel.contentView = body;
    NSRect visible = (overview.screen ?: NSScreen.mainScreen).visibleFrame;
    [guidePanel setFrameTopLeftPoint:NSMakePoint(NSMidX(visible)-250, NSMinY(visible)+190)];
    alignWithSettings();
    [guidePanel orderFrontRegardless];
    [positionTimer invalidate];
    positionTimer = [NSTimer scheduledTimerWithTimeInterval:0.25 repeats:YES block:^(NSTimer *timer) {
        if (!guidePanel.visible) {
            [timer invalidate];
            positionTimer = nil;
            return;
        }
        alignWithSettings();
    }];
}
