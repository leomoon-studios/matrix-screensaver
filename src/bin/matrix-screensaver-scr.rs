#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    matrix_screensaver::app::run(matrix_screensaver::app::HostMode::WindowsScreenSaver);
}
