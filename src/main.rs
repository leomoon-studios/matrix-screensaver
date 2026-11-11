use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(any(target_os = "windows", test))]
mod windows_host;

#[cfg(target_os = "linux")]
use winit::platform::wayland::WindowAttributesExtWayland;

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseScrollDelta, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    monitor::MonitorHandle,
    window::{Fullscreen, Window, WindowAttributes, WindowId},
};

#[cfg(test)]
use matrix_screensaver::{ColorTheme, RainType, settings::next_frame_deadline};
use matrix_screensaver::{
    Quality,
    renderer::{RenderStatus, Renderer},
};
const MONITOR_POLL_INTERVAL: Duration = Duration::from_secs(2);
const POINTER_STARTUP_GRACE: Duration = Duration::from_millis(300);
const POINTER_DISMISS_DISTANCE: f64 = 8.0;
const FOCUS_LOSS_GRACE: Duration = Duration::from_secs(1);
const FOCUS_STARTUP_GRACE: Duration = Duration::from_secs(3);

#[cfg(target_os = "windows")]
fn windows_message(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MB_OK, MessageBoxW};
    let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    let title: Vec<u16> = "Matrix Screensaver".encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), MB_OK);
    }
}

#[cfg(target_os = "windows")]
fn preview_size(parent: std::num::NonZeroIsize) -> Option<PhysicalSize<u32>> {
    use windows_sys::Win32::{
        Foundation::RECT,
        UI::WindowsAndMessaging::{GetClientRect, IsWindow},
    };
    let hwnd = parent.get() as *mut std::ffi::c_void;
    let mut rect = RECT::default();
    if unsafe { IsWindow(hwnd) == 0 || GetClientRect(hwnd, &mut rect) == 0 } {
        return None;
    }
    Some(PhysicalSize::new(
        (rect.right - rect.left).max(0) as u32,
        (rect.bottom - rect.top).max(0) as u32,
    ))
}

fn main() {
    #[allow(unused_mut)]
    let mut arguments: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(target_os = "windows")]
    let preview = {
        let is_scr = std::env::current_exe()
            .ok()
            .and_then(|p| {
                p.extension()
                    .map(|e| e.to_string_lossy().eq_ignore_ascii_case("scr"))
            })
            .unwrap_or(false);
        if is_scr {
            match windows_host::parse(&arguments) {
                Ok(windows_host::Mode::Fullscreen) => {
                    arguments = vec!["--screensaver".into()];
                    None
                }
                Ok(windows_host::Mode::Configure) => {
                    windows_message("Matrix Screensaver has no configurable settings.");
                    return;
                }
                Ok(windows_host::Mode::Preview(handle)) => {
                    arguments.clear();
                    Some(handle)
                }
                Err(error) => {
                    windows_message(error);
                    std::process::exit(2);
                }
            }
        } else {
            None
        }
    };
    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!(
            "Usage: matrix-screensaver [--screensaver] [--type modern|classic] [--color green|blue|red|yellow|purple|orange] [--fps 1..60] [--render-scale 0.25..1] [--app-id ID]\n\n--screensaver opens full-screen on each monitor and exits on input.\n--app-id sets the Wayland application ID for idle-manager integration.\nDefaults: --type modern --color green --fps 30 --render-scale 0.5\nEnvironment: MATRIX_FPS and MATRIX_RENDER_SCALE remain supported; CLI values take precedence."
        );
        return;
    }
    let (arguments, app_id) = match split_app_id(arguments) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("matrix-screensaver: {error}. Run --help for usage.");
            std::process::exit(2);
        }
    };
    #[cfg(not(target_os = "linux"))]
    if app_id.is_some() {
        eprintln!("matrix-screensaver: --app-id is only supported on Linux Wayland.");
        std::process::exit(2);
    }
    let quality =
        match Quality::from_environment().and_then(|quality| quality.with_arguments(arguments)) {
            Ok(quality) => quality,
            Err(error) => {
                eprintln!("matrix-screensaver: {error}. Run --help for usage.");
                std::process::exit(2);
            }
        };
    let event_loop = EventLoop::new().expect("failed to create the window event loop");
    #[cfg(target_os = "windows")]
    if let Some(parent) = preview {
        let proxy = event_loop.create_proxy();
        std::thread::spawn(move || {
            while preview_size(parent).is_some() {
                std::thread::sleep(Duration::from_millis(100));
            }
            let _ = proxy.send_event(());
        });
    }
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App {
        initial_size: initial_size(),
        quality,
        windows: Vec::new(),
        next_monitor_poll: Instant::now(),
        failed: false,
        #[cfg(target_os = "linux")]
        app_id,
        started_at: Instant::now(),
        unfocused_since: None,
        #[cfg(target_os = "windows")]
        preview,
    };
    event_loop.run_app(&mut app).expect("event loop error");
    if app.failed {
        std::process::exit(1);
    }
}

