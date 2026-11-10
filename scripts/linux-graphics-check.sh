#!/usr/bin/env bash
set -euo pipefail

# Run on a dedicated interactive Hyprland or native Xorg runner.
mode=${1:?Expected wayland or x11}
[[ $mode == wayland || $mode == x11 ]] || { echo 'Expected wayland or x11' >&2; exit 2; }
display_only=${2:-}
repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
binary="$repo_root/target/release/matrix-screensaver"
log_dir="$repo_root/graphics-check-logs"
mkdir -p "$log_dir"
child=''
cleanup() {
  if [[ -n $child ]] && kill -0 "$child" 2>/dev/null; then
    kill "$child" 2>/dev/null || true
    wait "$child" 2>/dev/null || true
  fi
}
trap cleanup EXIT

[[ -x $binary ]]

wait_for_exit() {
  local deadline=$((SECONDS + 10))
  while ((SECONDS < deadline)); do
    if ! kill -0 "$child" 2>/dev/null; then
      local status=0
      wait "$child" || status=$?
      child=''
      return "$status"
    fi
    sleep 0.1
  done
  return 1
}

if [[ $mode == wayland ]]; then
  command -v hyprctl >/dev/null
  command -v jq >/dev/null
  [[ -n ${WAYLAND_DISPLAY:-} ]]
  monitor_count=$(hyprctl monitors -j | jq 'length')
  ((monitor_count >= 2)) || { echo 'At least two Wayland outputs are required' >&2; exit 1; }
  env -u DISPLAY "$binary" --screensaver --app-id org.omarchy.matrix-screensaver \
    > "$log_dir/wayland.log" 2>&1 &
  child=$!
  deadline=$((SECONDS + 15))
  while ((SECONDS < deadline)); do
    hyprctl clients -j > "$log_dir/wayland-clients.json"
    found=$(jq '[.[] | select(.class == "org.omarchy.matrix-screensaver" and .fullscreen == 2)] | length' "$log_dir/wayland-clients.json")
    ((found >= monitor_count)) && break
    sleep 0.2
  done
  ((found >= monitor_count)) || { echo 'Wayland full-screen windows missing' >&2; exit 1; }
  jq -e --slurpfile monitors <(hyprctl monitors -j) \
    '[.[] | select(.class == "org.omarchy.matrix-screensaver") | .monitor] | unique | length >= ($monitors[0] | length)' \
    "$log_dir/wayland-clients.json" >/dev/null
  address=$(jq -r '.[] | select(.class == "org.omarchy.matrix-screensaver") | .address' "$log_dir/wayland-clients.json" | head -n 1)
  hyprctl dispatch "hl.dsp.focus({ window = \"address:$address\" })" >/dev/null
  active_class=$(hyprctl activewindow -j | jq -r '.class')
  [[ $active_class == org.omarchy.matrix-screensaver ]] || {
    echo "Could not focus screensaver window (active class: $active_class)" >&2
    exit 1
  }
  sleep 0.2
  hyprctl dispatch "hl.dsp.send_shortcut({ mods = \"\", key = \"Escape\", window = \"address:$address\" })" >/dev/null
  wait_for_exit || { echo 'Wayland key dismissal failed' >&2; exit 1; }
else
  command -v xdotool >/dev/null
  command -v xrandr >/dev/null
  command -v python3 >/dev/null
  [[ ${XDG_SESSION_TYPE:-} == x11 && -z ${WAYLAND_DISPLAY:-} && -n ${DISPLAY:-} ]] || {
    echo 'A native Xorg session is required; Xwayland is not an X11 integration test' >&2
    exit 1
  }
  xrandr --listmonitors > "$log_dir/x11-monitors.txt"
  x11_monitor_count=$(awk 'NR == 1 { print $2 }' "$log_dir/x11-monitors.txt")
  ((x11_monitor_count >= 2)) || { echo 'At least two X11 outputs are required' >&2; exit 1; }
  "$binary" --screensaver > "$log_dir/x11.log" 2>&1 &
  child=$!
  deadline=$((SECONDS + 15))
  while ((SECONDS < deadline)); do
    xdotool search --name '^Matrix Screensaver' > "$log_dir/x11-windows.txt" 2>/dev/null || true
    found=$(wc -l < "$log_dir/x11-windows.txt")
    ((found >= x11_monitor_count)) && break
    sleep 0.2
  done
  ((found >= x11_monitor_count)) || { echo 'X11 windows missing' >&2; exit 1; }
  python3 "$repo_root/scripts/check-x11-window-placement.py" \
    "$log_dir/x11-monitors.txt" "$log_dir/x11-windows.txt"
  window=$(head -n 1 "$log_dir/x11-windows.txt")
  xdotool windowactivate --sync "$window"
  xdotool key Escape
  wait_for_exit || { echo 'X11 key dismissal failed' >&2; exit 1; }
fi

if [[ $display_only == --display-only ]]; then
  echo "$mode display checks passed; lock and recovery checks were not run"
  exit 0
fi

# A dedicated runner supplies these scripts to check its actual locker and
# display recovery. Missing checks fail the graphics gate rather than skip it.
for check in MATRIX_GRAPHICS_LOCK_CHECK MATRIX_GRAPHICS_RECOVERY_CHECK; do
  path=${!check:-}
  [[ -n $path && -x $path ]] || { echo "$check executable is required" >&2; exit 1; }
  "$path" > "$log_dir/${check,,}.log" 2>&1
done

echo "$mode display, lock, and recovery checks passed"
