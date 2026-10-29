#!/bin/zsh
# Play Resistance 2 in a window, with a controller.
#
#   tools/play.sh
#   RES_LEVEL=<n> tools/play.sh      # experimental, see below
#
# Resistance 2 is a single executable: the EBOOT runs the menus and the
# campaign (there is no exitspawn into a second ELF as in Resistance: Fall of
# Man). The window belongs to the launcher (build/BCUS98120/Resistance, built
# here in seconds when missing): the game renders into it through shared
# IOSurfaces. A controller is required on macOS (DualSense, DualShock or
# Xbox, through SDL2). Close the game window (or Ctrl+C here) to quit.
#
# Saves, the HDD folders and the log live in RES_DATA (default
# ~/Library/Application Support/resistance-recomp-2/): dev_hdd0/, dev_hdd1/
# and last.log. The window title shows the frame rate (PS3RECOMP_METAL_FPS=1).
# PS3_PAD_LOG=1 (the default here) logs controller button changes;
# PS3_PAD_LOG=0 turns that off. RES_FRONTEND_PORT0=0 turns off the frontend
# input hook (port/patches.cpp) for comparison.
#
# RES_LEVEL=<n> passes "-level <n>" to the game (PS3_ARGV); whether this
# EBOOT honours it is not confirmed yet.
#
# Needs the build from tools/rebuild.sh and the disc; see tools/env.sh for the
# paths and overrides.
here=${0:A:h}; source "$here/env.sh"
[ -x "$B" ] || { echo "no build at $B: run tools/rebuild.sh first" >&2; exit 1; }
[ -r "$E" ] || { echo "no $E: run tools/rebuild.sh first" >&2; exit 1; }
res_disc || exit 1
if [ ! -x "$L" ]; then
    echo "building the game window ($L)..."
    cmake --build "$RES_BUILD/BCUS98120" --target Resistance >&2 && [ -x "$L" ] ||
        { echo "could not build $L (tools/rebuild.sh build)" >&2; exit 1; }
fi

mkdir -p "$RES_DATA/dev_hdd0" "$RES_DATA/dev_hdd1"; cd "$RES_DATA" || exit 1
LOG=$RES_DATA/last.log
[ -n "$RES_LEVEL" ] && export PS3_ARGV="-level;$RES_LEVEL"
echo "Disc: $PS3_VFS_ROOT"
echo "Saves: $RES_DATA"
echo "Log: $LOG"

unset PS3RECOMP_METAL_HEADLESS
PS3_HDD0_ROOT=$RES_DATA/dev_hdd0 PS3_HDD1_ROOT=$RES_DATA/dev_hdd1 \
PS3RECOMP_METAL_FPS=1 PS3_PAD_LOG=${PS3_PAD_LOG:-1} \
    caffeinate -i "$L" "$B" "$E" > "$LOG" 2>&1
st=$?
echo "The game exited (status $st). Log: $LOG"
exit $st
