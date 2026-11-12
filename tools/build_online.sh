#!/bin/zsh
# Build the game's 01.60 update, the version online play (PS Rewired) needs,
# from the official update packages: download, extract, decrypt, analyse,
# lift, compile. For a Mac that already builds the port (tools/install.sh or
# tools/rebuild.sh); the launcher's "Play online" then finds what this makes,
# and the launcher's headless --prepare (RecompLauncher) runs it after the
# campaign's build.
#
#   tools/build_online.sh [step ...]
#
# Steps, in order (default: all of them):
#
#   download  the game's two official update packages, 01.50 and 01.60
#             (about 340 MB together), from Sony's update server over plain
#             HTTP into game/BCUS98120_v160/pkg/. Each is checked against the
#             size and SHA-1 below (of everything but its last 32 bytes, where
#             the package keeps its own digest); one that checks out is not
#             downloaded again.
#   extract   the toolkit's tools/pkg_extract.py -> game/BCUS98120_v160/x150
#             and x160 (slow: a few minutes)
#   stage     the update's files as the console's HDD holds them ->
#             game/BCUS98120_v160/hdd0/game/BCUS98120/: PARAM.SFO and
#             USRDIR/EBOOT.BIN from 01.60, USRDIR/data/patch_01.psarc from
#             01.50 and USRDIR/data/patch_03.psarc from 01.60 (the launcher
#             copies them into the online folder's dev_hdd0)
#   decrypt   the 01.60 EBOOT.BIN -> game/BCUS98120_v160/EBOOT.elf, checked by
#             its SHA-256. It is an NPDRM SELF with a free licence (type 3),
#             so tools/decrypt_self.py decrypts it without RPCS3, with the key
#             file tools/fetch_keys.py --npdrm writes from RPCS3's public
#             source (PS3_KEYS: a key file of your own). RPCS3 (rpcs3
#             --decrypt; RPCS3=<path> if it is not in Applications) is only
#             the fallback when that fails. Skipped when EBOOT.elf is already
#             there, so an ELF decrypted some other way can be dropped in (it
#             is still checked).
#   analyse   toolkit tools/ppu_loader.py -> analysis/BCUS98120_v160
#   lift      toolkit tools/ppu_lifter.py -> recompiled/BCUS98120_v160
#   spu       the SPU code cut out of the 01.60 EBOOT.elf, as tools/rebuild.sh
#             spu does for 01.00, into its own folder port/spu_v160/ (the
#             policy module is the same bytes in both versions, so it takes
#             port/spu/r2_job_pm/register.c along) and
#             game/BCUS98120_v160/spu_images/
#   build     cmake + ninja -> build/BCUS98120_v160/ResistanceRecomp and
#             build/BCUS98120_v160/Resistance
#
# With no step named it continues where an earlier run stopped, as
# tools/install.sh does: download, extract and stage are skipped once the
# update's files are staged (the extracted folders are then removed; the
# packages are kept), decrypt while EBOOT.elf is there, analyse, lift and spu
# when they finished before with the same inputs (stamps in
# out/install_v160/), and the build is incremental. A step named runs
# whatever was done before. Failures end with an "ERROR: ..." line.
#
# Everything it writes is game-derived and .gitignore'd. It needs the
# toolkit (tools/rebuild.sh toolkit) and its Python (tools/env.sh's PY, or
# the checkout's .venv, with pycryptodome for a fast extract and for the
# decrypt), and curl.
# Environment: see tools/env.sh (PS3RECOMP_DIR, PY), PS3_KEYS / RES_SELF_KEYS
# (the key file), RPCS3, JOBS, RES_ONLINE_NEED_GB, and RES_V160 (default:
# game/BCUS98120_v160) for where the update goes.
set -e
setopt pipefail
here=${0:A:h}
# the venv the launcher's rebuild makes (the toolkit's Python packages), when
# there is one and PY is not set
[ -z "$PY" ] && [ -x "${RES_ROOT:-${here:h}}/.venv/bin/python" ] && PY=${RES_ROOT:-${here:h}}/.venv/bin/python
source "$here/env.sh"
V=BCUS98120_v160
RES_V160=${RES_V160:-$RES_ROOT/game/$V}
AN=$RES_ROOT/analysis/$V
RC=$RES_ROOT/recompiled/$V
SPU=$RES_ROOT/port/spu_v160
E=$RES_V160/EBOOT.elf
HDD=$RES_V160/hdd0/game/BCUS98120
ST=$RES_OUT/install_v160
auto=0
if (( $# )); then steps=("$@"); else steps=(download extract stage decrypt analyse lift spu build); auto=1; fi
jobs=(); [ -n "$JOBS" ] && jobs=(-j "$JOBS")

# sha256 of the decrypted 01.60 EBOOT.elf (tools/r2_layout.py EBOOT_BUILDS)
EBOOT_SHA=697bd1df98f79ce099e54e0e60e0a1744d3950ea43a2a06978f69a91e51b3d41

# The official packages: name, size, SHA-1 of all but the last 32 bytes.
UPDATE_URL=http://b0.ww.np.dl.playstation.net/tppkg/np/BCUS98120/BCUS98120_T18/27a5cba1036b9573
PKG150=UP9000-BCUS98120_00-RESISTANCE200001-A0150-V0100-PE.pkg
PKG160=UP9000-BCUS98120_00-RESISTANCE200001-A0160-V0100-PE.pkg
typeset -A PKG_SIZE PKG_SHA1
PKG_SIZE[$PKG150]=318340640; PKG_SHA1[$PKG150]=e28a9bc24b7dbde6d9855103014906a28562aec8
PKG_SIZE[$PKG160]=23780832;  PKG_SHA1[$PKG160]=5afb8b3d4ba00382a4420dea1a123d0e5e93ae9e
STAGED=(PARAM.SFO USRDIR/EBOOT.BIN USRDIR/data/patch_01.psarc USRDIR/data/patch_03.psarc)

# What the online build leaves on the disk is about 2.7 GB (measured on the
# development tree: the packages 0.33 GB, the staged files 0.31 GB, the lift
# 1.2 GB, the build 0.57 GB, the analysis and the EBOOT 0.02 GB), with the
# extracted packages (0.33 GB) and the build tree on the way.
NEED_GB=${RES_ONLINE_NEED_GB:-4}

say() { print -P "%B==> $*%b"; }
fail() { print -u2 -r -- ""; print -u2 -r -- "ERROR: $*"; exit 1; }

find_rpcs3() {
    for c in "$RPCS3" ~/Applications/RPCS3.app/Contents/MacOS/rpcs3 \
             /Applications/RPCS3.app/Contents/MacOS/rpcs3 "$(command -v rpcs3)"; do
        [ -n "$c" ] && [ -x "$c" ] && { echo "$c"; return 0; }
    done
    return 1
}

# pkg_ok <file> <name>: the package is complete and the official one
pkg_ok() {
    local f=$1 n=$2
    [ -f "$f" ] && [ "$(stat -L -f %z "$f")" = "${PKG_SIZE[$n]}" ] || return 1
    [ "$(head -c $(( ${PKG_SIZE[$n]} - 32 )) "$f" | shasum -a 1 | cut -d' ' -f1)" = "${PKG_SHA1[$n]}" ]
}

need_toolkit() {
    [ -f "$TK/tools/$1" ] || fail "no toolkit at $TK (run tools/rebuild.sh toolkit, or set PS3RECOMP_DIR)"
}

# an APFS clone where it works (no extra disk space), a plain copy otherwise
put() {
    mkdir -p "${2:h}"; rm -f "$2"
    cp -c "$1" "$2" 2>/dev/null || cp "$1" "$2"
}

spu_inputs() {
    local inputs=$E f
    for f in "$RES_V160"/spu_images/*.elf(N); do inputs="$inputs;$f"; done
    print -r -- "$inputs"
}

# the update's files are staged, by a stage step that finished
staged() {
    [ -f "$ST/stage" ] || return 1
    local f
    for f in $STAGED; do [ -f "$HDD/$f" ] || return 1; done
}

# analyse, lift, spu: each step's stamp chains the one before it (the ELF,
# the toolkit's pin and patch, the scripts), so a new analysis re-lifts too
typeset -A KEYS
step_keys() {
    local k=
    k=$({ cat -- "$E" "$here/setup_toolkit.sh" "$RES_ROOT"/patches/*.patch(N); } | shasum -a 256 | cut -d' ' -f1)
    KEYS[analyse]=$k
    k=$({ print -r -- "$k"; cat -- "$here/build_online.sh"; } | shasum -a 256 | cut -d' ' -f1)
    KEYS[lift]=$k
    k=$({ print -r -- "$k"; cat -- "$here"/prepare_r2_*.py(N) "$here/regen_r2_jobmods.sh" "$here/lift_r2_relocmods.py" \
            "$here/extract_jobmods_elf.py" "$RES_ROOT/port/spu/r2_job_pm/register.c"; } | shasum -a 256 | cut -d' ' -f1)
    KEYS[spu]=$k
}
# done_before <step>: skip it (with no step named, and the same inputs)
done_before() {
    (( auto )) || return 1
    (( ${#KEYS} )) || step_keys
    [ "$(cat "$ST/$1" 2>/dev/null)" = "${KEYS[$1]}" ] || { rm -f "$ST/$1"; return 1; }
    say "$1 (done before with the same inputs, skipped)"
}
stamp() {
    (( ${#KEYS} )) || step_keys
    print -r -- "${KEYS[$1]}" >"$ST/$1"
}

# One run at a time in this checkout.
mkdir -p "$ST"
if [ -f "$ST/pid" ] && kill -0 "$(cat "$ST/pid")" 2>/dev/null; then
    fail "another install is already running in $RES_ROOT (tools/build_online.sh, process $(cat "$ST/pid")). Wait for it, or stop it, and run this again."
fi
print -r -- $$ >"$ST/pid"
trap 'rm -f "$ST/pid"' EXIT

if (( auto )) && [ ! -x "$RES_BUILD/$V/ResistanceRecomp" ]; then
    mkdir -p "$RES_V160"
    free_gb=$(( $(df -k "$RES_V160" | awk 'NR==2 {print $4}') / 1024 / 1024 ))
    if (( free_gb < NEED_GB )); then
        fail "not enough free disk space for the online version: $free_gb GB free, about $NEED_GB GB are needed. Free some space and run this again."
    fi
fi
# the update's files are there: no package is needed again
skip_pkg=0
(( auto )) && staged && skip_pkg=1

for s in $steps; do
    case $s in
    download)
        if (( skip_pkg )); then say "download (the update's files are staged already, skipped)"; continue; fi
        say "download (the official 01.50 and 01.60 update packages)"
        mkdir -p "$RES_V160/pkg"
        for n in $PKG150 $PKG160; do
            f=$RES_V160/pkg/$n
            if pkg_ok "$f" $n; then echo "  $n: already here, checked"; continue; fi
            rm -f "$f"
            # resume a cut download; a complete .part needs no request
            if [ "$(stat -f %z "$f.part" 2>/dev/null)" != "${PKG_SIZE[$n]}" ]; then
                curl -fL --retry 3 -C - -o "$f.part" "$UPDATE_URL/$n" ||
                    fail "could not download the game's official update $n from Sony's update server. Check the internet connection and run this again (the download continues where it stopped)."
            fi
            mv "$f.part" "$f"
            pkg_ok "$f" $n || { rm -f "$f"; fail "the downloaded $n differs from the official package (size or SHA-1); it was removed. Run this again to download it again."; }
            echo "  $n: downloaded, checked"
        done ;;
    extract)
        if (( skip_pkg )); then say "extract (the update's files are staged already, skipped)"; continue; fi
        say "extract (a few minutes)"
        need_toolkit pkg_extract.py
        "$PY" -c 'import Crypto' 2>/dev/null || echo "note: $PY has no pycryptodome (the toolkit's requirements.txt): the extract runs in pure Python and takes much longer" >&2
        for n in $PKG150 $PKG160; do
            pkg_ok "$RES_V160/pkg/$n" $n || fail "$n is missing or not the official package: run the download step"
        done
        for v n in x150 $PKG150 x160 $PKG160; do
            rm -rf "$RES_V160/$v"
            "$PY" -I "$TK/tools/pkg_extract.py" "$RES_V160/pkg/$n" "$RES_V160/$v" || fail "could not extract $n (the toolkit's tools/pkg_extract.py)"
        done ;;
    stage)
        if (( skip_pkg )); then say "stage (the update's files are staged already, skipped)"; continue; fi
        say "stage (the update's files for the online folder's dev_hdd0)"
        X150=$RES_V160/x150; X160=$RES_V160/x160
        for f in $X160/PARAM.SFO $X160/USRDIR/EBOOT.BIN $X150/USRDIR/data/patch_01.psarc $X160/USRDIR/data/patch_03.psarc; do
            [ -f "$f" ] || fail "$f is missing: run the extract step"
        done
        rm -f "$ST/stage"
        rm -rf "$RES_V160/hdd0"
        put $X160/PARAM.SFO "$HDD/PARAM.SFO"
        put $X160/USRDIR/EBOOT.BIN "$HDD/USRDIR/EBOOT.BIN"
        put $X150/USRDIR/data/patch_01.psarc "$HDD/USRDIR/data/patch_01.psarc"
        put $X160/USRDIR/data/patch_03.psarc "$HDD/USRDIR/data/patch_03.psarc"
        print -r -- staged >"$ST/stage"
        # what the online build needs from the packages is staged now
        (( auto )) && rm -rf "$X150" "$X160"
        echo "  -> $HDD" ;;
    decrypt)
        say "decrypt (the 01.60 EBOOT.BIN)"
        if [ ! -f "$E" ]; then
            [ -f "$HDD/USRDIR/EBOOT.BIN" ] || fail "$HDD/USRDIR/EBOOT.BIN is missing: run the stage step"
            mkdir -p "$RES_V160"
            # tools/decrypt_self.py first (no RPCS3: a free NPDRM licence)
            keys=$PS3_KEYS
            if [ -z "$keys" ]; then
                # (RES_SELF_KEYS moves the key file; fetch_keys.py reads it itself)
                keys=$("$PY" "$here/fetch_keys.py" --npdrm) || keys=
            fi
            if [ -n "$keys" ] && "$PY" "$here/decrypt_self.py" "$HDD/USRDIR/EBOOT.BIN" -o "$E.partial" --keys "$keys"; then
                mv "$E.partial" "$E"
            else
                rm -f "$E.partial"
                print -u2 -r -- "decrypting without RPCS3 failed; trying RPCS3"
                rp=$(find_rpcs3) || fail "could not decrypt the 01.60 EBOOT.BIN: decrypting it without RPCS3 failed (its keys are downloaded once from RPCS3's source code on GitHub: check the internet connection), and RPCS3 is not installed to try instead. Run this again, or install RPCS3, or put a decrypted 01.60 EBOOT.elf at $E."
                put "$HDD/USRDIR/EBOOT.BIN" "$RES_V160/EBOOT.BIN"
                "$rp" --decrypt "$RES_V160/EBOOT.BIN" >/dev/null 2>&1 || true
                rm -f "$RES_V160/EBOOT.BIN"
                [ -f "$E" ] || fail "could not decrypt the 01.60 EBOOT.BIN (without RPCS3, nor with RPCS3 at $rp)"
            fi
        fi
        [ "$(head -c 4 "$E")" = $'\x7fELF' ] || fail "$E is not an ELF: remove it and run this again"
        if [ "$(shasum -a 256 "$E" | cut -d' ' -f1)" != "$EBOOT_SHA" ]; then
            fail "$E is not the 01.60 executable this port knows (SHA-256 differs): remove it and run this again"
        fi
        echo "  $E: the 01.60 executable" ;;
    analyse)
        done_before analyse && continue
        say "analyse"
        need_toolkit ppu_loader.py
        "$PY" "$TK/tools/ppu_loader.py" "$E" -o "$AN"
        stamp analyse ;;
    lift)
        done_before lift && continue
        say "lift"
        need_toolkit ppu_lifter.py
        "$PY" "$TK/tools/ppu_lifter.py" "$E" "${jobs[@]}" --chunks 24 \
            --functions "$AN/EBOOT.functions.json" --hle-stubs "$AN/EBOOT.imports.json" \
            --output "$RC"
        stamp lift ;;
    spu)
        done_before spu && continue
        say "spu (policy module, raw-SPU programs, job modules, relocatable modules, raw-SPU loader)"
        mkdir -p "$SPU/r2_job_pm"
        PS3RECOMP_DIR=$TK "$PY" "$here/prepare_r2_job_pm.py" --eboot "$E" --dest "$SPU/r2_job_pm"
        cp "$RES_ROOT/port/spu/r2_job_pm/register.c" "$SPU/r2_job_pm/register.c"
        PS3RECOMP_DIR=$TK "$PY" "$here/prepare_r2_rawspu.py" --eboot "$E" --spu-dir "$SPU"
        PS3RECOMP_DIR=$TK R2_SPU_DIR=$SPU "$here/regen_r2_jobmods.sh" "$E"
        PS3RECOMP_DIR=$TK "$PY" "$here/lift_r2_relocmods.py" "$E" --out "$SPU/relocmods"
        PS3RECOMP_DIR=$TK "$PY" "$here/prepare_r2_rawspu_loader.py" --eboot "$E" --out "$RES_V160/spu_images"
        stamp spu ;;
    build)
        say "build"
        bp=${HOMEBREW_PREFIX:-$(brew --prefix 2>/dev/null)}
        cmake -S "$RES_ROOT/port" -B "$RES_BUILD/$V" -G Ninja \
              -DPS3RECOMP_DIR="$TK" -DRECOMP_DIR="$RC" -DR2_EBOOT="$E" -DSPU_DIR="$SPU" \
              -DPORT_SPU_INPUTS="$(spu_inputs)" -DPython3_EXECUTABLE="$PY" \
              -DPS3RECOMP_FFMPEG_RUNTIME=ON -DPS3RECOMP_FFMPEG_INCLUDE_DIR="${bp:-/opt/homebrew}/include"
        cmake --build "$RES_BUILD/$V" "${jobs[@]}" ;;
    *)
        echo "unknown step '$s' (download extract stage decrypt analyse lift spu build)" >&2; exit 2 ;;
    esac
done
say "done"
