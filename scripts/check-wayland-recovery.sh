#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
binary="$repo_root/target/release/matrix-screensaver"
monitor=$(hyprctl monitors -j | jq -r '.[0].name')
[[ -n $monitor ]] || { echo 'No monitor for recovery check' >&2; exit 1; }
child=''
restore() {
  hyprctl dispatch "hl.dsp.dpms({ action = \"enable\", monitor = \"$monitor\" })" >/dev/null 2>&1 || true
  if [[ -n $child ]] && kill -0 "$child" 2>/dev/null; then
    kill "$child" 2>/dev/null || true
    wait "$child" 2>/dev/null || true
  fi
}
trap restore EXIT

"$binary" --screensaver --app-id org.omarchy.matrix-screensaver &
child=$!
sleep 3
kill -0 "$child"
before=$(hyprctl clients -j | jq '[.[] | select(.class == "org.omarchy.matrix-screensaver")] | length')
((before >= 2)) || { echo 'Expected two screensaver windows' >&2; exit 1; }

hyprctl dispatch "hl.dsp.dpms({ action = \"disable\", monitor = \"$monitor\" })" >/dev/null
sleep 3
hyprctl dispatch "hl.dsp.dpms({ action = \"enable\", monitor = \"$monitor\" })" >/dev/null
sleep 3
kill -0 "$child"
after=$(hyprctl clients -j | jq '[.[] | select(.class == "org.omarchy.matrix-screensaver" and .fullscreen == 2)] | length')
((after >= 2)) || { echo "Recovery left only $after fullscreen windows" >&2; exit 1; }
address=$(hyprctl clients -j | jq -r '.[] | select(.class == "org.omarchy.matrix-screensaver") | .address' | head -n 1)
hyprctl dispatch "hl.dsp.focus({ window = \"address:$address\" })" >/dev/null
sleep 0.2
wtype -k Escape
deadline=$((SECONDS + 10))
while ((SECONDS < deadline)) && kill -0 "$child" 2>/dev/null; do sleep 0.1; done
if kill -0 "$child" 2>/dev/null; then echo 'Screensaver did not dismiss after recovery' >&2; exit 1; fi
wait "$child"
child=''
echo "DPMS recovery passed on $monitor"
