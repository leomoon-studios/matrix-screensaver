# Windows installation

The package contains the x86-64 Windows screensaver with an embedded Matrix icon, a reusable PNG of that icon, the project license and third-party attribution. A compatible DirectX 12 graphics driver and the Microsoft Visual C++ runtime used by the Rust MSVC build are required; Rust and build tools are not needed to run it.

Copy `Matrix Screensaver.scr` to a permanent location and right-click it, then choose **Install**. Select Matrix Screensaver in Windows Screen Saver Settings, choose a wait time and optionally enable the Windows sign-in requirement. Windows controls locking and authentication. Remove the copied `.scr` and select a different saver in Screen Saver Settings to uninstall.

Choose **Settings** in Windows Screen Saver Settings to select modern or classic rain, one of six colors, FPS from 1 to 60, and render scale from 0.25 to 1 in 0.05 steps. Click **Save** to apply the choices to future full-screen and preview launches. Settings are stored for the current Windows user under `%APPDATA%\Matrix Screensaver\settings.txt`; remove that file to restore defaults. The dedicated Windows GUI host does not create a command window. The separate console-subsystem `matrix-screensaver.exe` remains available for development, retains renderer diagnostics, and accepts the command-line options in the main README.

Full-screen rendering, Escape dismissal, and embedded preview were tested locally on one Windows display. Multi-monitor behavior, DPI changes, and lock-screen interaction still need an interactive Actions run on a machine with the required displays and GPU.
