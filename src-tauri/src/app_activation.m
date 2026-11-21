#import <Cocoa/Cocoa.h>
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

// Tao's set_focus skips a window while its deminiaturization is animated.
// Complete this explicit focus request on the native completion notification,
// but never after a subsequent user input or application switch.
void jajan_activation_focus_manager(void *pointer) {
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

void jajan_activation_install(void (*callback)(void)) {
    static id activeObserver, inactiveObserver, mouseMonitor;
    if (activeObserver) return;
    (void)inactiveObserver;
    (void)mouseMonitor;
    openManager = callback;
    NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
    inactiveObserver = [center addObserverForName:NSApplicationDidResignActiveNotification
        object:NSApp queue:nil usingBlock:^(NSNotification *notification) {
            (void)notification;
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
            if (contentActivation) {
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
