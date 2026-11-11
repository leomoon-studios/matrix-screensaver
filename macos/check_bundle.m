#import <AppKit/AppKit.h>
#import <QuartzCore/CAMetalLayer.h>
#import <objc/runtime.h>
#import <ScreenSaver/ScreenSaver.h>

static NSView *FindView(NSView *parent, NSString *identifier) {
    if ([parent.identifier isEqualToString:identifier]) return parent;
    for (NSView *child in parent.subviews) {
        NSView *match = FindView(child, identifier);
        if (match) return match;
    }
    return nil;
}

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
        CALayer *renderLayer = view.layer.sublayers.firstObject;
        if (![renderLayer isKindOfClass:CAMetalLayer.class]) {
            fprintf(stderr, "ScreenSaverView has no CAMetalLayer sublayer\n");
            return 1;
        }
        if (![view hasConfigureSheet]) {
            fprintf(stderr, "ScreenSaverView has no Options sheet\n");
            return 1;
        }
        NSWindow *options = [view configureSheet];
        NSView *content = options.contentView;
        if (!options || ![FindView(content, @"type") isKindOfClass:NSPopUpButton.class] ||
            ![FindView(content, @"color") isKindOfClass:NSPopUpButton.class] ||
            ![FindView(content, @"fps") isKindOfClass:NSSlider.class] ||
            ![FindView(content, @"render-scale") isKindOfClass:NSSlider.class]) {
            fprintf(stderr, "Options sheet is missing a control\n");
            return 1;
        }
        printf("loaded %s (%s) with CAMetalLayer sublayer\n", class_getName(principal),
               bundle.bundleIdentifier.UTF8String);
    }
    return 0;
}
