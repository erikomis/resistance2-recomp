#!/usr/bin/env python3
"""Extract and lift Resistance 2's raw WWS SPU policy module from EBOOT.elf.

BCUS98120 embeds its Wws_Job policy module as raw SPU bytes (not an ELF) at
EBOOT VA 0x00CF9680, 12,544 bytes, loaded into local store at 0xA00. The SPU
image scanner cannot see it; without it the job queue never advances. This
cuts it out of your decrypted EBOOT, checks its FNV-1a fingerprint and lifts
it into port/spu/r2_job_pm/ (spu_recomp.c/.h, image.bin, functions.json,
lift.json: translated game code, .gitignore'd). port/spu/r2_job_pm/register.c
registers it under that fingerprint. Re-run cmake afterwards.

  tools/prepare_r2_job_pm.py            # extract and lift
  tools/prepare_r2_job_pm.py --check    # the lifted image matches the EBOOT
"""
import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import r2_layout  # noqa: E402

VA = 0x00CF9680
SIZE = 12544
LS_BASE = 0xA00
FINGERPRINT = 0x924CAF0A30D9F98F


def image(eboot):
    data = r2_layout.eboot_bytes(eboot, VA, SIZE, "the WWS policy module")
    if r2_layout.fnv1a64(data) != FINGERPRINT:
        raise SystemExit("the WWS policy module's fingerprint differs: not the tested BCUS98120 EBOOT?")
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="only check the current extraction")
    parser.add_argument("--eboot", type=Path, default=r2_layout.eboot())
    parser.add_argument("--dest", type=Path, default=r2_layout.SPU_DIR / "r2_job_pm")
    parser.add_argument("--toolkit", type=Path, default=None, help="default: PS3RECOMP_DIR or ./ps3recomp")
    args = parser.parse_args()
    raw = image(args.eboot)
    if args.check:
        r2_layout.check_image(args.dest, raw, "WWS PM", "prepare_r2_job_pm.py")
        print(f"R2 WWS PM: {SIZE} bytes, FNV {FINGERPRINT:016X}, OK")
        return
    n = r2_layout.lift_raw(raw, args.dest, LS_BASE, "r2_job_pm_", args.toolkit)
    print(f"R2 WWS PM ready: {SIZE} bytes, FNV {FINGERPRINT:016X}, {n} functions -> {args.dest}")


if __name__ == "__main__":
    main()
