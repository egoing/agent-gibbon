#import <Cocoa/Cocoa.h>

// AppKit does not expose a Command-Tab-specific activation notification. Treat
// non-mouse activation as an app-level selection, excluding our passive shows.
// Keep Tao's application delegate intact (including its Dock reopen handler).
static NSUInteger generation;
static BOOL passiveTurn;
static void (*openManager)(void);
static id managerRestoreObserver;

static void cancelManagerRestore(void) {
    if (managerRestoreObserver) {
        [NSNotificationCenter.defaultCenter removeObserver:managerRestoreObserver];
        managerRestoreObserver = nil;
    }
}

// Tao's set_focus skips a window while its deminiaturization is animated.
// Complete this explicit focus request on the native completion notification,
// but never after a subsequent user input or application switch.
void jajan_activation_focus_manager(void *pointer) {
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
            ++generation; // A direct window click takes precedence over selection.
            passiveTurn = NO;
            cancelManagerRestore();
            return event;
        }];
    activeObserver = [center addObserverForName:NSApplicationDidBecomeActiveNotification
        object:NSApp queue:nil usingBlock:^(NSNotification *notification) {
            (void)notification;
            if (passiveTurn || NSEvent.pressedMouseButtons != 0) return;
            NSUInteger ticket = generation;
            dispatch_async(dispatch_get_main_queue(), ^{
                if (ticket != generation || passiveTurn || !NSApp.active ||
                    NSEvent.pressedMouseButtons != 0 || NSApp.modalWindow) return;
                openManager();
            });
        }];
}
