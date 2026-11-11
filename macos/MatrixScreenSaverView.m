#import <AppKit/AppKit.h>
#import <QuartzCore/CAMetalLayer.h>
#import <ScreenSaver/ScreenSaver.h>
#include <stdbool.h>
#include <stdint.h>

extern void *matrix_saver_create(void *metalLayer, uint32_t width, uint32_t height,
                                 uint32_t rainType, uint32_t color, uint32_t fps,
                                 float renderScale);
extern int32_t matrix_saver_render(void *handle);
extern void matrix_saver_resize(void *handle, uint32_t width, uint32_t height);
extern void matrix_saver_set_paused(void *handle, bool paused);
extern const char *matrix_saver_error(const void *handle);
extern void matrix_saver_destroy(void *handle);
extern const char *matrix_saver_ffi_version(void);

static NSString *const MatrixPreferencesDomain = @"com.leomoon-studios.matrix-screensaver";

static id MatrixPreference(NSString *key) {
    return CFBridgingRelease(CFPreferencesCopyAppValue((__bridge CFStringRef)key,
                                                       (__bridge CFStringRef)MatrixPreferencesDomain));
}

static uint32_t MatrixRainType(void) {
    NSString *value = MatrixPreference(@"type");
    return [value isKindOfClass:NSString.class] && [value caseInsensitiveCompare:@"classic"] == NSOrderedSame ? 1 : 0;
}

static uint32_t MatrixColor(void) {
    NSString *value = MatrixPreference(@"color");
    if (![value isKindOfClass:NSString.class]) return 0;
    NSArray<NSString *> *colors = @[@"green", @"blue", @"red", @"yellow", @"purple", @"orange"];
    NSUInteger index = [colors indexOfObject:value.lowercaseString];
    return index == NSNotFound ? 0 : (uint32_t)index;
}

static uint32_t MatrixFPS(void) {
    NSNumber *value = MatrixPreference(@"fps");
    NSInteger fps = [value respondsToSelector:@selector(integerValue)] ? value.integerValue : 30;
    return (uint32_t)MAX(1, MIN(60, fps));
}

static float MatrixRenderScale(void) {
    NSNumber *value = MatrixPreference(@"render-scale");
    double scale = [value respondsToSelector:@selector(doubleValue)] ? value.doubleValue : 0.5;
    return (float)MAX(0.25, MIN(1.0, scale));
}

@interface MatrixScreenSaverView : ScreenSaverView
@property(nonatomic) void *matrixRenderer;
@property(nonatomic, strong) CAMetalLayer *matrixLayer;
@property(nonatomic) BOOL reportedRendererError;
@end

@implementation MatrixScreenSaverView

- (instancetype)initWithFrame:(NSRect)frame isPreview:(BOOL)isPreview {
    self = [super initWithFrame:frame isPreview:isPreview];
    if (self) {
        self.animationTimeInterval = 1.0 / 30.0;
        self.wantsLayer = YES;
        self.matrixLayer = [CAMetalLayer layer];
        self.matrixLayer.opaque = YES;
        self.matrixLayer.backgroundColor = NSColor.blackColor.CGColor;
        self.layer = self.matrixLayer;
    }
    return self;
}

- (BOOL)isOpaque {
    return YES;
}

- (CGFloat)backingScale {
    NSScreen *screen = self.window.screen ?: NSScreen.mainScreen;
    return screen ? screen.backingScaleFactor : 1.0;
}

- (void)updateDrawableSize {
    CGFloat scale = [self backingScale];
    NSSize points = self.bounds.size;
    uint32_t width = (uint32_t)MAX(1.0, round(points.width * scale));
    uint32_t height = (uint32_t)MAX(1.0, round(points.height * scale));
    self.matrixLayer.contentsScale = scale;
    self.matrixLayer.drawableSize = CGSizeMake(width, height);
    if (self.matrixRenderer) matrix_saver_resize(self.matrixRenderer, width, height);
}

- (void)createRendererIfNeeded {
    if (self.matrixRenderer) return;
    self.reportedRendererError = NO;
    [self updateDrawableSize];
    CGSize size = self.matrixLayer.drawableSize;
    uint32_t fps = MatrixFPS();
    self.animationTimeInterval = 1.0 / (NSTimeInterval)fps;
    self.matrixRenderer = matrix_saver_create((__bridge void *)self.matrixLayer,
                                               (uint32_t)size.width, (uint32_t)size.height,
                                               MatrixRainType(), MatrixColor(), fps,
                                               MatrixRenderScale());
    if (!self.matrixRenderer || matrix_saver_render(self.matrixRenderer) < 0) {
        const char *error = self.matrixRenderer ? matrix_saver_error(self.matrixRenderer) : "Rust renderer creation panicked";
        NSLog(@"Matrix Screensaver could not initialize: %s", error);
        self.reportedRendererError = YES;
    }
}

- (void)startAnimation {
    [super startAnimation];
    [self createRendererIfNeeded];
    if (self.matrixRenderer) matrix_saver_set_paused(self.matrixRenderer, false);
}

- (void)stopAnimation {
    if (self.matrixRenderer) {
        matrix_saver_set_paused(self.matrixRenderer, true);
        matrix_saver_destroy(self.matrixRenderer);
        self.matrixRenderer = NULL;
    }
    [super stopAnimation];
}

- (void)animateOneFrame {
    [self createRendererIfNeeded];
    if (!self.matrixRenderer || self.reportedRendererError) return;
    if (matrix_saver_render(self.matrixRenderer) < 0) {
        NSLog(@"Matrix Screensaver stopped rendering: %s", matrix_saver_error(self.matrixRenderer));
        self.reportedRendererError = YES;
    }
}

- (void)setFrameSize:(NSSize)newSize {
    [super setFrameSize:newSize];
    [self updateDrawableSize];
}

- (void)viewDidChangeBackingProperties {
    [super viewDidChangeBackingProperties];
    [self updateDrawableSize];
}

- (void)dealloc {
    if (_matrixRenderer) matrix_saver_destroy(_matrixRenderer);
}

- (BOOL)hasConfigureSheet {
    return NO;
}

@end