fn split_app_id(args: Vec<String>) -> Result<(Vec<String>, Option<String>), String> {
    let mut remaining = Vec::new();
    let mut app_id = None;
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        let value = if argument == "--app-id" {
            Some(args.next().ok_or("--app-id needs a value")?)
        } else {
            argument.strip_prefix("--app-id=").map(str::to_owned)
        };
        if let Some(value) = value {
            if value.is_empty()
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            {
                return Err(
                    "--app-id must contain only letters, digits, dots, dashes, or underscores"
                        .to_owned(),
                );
            }
            if app_id.replace(value).is_some() {
                return Err("--app-id may only be set once".to_owned());
            }
        } else {
            remaining.push(argument);
        }
    }
    Ok((remaining, app_id))
}

fn initial_size() -> PhysicalSize<u32> {
    let Ok(size) = std::env::var("MATRIX_WINDOW_SIZE") else {
        return PhysicalSize::new(1280, 720);
    };
    let Some((width, height)) = size.split_once('x') else {
        return PhysicalSize::new(1280, 720);
    };
    let (Ok(width), Ok(height)) = (width.parse(), height.parse()) else {
        return PhysicalSize::new(1280, 720);
    };
    PhysicalSize::new(width, height)
}

struct App {
    initial_size: PhysicalSize<u32>,
    quality: Quality,
    windows: Vec<ScreenWindow>,
    next_monitor_poll: Instant,
    failed: bool,
    #[cfg(target_os = "linux")]
    app_id: Option<String>,
    started_at: Instant,
    unfocused_since: Option<Instant>,
    #[cfg(target_os = "windows")]
    preview: Option<std::num::NonZeroIsize>,
}

