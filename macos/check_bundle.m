#import <AppKit/AppKit.h>
#import <QuartzCore/CAMetalLayer.h>
#import <objc/runtime.h>
#import <ScreenSaver/ScreenSaver.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc != 2) {
            fprintf(stderr, "usage: check_bundle PATH.saver\n");
            return 2;
        }
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
        [NSApplication sharedApplication];
        ScreenSaverView *view = [[principal alloc] initWithFrame:NSMakeRect(0, 0, 640, 360)
                                                        isPreview:YES];
        if (!view) {
            fprintf(stderr, "could not instantiate the principal ScreenSaverView\n");
            return 1;
        }
        if (![view.layer isKindOfClass:CAMetalLayer.class]) {
            fprintf(stderr, "ScreenSaverView is not backed by CAMetalLayer\n");
            return 1;
        }
        printf("loaded %s (%s) with CAMetalLayer backing\n", class_getName(principal),
               bundle.bundleIdentifier.UTF8String);
    }
    return 0;
}
