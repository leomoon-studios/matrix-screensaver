#[cfg(target_os = "windows")]
use crate::Quality;
use std::num::NonZeroIsize;

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Fullscreen,
    Configure,
    Preview(NonZeroIsize),
}

pub fn parse(args: &[String]) -> Result<Mode, &'static str> {
    if args.is_empty() {
        return Ok(Mode::Configure);
    }
    let first = args[0].to_ascii_lowercase();
    let (switch, inline) = first
        .split_once(':')
        .map_or((first.as_str(), None), |(a, b)| (a, Some(b)));
    match switch {
        "/s" if args.len() == 1 && inline.is_none() => Ok(Mode::Fullscreen),
        "/c" if args.len() <= 2 => Ok(Mode::Configure),
        "/p" if args.len() <= 2 => {
            let value = inline
                .or_else(|| args.get(1).map(String::as_str))
                .ok_or("preview requires a window handle")?;
            if inline.is_some() && args.len() != 1 {
                return Err("unexpected preview argument");
            }
            let handle = value
                .parse::<isize>()
                .ok()
                .and_then(NonZeroIsize::new)
                .ok_or("invalid preview window handle")?;
            Ok(Mode::Preview(handle))
        }
        _ => Err("expected /s, /c, or /p <HWND>"),
    }
}

#[cfg(target_os = "windows")]
pub fn load_quality() -> Quality {
    let path = settings_path();
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Quality::default();
    };
    let args: Vec<String> = contents.lines().map(str::to_owned).collect();
    Quality::default().with_arguments(args).unwrap_or_default()
}

#[cfg(target_os = "windows")]
fn settings_path() -> std::path::PathBuf {
    std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Matrix Screensaver")
        .join("settings.txt")
}

#[cfg(target_os = "windows")]
fn log_path() -> std::path::PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Matrix Screensaver")
        .join("last-run.log")
}

#[cfg(target_os = "windows")]
pub fn start_log(mode: &Mode) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, format!("started: {mode:?}\n"));
}

#[cfg(target_os = "windows")]
pub fn append_log(message: &str) {
    use std::io::Write;
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{message}");
    }
}

#[cfg(target_os = "windows")]
fn choices(id: i32) -> Vec<String> {
    match id {
        10 => ["modern", "classic"].map(str::to_owned).to_vec(),
        11 => ["green", "blue", "red", "yellow", "purple", "orange"]
            .map(str::to_owned)
            .to_vec(),
        12 => (1..=60).map(|fps| fps.to_string()).collect(),
        13 => (0..=15)
            .map(|step| format!("{:.2}", 0.25 + step as f32 * 0.05))
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
pub fn configure() -> Result<(), String> {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    unsafe extern "system" fn proc(
        hwnd: windows_sys::Win32::Foundation::HWND,
        msg: u32,
        wp: usize,
        lp: isize,
    ) -> isize {
        match msg {
            WM_COMMAND if (wp & 0xffff) == 1 => {
                let mut args = Vec::new();
                for (id, flag) in [
                    (10, "--type"),
                    (11, "--color"),
                    (12, "--fps"),
                    (13, "--render-scale"),
                ] {
                    let combo = unsafe { GetDlgItem(hwnd, id) };
                    let selected = unsafe { SendMessageW(combo, CB_GETCURSEL, 0, 0) };
                    if selected < 0 {
                        return 0;
                    }
                    args.push(flag.to_owned());
                    args.push(choices(id)[selected as usize].clone());
                }
                let path = settings_path();
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(path, args.join("\n"));
                unsafe { DestroyWindow(hwnd) };
                0
            }
            WM_COMMAND if (wp & 0xffff) == 2 => {
                unsafe { DestroyWindow(hwnd) };
                0
            }
            WM_CLOSE => {
                unsafe { DestroyWindow(hwnd) };
                0
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                0
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
        }
    }
    unsafe {
        let instance = GetModuleHandleW(null());
        let class = wide("MatrixSaverSettings");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassW(&wc) == 0 {
            return Err("could not register settings window".into());
        }
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("Matrix Screensaver Settings").as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            410,
            280,
            null_mut(),
            null_mut(),
            instance,
            null(),
        );
        if hwnd.is_null() {
            return Err("could not open settings window".into());
        }
        let current = load_quality();
        let selections = [
            match current.rain_type {
                crate::RainType::Modern => 0,
                crate::RainType::Classic => 1,
            },
            match current.color {
                crate::ColorTheme::Green => 0,
                crate::ColorTheme::Blue => 1,
                crate::ColorTheme::Red => 2,
                crate::ColorTheme::Yellow => 3,
                crate::ColorTheme::Purple => 4,
                crate::ColorTheme::Orange => 5,
            },
            (current.fps - 1) as usize,
            ((current.render_scale - 0.25) / 0.05)
                .round()
                .clamp(0.0, 15.0) as usize,
        ];
        for (i, label) in ["Type", "Color", "FPS", "Render scale"].iter().enumerate() {
            let y = 18 + i as i32 * 47;
            CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide(label).as_ptr(),
                WS_CHILD | WS_VISIBLE,
                18,
                y,
                110,
                24,
                hwnd,
                null_mut(),
                instance,
                null(),
            );
            let combo = CreateWindowExW(
                0,
                wide("COMBOBOX").as_ptr(),
                null(),
                WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST as u32,
                130,
                y,
                225,
                150,
                hwnd,
                (10 + i) as *mut _,
                instance,
                null(),
            );
            for choice in choices(10 + i as i32) {
                SendMessageW(combo, CB_ADDSTRING, 0, wide(&choice).as_ptr() as isize);
            }
            SendMessageW(combo, CB_SETCURSEL, selections[i], 0);
        }
        for (id, label, x) in [(1, "Save", 180), (2, "Cancel", 275)] {
            CreateWindowExW(
                0,
                wide("BUTTON").as_ptr(),
                wide(label).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                x,
                210,
                85,
                28,
                hwnd,
                id as *mut _,
                instance,
                null(),
            );
        }
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        let mut message = std::mem::zeroed();
        while GetMessageW(&mut message, null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_windows_modes_and_handles() {
        let args = |values: &[&str]| values.iter().map(|v| v.to_string()).collect::<Vec<_>>();
        assert_eq!(parse(&[]), Ok(Mode::Configure));
        assert_eq!(parse(&args(&["/S"])), Ok(Mode::Fullscreen));
        assert_eq!(parse(&args(&["/c:123"])), Ok(Mode::Configure));
        assert_eq!(
            parse(&args(&["/p", "123"])),
            Ok(Mode::Preview(NonZeroIsize::new(123).unwrap()))
        );
        assert_eq!(
            parse(&args(&["/P:123"])),
            Ok(Mode::Preview(NonZeroIsize::new(123).unwrap()))
        );
        for input in [
            &["/p"][..],
            &["/p", "0"],
            &["/p", "abc"],
            &["/s", "extra"],
            &["/p:1", "2"],
            &["/x"],
        ] {
            assert!(parse(&args(input)).is_err());
        }
    }
}
