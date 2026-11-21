// Exercise the real native interception without activating any application.
#import <Cocoa/Cocoa.h>
#import <objc/runtime.h>
#include <assert.h>
extern void jajan_activation_begin_creation(void);
extern void jajan_activation_end_creation(void);
extern void jajan_activation_install(void (*callback)(void));
extern void jajan_activation_focus_content(void *pointer);
static BOOL testActive, focusSucceeds = YES;
static unsigned managers;
static BOOL isActive(id app, SEL selector) { (void)app; (void)selector; return testActive; }
static BOOL focusApplication(id app, SEL selector, NSApplicationActivationOptions options) {
    (void)app; (void)selector; (void)options; return focusSucceeds;
}
static NSUInteger mouseButtons(id cls, SEL selector) { (void)cls; (void)selector; return 0; }
static void openManager(void) { ++managers; }
static void drain(void) {
    [NSRunLoop.mainRunLoop runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.05]];
}
static void activated(void) {
    [NSNotificationCenter.defaultCenter postNotificationName:NSApplicationDidBecomeActiveNotification
        object:NSApp];
    drain();
}
@interface ContentWindow : NSObject
- (void)makeKeyAndOrderFront:(id)sender;
@end
@implementation ContentWindow
- (void)makeKeyAndOrderFront:(id)sender { (void)sender; }
@end
static unsigned requests;
static BOOL lastIgnore;
static void countActivate(id app, SEL selector) { (void)app; (void)selector; ++requests; }
static void countIgnoring(id app, SEL selector, BOOL ignore) {
    (void)app; (void)selector; ++requests; lastIgnore = ignore;
}
static void requestActivation(void) {
    [NSApp activateIgnoringOtherApps:YES];
    if ([NSApp respondsToSelector:NSSelectorFromString(@"activate")])
        [NSApp performSelector:NSSelectorFromString(@"activate")];
}
int main(void) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        Class cls = NSApplication.class;
        Method modern = class_getInstanceMethod(cls, NSSelectorFromString(@"activate"));
        if (modern) method_setImplementation(modern, (IMP)countActivate);
        method_setImplementation(class_getInstanceMethod(cls, @selector(activateIgnoringOtherApps:)),
            (IMP)countIgnoring);
        unsigned perRequest = modern ? 2 : 1;
        requestActivation();
        assert(requests == perRequest);
        jajan_activation_begin_creation();
        requestActivation();
        assert(requests == perRequest);
        jajan_activation_begin_creation();
        requestActivation();
        jajan_activation_end_creation();
        requestActivation();
        assert(requests == perRequest);
        jajan_activation_end_creation();
        requestActivation();
        assert(requests == perRequest * 2);
        assert(lastIgnore);
        [NSApp activateIgnoringOtherApps:NO];
        assert(requests == perRequest * 2 + 1 && !lastIgnore);
        // Native state is stubbed only in this isolated test process. No user
        // application is activated and no real window or input is synthesized.
        method_setImplementation(class_getInstanceMethod(cls, @selector(isActive)), (IMP)isActive);
        method_setImplementation(class_getInstanceMethod(NSRunningApplication.class,
            @selector(activateWithOptions:)), (IMP)focusApplication);
        method_setImplementation(class_getClassMethod(NSEvent.class,
            @selector(pressedMouseButtons)), (IMP)mouseButtons);
        jajan_activation_install(openManager);
        ContentWindow *content = [ContentWindow new];
        jajan_activation_focus_content((__bridge void *)content);
        drain(); // The deliberate focus marker must survive queue-turn delays.
        testActive = YES;
        activated();
        assert(managers == 0);
        activated(); // A later explicit app selection is not swallowed.
        assert(managers == 1);
        testActive = NO;
        focusSucceeds = NO;
        jajan_activation_focus_content((__bridge void *)content);
        testActive = YES;
        activated(); // A failed request must not swallow the next selection.
        assert(managers == 2);
        puts("PASS: passive creation nesting, delayed content activation, failed focus, explicit selection");
    }
}
