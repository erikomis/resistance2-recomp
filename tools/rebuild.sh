#!/bin/zsh
# Rebuild the port from your own disc: decrypt, analyse, lift, compile.
#
#   tools/rebuild.sh [step ...]
#
# Steps, in order (default: all of them): toolkit decrypt analyse lift spu build
#
#   toolkit  tools/setup_toolkit.sh (./ps3recomp at the pinned commit + patch)
#   decrypt  copies PS3_GAME/USRDIR/EBOOT.BIN and PS3_GAME/PARAM.SFO from the
#            disc to game/BCUS98120/ and decrypts the EBOOT with RPCS3
#            (rpcs3 --decrypt; RPCS3=<path> if it is not in /Applications).
#            Skipped when game/BCUS98120/EBOOT.elf is already there, so an ELF
#            decrypted some other way can simply be dropped in.
#   analyse  toolkit tools/ppu_loader.py -> analysis/BCUS98120
#   lift     toolkit tools/ppu_lifter.py -> recompiled/BCUS98120 (24 chunks)
#   spu      the SPU code the image scanner cannot find on its own, all cut
#            out of EBOOT.elf: tools/prepare_r2_job_pm.py (the Wws_Job policy
#            module), tools/prepare_r2_bink.py (the raw-SPU Bink decoder),
#            tools/regen_r2_jobmods.sh (the WWS job modules) -> port/spu/, and
#            tools/prepare_r2_rawspu.py (the raw-SPU loader) -> game/BCUS98120/spu_images/
#   build    cmake + ninja -> build/BCUS98120/ResistanceRecomp and
#            build/BCUS98120/Resistance (the window tools/play.sh runs);
#            Metal, FFmpeg video. The lifted PPU code's frontend input hook
#            (tools/apply_r2_frontend_patch.py) is applied at configure time.
#
# Everything it writes (game/, analysis/, recompiled/, build/, out/,
# port/spu/*/spu_recomp.*) is game-derived and .gitignore'd.
# Environment: see tools/env.sh (PS3_VFS_ROOT, PS3RECOMP_DIR, PY), RPCS3, JOBS.
set -e
here=${0:A:h}; source "$here/env.sh"
AN=$RES_ROOT/analysis/BCUS98120
RC=$RES_ROOT/recompiled/BCUS98120
if (( $# )); then steps=("$@"); else steps=(toolkit decrypt analyse lift spu build); fi
jobs=(); [ -n "$JOBS" ] && jobs=(-j "$JOBS")

# sha256 of the decrypted BCUS98120 EBOOT.elf the port was made from
EBOOT_SHA=408eb55fa3f00fc43823517df28044451d9192e2023219c98ebb6f95e41e89f5

say() { print -P "%B==> $*%b"; }

find_rpcs3() {
    for c in "$RPCS3" ~/Applications/RPCS3.app/Contents/MacOS/rpcs3 \
             /Applications/RPCS3.app/Contents/MacOS/rpcs3 "$(command -v rpcs3)"; do
        [ -n "$c" ] && [ -x "$c" ] && { echo "$c"; return 0; }
    done
    return 1
}

for s in $steps; do
    case $s in
    toolkit)
        say "toolkit"; "$here/setup_toolkit.sh" "$TK" ;;
    decrypt)
        say "decrypt"
        mkdir -p "$RES_GAME"
        if [ ! -f "$E" ]; then
            res_disc
            rp=$(find_rpcs3) || { echo "RPCS3 not found (set RPCS3=<path>), or put a decrypted EBOOT.elf in $RES_GAME" >&2; exit 1; }
            cp "$PS3_VFS_ROOT/PS3_GAME/USRDIR/EBOOT.BIN" "$RES_GAME/"
            "$rp" --decrypt "$RES_GAME/EBOOT.BIN" >/dev/null 2>&1 || true
            [ -f "$E" ] || { echo "RPCS3 did not write $E" >&2; exit 1; }
        fi
        [ -f "$RES_GAME/PARAM.SFO" ] || { [ -n "$PS3_VFS_ROOT" ] && cp "$PS3_VFS_ROOT/PS3_GAME/PARAM.SFO" "$RES_GAME/" 2>/dev/null || true; }
        [ "$(head -c 4 "$E")" = $'\x7fELF' ] || { echo "$E is not an ELF" >&2; exit 1; }
        if [ "$(shasum -a 256 "$E" | cut -d' ' -f1)" != "$EBOOT_SHA" ]; then
            echo "WARNING: EBOOT.elf differs from the tested BCUS98120 executable; the port may not match it" >&2
        fi ;;
    analyse)
        say "analyse"
        "$PY" "$TK/tools/ppu_loader.py" "$E" -o "$AN" ;;
    lift)
        say "lift"
        "$PY" "$TK/tools/ppu_lifter.py" "$E" "${jobs[@]}" --chunks 24 \
            --functions "$AN/EBOOT.functions.json" --hle-stubs "$AN/EBOOT.imports.json" \
            --output "$RC" ;;
    spu)
        say "spu (policy module, Bink decoder, job modules, raw-SPU loader)"
        PS3RECOMP_DIR=$TK "$PY" "$here/prepare_r2_job_pm.py" --eboot "$E"
        PS3RECOMP_DIR=$TK "$PY" "$here/prepare_r2_bink.py" --eboot "$E"
        PS3RECOMP_DIR=$TK "$here/regen_r2_jobmods.sh" "$E"
        PS3RECOMP_DIR=$TK "$PY" "$here/prepare_r2_rawspu.py" --eboot "$E" --out "$RES_GAME/spu_images" ;;
    build)
        say "build"
        bp=${HOMEBREW_PREFIX:-$(brew --prefix 2>/dev/null)}
        cmake -S "$RES_ROOT/port" -B "$RES_BUILD/BCUS98120" -G Ninja \
              -DPS3RECOMP_DIR="$TK" -DRECOMP_DIR="$RC" -DR2_EBOOT="$E" \
              -DPORT_SPU_INPUTS="$(res_spu_inputs)" -DPython3_EXECUTABLE="$PY" \
              -DPS3RECOMP_FFMPEG_RUNTIME=ON -DPS3RECOMP_FFMPEG_INCLUDE_DIR="${bp:-/opt/homebrew}/include"
        cmake --build "$RES_BUILD/BCUS98120" "${jobs[@]}" ;;
    *)
        echo "unknown step '$s' (toolkit decrypt analyse lift spu build)" >&2; exit 2 ;;
    esac
done
say "done"
