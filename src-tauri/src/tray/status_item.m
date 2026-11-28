#import <AppKit/AppKit.h>

// Keep tray-icon's mouse-enter tracking area for the recent-items refresh.
// Let the status button handle the original click and track its attached menu.
@interface JajanStatusButtonHitView : NSView
@end

@implementation JajanStatusButtonHitView
- (NSView *)hitTest:(NSPoint)point {
    return NSPointInRect(point, self.frame) ? self.superview : nil;
}
@end

void jajan_attach_status_menu(NSStatusItem *item, NSMenu *menu) {
    NSStatusBarButton *button = item.button;
    if (![button.subviews.lastObject isKindOfClass:JajanStatusButtonHitView.class]) {
        NSView *hit = [[JajanStatusButtonHitView alloc] initWithFrame:button.bounds];
        hit.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
        [button addSubview:hit positioned:NSWindowAbove relativeTo:nil];
    }
    item.menu = menu;
}
