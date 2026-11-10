# Linux installation and session setup

The Linux archive contains one executable, this guide, and license notices. It
needs a graphical Wayland or X11 session, a working Vulkan driver (or another
`wgpu` backend that supports the display), and the system C runtime. The
GitHub workflow is configured to build on Ubuntu 24.04, giving any artifact
from that run its glibc baseline. Local builds use the builder's own glibc
baseline. CPU-only software rendering is rejected.

The GitHub workflow verifies the Linux build and archive structure. Rendering,
input dismissal, monitor behavior, idle integration, and locking are manual
tests performed on the target desktop.

On the local x86_64 build, `ldd` lists `libc.so.6`, `libm.so.6`, and
`libgcc_s.so.1`. At run time, the selected display backend also needs its
Wayland or X11 client libraries and keyboard libraries; Vulkan rendering
needs a Vulkan loader and a compatible GPU driver/ICD. Use `ldd
matrix-screensaver` on the installed binary to find missing directly linked
libraries. The archive does not bundle the graphics driver or C runtime.

Extract the archive and install the executable for your user:

```sh
tar -xzf matrix-screensaver-*-linux-*.tar.gz
install -Dm755 matrix-screensaver-*-linux-*/matrix-screensaver "$HOME/.local/bin/matrix-screensaver"
"$HOME/.local/bin/matrix-screensaver" --help
```

Run `matrix-screensaver` for a resizable preview or
`matrix-screensaver --screensaver --type modern` for full-screen output on all
active monitors. `--type classic` selects flat rain. A focused screensaver
window exits on keyboard, button, wheel, touch, meaningful pointer movement,
or loss of focus. It does not authenticate or lock the session. Keep your
desktop's existing lock and suspend commands enabled.

## Omarchy (Quickshell idle service)

Omarchy already owns idle and lock timing. Clone its service with
`omarchy plugin clone omarchy.idle`. In the new user-owned
`~/.config/omarchy/plugins/<username>.idle/Service.qml`, set:

```qml
readonly property string screensaverClass: "org.omarchy.matrix-screensaver"
```

In `launchScreensaver()`, change only its `runProcess` command to use the
absolute installed executable path:

```qml
runProcess(screensaverProcess, "screensaver", "[[ $(omarchy-shell lock isLocked 2>/dev/null) == \"true\" ]] || exec /home/YOU/.local/bin/matrix-screensaver --screensaver --app-id org.omarchy.matrix-screensaver")
```

Replace `YOU` with your login name; the path in QML must be absolute. Restart
the shell with `omarchy restart shell`, then turn off Stay Awake when you want
idle activation. Omarchy's packaged rule sends `org.omarchy.screensaver` to a
hidden terminal workspace, so the native app uses the distinct class above.
The cloned service tracks that class while its existing lock timer remains in
control. Remove the clone with `omarchy plugin remove <username>.idle --yes`
and `omarchy restart shell` to restore the packaged service. Do not edit
`/usr/share/omarchy/`.

## Hyprland with hypridle

If your session actually uses `hypridle`, add a listener to its user-owned
configuration, adapting the executable path and timeout:

```conf
listener {
    timeout = 150
    on-timeout = /home/YOU/.local/bin/matrix-screensaver --screensaver &
}
```

Leave existing lock, display power, and suspend listeners in place. Do not
run both this listener and Omarchy's Quickshell idle service for the same
screensaver. Remove this listener to undo the integration. The app should
receive focus and dismiss itself on input; check that in your compositor.

## Sway with swayidle

Add a timeout event to the `swayidle` command or its config; keep your existing
lock and before-sleep events. For a config file:

```conf
timeout 150 '/home/YOU/.local/bin/matrix-screensaver --screensaver &'
```

Restart `swayidle` after changing its config. Remove that line to undo the
integration. The binary uses normal windows, so verify that Sway focuses the
screensaver and delivers input on each output.

## X11

Run the binary directly under X11, or use a standalone idle launcher such as
`xautolock` to run it after inactivity. For example, start one `xautolock`
instance in your X11 session with
`xautolock -time 3 -locker "$HOME/.local/bin/matrix-screensaver --screensaver"`.
Keep a separate real screen locker. Remove that startup command to undo it.
This self-created window is not an XScreenSaver module: it does not draw into
`XSCREENSAVER_WINDOW`. GNOME and KDE screensaver integration is not claimed.
Native Xorg multi-monitor behavior awaits an interactive runner check. The
local Xwayland probe placed both windows on one monitor and is not a passing
X11 integration test.

The idle command syntax above follows the [Hypridle listener documentation](https://wiki.hypr.land/hypr-ecosystem/user/hypridle/),
the [swayidle manual](https://man.archlinux.org/man/swayidle.1), and the
[xautolock manual](https://manpages.ubuntu.com/manpages/noble/man1/xautolock.1.html).

## Removal and troubleshooting

Remove the session-specific listener or clone first, then remove
`$HOME/.local/bin/matrix-screensaver`. If the app reports that it cannot create
a GPU surface, check that your graphics driver and Vulkan loader work in the
same graphical session. If it opens behind other windows, inspect your
compositor's window rules and focus policy. If it closes immediately, check
for synthetic startup input or another window taking focus. The packaged
binary has no network or asset-file dependency at run time.
