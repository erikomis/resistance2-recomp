#!/usr/bin/env python3
"""Extract and lift Resistance 2's raw-SPU Bink decoder from EBOOT.elf.

igInitRawSpu (the 320-byte loader both raw SPUs start with, see
prepare_r2_rawspu.py) DMAs this body from EBOOT VA 0x00EB6580 into local store
at 0x200 when a movie starts. It is raw SPU bytes, not an ELF, so the image
scanner never sees it and the decoder ran in the interpreter (~3 FPS video).
Lifted at LS 0x200 into port/spu/r2_bink/ (translated game code,
.gitignore'd) and registered as a resident code region keyed on that source
EA (port/spu/r2_bink/register.c). Re-run cmake afterwards.

  tools/prepare_r2_bink.py            # extract and lift
  tools/prepare_r2_bink.py --check    # the lifted image matches the EBOOT
"""
import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import r2_layout  # noqa: E402

VA = 0x00EB6580
SIZE = 0x18010
LS_BASE = 0x200
FINGERPRINT = 0xA775870FC5D6B6F5


def image(eboot):
    data = r2_layout.eboot_bytes(eboot, VA, SIZE, "the Bink SPU decoder")
    if r2_layout.fnv1a64(data) != FINGERPRINT:
        raise SystemExit("the Bink SPU decoder's fingerprint differs: not the tested BCUS98120 EBOOT?")
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="only check the current extraction")
    parser.add_argument("--eboot", type=Path, default=r2_layout.eboot())
    parser.add_argument("--dest", type=Path, default=r2_layout.SPU_DIR / "r2_bink")
    parser.add_argument("--toolkit", type=Path, default=None, help="default: PS3RECOMP_DIR or ./ps3recomp")
    args = parser.parse_args()
    raw = image(args.eboot)
    if args.check:
        r2_layout.check_image(args.dest, raw, "Bink SPU", "prepare_r2_bink.py")
        print(f"R2 Bink SPU: {SIZE} bytes, FNV {FINGERPRINT:016X}, OK")
        return
    n = r2_layout.lift_raw(raw, args.dest, LS_BASE, "r2_bink_", args.toolkit)
    print(f"R2 Bink SPU ready: {SIZE} bytes, FNV {FINGERPRINT:016X}, {n} functions -> {args.dest}")


if __name__ == "__main__":
    main()
