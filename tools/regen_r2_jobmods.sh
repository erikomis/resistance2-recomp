#!/bin/sh
# Lift the WWS SPU job modules Resistance 2 links into its EBOOT.
#
#   tools/regen_r2_jobmods.sh [EBOOT_ELF]
#
# EBOOT_ELF defaults to game/BCUS98120/EBOOT.elf. Each module has a 0xC0DEC0DE
# header and is streamed by the Wws_Job policy module (prepare_r2_job_pm.py)
# into its job code buffer at LS 0x4000. Interpreted, they kept the first
# level at ~5 FPS. tools/extract_jobmods_elf.py cuts them out and the
# toolkit's tools/lift_jobmods.py writes
#   port/spu/jobmods/jobmod_*/spu_recomp.{c,h}
#   port/spu/jobmods/jobmods_register.c
# (SPU images matched by the module's 16-byte content signature when the job
# manager DMAs it in). Translated game code: regenerate it, never commit it.
# Re-run cmake afterwards (tools/rebuild.sh build).
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
TK=${PS3RECOMP_DIR:-$here/ps3recomp}
PY=${PY:-python3}
elf=${1:-$here/game/BCUS98120/EBOOT.elf}
work=${RES_OUT:-$here/out}/r2-jobmods-elf
dest=${R2_SPU_DIR:-$here/port/spu}/jobmods

[ -f "$TK/tools/lift_jobmods.py" ] || { echo "no toolkit at $TK (run tools/setup_toolkit.sh)" >&2; exit 1; }
[ -f "$elf" ] || { echo "no $elf: decrypt your EBOOT.BIN first (tools/rebuild.sh decrypt)" >&2; exit 1; }
rm -rf "$work" "$dest"; mkdir -p "$work" "$dest"
"$PY" -I "$here/tools/extract_jobmods_elf.py" "$elf" --out "$work"
"$PY" "$TK/tools/lift_jobmods.py" --jobmods "$work" --out "$dest" \
      --register "$dest/jobmods_register.c" --symbol-prefix r2
echo "lifted job modules -> $dest (re-run cmake so they are picked up)"
