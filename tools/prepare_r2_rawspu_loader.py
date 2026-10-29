#!/usr/bin/env python3
"""Cut Resistance 2's raw-SPU loader out of EBOOT.elf as an SPU ELF.

Both raw SPUs start with igInitRawSpu, a 320-byte program the PPU writes to
local store 0x10 (entry 0x20) before it streams the real body in (the Bink
decoder and two programs the levels run, prepare_r2_rawspu.py). It is raw
bytes at EBOOT VA 0x00ED2330, not an ELF, so the SPU image scanner does not
find it. This wraps it with the toolkit's wrap_spu_elf.py into

  game/BCUS98120/spu_images/rawspu_E9CE1BC6DD6D3B87_00010.elf

which tools/rebuild.sh and tools/play.sh pass to the build beside EBOOT.elf
(PORT_SPU_INPUTS), so it is lifted and registered with the other SPU images.
The output is game-derived and stays under game/ (.gitignore'd).
"""
import argparse
from pathlib import Path
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parent))
import r2_layout  # noqa: E402

VA = 0x00ED2330
SIZE = 320
LS_BASE = 0x10
ENTRY = 0x20
FINGERPRINT = 0xE9CE1BC6DD6D3B87
NAME = f"rawspu_{FINGERPRINT:016X}_{LS_BASE:05X}.elf"


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--eboot", type=Path, default=r2_layout.eboot())
    parser.add_argument("--out", type=Path, default=None,
                        help="output folder (default: spu_images/ beside the EBOOT)")
    parser.add_argument("--toolkit", type=Path, default=None, help="default: PS3RECOMP_DIR or ./ps3recomp")
    args = parser.parse_args()
    tk = args.toolkit or r2_layout.toolkit()
    wrap = tk / "tools/wrap_spu_elf.py"
    if not wrap.is_file():
        raise SystemExit(f"no toolkit at {tk} (run tools/setup_toolkit.sh, or set PS3RECOMP_DIR)")
    raw = r2_layout.eboot_bytes(args.eboot, VA, SIZE, "the raw-SPU loader")
    if r2_layout.fnv1a64(raw) != FINGERPRINT:
        raise SystemExit("the raw-SPU loader's fingerprint differs: not the tested BCUS98120 EBOOT?")
    out = args.out or args.eboot.parent / "spu_images"
    out.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as temp:
        binary = Path(temp) / "loader.bin"
        binary.write_bytes(raw)
        subprocess.run([sys.executable, str(wrap), str(binary), "--base", hex(LS_BASE),
                        "--entry", hex(ENTRY), "--out", str(out / NAME)], check=True)
    print(out / NAME)


if __name__ == "__main__":
    main()
