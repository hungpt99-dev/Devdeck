#!/bin/bash
set -u
cd /root/Devdeck/research/spikes/slint-spike
echo "=== smoke start $(date -u +%FT%TZ) ==="
export DISPLAY=:98
Xvfb :98 -screen 0 1024x768x24 >xvfb.log 2>&1 &
xpid=$!
sleep 2
SLINT_BACKEND=winit-software ./target/debug/slint-spike >run.log 2>&1 &
apppid=$!
sleep 5
echo "=== xwininfo ==="
xwininfo -root -tree 2>&1 | grep -i -E 'Devdeck|slint' || echo "NO_WINDOW_MATCH"
echo "=== app log ==="
cat run.log
kill "$apppid" 2>/dev/null || true
wait "$apppid" 2>/dev/null || true
kill "$xpid" 2>/dev/null || true
wait "$xpid" 2>/dev/null || true
echo "=== smoke end ==="
