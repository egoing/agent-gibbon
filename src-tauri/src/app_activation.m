#import <Cocoa/Cocoa.h>
#import <WebKit/WebKit.h>
#import <Carbon/Carbon.h>
#import <objc/runtime.h>

// Wry explicitly activates NSApplication while attaching every WKWebView.
// macOS may deliver that activation after the Rust builder has returned. Do not
// enqueue the request at all during passive creation; notification timing cannot
// reliably distinguish it from a later Command-Tab selection.
static NSUInteger passiveCreationDepth;
static void (*originalActivate)(id, SEL);
static void (*originalActivateIgnoring)(id, SEL, BOOL);
static void passiveActivate(id app, SEL selector) {
    if (!passiveCreationDepth) originalActivate(app, selector);
}
static void passiveActivateIgnoring(id app, SEL selector, BOOL ignore) {
    if (!passiveCreationDepth) originalActivateIgnoring(app, selector, ignore);
}
void jajan_activation_begin_creation(void) {
    NSCAssert(NSThread.isMainThread, @"Passive creation is main-thread only");
    static dispatch_once_t once;
    dispatch_once(&once, ^{
        Class cls = NSApplication.class;
        // activate is available on macOS 14+. Earlier Wry uses the older method.
        Method method = class_getInstanceMethod(cls, NSSelectorFromString(@"activate"));
        if (method) originalActivate = (void (*)(id, SEL))
            method_setImplementation(method, (IMP)passiveActivate);
        method = class_getInstanceMethod(cls, @selector(activateIgnoringOtherApps:));
        originalActivateIgnoring = (void (*)(id, SEL, BOOL))
            method_setImplementation(method, (IMP)passiveActivateIgnoring);
    });
    ++passiveCreationDepth;
}
void jajan_activation_end_creation(void) {
    NSCAssert(NSThread.isMainThread && passiveCreationDepth, @"Unbalanced passive creation");
    --passiveCreationDepth;
}

static BOOL loginEvent(void) {
    NSAppleEventDescriptor *event = NSAppleEventManager.sharedAppleEventManager.currentAppleEvent;
    return event.eventID == kAEOpenApplication &&
        [event paramDescriptorForKeyword:keyAEPropData].enumCodeValue == keyAELaunchedAsLogInItem;
}

// AppKit does not expose a Command-Tab-specific activation notification. Treat
// non-mouse activation as an app-level selection, excluding our passive shows.
// Keep Tao's application delegate intact (including its Dock reopen handler).
static NSUInteger generation;
static BOOL passiveTurn;
static void (*openManager)(void);
static id managerRestoreObserver;
static BOOL contentActivation;
static BOOL activatingWithMouse;

static void cancelManagerRestore(void) {
    if (managerRestoreObserver) {
        [NSNotificationCenter.defaultCenter removeObserver:managerRestoreObserver];
        managerRestoreObserver = nil;
    }
}

// Content selection intentionally focuses the editor/overlay. Consume the
// resulting activation notification instead of interpreting it as Command-Tab.
// This marker follows the activation request, not an arbitrary queue-turn delay.
void jajan_activation_focus_content(void *pointer) {
    NSCAssert(NSThread.isMainThread, @"Content focus is main-thread only");
    NSWindow *window = (__bridge NSWindow *)pointer;
    ++generation;
    cancelManagerRestore();
    contentActivation = !NSApp.active;
    [window makeKeyAndOrderFront:nil];
    // Keep the existing pre-macOS-14 explicit capture/editor focus behavior.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    BOOL activated = [NSRunningApplication.currentApplication
        activateWithOptions:NSApplicationActivateIgnoringOtherApps];
#pragma clang diagnostic pop
    if (!activated) contentActivation = NO;
}

// AppKit sends the Dock action to its target, potentially with a nil sender.
// Use the actual window as target so equal titles never share an action.
@interface NSWindow (JajanDock)
- (void)jajanSelectDockWindow:(id)sender;
@end
@implementation NSWindow (JajanDock)
- (void)jajanSelectDockWindow:(id)sender {
    if (self.visible && !self.miniaturized)
        jajan_activation_focus_content((__bridge void *)self);
}
@end

extern void jajan_populate_dock_menu(void *menu);

static NSMenu *dockMenu(id delegate, SEL selector, NSApplication *app) {
    (void)delegate;
    (void)selector;
    (void)app;
    NSMenu *menu = [[NSMenu alloc] initWithTitle:@""];
    jajan_populate_dock_menu((__bridge void *)menu);
    return menu;
}

