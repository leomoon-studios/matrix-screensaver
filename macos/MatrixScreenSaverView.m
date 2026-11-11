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
static const CGFloat MatrixOptionsWidth = 440;
static const CGFloat MatrixOptionsHeight = 290;

static NSArray<NSString *> *MatrixColorNames(void) {
    return @[@"green", @"blue", @"red", @"yellow", @"purple", @"orange"];
}

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
    NSUInteger index = [MatrixColorNames() indexOfObject:value.lowercaseString];
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

// The remote screensaver window reports occluded even while its view is visible.
// wgpu checks delegate.window before acquiring a drawable, so this sublayer
// must not expose the remote window through its delegate.
@interface MatrixMetalLayerDelegate : NSObject <CALayerDelegate>
@end

@implementation MatrixMetalLayerDelegate

- (NSWindow *)window {
    return nil;
}

@end

@interface MatrixScreenSaverView : ScreenSaverView
@property(nonatomic) void *matrixRenderer;
@property(nonatomic, strong) CAMetalLayer *matrixLayer;
@property(nonatomic, strong) MatrixMetalLayerDelegate *matrixLayerDelegate;
@property(nonatomic) BOOL reportedRendererError;
@property(nonatomic, strong) NSWindow *optionsSheet;
@property(nonatomic, strong) NSPopUpButton *typeOption;
@property(nonatomic, strong) NSPopUpButton *colorOption;
@property(nonatomic, strong) NSSlider *fpsOption;
@property(nonatomic, strong) NSSlider *scaleOption;
@property(nonatomic, strong) NSTextField *fpsValue;
@property(nonatomic, strong) NSTextField *scaleValue;
@property(nonatomic, strong) NSTextField *optionsError;
@end

@implementation MatrixScreenSaverView

