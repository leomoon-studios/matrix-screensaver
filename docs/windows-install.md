# Windows installation

The package contains the x86-64 Windows screensaver, the project license and third-party attribution. A compatible DirectX 12 graphics driver and the Microsoft Visual C++ runtime used by the Rust MSVC build are required; Rust and build tools are not needed to run it.

Copy `Matrix Screensaver.scr` to a permanent location and right-click it, then choose **Install**. Select Matrix Screensaver in Windows Screen Saver Settings, choose a wait time and optionally enable the Windows sign-in requirement. Windows controls locking and authentication. Remove the copied `.scr` and select a different saver in Screen Saver Settings to uninstall.

The saver has no settings dialog. The separate `matrix-screensaver.exe` remains available for development and accepts the command-line options in the main README.

Windows preview, multi-monitor behavior and lock-screen interaction have not yet been validated in an interactive Actions run. Do not treat the package as a verified release until those checks pass.
