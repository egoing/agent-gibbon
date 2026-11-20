#import <Cocoa/Cocoa.h>
#import <ServiceManagement/ServiceManagement.h>
#import <Carbon/Carbon.h>
#include <stdbool.h>

// Weak availability preserves the application's macOS 10.15 baseline.
static bool supported(void) {
    return [NSBundle.mainBundle.bundleURL.pathExtension isEqualToString:@"app"] &&
        [NSBundle.mainBundle.bundleIdentifier isEqualToString:@"app.jajan"];
}
int jajan_autostart_status(void) {
    if (@available(macOS 13.0, *)) {
        if (supported()) return (int)SMAppService.mainAppService.status;
    }
    return -1;
}
bool jajan_autostart_set(bool enabled) {
    if (@available(macOS 13.0, *)) {
        if (!supported()) return false;
        SMAppService *service = SMAppService.mainAppService;
        // Do not re-register an OS-disabled item to override user approval.
        if (enabled && (service.status == SMAppServiceStatusEnabled ||
                        service.status == SMAppServiceStatusRequiresApproval)) return true;
        if (!enabled && service.status == SMAppServiceStatusNotRegistered) return true;
        NSError *error = nil;
        return enabled ? [service registerAndReturnError:&error] : [service unregisterAndReturnError:&error];
    }
    return false;
}
void jajan_autostart_settings(void) {
    if (@available(macOS 13.0, *)) [SMAppService openSystemSettingsLoginItems];
}
bool jajan_login_event(void) {
    NSAppleEventDescriptor *event = NSAppleEventManager.sharedAppleEventManager.currentAppleEvent;
    return event.eventID == kAEOpenApplication &&
        [event paramDescriptorForKeyword:keyAEPropData].enumCodeValue == keyAELaunchedAsLogInItem;
}
bool jajan_login_launch(void) {
    static bool loginLaunch;
    if (jajan_login_event()) loginLaunch = true;
    return loginLaunch;
}