void jajan_add_dock_window(void *menuPointer, void *windowPointer,
                           const unsigned char *bytes, size_t length) {
    NSMenu *menu = (__bridge NSMenu *)menuPointer;
    NSWindow *window = (__bridge NSWindow *)windowPointer;
    if (!window.visible || window.miniaturized) return;
    NSString *title = [[NSString alloc] initWithBytes:bytes length:length
                                             encoding:NSUTF8StringEncoding];
    NSMenuItem *item = [menu addItemWithTitle:title action:@selector(jajanSelectDockWindow:)
                                keyEquivalent:@""];
    // Keep the target valid if the user closes the window while the menu is open.
    item.representedObject = window;
    item.target = window;
    item.state = window.keyWindow ? NSControlStateValueOn : NSControlStateValueOff;
    if (@available(macOS 11, *)) {
        item.image = [NSImage imageWithSystemSymbolName:@"macwindow" accessibilityDescription:nil];
    }
}

// Tao's set_focus skips a window while its deminiaturization is animated.
// Complete this explicit focus request on the native completion notification,
// but never after a subsequent user input or application switch.
void jajan_activation_focus_manager(void *pointer) {
    ++generation; // An explicit manager selection supersedes a pending memo close.
    passiveTurn = NO;
    contentActivation = NO;
    NSWindow *window = (__bridge NSWindow *)pointer;
    cancelManagerRestore();
    if (window.miniaturized) {
        NSUInteger ticket = generation;
        __weak NSWindow *weakWindow = window;
        managerRestoreObserver = [NSNotificationCenter.defaultCenter
            addObserverForName:NSWindowDidDeminiaturizeNotification object:window queue:nil
            usingBlock:^(NSNotification *notification) {
                (void)notification;
                cancelManagerRestore();
                if (ticket == generation && NSApp.active && weakWindow.visible)
                    [weakWindow makeKeyAndOrderFront:nil];
            }];
    }
    [window makeKeyAndOrderFront:nil];
}

void jajan_activation_preserve_focus(void) {
    NSCAssert([NSThread isMainThread], @"Activation state is main-thread only");
    NSUInteger ticket = ++generation;
    passiveTurn = YES;
    dispatch_async(dispatch_get_main_queue(), ^{
        if (generation == ticket) passiveTurn = NO;
    });
}

// Tao destroys the NSWindow asynchronously. Preserve the manager's position
// across that native close, including other applications at the same level.
static char closeCompletionKey;
static NSArray<NSNumber *> *peersOf(NSWindow *manager, CGWindowListOption side) {
    NSArray *list = CFBridgingRelease(CGWindowListCopyWindowInfo(
        side, (CGWindowID)manager.windowNumber));
    NSMutableArray *peers = [NSMutableArray array];
    for (NSDictionary *entry in list) {
        if ([entry[(id)kCGWindowLayer] integerValue] == manager.level)
            [peers addObject:entry[(id)kCGWindowNumber]];
    }
    return peers;
}

void jajan_activation_prepare_close(void *memoPointer, void *managerPointer) {
    NSWindow *memo = (__bridge NSWindow *)memoPointer;
    NSWindow *manager = (__bridge NSWindow *)managerPointer;
    __weak NSWindow *weakManager = manager;
    if (!manager.visible || manager.miniaturized) return;
    NSSet *above = [NSSet setWithArray:peersOf(manager, kCGWindowListOptionOnScreenAboveWindow)];
    if (!above.count) return;
    // If closing the active memo uncovers another application, relinquish
    // activation before AppKit picks a replacement key window during teardown.
    if (NSApp.active && NSApp.keyWindow == memo) {
        NSArray *below = CFBridgingRelease(CGWindowListCopyWindowInfo(
            kCGWindowListOptionOnScreenBelowWindow | kCGWindowListExcludeDesktopElements,
            (CGWindowID)memo.windowNumber));
        for (NSDictionary *entry in below) {
            if ([entry[(id)kCGWindowLayer] integerValue] != NSNormalWindowLevel) continue;
            if ([above containsObject:entry[(id)kCGWindowNumber]] &&
                [entry[(id)kCGWindowOwnerPID] intValue] != NSProcessInfo.processInfo.processIdentifier)
                [NSApp deactivate];
            break;
        }
    }
    NSUInteger ticket = generation;
    dispatch_block_t completion = ^{
        NSWindow *window = weakManager;
        if (!window.visible || window.miniaturized || ticket != generation) return;
        // Only move down, and only past windows which were above this manager.
        for (NSNumber *peer in [peersOf(window, kCGWindowListOptionOnScreenBelowWindow)
                reverseObjectEnumerator]) {
            if ([above containsObject:peer]) {
                [window orderWindow:NSWindowBelow relativeTo:peer.integerValue];
                break;
            }
        }
    };
    objc_setAssociatedObject(memo, &closeCompletionKey, completion, OBJC_ASSOCIATION_COPY_NONATOMIC);
}

