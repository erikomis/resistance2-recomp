# Shared settings for the shell scripts in tools/. Sourced, not run (zsh).
# Every value can be overridden from the environment.
#
#   RES_ROOT      this repository (default: the parent of tools/)
#   PS3RECOMP_DIR the patched toolkit (default: $RES_ROOT/ps3recomp,
#                 tools/setup_toolkit.sh)
#   RES_BUILD     build directories (default: $RES_ROOT/build)
#                 $RES_BUILD/BCUS98120/ResistanceRecomp  the game
#                 $RES_BUILD/BCUS98120/Resistance        the window (launcher)
#   RES_GAME      the decrypted EBOOT and what is cut out of it (default:
#                 $RES_ROOT/game/BCUS98120): EBOOT.elf, PARAM.SFO,
#                 spu_images/ (tools/prepare_r2_rawspu_loader.py)
#   RES_OUT       scratch output of the SPU tools (default: $RES_ROOT/out)
#   RES_DATA      saves, HDD folders and the log (default:
#                 ~/Library/Application Support/resistance-recomp-2)
#   PS3_VFS_ROOT  your disc: the folder holding PS3_GAME/. Default:
#                 $RES_GAME when the disc was extracted there, else the first
#                 volume under /Volumes whose PARAM.SFO says BCUS98120
#   PS3RECOMP_FFMPEG_AVCODEC / PS3RECOMP_FFMPEG_AVUTIL
#                 FFmpeg libraries for the video decoder (default: Homebrew's)
#   PY            Python 3.10 or newer (default: the newest python3.1x on PATH)

RES_ROOT=${RES_ROOT:-${here:h}}
TK=${PS3RECOMP_DIR:-$RES_ROOT/ps3recomp}
RES_BUILD=${RES_BUILD:-$RES_ROOT/build}
RES_GAME=${RES_GAME:-$RES_ROOT/game/BCUS98120}
RES_OUT=${RES_OUT:-$RES_ROOT/out}
RES_DATA=${RES_DATA:-$HOME/Library/Application Support/resistance-recomp-2}
B=$RES_BUILD/BCUS98120/ResistanceRecomp
L=$RES_BUILD/BCUS98120/Resistance
E=$RES_GAME/EBOOT.elf
export RES_OUT

if [ -z "$PY" ]; then
    for c in python3.14 python3.13 python3.12 python3.11 python3.10 python3; do
        command -v $c >/dev/null && { PY=$(command -v $c); break; }
    done
fi
export PY

# The SPU programs the build scans: the EBOOT's own SPU ELFs, plus the raw-SPU
# loader cut out of it (tools/prepare_r2_rawspu_loader.py).
res_spu_inputs() {
    local inputs=$E f
    for f in "$RES_GAME"/spu_images/*.elf(N); do inputs="$inputs;$f"; done
    print -r -- "$inputs"
}

if [ -z "$PS3_VFS_ROOT" ]; then
    if [ -d "$RES_GAME/PS3_GAME" ]; then
        PS3_VFS_ROOT=$RES_GAME
    else
        for d in /Volumes/*(N/); do
            if [ -f "$d/PS3_GAME/USRDIR/EBOOT.BIN" ] && grep -q BCUS98120 "$d/PS3_GAME/PARAM.SFO" 2>/dev/null; then
                PS3_VFS_ROOT=${d%/}; break
            fi
        done
    fi
fi
res_disc() {
    [ -n "$PS3_VFS_ROOT" ] && [ -d "$PS3_VFS_ROOT/PS3_GAME" ] && return 0
    echo "disc not found: mount your BCUS98120 disc image (or extract it to $RES_GAME)" >&2
    echo "and set PS3_VFS_ROOT to the folder holding PS3_GAME/" >&2
    return 1
}
export PS3_VFS_ROOT

# FFmpeg for the video decoder (built with -DPS3RECOMP_FFMPEG_RUNTIME=ON).
if [ -z "$PS3RECOMP_FFMPEG_AVCODEC" ] || [ -z "$PS3RECOMP_FFMPEG_AVUTIL" ]; then
    brew_prefix=${HOMEBREW_PREFIX:-$(brew --prefix 2>/dev/null)}
    brew_prefix=${brew_prefix:-/opt/homebrew}
    PS3RECOMP_FFMPEG_AVCODEC=${PS3RECOMP_FFMPEG_AVCODEC:-$brew_prefix/lib/libavcodec.dylib}
    PS3RECOMP_FFMPEG_AVUTIL=${PS3RECOMP_FFMPEG_AVUTIL:-$brew_prefix/lib/libavutil.dylib}
fi
export PS3RECOMP_FFMPEG_AVCODEC PS3RECOMP_FFMPEG_AVUTIL
