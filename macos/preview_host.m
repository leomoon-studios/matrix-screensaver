#import <AppKit/AppKit.h>
#import <ScreenSaver/ScreenSaver.h>

static void RunFrames(ScreenSaverView *view, NSTimeInterval seconds) {
    NSDate *end = [NSDate dateWithTimeIntervalSinceNow:seconds];
    while (end.timeIntervalSinceNow > 0) {
        [view animateOneFrame];
        [[NSRunLoop currentRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:1.0 / 30.0]];
    }
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc != 2) {
            fprintf(stderr, "usage: preview_host PATH.saver\n");
            return 2;
        }
        [NSApplication sharedApplication];
        NSApp.activationPolicy = NSApplicationActivationPolicyRegular;
        NSString *path = [NSString stringWithUTF8String:argv[1]];
        NSBundle *bundle = [NSBundle bundleWithPath:path];
        NSError *error = nil;
        if (!bundle || ![bundle loadAndReturnError:&error]) {
            fprintf(stderr, "could not load bundle: %s\n", error.localizedDescription.UTF8String);
            return 1;
        }
        Class principal = bundle.principalClass;
        if (!principal || ![principal isSubclassOfClass:ScreenSaverView.class]) {
            fprintf(stderr, "principal class is not a ScreenSaverView\n");
            return 1;
        }
        NSRect frame = NSMakeRect(0, 0, 960, 540);
        ScreenSaverView *view = [[principal alloc] initWithFrame:frame isPreview:YES];
        if (!view) {
            fprintf(stderr, "could not instantiate the ScreenSaverView\n");
            return 1;
        }
        NSWindow *window = [[NSWindow alloc]
            initWithContentRect:frame
                      styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable | NSWindowStyleMaskResizable
                        backing:NSBackingStoreBuffered
                          defer:NO];
        window.title = @"Matrix Screensaver preview test";
        window.contentView = view;
        [window center];
        [window makeKeyAndOrderFront:nil];
        [NSApp activateIgnoringOtherApps:YES];
        [view startAnimation];
        RunFrames(view, 4.0);
        [window setContentSize:NSMakeSize(720, 720)];
        RunFrames(view, 3.0);
        [view stopAnimation];
        [window close];
        printf("preview lifecycle passed\n");
    }
    return 0;
}
