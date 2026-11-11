#import <AppKit/AppKit.h>
#import <ScreenSaver/ScreenSaver.h>
#include <math.h>

static NSString *const PreferencesDomain = @"com.leomoon-studios.matrix-screensaver";

static NSView *FindView(NSView *parent, NSString *identifier) {
    if ([parent.identifier isEqualToString:identifier]) return parent;
    for (NSView *child in parent.subviews) {
        NSView *match = FindView(child, identifier);
        if (match) return match;
    }
    return nil;
}

static id ReadPreference(NSString *key) {
    return CFBridgingRelease(CFPreferencesCopyAppValue((__bridge CFStringRef)key,
                                                       (__bridge CFStringRef)PreferencesDomain));
}

static NSDictionary<NSString *, id> *SavedPreferences(NSArray<NSString *> *keys) {
    NSMutableDictionary<NSString *, id> *saved = [NSMutableDictionary dictionary];
    for (NSString *key in keys) saved[key] = ReadPreference(key) ?: NSNull.null;
    return saved;
}

static BOOL PreferencesMatch(NSDictionary<NSString *, id> *expected) {
    for (NSString *key in expected) {
        id actual = ReadPreference(key) ?: NSNull.null;
        if (![actual isEqual:expected[key]]) return NO;
    }
    return YES;
}

static void RestorePreferences(NSDictionary<NSString *, id> *saved) {
    CFStringRef domain = (__bridge CFStringRef)PreferencesDomain;
    for (NSString *key in saved) {
        id value = saved[key];
        CFPreferencesSetAppValue((__bridge CFStringRef)key,
                                 value == NSNull.null ? NULL : (__bridge CFPropertyListRef)value,
                                 domain);
    }
    CFPreferencesAppSynchronize(domain);
}

static void EndSheet(NSWindow *parent, NSWindow *sheet, NSButton *button) {
    [parent beginSheet:sheet completionHandler:nil];
    [button performClick:nil];
    [[NSRunLoop currentRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.2]];
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc != 2) {
            fprintf(stderr, "usage: check_options PATH.saver\n");
            return 2;
        }
        NSArray<NSString *> *keys = @[@"type", @"color", @"fps", @"render-scale"];
        NSDictionary<NSString *, id> *original = SavedPreferences(keys);
        int result = 1;
        @try {
            [NSApplication sharedApplication];
            NSApp.activationPolicy = NSApplicationActivationPolicyRegular;
            NSBundle *bundle = [NSBundle bundleWithPath:[NSString stringWithUTF8String:argv[1]]];
            NSError *error = nil;
            if (!bundle || ![bundle loadAndReturnError:&error]) {
                fprintf(stderr, "could not load bundle: %s\n", error.localizedDescription.UTF8String);
                return 1;
            }
            ScreenSaverView *view = [[bundle.principalClass alloc] initWithFrame:NSMakeRect(0, 0, 480, 320)
                                                                     isPreview:YES];
            if (![view hasConfigureSheet]) {
                fprintf(stderr, "Options sheet is unavailable\n");
                return 1;
            }
            NSWindow *host = [[NSWindow alloc] initWithContentRect:NSMakeRect(0, 0, 480, 320)
                                                         styleMask:NSWindowStyleMaskTitled
                                                           backing:NSBackingStoreBuffered
                                                             defer:NO];
            host.releasedWhenClosed = NO;
            host.contentView = view;
            [host makeKeyAndOrderFront:nil];
            [view startAnimation];
            NSWindow *sheet = [view configureSheet];
            NSPopUpButton *type = (NSPopUpButton *)FindView(sheet.contentView, @"type");
            NSPopUpButton *color = (NSPopUpButton *)FindView(sheet.contentView, @"color");
            NSSlider *fps = (NSSlider *)FindView(sheet.contentView, @"fps");
            NSSlider *scale = (NSSlider *)FindView(sheet.contentView, @"render-scale");
            NSButton *cancel = (NSButton *)FindView(sheet.contentView, @"cancel");
            NSButton *save = (NSButton *)FindView(sheet.contentView, @"save");
            if (!type || !color || !fps || !scale || !cancel || !save) {
                fprintf(stderr, "Options sheet is missing controls\n");
                return 1;
            }

            NSWindow *firstSheet = sheet;
            [type selectItemAtIndex:type.indexOfSelectedItem == 0 ? 1 : 0];
            [color selectItemAtIndex:(color.indexOfSelectedItem + 1) % 6];
            fps.integerValue = fps.integerValue == 42 ? 43 : 42;
            scale.doubleValue = scale.doubleValue == 0.75 ? 0.8 : 0.75;
            EndSheet(host, sheet, cancel);
            if (!PreferencesMatch(original)) {
                fprintf(stderr, "Cancel changed saved preferences\n");
                return 1;
            }

            sheet = [view configureSheet];
            if (sheet == firstSheet) {
                fprintf(stderr, "Options reused a dismissed sheet\n");
                return 1;
            }
            type = (NSPopUpButton *)FindView(sheet.contentView, @"type");
            color = (NSPopUpButton *)FindView(sheet.contentView, @"color");
            fps = (NSSlider *)FindView(sheet.contentView, @"fps");
            scale = (NSSlider *)FindView(sheet.contentView, @"render-scale");
            save = (NSButton *)FindView(sheet.contentView, @"save");
            [type selectItemAtIndex:type.indexOfSelectedItem == 0 ? 1 : 0];
            [color selectItemAtIndex:(color.indexOfSelectedItem + 1) % 6];
            fps.integerValue = 42;
            scale.doubleValue = 0.75;
            NSString *savedType = type.indexOfSelectedItem == 1 ? @"classic" : @"modern";
            NSArray<NSString *> *colors = @[@"green", @"blue", @"red", @"yellow", @"purple", @"orange"];
            NSString *savedColor = colors[color.indexOfSelectedItem];
            EndSheet(host, sheet, save);
            NSDictionary<NSString *, id> *expected = @{
                @"type": savedType, @"color": savedColor, @"fps": @42, @"render-scale": @0.75
            };
            if (!PreferencesMatch(expected)) {
                fprintf(stderr, "Save did not persist all preferences\n");
                return 1;
            }
            if (fabs(view.animationTimeInterval - 1.0 / 42.0) > 0.001) {
                fprintf(stderr, "Save did not update the running preview frame rate\n");
                return 1;
            }
            [view stopAnimation];
            [view startAnimation];
            [view animateOneFrame];
            NSWindow *afterPreview = [view configureSheet];
            if (!afterPreview || afterPreview == sheet) {
                fprintf(stderr, "Options did not open a fresh sheet after preview\n");
                return 1;
            }
            cancel = (NSButton *)FindView(afterPreview.contentView, @"cancel");
            EndSheet(host, afterPreview, cancel);
            if (!PreferencesMatch(expected)) {
                fprintf(stderr, "Options after preview changed saved preferences\n");
                return 1;
            }
            [view stopAnimation];
            [host close];
            result = 0;
        } @finally {
            RestorePreferences(original);
        }
        if (result == 0) printf("Options Save and Cancel passed\n");
        return result;
    }
}
