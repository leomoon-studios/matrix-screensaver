#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
binary="$repo_root/target/release/matrix-screensaver"
[[ $(omarchy-shell lock isLocked) == false ]] || { echo 'Session already locked' >&2; exit 1; }
child=''
cleanup() {
  if [[ -n $child ]] && kill -0 "$child" 2>/dev/null; then
    kill "$child" 2>/dev/null || true
    wait "$child" 2>/dev/null || true
  fi
}
trap cleanup EXIT

"$binary" --screensaver --app-id org.omarchy.matrix-screensaver &
child=$!
sleep 3
kill -0 "$child"
count=$(hyprctl clients -j | jq '[.[] | select(.class == "org.omarchy.matrix-screensaver")] | length')
((count >= 2)) || { echo 'Expected two screensaver windows before lock' >&2; exit 1; }

omarchy-system-lock
deadline=$((SECONDS + 10))
while ((SECONDS < deadline)); do
  status=$(omarchy-shell lock status)
  if jq -e '.locked and .secure' <<< "$status" >/dev/null; then break; fi
  sleep 0.2
done
jq -e '.locked and .secure' <<< "$status" >/dev/null || { echo 'Secure lock did not activate' >&2; exit 1; }
deadline=$((SECONDS + 10))
while ((SECONDS < deadline)) && kill -0 "$child" 2>/dev/null; do sleep 0.1; done
if kill -0 "$child" 2>/dev/null; then echo 'Screensaver stayed active over lock' >&2; exit 1; fi
wait "$child"
child=''
echo 'Secure Omarchy lock activated; screensaver exited. Unlock the desktop manually.'
