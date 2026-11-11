# macOS installation

The package contains a universal `Matrix Screensaver.saver` bundle for Apple Silicon and Intel Macs. It uses the system `ScreenSaverView` lifecycle and Metal renderer. Rust and development tools are not required after the bundle is built.

Open the `.saver` bundle and approve installation for the current user, or copy it to `~/Library/Screen Savers/`. Then open **System Settings > Screen Saver** and select **Matrix Screensaver**. macOS controls full-screen presentation, activity dismissal, authentication, and one screensaver view per display.

The default is modern green rain at 30 FPS and 0.5 internal render scale. Advanced choices can be set in Terminal before opening the preview:

```bash
defaults write com.leomoon-studios.matrix-screensaver type classic
defaults write com.leomoon-studios.matrix-screensaver color purple
defaults write com.leomoon-studios.matrix-screensaver fps 30
defaults write com.leomoon-studios.matrix-screensaver render-scale 0.5
```

Valid types are `modern` and `classic`. Valid colors are `green`, `blue`, `red`, `yellow`, `purple`, and `orange`. FPS is clamped to 1 through 60, and render scale is clamped to 0.25 through 1. Restore the defaults with:

```bash
defaults delete com.leomoon-studios.matrix-screensaver
```

To uninstall, remove `~/Library/Screen Savers/Matrix Screensaver.saver` and select another screensaver. The development bundle is ad hoc signed. A public release should be signed with a Developer ID certificate and notarized before distribution.
