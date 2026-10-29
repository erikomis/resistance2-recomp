#!/usr/bin/env python3
"""Capture R2's input path using disposable copies of the user's HDDs.

A headless diagnostic run: copies the HDD folders tools/play.sh uses (RES_DATA,
default ~/Library/Application Support/resistance-recomp-2) into a fresh
build/r2-probe-<label>-*/ folder, runs the build with scripted pad input
(PAD_SCRIPT), saves frame dumps every five seconds and writes probe.json and
summary.json there. Your own saves are never touched. Needs the build
(tools/rebuild.sh) and the disc (PS3_VFS_ROOT, see tools/r2_layout.py).
"""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time
import tempfile
from statistics import median
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import r2_layout  # noqa: E402

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--always-change", action="store_true")
parser.add_argument("--buttons", default="12:0x4000,20:0x4000")
parser.add_argument("--watch", action="store_true")
parser.add_argument("--watch-ui", action="store_true")
parser.add_argument("--enable-ui", action="store_true",
                    help="Diagnostic only: force the UI port-0 enable read")
parser.add_argument("--compat-off", action="store_true")
parser.add_argument("--label", choices=("normal", "always", "patched", "control", "perf-poll", "perf-events", "perf-fixed", "perf-fixed-events", "perf-spin", "perf-freeze", "perf-pm", "perf-hang", "perf-clock", "perf-ready", "perf-fresh", "perf-sched", "perf-video", "perf-quiet", "level"))
parser.add_argument("--lwcond", choices=("poll", "events"))
parser.add_argument("--sample", action="store_true", help="Capture a 3-second macOS CPU profile at 35 seconds")
parser.add_argument("--short-sleep-spin", type=int, default=0, help="Use the runtime's precise short-wait path up to this many microseconds")
parser.add_argument("--seconds", type=int, default=45, help="Capture duration, rounded down to a multiple of five seconds")
parser.add_argument("--sample-at", type=int, default=35, help="When to collect the optional CPU profile")
parser.add_argument("--poll-stats", action="store_true", help="Log syscall sleep sites and the Bink reply queue")
parser.add_argument("--pm-stats", action="store_true", help="Log SPURS policy-module scheduling")
parser.add_argument("--watch-ready", action="store_true", help="Watch PPU stores to the render-ready byte found in this title")
parser.add_argument("--pm-fresh", action="store_true", help="Restart the SPURS policy context on every dispatch (diagnostic)")
parser.add_argument("--rpcs3-sched", action="store_true", help="Use the SPURS workload selection modeled after RPCS3")
parser.add_argument("--interp-stats", action="store_true", help="Log the busiest SPU interpreter pages every 5 seconds")
parser.add_argument("--env", action="append", default=[], metavar="KEY=VALUE",
                    help="Extra diagnostic environment for this run (repeatable)")
parser.add_argument("--dump-spu", action="store_true", help="Capture dynamically loaded SPU bodies for offline analysis")
args = parser.parse_args()
if args.seconds < 5 or args.sample_at < 5 or args.sample_at % 5:
    parser.error("duration must be at least 5 seconds; sample-at must be a positive multiple of five")
duration = args.seconds // 5 * 5
label = args.label or ("always" if args.always_change else "normal")
# Each run starts from the same source HDDs, without old frames, gates or
# guest-written caches from a previous experiment carrying over.
r2_layout.BUILD.mkdir(exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="r2-probe-" + label + "-", dir=r2_layout.BUILD))
source = Path(os.environ.get("RES_DATA") or
              Path.home() / "Library/Application Support/resistance-recomp-2")
executable = r2_layout.executable()
eboot = r2_layout.eboot()
for name in ("dev_hdd0", "dev_hdd1"):
    if (source / name).is_dir():
        shutil.copytree(source / name, work / name, dirs_exist_ok=True)
    else:
        (work / name).mkdir(exist_ok=True)
env = os.environ.copy()
for name in ("PAD_ALWAYS_CHANGE", "PAD_AUTOPRESS", "PAD_FILE", "YDKJ_INJECT_PAD",
             "PPU_FORCE_READ_ADDR", "PPU_FORCE_READ_VAL", "PPU_RWATCH",
             "PPU_WWATCH", "PPU_RWATCH_NONZERO", "RES_FRONTEND_PORT0", "PS3_LWCOND_EVENTS",
             "PAD_SCRIPT", "SPURS_PM_FRESH", "SPURS_RPCS3_SCHED", "SPU_INTERP_STATS",
             "SPURS_STATS", "SPURS_PM_TRACE", "PS3_POLLTOP", "PS3_WAIT_OBJ",
             "SPU_DUMP_OVL", "SPU_RAW_DUMP_DIR", "SPU_DUMP_MISS"):
    env.pop(name, None)
env.update(
    PS3_VFS_ROOT=str(r2_layout.disc_root()), PS3_HDD0_ROOT=str(work / "dev_hdd0"),
    PS3_HDD1_ROOT=str(work / "dev_hdd1"), PS3RECOMP_METAL_HEADLESS="1",
    PS3RECOMP_METAL_FRAME_DUMP=str(work / "frame.ppm"),
    PAD_SCRIPT=args.buttons, PS3_PAD_SDL="0", PS3_PAD_LOG="1",
    SDL_AUDIODRIVER="dummy", BCTRL_RING=str(work / "calls.log"),
    PS3_ICALL_GATE=str(work / "icall.gate"),
    PS3RECOMP_FRAME_STATS="1",
    PS3_USLEEP_SPIN_US=str(max(0, args.short_sleep_spin)),
)
if args.watch:
    env.update(PPU_WWATCH="012E0A00", PPU_WWATCH_LEN="16", PPU_WWATCH_MAX="5000",
               PPU_RWATCH="012E0A00,16", PPU_RWATCH_NONZERO="1")
