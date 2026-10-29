#!/usr/bin/env python3
"""Snapshot and compare an existing BCUS98120 RPCS3 run with the R2 probe."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import shutil

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--rpcs3-log", type=Path,
                    default=Path.home() / "Library/Caches/rpcs3/RPCS3.log")
parser.add_argument("--recomp-log", type=Path,
                    default=root / "build/r2-probe-patched/boot.log")
parser.add_argument("--out", type=Path,
                    default=root / "analysis/BCUS98120/rpcs3-reference")
args = parser.parse_args()
reference = args.rpcs3_log.read_text(errors="replace")
recomp = args.recomp_log.read_text(errors="replace")
if "Serial: BCUS98120" not in reference or "BCUS98120" not in recomp:
    raise SystemExit("Both logs must identify Resistance 2 BCUS98120")
args.out.mkdir(parents=True, exist_ok=True)
shutil.copyfile(args.rpcs3_log, args.out / "RPCS3.reference.log")
shutil.copyfile(args.recomp_log, args.out / "recomp.reference.log")
raw_blocks = Counter(re.findall(r"RawSPU\[0x([0-9a-f]+)\].*Building function", reference))
raw_addresses = sorted({int(value, 16) for value in re.findall(
    r"RawSPU\[0x[0-9a-f]+\].*Building function 0x([0-9a-f]+)", reference)})
movie = "c00_insom_fmv/c00_insom_fmv.bik"
summary = {
    "title_id": "BCUS98120",
    "rpcs3_source": str(args.rpcs3_log),
    "recomp_source": str(args.recomp_log),
    "rpcs3_sha256": hashlib.sha256(args.rpcs3_log.read_bytes()).hexdigest(),
    "rpcs3_version": reference.splitlines()[0].lstrip("\ufeff"),
    "rpcs3_log_start": re.search(r"Current Time: (.*)", reference).group(1),
    "rpcs3_title_initializations": reference.count("SYS: Title: Resistance"),
    "raw_spu_compile_events": dict(raw_blocks),
    "raw_spu_unique_block_addresses": [f"0x{address:05X}" for address in raw_addresses],
    "reference_movie_open_lines": [n for n, line in enumerate(reference.splitlines(), 1)
                                   if movie in line and "sys_fs_open(path=" in line],
    "recomp_movie_open_lines": [n for n, line in enumerate(recomp.splitlines(), 1)
                               if movie in line and "open OK" in line],
    "reference_fixed_callback_minus4": "funcFixed returned result=-4" in reference,
    "recomp_fixed_callback_minus4": "funcFixed cb=-4" in recomp,
    "reference_has_late_rsx_failure": "Invalid RSX method" in reference,
}
(args.out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")

def excerpts(text, patterns, context):
    lines = text.splitlines()
    selected = set()
    for pattern in patterns:
        hit = next((n for n, line in enumerate(lines) if pattern in line), None)
        if hit is not None:
            selected.update(range(max(0, hit - 2), min(len(lines), hit + context)))
    return "\n".join(f"{n + 1}: {lines[n]}" for n in sorted(selected)) + "\n"

(args.out / "RPCS3.excerpts.txt").write_text(excerpts(reference, [
    "sys_raw_spu_create(id=", "funcFixed returned result=-4", movie,
    "file='GAME.SAV'", "Invalid RSX method",
], 24))
(args.out / "recomp.excerpts.txt").write_text(excerpts(recomp, [
    "proxy-loaded LS", "funcFixed cb=-4", movie,
    "REJECTED malformed transfer", "[R2] frontend",
], 12))
print("Reference preserved:", args.out)
print("RawSPU compile events:", dict(raw_blocks))
print("Both logs open the intro movie:", bool(summary["reference_movie_open_lines"] and
                                              summary["recomp_movie_open_lines"]))
