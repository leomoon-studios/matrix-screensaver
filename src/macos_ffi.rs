use crate::{
    ColorTheme, Quality, RainType,
    renderer::{RenderStatus, Renderer, native_backends},
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
};
use winit::dpi::PhysicalSize;

struct MacSaver {
    renderer: Option<Renderer>,
    size: PhysicalSize<u32>,
    error: CString,
}

impl MacSaver {
    fn set_error(&mut self, message: impl Into<String>) {
        let message = message.into().replace('\0', " ");
        self.error = CString::new(message).expect("NUL bytes were removed");
    }
}

fn rain_type(value: u32) -> RainType {
    match value {
        1 => RainType::Classic,
        _ => RainType::Modern,
    }
}

fn color(value: u32) -> ColorTheme {
    match value {
        1 => ColorTheme::Blue,
        2 => ColorTheme::Red,
        3 => ColorTheme::Yellow,
        4 => ColorTheme::Purple,
        5 => ColorTheme::Orange,
        _ => ColorTheme::Green,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_saver_create(
    metal_layer: *mut c_void,
    width: u32,
    height: u32,
    rain_type_value: u32,
    color_value: u32,
    fps: u32,
    render_scale: f32,
) -> *mut c_void {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let size = PhysicalSize::new(width.max(1), height.max(1));
        let mut host = Box::new(MacSaver {
            renderer: None,
            size,
            error: CString::default(),
        });
        if metal_layer.is_null() {
            host.set_error("the ScreenSaverView supplied no CAMetalLayer");
            return Box::into_raw(host).cast();
        }

        let quality = Quality {
            fps: fps.clamp(1, 60),
            render_scale: render_scale.clamp(0.25, 1.0),
            rain_type: rain_type(rain_type_value),
            color: color(color_value),
            screensaver: true,
        };
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        descriptor.backends &= native_backends();
        let instance = wgpu::Instance::new(descriptor);
        let surface = unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(metal_layer))
        };
        match surface {
            Ok(surface) => match pollster::block_on(Renderer::new_for_surface(
                instance, surface, size, quality, None, false,
            )) {
                Ok(renderer) => host.renderer = Some(renderer),
                Err(error) => host.set_error(error),
            },
            Err(error) => host.set_error(format!("could not create the Metal surface: {error}")),
        }
        Box::into_raw(host).cast()
    }));
    result.unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_saver_render(handle: *mut c_void) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let Some(host) = (unsafe { handle.cast::<MacSaver>().as_mut() }) else {
            return -1;
        };
        let Some(renderer) = host.renderer.as_mut() else {
            return -1;
        };
        match renderer.render() {
            Ok(RenderStatus::Presented) => 1,
            Ok(RenderStatus::Skipped) => 0,
            Ok(RenderStatus::SurfaceLost) => {
                renderer.resize(host.size);
                0
            }
            Err(error) => {
                host.set_error(error);
                host.renderer = None;
                -1
            }
        }
    }));
    result.unwrap_or(-1)
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_saver_resize(handle: *mut c_void, width: u32, height: u32) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let Some(host) = (unsafe { handle.cast::<MacSaver>().as_mut() }) else {
            return;
        };
        host.size = PhysicalSize::new(width.max(1), height.max(1));
        if let Some(renderer) = host.renderer.as_mut() {
            renderer.resize(host.size);
        }
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_saver_set_paused(handle: *mut c_void, paused: bool) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let Some(host) = (unsafe { handle.cast::<MacSaver>().as_mut() }) else {
            return;
        };
        if let Some(renderer) = host.renderer.as_mut() {
            if paused {
                renderer.pause();
            } else {
                renderer.resume();
            }
        }
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_saver_error(handle: *const c_void) -> *const c_char {
    let Some(host) = (unsafe { handle.cast::<MacSaver>().as_ref() }) else {
        return c"invalid Matrix screensaver handle".as_ptr();
    };
    host.error.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_saver_destroy(handle: *mut c_void) {
    if handle.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
        drop(Box::from_raw(handle.cast::<MacSaver>()));
    }));
}

#[unsafe(no_mangle)]
pub extern "C" fn matrix_saver_ffi_version() -> *const c_char {
    const VERSION: &CStr = c"matrix-saver-ffi-1";
    VERSION.as_ptr()
}

#[cfg(test)]
mod tests {
    use super::{color, rain_type};
    use crate::{ColorTheme, RainType};

    #[test]
    fn native_values_have_safe_defaults() {
        assert_eq!(rain_type(0), RainType::Modern);
        assert_eq!(rain_type(1), RainType::Classic);
        assert_eq!(rain_type(99), RainType::Modern);
        assert_eq!(color(0), ColorTheme::Green);
        assert_eq!(color(5), ColorTheme::Orange);
        assert_eq!(color(99), ColorTheme::Green);
    }
}
