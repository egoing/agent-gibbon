#import <Cocoa/Cocoa.h>
#import <ServiceManagement/ServiceManagement.h>
#import <Carbon/Carbon.h>
#include <stdbool.h>
#include <limits.h>
#import <os/log.h>

static os_log_t startupLog(void) {
    static os_log_t log;
    static dispatch_once_t once;
    dispatch_once(&once, ^{ log = os_log_create("app.jajan", "autostart"); });
    return log;
}

// Weak availability preserves the application's macOS 10.15 baseline.
static bool supported(void) {
    return [NSBundle.mainBundle.bundleURL.pathExtension isEqualToString:@"app"] &&
        [NSBundle.mainBundle.bundleIdentifier isEqualToString:@"app.jajan"];
}
// Stable bridge codes: OS, invalid bundle, and translocation are distinct.
int jajan_autostart_environment(void) {
    if (@available(macOS 13.0, *)) {
        if (!supported()) return -2;
        if ([NSBundle.mainBundle.bundleURL.path.pathComponents containsObject:@"AppTranslocation"])
            return -3;
        return 0;
    }
    return -1;
}
int jajan_autostart_status(void) {
    int environment = jajan_autostart_environment();
    int status = environment;
    if (@available(macOS 13.0, *)) {
        if (supported()) status = (int)SMAppService.mainAppService.status;
    }
    // Record the initial observation and transitions, not every settings refresh.
    static int previousEnvironment = INT_MIN, previousStatus = INT_MIN;
    @synchronized (NSBundle.mainBundle) {
        if (previousEnvironment != environment || previousStatus != status) {
            os_log(startupLog(), "query environment=%{public}d status=%{public}d", environment, status);
            previousEnvironment = environment;
            previousStatus = status;
        }
    }
    return status;
}
bool jajan_autostart_set(bool enabled) {
    if (@available(macOS 13.0, *)) {
        if (jajan_autostart_environment() != 0) return false;
        SMAppService *service = SMAppService.mainAppService;
        // Do not re-register an OS-disabled item to override user approval.
        if (enabled && (service.status == SMAppServiceStatusEnabled ||
                        service.status == SMAppServiceStatusRequiresApproval)) return true;
        if (!enabled && service.status == SMAppServiceStatusNotRegistered) return true;
        NSError *error = nil;
        BOOL success = enabled ? [service registerAndReturnError:&error] : [service unregisterAndReturnError:&error];
        if (!success) {
            os_log_error(startupLog(), "change enabled=%{public}d failed domain=%{public}@ code=%{public}ld status=%{public}ld",
                         enabled, error.domain, (long)error.code, (long)service.status);
        } else {
            os_log_info(startupLog(), "change enabled=%{public}d succeeded status=%{public}ld", enabled, (long)service.status);
        }
        return success;
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