impl ApplicationHandler for App {
    fn user_event(&mut self, event_loop: &ActiveEventLoop, _event: ()) {
        event_loop.exit();
    }
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.quality.screensaver {
            self.sync_monitors(event_loop);
        } else if self.windows.is_empty() {
            self.create_window(event_loop, None);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(index) = self
            .windows
            .iter()
            .position(|entry| entry.window.id() == window_id)
        else {
            return;
        };
        let scale_factor = self.windows[index].window.scale_factor();
        if self.quality.screensaver
            && self.windows[index]
                .dismissal
                .handles(&event, Instant::now(), scale_factor)
        {
            event_loop.exit();
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Destroyed => {
                self.windows.remove(index);
                if !self.quality.screensaver {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => self.windows[index].renderer.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = self.windows[index].window.inner_size();
                self.windows[index].renderer.resize(size);
            }
            WindowEvent::Occluded(occluded) => {
                self.windows[index].renderer.set_occluded(occluded);
                if !occluded {
                    self.windows[index].window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if self.quality.screensaver {
                    let attached = self.windows[index].monitor.as_ref().unwrap();
                    if !event_loop
                        .available_monitors()
                        .any(|active| active == *attached)
                    {
                        // Do not acquire a Vulkan swapchain image after its
                        // Wayland output disappears. Some drivers crash first.
                        self.sync_monitors(event_loop);
                        return;
                    }
                }
                match self.windows[index].renderer.render() {
                    Ok(RenderStatus::SurfaceLost) => {
                        let window = self.windows[index].window.clone();
                        if let Err(error) = self.windows[index]
                            .renderer
                            .recreate_surface_for_window(window)
                        {
                            eprintln!(
                                "Matrix Screensaver could not recover its GPU surface: {error}"
                            );
                            self.failed = true;
                            event_loop.exit();
                        }
                    }
                    Err(error) => {
                        eprintln!("Matrix Screensaver rendering failed: {error}");
                        self.failed = true;
                        event_loop.exit();
                    }
                    Ok(RenderStatus::Presented | RenderStatus::Skipped) => {}
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        #[cfg(target_os = "windows")]
        if let Some(parent) = self.preview {
            if let Some(size) = preview_size(parent) {
                if let Some(entry) = self.windows.first() {
                    if entry.window.inner_size() != size {
                        let _ = entry.window.request_inner_size(size);
                    }
                }
            } else {
                event_loop.exit();
                return;
            }
        }
        if self.quality.screensaver && !self.windows.is_empty() {
            if self.windows.iter().any(|entry| entry.window.has_focus()) {
                self.unfocused_since = None;
            } else if now.duration_since(self.started_at) >= FOCUS_STARTUP_GRACE {
                let since = self.unfocused_since.get_or_insert(now);
                if now.duration_since(*since) >= FOCUS_LOSS_GRACE {
                    event_loop.exit();
                    return;
                }
            }
        }
        if self.quality.screensaver && now >= self.next_monitor_poll {
            self.sync_monitors(event_loop);
            self.next_monitor_poll = now + MONITOR_POLL_INTERVAL;
        }
        let mut next_wake = self.quality.screensaver.then_some(self.next_monitor_poll);
        if let Some(since) = self.unfocused_since {
            let deadline = since + FOCUS_LOSS_GRACE;
            next_wake = Some(next_wake.map_or(deadline, |wake| wake.min(deadline)));
        }
        for entry in &self.windows {
            let renderer = &entry.renderer;
            if renderer.is_occluded() {
                continue;
            }
            if now >= renderer.next_frame_at() {
                entry.window.request_redraw();
            }
            next_wake = Some(next_wake.map_or(renderer.next_frame_at(), |wake| {
                wake.min(renderer.next_frame_at())
            }));
        }
        if let Some(deadline) = next_wake {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }
}

struct ScreenWindow {
    monitor: Option<MonitorHandle>,
    renderer: Renderer,
    window: Arc<Window>,
    dismissal: Dismissal,
}

impl App {
    fn sync_monitors(&mut self, event_loop: &ActiveEventLoop) {
        if self.failed {
            return;
        }
        let monitors: Vec<_> = event_loop.available_monitors().collect();
        if monitors.is_empty() {
            // A display may disappear during DPMS or suspend. Release its GPU
            // resources and wait for the compositor to advertise it again.
            self.windows.clear();
            return;
        }
        self.windows.retain(|entry| {
            entry
                .monitor
                .as_ref()
                .is_some_and(|monitor| monitors.contains(monitor))
        });
        for monitor in monitors {
            if !self
                .windows
                .iter()
                .any(|entry| entry.monitor.as_ref() == Some(&monitor))
            {
                self.create_window(event_loop, Some(monitor));
                if self.failed {
                    return;
                }
            }
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop, monitor: Option<MonitorHandle>) {
        let mut attrs = WindowAttributes::default().with_title(match &monitor {
            Some(monitor) => format!(
                "Matrix Screensaver [{}]",
                monitor.name().unwrap_or_default()
            ),
            None => "Matrix Screensaver".to_owned(),
        });
        if let Some(monitor) = &monitor {
            attrs = attrs
                .with_fullscreen(Some(Fullscreen::Borderless(Some(monitor.clone()))))
                .with_decorations(false)
                .with_resizable(false);
        } else {
            attrs = attrs
                .with_inner_size(self.initial_size)
                .with_resizable(true);
        }
        #[cfg(target_os = "linux")]
        if let Some(app_id) = &self.app_id {
            attrs = attrs.with_name(app_id.clone(), app_id.clone());
        }
        #[cfg(target_os = "windows")]
        if let Some(parent) = self.preview {
            use winit::raw_window_handle::{RawWindowHandle, Win32WindowHandle};
            let Some(size) = preview_size(parent) else {
                eprintln!("Matrix Screensaver: invalid preview window");
                self.failed = true;
                event_loop.exit();
                return;
            };
            attrs = attrs
                .with_decorations(false)
                .with_position(PhysicalPosition::new(0, 0))
                .with_inner_size(size);
            // Windows supplied the parent; preview_size checked that it is still a window.
            attrs = unsafe {
                attrs.with_parent_window(Some(RawWindowHandle::Win32(Win32WindowHandle::new(
                    parent,
                ))))
            };
        }
        #[cfg(target_os = "windows")]
        if self.quality.screensaver || self.preview.is_some() {
            attrs = attrs.with_visible(false);
        }
        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("Matrix Screensaver could not create a window: {error}");
                self.failed = true;
                event_loop.exit();
                return;
            }
        };
        if self.quality.screensaver {
            window.set_cursor_visible(false);
        }
        match pollster::block_on(Renderer::new_for_window(
            window.clone(),
            self.quality,
            event_loop.owned_display_handle(),
            self.windows.first().map(|entry| &entry.renderer),
        )) {
            Ok(renderer) => {
                #[cfg(target_os = "windows")]
                if self.quality.screensaver || self.preview.is_some() {
                    let mut renderer = renderer;
                    match renderer.render() {
                        Ok(RenderStatus::Presented) => {}
                        Ok(RenderStatus::Skipped | RenderStatus::SurfaceLost) => {
                            eprintln!(
                                "Matrix Screensaver could not present its first frame while hidden"
                            );
                            self.failed = true;
                            event_loop.exit();
                            return;
                        }
                        Err(error) => {
                            eprintln!("Matrix Screensaver first frame failed: {error}");
                            self.failed = true;
                            event_loop.exit();
                            return;
                        }
                    }
                    window.set_visible(true);
                    self.windows.push(ScreenWindow {
                        monitor,
                        renderer,
                        window,
                        dismissal: Dismissal::new(Instant::now()),
                    });
                    return;
                }
                self.windows.push(ScreenWindow {
                    monitor,
                    renderer,
                    window,
                    dismissal: Dismissal::new(Instant::now()),
                });
            }
            Err(error) => {
                eprintln!("Matrix Screensaver could not initialize its GPU renderer: {error}");
                self.failed = true;
                event_loop.exit();
            }
        }
    }
}

struct Dismissal {
    started_at: Instant,
    pointer_origin: Option<PhysicalPosition<f64>>,
}

enum DismissalInput {
    Key { pressed: bool, synthetic: bool },
    Modifier { active: bool },
    MouseButton { pressed: bool },
    Scroll { moved: bool },
    Touch { started: bool },
    Pointer(PhysicalPosition<f64>),
}

impl Dismissal {
    fn new(started_at: Instant) -> Self {
        Self {
            started_at,
            pointer_origin: None,
        }
    }

    fn handles(&mut self, event: &WindowEvent, now: Instant, scale_factor: f64) -> bool {
        if matches!(
            event,
            WindowEvent::CursorLeft { .. } | WindowEvent::Focused(false)
        ) {
            self.pointer_origin = None;
            return false;
        }
        let input = match event {
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => DismissalInput::Key {
                pressed: event.state == ElementState::Pressed,
                synthetic: *is_synthetic,
            },
            WindowEvent::ModifiersChanged(modifiers) => DismissalInput::Modifier {
                active: !modifiers.state().is_empty(),
            },
            WindowEvent::MouseInput { state, .. } => DismissalInput::MouseButton {
                pressed: *state == ElementState::Pressed,
            },
            WindowEvent::MouseWheel { delta, .. } => DismissalInput::Scroll {
                moved: match delta {
                    MouseScrollDelta::LineDelta(x, y) => *x != 0.0 || *y != 0.0,
                    MouseScrollDelta::PixelDelta(position) => {
                        position.x != 0.0 || position.y != 0.0
                    }
                },
            },
            WindowEvent::Touch(touch) => DismissalInput::Touch {
                started: touch.phase == TouchPhase::Started,
            },
            WindowEvent::CursorMoved { position, .. } => DismissalInput::Pointer(*position),
            _ => return false,
        };
        self.on_input(input, now, scale_factor)
    }

    fn on_input(&mut self, input: DismissalInput, now: Instant, scale_factor: f64) -> bool {
        match input {
            DismissalInput::Key { pressed, synthetic } => pressed && !synthetic,
            DismissalInput::Modifier { active } => active,
            DismissalInput::MouseButton { pressed } => pressed,
            DismissalInput::Scroll { moved } => moved,
            DismissalInput::Touch { started } => started,
            DismissalInput::Pointer(position) => self.pointer_moved(position, now, scale_factor),
        }
    }

    fn pointer_moved(
        &mut self,
        position: PhysicalPosition<f64>,
        now: Instant,
        scale_factor: f64,
    ) -> bool {
        if now.duration_since(self.started_at) < POINTER_STARTUP_GRACE {
            self.pointer_origin = Some(position);
            return false;
        }
        let Some(origin) = self.pointer_origin else {
            self.pointer_origin = Some(position);
            #[cfg(target_os = "windows")]
            return false;
            // Wayland may not send an initial position. After the startup
            // grace, the first motion is then the user's only activity.
            #[cfg(not(target_os = "windows"))]
            return true;
        };
        let distance = (position.x - origin.x).hypot(position.y - origin.y);
        distance >= POINTER_DISMISS_DISTANCE * scale_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn invalid_preview_handle_is_rejected() {
        assert!(preview_size(std::num::NonZeroIsize::new(-1).unwrap()).is_none());
    }

    #[test]
    fn wayland_app_id_is_extracted_and_validated() {
        let (remaining, app_id) = split_app_id(
            [
                "--screensaver",
                "--app-id=org.omarchy.screensaver",
                "--type",
                "classic",
            ]
            .map(str::to_owned)
            .to_vec(),
        )
        .unwrap();
        assert_eq!(app_id.as_deref(), Some("org.omarchy.screensaver"));
        assert_eq!(remaining, ["--screensaver", "--type", "classic"]);
        assert!(split_app_id(vec!["--app-id".to_owned()]).is_err());
        assert!(split_app_id(vec!["--app-id=bad id".to_owned()]).is_err());
        assert!(split_app_id(vec!["--app-id=a".to_owned(), "--app-id=b".to_owned()]).is_err());
    }

    #[test]
    fn command_line_quality_overrides_defaults_and_rejects_bad_values() {
        let defaults = Quality::default();
        let quality = defaults
            .with_arguments(["--fps", "60", "--render-scale=0.75"].map(str::to_owned))
            .unwrap();
        assert_eq!(quality.fps, 60);
        assert_eq!(quality.render_scale, 0.75);
        assert_eq!(quality.rain_type, RainType::Modern);
        assert_eq!(quality.color, ColorTheme::Green);
        for (name, expected) in [
            ("green", ColorTheme::Green),
            ("blue", ColorTheme::Blue),
            ("red", ColorTheme::Red),
            ("yellow", ColorTheme::Yellow),
            ("purple", ColorTheme::Purple),
            ("orange", ColorTheme::Orange),
        ] {
            assert_eq!(
                defaults
                    .with_arguments([format!("--color={name}")])
                    .unwrap()
                    .color,
                expected
            );
        }
        assert!(
            defaults
                .with_arguments(["--color=cyan".to_owned()])
                .is_err()
        );
        assert!(defaults.with_arguments(["--color".to_owned()]).is_err());
        assert!(
            defaults
                .with_arguments(["--screensaver".to_owned()])
                .unwrap()
                .screensaver
        );
        assert!(
            defaults
                .with_arguments(["--screensaver=true".to_owned()])
                .is_err()
        );
        assert_eq!(
            defaults
                .with_arguments(["--type=classic".to_owned()])
                .unwrap()
                .rain_type,
            RainType::Classic
        );
        assert!(defaults.with_arguments(["--type=3d".to_owned()]).is_err());
        assert!(
            defaults
                .with_arguments(["--fps", "0"].map(str::to_owned))
                .is_err()
        );
        assert!(
            defaults
                .with_arguments(["--render-scale", "NaN"].map(str::to_owned))
                .is_err()
        );
        assert!(defaults.with_arguments(["--unknown".to_owned()]).is_err());
    }

    #[test]
    fn frame_pacer_holds_a_60_hz_deadline_after_slow_frames() {
        let start = Instant::now();
        let frame_time = Quality {
            fps: 60,
            ..Quality::default()
        }
        .frame_time();
        assert_eq!(
            next_frame_deadline(start, start, frame_time),
            start + frame_time
        );
        assert_eq!(
            next_frame_deadline(start, start + frame_time + frame_time / 2, frame_time),
            start + frame_time * 2
        );
    }

    #[test]
    fn screensaver_input_dismisses_after_real_activity() {
        let start = Instant::now();
        let mut dismissal = Dismissal::new(start);
        let pointer = |x, y| DismissalInput::Pointer(PhysicalPosition::new(x, y));
        assert!(!dismissal.on_input(pointer(100.0, 100.0), start, 1.0));
        assert!(!dismissal.on_input(
            pointer(200.0, 200.0),
            start + Duration::from_millis(200),
            1.0
        ));
        assert!(!dismissal.on_input(
            pointer(200.0, 200.0),
            start + Duration::from_millis(350),
            1.0
        ));
        assert!(!dismissal.on_input(
            pointer(203.0, 200.0),
            start + Duration::from_millis(360),
            1.0
        ));
        assert!(!dismissal.on_input(
            pointer(207.0, 200.0),
            start + Duration::from_millis(370),
            1.0
        ));
        assert!(dismissal.on_input(
            pointer(208.0, 200.0),
            start + Duration::from_millis(380),
            1.0
        ));

        let mut scaled = Dismissal::new(start);
        assert!(!scaled.on_input(pointer(0.0, 0.0), start, 2.0));
        assert!(!scaled.on_input(pointer(12.0, 0.0), start + Duration::from_millis(400), 2.0));
        assert!(scaled.on_input(pointer(16.0, 0.0), start + Duration::from_millis(410), 2.0));

        let mut refocused = Dismissal::new(start);
        assert!(!refocused.on_input(pointer(10.0, 10.0), start, 1.0));
        assert!(!refocused.handles(
            &WindowEvent::Focused(false),
            start + Duration::from_millis(400),
            1.0
        ));
        assert_eq!(
            refocused.on_input(
                pointer(100.0, 100.0),
                start + Duration::from_millis(500),
                1.0
            ),
            !cfg!(target_os = "windows")
        );

        let mut no_initial_position = Dismissal::new(start);
        assert_eq!(
            no_initial_position.on_input(
                pointer(100.0, 100.0),
                start + Duration::from_millis(400),
                1.0
            ),
            !cfg!(target_os = "windows")
        );

        for input in [
            DismissalInput::Key {
                pressed: true,
                synthetic: false,
            },
            DismissalInput::Modifier { active: true },
            DismissalInput::MouseButton { pressed: true },
            DismissalInput::Scroll { moved: true },
            DismissalInput::Touch { started: true },
        ] {
            assert!(Dismissal::new(start).on_input(input, start, 1.0));
        }
        for input in [
            DismissalInput::Key {
                pressed: true,
                synthetic: true,
            },
            DismissalInput::Modifier { active: false },
            DismissalInput::Key {
                pressed: false,
                synthetic: false,
            },
            DismissalInput::MouseButton { pressed: false },
            DismissalInput::Scroll { moved: false },
            DismissalInput::Touch { started: false },
        ] {
            assert!(!Dismissal::new(start).on_input(input, start, 1.0));
        }
    }
}