void jajan_activation_install(void (*callback)(void)) {
    static id activatingObserver, activeObserver, inactiveObserver, mouseMonitor;
    if (activeObserver) return;
    (void)activatingObserver;
    (void)inactiveObserver;
    (void)mouseMonitor;
    openManager = callback;
    // Supply only the optional Dock menu method on Tao's existing delegate.
    class_addMethod(object_getClass(NSApp.delegate), @selector(applicationDockMenu:),
                    (IMP)dockMenu, "@@:@");
    NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
    // Remember the input at the start of this activation. A quick click on a
    // memo can be released before DidBecomeActive; never carry it into a
    // later app-switcher selection.
    activatingObserver = [center addObserverForName:NSApplicationWillBecomeActiveNotification
        object:NSApp queue:nil usingBlock:^(NSNotification *notification) {
            (void)notification;
            activatingWithMouse = NSEvent.pressedMouseButtons != 0;
        }];
    [center addObserverForName:NSWindowWillCloseNotification object:nil queue:nil
        usingBlock:^(NSNotification *notification) {
            NSWindow *window = notification.object;
            dispatch_block_t completion = objc_getAssociatedObject(window, &closeCompletionKey);
            if (completion) {
                objc_setAssociatedObject(window, &closeCompletionKey, nil, OBJC_ASSOCIATION_ASSIGN);
                dispatch_async(dispatch_get_main_queue(), completion);
            }
        }];
    inactiveObserver = [center addObserverForName:NSApplicationDidResignActiveNotification
        object:NSApp queue:nil usingBlock:^(NSNotification *notification) {
            (void)notification;
            activatingWithMouse = NO;
            contentActivation = NO;
            ++generation; // Never complete an old selection after switching away.
            passiveTurn = NO;
            cancelManagerRestore();
        }];
    NSEventMask clicks = NSEventMaskLeftMouseDown | NSEventMaskRightMouseDown |
        NSEventMaskOtherMouseDown;
    mouseMonitor = [NSEvent addLocalMonitorForEventsMatchingMask:clicks | NSEventMaskKeyDown
        handler:^NSEvent *(NSEvent *event) {
            contentActivation = NO;
            ++generation; // A direct window click takes precedence over selection.
            passiveTurn = NO;
            cancelManagerRestore();
            return event;
        }];
    activeObserver = [center addObserverForName:NSApplicationDidBecomeActiveNotification
        object:NSApp queue:nil usingBlock:^(NSNotification *notification) {
            (void)notification;
            if (activatingWithMouse || contentActivation) {
                activatingWithMouse = NO;
                contentActivation = NO;
                return;
            }
            if (passiveTurn || loginEvent() || NSEvent.pressedMouseButtons != 0) return;
            NSUInteger ticket = generation;
            dispatch_async(dispatch_get_main_queue(), ^{
                if (ticket != generation || passiveTurn || loginEvent() || !NSApp.active ||
                    NSEvent.pressedMouseButtons != 0 || NSApp.modalWindow) return;
                openManager();
            });
        }];
}

// A first click normally orders the window before the DOM receives mousedown.
// Give only the close button AppKit's delayed-ordering first-mouse contract.
@interface JajanCloseHitView : NSView
@property(nonatomic, weak) WKWebView *web;
@end

@implementation JajanCloseHitView
- (BOOL)acceptsFirstMouse:(NSEvent *)event { return YES; }
- (BOOL)shouldDelayWindowOrderingForEvent:(NSEvent *)event { return YES; }
- (void)mouseDown:(NSEvent *)event { [NSApp preventWindowOrdering]; }
- (void)mouseUp:(NSEvent *)event {
    if (NSPointInRect([self convertPoint:event.locationInWindow fromView:nil], self.bounds))
        [self.web evaluateJavaScript:@"document.getElementById('close')?.click()" completionHandler:nil];
}
@end

void jajan_close_hit_rect(void *pointer, double x, double y, double width, double height) {
    WKWebView *web = (__bridge WKWebView *)pointer;
    static char key;
    JajanCloseHitView *hit = objc_getAssociatedObject(web, &key);
    if (!hit) {
        hit = [[JajanCloseHitView alloc] initWithFrame:NSZeroRect];
        hit.web = web;
        [web addSubview:hit positioned:NSWindowAbove relativeTo:nil];
        objc_setAssociatedObject(web, &key, hit, OBJC_ASSOCIATION_RETAIN_NONATOMIC);
    }
    hit.frame = NSMakeRect(x, web.flipped ? y : NSHeight(web.bounds) - y - height, width, height);
}
