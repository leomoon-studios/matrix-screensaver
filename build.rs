fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/matrix-screensaver.ico");
        resource.set("FileDescription", "Matrix Screensaver");
        resource.set("ProductName", "Matrix Screensaver");
        resource.compile().expect("could not embed Windows icon");
    }
}