- (instancetype)initWithFrame:(NSRect)frame isPreview:(BOOL)isPreview {
    self = [super initWithFrame:frame isPreview:isPreview];
    if (self) {
        self.animationTimeInterval = 1.0 / 30.0;
        self.wantsLayer = YES;
        self.matrixLayerDelegate = [MatrixMetalLayerDelegate new];
        self.matrixLayer = [CAMetalLayer layer];
        self.matrixLayer.delegate = self.matrixLayerDelegate;
        self.matrixLayer.opaque = YES;
        self.matrixLayer.backgroundColor = NSColor.blackColor.CGColor;
        [self.layer addSublayer:self.matrixLayer];
        [self updateDrawableSize];
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
    self.matrixLayer.frame = self.bounds;
    self.matrixLayer.contentsScale = scale;
    self.matrixLayer.drawableSize = CGSizeMake(width, height);
    if (self.matrixRenderer) matrix_saver_resize(self.matrixRenderer, width, height);
}

- (void)createRendererIfNeeded {
    if (self.matrixRenderer || !self.window) return;
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

- (void)viewDidMoveToWindow {
    [super viewDidMoveToWindow];
    [self updateDrawableSize];
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
    _matrixLayer.delegate = nil;
}

- (BOOL)hasConfigureSheet {
    return YES;
}

- (NSWindow *)configureSheet {
    [self createOptionsSheet];
    [self loadOptions];
    return self.optionsSheet;
}

- (NSTextField *)optionsLabel:(NSString *)title frame:(NSRect)frame {
    NSTextField *label = [NSTextField labelWithString:title];
    label.frame = frame;
    return label;
}

- (void)createOptionsSheet {
    NSRect frame = NSMakeRect(0, 0, MatrixOptionsWidth, MatrixOptionsHeight);
    self.optionsSheet = [[NSWindow alloc] initWithContentRect:frame
                                                   styleMask:NSWindowStyleMaskTitled
                                                     backing:NSBackingStoreBuffered
                                                       defer:NO];
    self.optionsSheet.title = @"Matrix Screensaver Options";
    self.optionsSheet.releasedWhenClosed = NO;
    NSView *content = [[NSView alloc] initWithFrame:frame];
    self.optionsSheet.contentView = content;

    [content addSubview:[self optionsLabel:@"Effect" frame:NSMakeRect(24, 237, 112, 22)]];
    self.typeOption = [[NSPopUpButton alloc] initWithFrame:NSMakeRect(154, 231, 258, 30) pullsDown:NO];
    self.typeOption.identifier = @"type";
    [self.typeOption addItemsWithTitles:@[@"Modern (3D)", @"Classic (2D)"]];
    [content addSubview:self.typeOption];

    [content addSubview:[self optionsLabel:@"Color" frame:NSMakeRect(24, 190, 112, 22)]];
    self.colorOption = [[NSPopUpButton alloc] initWithFrame:NSMakeRect(154, 184, 258, 30) pullsDown:NO];
    self.colorOption.identifier = @"color";
    [self.colorOption addItemsWithTitles:@[@"Green", @"Blue", @"Red", @"Yellow", @"Purple", @"Orange"]];
    [content addSubview:self.colorOption];

    [content addSubview:[self optionsLabel:@"Frame rate" frame:NSMakeRect(24, 143, 112, 22)]];
    self.fpsOption = [[NSSlider alloc] initWithFrame:NSMakeRect(154, 139, 194, 26)];
    self.fpsOption.identifier = @"fps";
    self.fpsOption.minValue = 1;
    self.fpsOption.maxValue = 60;
    self.fpsOption.target = self;
    self.fpsOption.action = @selector(updateOptionValues:);
    [content addSubview:self.fpsOption];
    self.fpsValue = [self optionsLabel:@"30 FPS" frame:NSMakeRect(356, 143, 66, 22)];
    [content addSubview:self.fpsValue];

    [content addSubview:[self optionsLabel:@"Render scale" frame:NSMakeRect(24, 96, 112, 22)]];
    self.scaleOption = [[NSSlider alloc] initWithFrame:NSMakeRect(154, 92, 194, 26)];
    self.scaleOption.identifier = @"render-scale";
    self.scaleOption.minValue = 0.25;
    self.scaleOption.maxValue = 1.0;
    self.scaleOption.target = self;
    self.scaleOption.action = @selector(updateOptionValues:);
    [content addSubview:self.scaleOption];
    self.scaleValue = [self optionsLabel:@"50%" frame:NSMakeRect(356, 96, 66, 22)];
    [content addSubview:self.scaleValue];

    self.optionsError = [self optionsLabel:@"" frame:NSMakeRect(24, 57, 388, 20)];
    self.optionsError.textColor = NSColor.systemRedColor;
    [content addSubview:self.optionsError];

    NSButton *cancel = [NSButton buttonWithTitle:@"Cancel" target:self action:@selector(cancelOptions:)];
    cancel.identifier = @"cancel";
    cancel.frame = NSMakeRect(246, 15, 82, 32);
    cancel.keyEquivalent = @"\033";
    [content addSubview:cancel];
    NSButton *save = [NSButton buttonWithTitle:@"Save" target:self action:@selector(saveOptions:)];
    save.identifier = @"save";
    save.frame = NSMakeRect(334, 15, 82, 32);
    save.keyEquivalent = @"\r";
    [content addSubview:save];
}

- (void)loadOptions {
    [self.typeOption selectItemAtIndex:MatrixRainType()];
    [self.colorOption selectItemAtIndex:MatrixColor()];
    self.fpsOption.integerValue = MatrixFPS();
    self.scaleOption.doubleValue = MatrixRenderScale();
    self.optionsError.stringValue = @"";
    [self updateOptionValues:nil];
}

- (void)updateOptionValues:(id)sender {
    self.fpsValue.stringValue = [NSString stringWithFormat:@"%ld FPS", (long)lround(self.fpsOption.doubleValue)];
    self.scaleValue.stringValue = [NSString stringWithFormat:@"%ld%%", (long)lround(self.scaleOption.doubleValue * 100)];
}

- (void)saveOptions:(id)sender {
    NSArray<NSString *> *colors = MatrixColorNames();
    NSInteger colorIndex = self.colorOption.indexOfSelectedItem;
    if (colorIndex < 0 || colorIndex >= (NSInteger)colors.count) colorIndex = 0;
    NSString *color = colors[colorIndex];
    NSNumber *fps = @(lround(self.fpsOption.doubleValue));
    NSNumber *scale = @(lround(self.scaleOption.doubleValue * 100) / 100.0);
    CFStringRef domain = (__bridge CFStringRef)MatrixPreferencesDomain;
    CFPreferencesSetAppValue(CFSTR("type"), (__bridge CFStringRef)(self.typeOption.indexOfSelectedItem == 1 ? @"classic" : @"modern"), domain);
    CFPreferencesSetAppValue(CFSTR("color"), (__bridge CFStringRef)color, domain);
    CFPreferencesSetAppValue(CFSTR("fps"), (__bridge CFNumberRef)fps, domain);
    CFPreferencesSetAppValue(CFSTR("render-scale"), (__bridge CFNumberRef)scale, domain);
    if (!CFPreferencesAppSynchronize(domain)) {
        self.optionsError.stringValue = @"Could not save settings. Please try again.";
        return;
    }

    if (self.matrixRenderer) {
        matrix_saver_destroy(self.matrixRenderer);
        self.matrixRenderer = NULL;
        [self createRendererIfNeeded];
    }
    [NSApp endSheet:self.optionsSheet returnCode:NSModalResponseOK];
    [self.optionsSheet orderOut:nil];
}

- (void)cancelOptions:(id)sender {
    [NSApp endSheet:self.optionsSheet returnCode:NSModalResponseCancel];
    [self.optionsSheet orderOut:nil];
}

@end
