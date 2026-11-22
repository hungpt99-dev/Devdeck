#!/bin/bash
set -u
cd /root/Devdeck/spikes/iced-spike
export DISPLAY=:97
Xvfb :97 -screen 0 1024x768x24 >/dev/null 2>&1 &
xpid=$!
sleep 2
./target/debug/iced-spike >/dev/null 2>&1 &
apppid=$!
sleep 5
echo "=== xwininfo ==="
xwininfo -root -tree 2>&1 | grep -i 'Devdeck UI spike' || echo NO_WINDOW_MATCH
kill "$apppid" 2>/dev/null || true
wait "$apppid" 2>/dev/null || true
kill "$xpid" 2>/dev/null || true
wait "$xpid" 2>/dev/null || true
