"""Where things live, for the Python tools in this folder.

Two layouts are understood:

  this repository  <repo>/port/ (the port), <repo>/ps3recomp/ (the toolkit,
                   tools/setup_toolkit.sh), <repo>/game/BCUS98120/EBOOT.elf
  a toolkit tree   the port in <toolkit>/project/, the EBOOT in
                   <toolkit>/game/EBOOT.elf, these tools in <toolkit>/tools/
                   (how the port was first developed)

Environment overrides: PS3RECOMP_DIR (the toolkit), R2_EBOOT (the decrypted
EBOOT.elf).
"""
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
IN_REPO = (ROOT / "port").is_dir()
PORT = ROOT / ("port" if IN_REPO else "project")
SPU_DIR = PORT / "spu"
BUILD = ROOT / "build"
TITLE_ID = "BCUS98120"


def toolkit():
    env = os.environ.get("PS3RECOMP_DIR")
    if env:
        return Path(env).resolve()
    return ROOT / "ps3recomp" if IN_REPO else ROOT


def eboot():
    env = os.environ.get("R2_EBOOT")
    if env:
        return Path(env).resolve()
    return ROOT / ("game/BCUS98120/EBOOT.elf" if IN_REPO else "game/EBOOT.elf")


def executable():
    return BUILD / TITLE_ID / "ResistanceRecomp"


def disc_root():
    """The folder holding PS3_GAME/: PS3_VFS_ROOT, game/BCUS98120 when the
    disc was extracted there, or the toolkit tree itself (dev layout)."""
    env = os.environ.get("PS3_VFS_ROOT")
    if env:
        return Path(env)
    if IN_REPO:
        return ROOT / "game" / TITLE_ID
    return ROOT


# The builds of the executable the tools know, by the SHA-256 of the decrypted
# EBOOT.elf: where the SPU code sits in each. 01.00 is the disc; 01.60 is the
# last game update (the one the community servers expect).
#   pm       VA of the Wws_Job policy module (the same 12,544 bytes in both)
#   rawspu   name: (VA, bytes, FNV-1a of those bytes)
#   loader   VA of the 320-byte raw-SPU loader (the same bytes in both)
#   data     file offsets of the second PT_LOAD (where the relocatable SPU
#            blobs are looked for)
EBOOT_BUILDS = {
    "408eb55fa3f00fc43823517df28044451d9192e2023219c98ebb6f95e41e89f5": {
        "version": "01.00",
        "pm": 0x00CF9680,
        "rawspu": {"bink":     (0x00EB6580, 0x18010, 0xA775870FC5D6B6F5),
                   "rawspu_a": (0x00E9B400, 0x07120, 0x020CB0B057A2388F),
                   "rawspu_b": (0x00EA2580, 0x13D70, 0x642B471A9CFBBDF1)},
        "loader": 0x00ED2330,
        "data": (0xCF0000, 0xEF12FC),
    },
    "697bd1df98f79ce099e54e0e60e0a1744d3950ea43a2a06978f69a91e51b3d41": {
        "version": "01.60",
        "pm": 0x00D0A180,
        "rawspu": {"bink":     (0x00EC5600, 0x18010, 0xFB1BC0994343AD86),
                   "rawspu_a": (0x00EAA480, 0x07120, 0xE1EB41653B7071E7),
                   "rawspu_b": (0x00EB1600, 0x13D70, 0x4D6CD8F46BF8D72D)},
        "loader": 0x00EE13B0,
        "data": (0xD00000, 0xF004EC),
    },
}
_build_cache = {}


def eboot_build(path):
    """The EBOOT_BUILDS entry for the EBOOT.elf at path."""
    import hashlib
    path = Path(path).resolve()
    if path not in _build_cache:
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest not in EBOOT_BUILDS:
            raise SystemExit(f"{path}: not a known BCUS98120 executable (01.00 or 01.60), sha256 {digest}")
        _build_cache[path] = EBOOT_BUILDS[digest]
    return _build_cache[path]


def fnv1a64(data):
    value = 1469598103934665603
    for byte in data:
        value = ((value ^ byte) * 1099511628211) & 0xFFFFFFFFFFFFFFFF
    return value


def eboot_bytes(path, va, size, what):
    """The size bytes at guest address va of the big-endian ELF64 at path."""
    import struct
    elf = Path(path).read_bytes()
    if elf[:6] != b"\x7fELF\x02\x02":
        raise SystemExit(f"{path}: not a big-endian ELF64 (decrypt EBOOT.BIN first)")
    phoff = struct.unpack_from(">Q", elf, 32)[0]
    phentsize, phnum = struct.unpack_from(">HH", elf, 54)
    for i in range(phnum):
        off = phoff + i * phentsize
        kind = struct.unpack_from(">I", elf, off)[0]
        fileoff, vaddr, _, filesz = struct.unpack_from(">QQQQ", elf, off + 8)
        if kind == 1 and vaddr <= va and va + size <= vaddr + filesz:
            return elf[fileoff + va - vaddr:fileoff + va - vaddr + size]
    raise SystemExit(f"{what} not found in the segments of {path}")


def write_if_changed(path, data):
    path = Path(path)
    if not path.exists() or path.read_bytes() != data:
        path.write_bytes(data)


def lift_raw(raw, dest, base, prefix, tk=None):
    """Lift raw SPU bytes loaded at LS base into dest/spu_recomp.{c,h} with the
    toolkit's find_spu_functions.py and spu_lifter.py. Writes image.bin,
    functions.json and lift.json beside them (all game-derived, .gitignore'd).
    Returns the number of functions."""
    import json
    import subprocess
    import sys
    import tempfile
    tk = Path(tk) if tk else toolkit()
    if not (tk / "tools/spu_lifter.py").is_file():
        raise SystemExit(f"no toolkit at {tk} (run tools/setup_toolkit.sh, or set PS3RECOMP_DIR)")
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    write_if_changed(dest / "image.bin", raw)
    sys.path.insert(0, str(tk / "tools"))
    from find_spu_functions import detect_functions
    functions, _ = detect_functions(raw, base_override=base, verbose=False, raw=True)
    bounds = json.dumps([{"start": a, "end": b} for a, b in functions], indent=2).encode()
    write_if_changed(dest / "functions.json", bounds)
    with tempfile.TemporaryDirectory() as temp:
        out = Path(temp)
        subprocess.run([sys.executable, str(tk / "tools/spu_lifter.py"),
                        str(dest / "image.bin"), "--base", hex(base),
                        "--functions", str(dest / "functions.json"),
                        "--symbol-prefix", prefix, "--return-entries",
                        "--output", str(out), "--report", str(out / "lift.json")], check=True)
        for name in ("spu_recomp.c", "spu_recomp.h", "lift.json"):
            content = (out / name).read_bytes()
            content = content.replace(b'"../../runtime/spu/spu_helpers.h"', b'"spu_helpers.h"')
            content = content.replace(b'"../../runtime/spu/spu_context.h"', b'"spu_context.h"')
            write_if_changed(dest / name, content)
    return len(functions)


def check_image(dest, raw, what, script):
    """--check: the image.bin a previous lift wrote is these bytes."""
    image = Path(dest) / "image.bin"
    if not image.is_file() or image.read_bytes() != raw:
        raise SystemExit(f"{image} is missing or stale: run tools/{script}")