if args.watch_ui:
    env.update(PPU_WWATCH="013805A8", PPU_WWATCH_LEN="80", PPU_WWATCH_MAX="2000",
               PPU_RWATCH="013805A8,80", PPU_RWATCH_NONZERO="0")
if args.enable_ui:
    env.update(PPU_FORCE_READ_ADDR="013805A8", PPU_FORCE_READ_VAL="1")
if args.compat_off:
    env["RES_FRONTEND_PORT0"] = "0"
if args.lwcond:
    env["PS3_LWCOND_EVENTS"] = "1" if args.lwcond == "events" else "0"
if args.poll_stats:
    env.update(PS3_POLLTOP="5", PS3_WAIT_OBJ="00B20AD4")
if args.pm_stats:
    env.update(SPURS_STATS="5", SPURS_PM_TRACE="1")
if args.watch_ready:
    env.update(PPU_WWATCH="012B7F00", PPU_WWATCH_LEN="1", PPU_WWATCH_MAX="2000")
if args.pm_fresh:
    env["SPURS_PM_FRESH"] = "1"
if args.rpcs3_sched:
    env["SPURS_RPCS3_SCHED"] = "1"
if args.interp_stats:
    env["SPU_INTERP_STATS"] = "5"
if args.always_change:
    env["PAD_ALWAYS_CHANGE"] = "1"
if args.dump_spu:
    for setting, directory in (("SPU_DUMP_OVL", "overlays"),
                               ("SPU_RAW_DUMP_DIR", "raw-spu"),
                               ("SPU_DUMP_MISS", "missing-spu")):
        target = work / directory
        target.mkdir()
        env[setting] = str(target)
for item in args.env:
    key, _, value = item.partition("=")
    env[key] = value
(work / "probe.json").write_text(json.dumps({
    "options": vars(args), "duration_seconds": duration,
    "executable": str(executable),
    "source_hdd": str(source),
    "environment": {key: value for key, value in env.items()
                    if key.startswith(("PS3", "SPU", "RES_", "PAD_", "PPU_"))},
}, indent=2) + "\n")
with (work / "boot.log").open("w") as log:
    process = subprocess.Popen(
        [str(executable), str(eboot)],
        cwd=work, env=env, stdout=log, stderr=subprocess.STDOUT,
    )
    print("Probe:", label, "pid:", process.pid, "artifacts:", work, flush=True)
    try:
        for elapsed in range(5, duration + 1, 5):
            time.sleep(5)
            if "Metal backend init FAILED" in (work / "boot.log").read_text(errors="replace"):
                print("Metal initialization failed; stopping invalid graphics probe", flush=True)
                break
            if process.poll() is not None:
                print("Guest exited early at probe second", elapsed, flush=True)
                break
            if elapsed == 10:
                (work / "icall.gate").touch()
            elif elapsed == 35:
                (work / "icall.gate").unlink(missing_ok=True)
            if elapsed == args.sample_at and args.sample and process.poll() is None:
                subprocess.run(["sample", str(process.pid), "3", "-file", str(work / "sample.txt")],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=10)
            if (work / "frame.ppm").exists():
                shutil.copyfile(work / "frame.ppm", work / f"frame-{elapsed:02d}.ppm")
                if elapsed in (10, duration):
                    shutil.copyfile(work / "frame.ppm", work / ("before.ppm" if elapsed == 10 else "after.ppm"))
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
    print("Guest exit (probe requests shutdown):", process.returncode, flush=True)

log_text = (work / "boot.log").read_text(errors="replace")
frames = [(float(t), float(fps), int(flips)) for t, fps, flips in re.findall(
    r"\[frame-stats\] t=([\d.]+) guest=([\d.]+) flips=(\d+)", log_text)]
last_change = None
previous = 0
for timestamp, _, flips in frames:
    if flips != previous:
        last_change = timestamp
    previous = flips
summary = {
    "metal_failed": "Metal backend init FAILED" in log_text,
    "movie_opened": bool(re.search(r"open OK.*c00_insom_fmv\.bik", log_text)),
    "render_timeout": "render db spu timeout in WaitTillReady" in log_text,
    "frame_samples": len(frames),
    "last_flips": frames[-1][2] if frames else 0,
    "last_flip_change_seconds": last_change,
    "seconds_without_new_flip": round(frames[-1][0] - last_change, 2)
        if frames and last_change is not None else None,
    "median_guest_fps_32_to_45_seconds": median(
        [fps for t, fps, _ in frames if 32 <= t <= 45])
        if any(32 <= t <= 45 for t, _, _ in frames) else None,
    "guest_returncode": process.returncode,
    "spu_dump_files": [str(p.relative_to(work)) for folder in
        ("overlays", "raw-spu", "missing-spu") for p in sorted((work / folder).glob("*.bin"))],
}
(work / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, indent=2), flush=True)
if summary["metal_failed"]:
    raise SystemExit(2)
