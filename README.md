# Resistance 2 — static recompilation for PC (macOS)

🌐 **English** · [Português (BR)](README.pt-BR.md)

A native port of **Resistance 2** (PS3, disc `BCUS98120`) to **macOS/arm64**,
built with static recompilation: the game's PowerPC and SPU code is
translated to C/C++ and compiled into a native executable, with graphics on
Metal. It is not an emulator.

**It boots and reaches the first level.** The game runs from the boot and the
auto-save notice through the intro movies and the menus to a new campaign,
and the first level renders in the engine. The Bink movies play at their
30 fps now that the raw-SPU Bink decoder is lifted instead of interpreted.
Inside the level it is still far too slow to play (about 5 fps) and the view
has a white border; see [Status](#status).

Built on [ps3recomp](https://github.com/sp00nznet/ps3recomp), with the
toolkit, the layout and the tools of the author's
[Resistance: Fall of Man port](https://github.com/erikomis/resistance-recomp).
See [CREDITS.md](CREDITS.md).

> [!WARNING]
> **Early work in progress.** Not playable yet: the campaign starts, but the
> first level runs at about 5 fps. There are no releases: you build the game
> yourself from your own disc. Apple-silicon macOS only, with a controller.
> What is missing is in [ROADMAP.md](ROADMAP.md).

> **This repository contains no game data, no firmware, no Sony keys and
> nothing translated from them.** Everyone generates all of that locally from
> their own legitimate copy. A fan project, not affiliated with Sony
> Interactive Entertainment or Insomniac Games.

## Status

As of 2026-10-29:

- **Boot and menus:** Metal, audio and the controller come up; the auto-save
  notice, the intro movies and the menus run to **a new campaign**.
- **Input at the first screen:** the front end's input dispatcher read
  port 0 as disabled before a local player existed, so X did nothing at the
  auto-save notice. A narrowly scoped hook in the lifted dispatcher
  (`tools/apply_r2_frontend_patch.py`, applied at configure time; switch:
  `RES_FRONTEND_PORT0=0`) keeps port 0's events flowing. It injects nothing.
- **Movies at 30 fps:** the Bink decoder runs on a raw SPU and is streamed
  in as raw bytes (not an ELF), so the image scanner never saw it and it ran
  in the SPU interpreter at ~3 fps. It is now cut out of the EBOOT and lifted
  (`tools/prepare_r2_bink.py`), with the 320-byte raw-SPU loader
  (`tools/prepare_r2_rawspu.py`).
- **The job queue runs:** the Wws_Job SPU policy module is raw bytes in the
  EBOOT too; it is lifted (`tools/prepare_r2_job_pm.py`) and registered by
  fingerprint. A lost wake-up in the SPURS task `WAIT_SIGNAL` path (a job
  pushed while the task was awake was never seen) froze the level load; the
  toolkit now treats a wait on a lock-free queue that already holds entries
  as satisfied.
- **New Game:** the game polls its save "check thread" until LV2 reports it
  gone (`ESRCH`); the runtime reported it alive for ever and "New Game"
  hung. Fixed in `sys_ppu_thread_get_priority`.
- **First level in the engine:** the campaign loads and renders the first
  level.

Known issues, being worked on:

- **About 5 fps in the level.** The WWS SPU job modules (29 `0xC0DEC0DE`
  modules in the EBOOT) still run in the SPU interpreter. Lifting them
  (`tools/regen_r2_jobmods.sh`) is in progress: some report unsupported
  instructions and they still need checking one by one.
- **Viewport:** a white border around the view inside the level.
- **Intermittent problems** (occasional stalls, the shutdown needing a
  kill) under investigation.
- Saves, audio during play, and anything past the first level are untested.

## How it works

Same pipeline as the Resistance: Fall of Man port:

1. **Decrypt** your disc's `EBOOT.BIN` with RPCS3 into `game/BCUS98120/EBOOT.elf`.
2. **Analyse and lift** it with the toolkit's `ppu_loader.py` and
   `ppu_lifter.py`: about 41,500 PowerPC functions become C++ in
   `recompiled/BCUS98120/` (24 translation units).
3. **SPU code:** the SPU ELFs embedded in the EBOOT are found and lifted by
   the build (`PORT_SPU_INPUTS`). Resistance 2 also keeps SPU code as raw
   bytes, which no scanner finds: the Wws_Job policy module (EBOOT VA
   `0x00CF9680`, LS `0xA00`), the raw-SPU Bink decoder (`0x00EB6580`,
   LS `0x200`), the raw-SPU loader `igInitRawSpu` (`0x00ED2330`, LS `0x10`)
   and the WWS job modules. The `tools/prepare_r2_*.py` scripts and
   `tools/regen_r2_jobmods.sh` cut each one out of your EBOOT, check its
   fingerprint against the tested executable and lift it.
4. **Build** with CMake/Ninja against the patched toolkit: the runtime (LV2
   syscalls, HLE firmware libraries, SPURS, the Metal RSX backend, FFmpeg for
   video) plus the lifted code, into `build/BCUS98120/ResistanceRecomp`, and
   the small `Resistance` program that owns the window.

The toolkit is upstream ps3recomp v0.17.0 (`a6d76c8`) plus one patch,
[`patches/ps3recomp-a6d76c8-resistance2.patch`](patches/): the whole
Resistance: Fall of Man toolkit patch plus the Resistance 2 changes (the
SPURS lost-wake fix, `ESRCH` for exited threads, optional real `sys_lwcond`
notifications with `PS3_LWCOND_EVENTS=1`, a timebase anchored at first read,
and input logging with `PS3_PAD_LOG=1`).

## Building from source

### What you need

- An Apple-silicon Mac (macOS with Metal).
- Your own **Resistance 2 disc, `BCUS98120`**, dumped to an image and
  mounted, or extracted to `game/BCUS98120/` (the folder that holds
  `PS3_GAME/`).
- [RPCS3](https://rpcs3.net) to decrypt `EBOOT.BIN` (if decryption fails,
  install the PS3 firmware in RPCS3 first). An ELF you decrypted some other
  way works too: put it at `game/BCUS98120/EBOOT.elf`.
- Xcode Command Line Tools and Homebrew:

```sh
brew install cmake ninja sdl2 glslang spirv-cross spirv-tools ffmpeg python@3.12
```

### Building

```sh
git clone https://github.com/erikomis/resistance2-recomp.git
cd resistance2-recomp
tools/rebuild.sh            # toolkit, decrypt, analyse, lift, spu, build
```

`tools/rebuild.sh` runs these steps (run any of them alone by name, e.g.
`tools/rebuild.sh spu build`):

| Step | What it does |
| --- | --- |
| `toolkit` | `tools/setup_toolkit.sh`: clones ps3recomp to `./ps3recomp`, checks out `a6d76c8`, applies the patch |
| `decrypt` | copies `EBOOT.BIN` and `PARAM.SFO` from the disc to `game/BCUS98120/` and decrypts the EBOOT with RPCS3 (`RPCS3=<path>` if it is not in `/Applications`) |
| `analyse` | `ppu_loader.py` → `analysis/BCUS98120/` |
| `lift` | `ppu_lifter.py` → `recompiled/BCUS98120/` (24 chunks; set `JOBS` to limit parallelism) |
| `spu` | `prepare_r2_job_pm.py`, `prepare_r2_bink.py`, `regen_r2_jobmods.sh` → `port/spu/`; `prepare_r2_rawspu.py` → `game/BCUS98120/spu_images/` |
| `build` | CMake + Ninja → `build/BCUS98120/ResistanceRecomp` and `build/BCUS98120/Resistance` |

The first build compiles a lot of generated code and takes a while. The
tools need Python 3.10 or newer (`PY=<python>` picks one). The disc is found
through `PS3_VFS_ROOT`, `game/BCUS98120/PS3_GAME`, or a volume under
`/Volumes` with `BCUS98120` in its `PARAM.SFO` (see `tools/env.sh`).

## Playing

Mount your disc image (or keep it extracted in `game/BCUS98120/`), connect a
controller (DualSense, DualShock or Xbox), then:

```sh
tools/play.sh
```

The game opens in a window owned by the `Resistance` launcher. Saves, the
HDD folders and the log (`last.log`) are kept in
`~/Library/Application Support/resistance-recomp-2/` (`RES_DATA` changes
that). The window title shows the frame rate. Close the window, or press
Ctrl+C in the terminal, to quit.

### Environment

| Variable | Effect |
| --- | --- |
| `PS3_VFS_ROOT` | the folder holding `PS3_GAME/` |
| `RES_DATA` | saves, HDD folders and the log |
| `RES_FRONTEND_PORT0=0` | turns off the front-end input hook (for comparison) |
| `RES_HTTP_POLL_COMPAT=0` | `cellHttpClientPollConnections` back to the toolkit's default (it now reports "unavailable" so the game's HTTP thread does not spin) |
| `RES_LEVEL=<n>` | passes `-level <n>` to the game; not confirmed to work yet |
| `PS3_PAD_LOG=0` | turns off the controller button log (on in `play.sh`) |
| `PS3_LWCOND_EVENTS=1` | real `sys_lwcond` notifications instead of polling (experimental) |
| `SPURS_LFQ_NOWAKE=1` | turns off the SPURS lost-wake fix (for comparison) |

### Diagnostics

`tools/r2_input_probe.py` runs the build headless on a throwaway copy of
your HDD folders, presses buttons from a script (`--buttons`), saves frame
dumps every five seconds and writes `probe.json` and `summary.json` in
`build/r2-probe-<label>-*/` (`--help` lists the probes: SPU dumps, CPU
sampling, SPURS statistics, memory watches). `tools/compare_r2_logs.py`
lines a probe log up against an RPCS3 log of the same title.

## Updating the toolkit patch

The port was developed in a plain copy of the toolkit. To turn the toolkit
changes of such a folder (or of `./ps3recomp` after editing it) back into
the patch:

```sh
tools/export_toolkit_patch.sh /path/to/toolkit-folder
```

It diffs the folder against a pristine `a6d76c8` (keeping the current
patch's changes to files the folder lacks), leaves out the game, the lift,
lifted SPU code, logs and this repository's own tools, refuses absolute
home paths, and checks the result applies to a clean `a6d76c8`.

## Layout

| Path | What it is |
| --- | --- |
| [`port/`](port/) | The port: `main.cpp` (boot, guest thread), `stubs.cpp`, `patches.cpp` (the front-end input hook switch, `cellHttpClientPollConnections`), `CMakeLists.txt` (with the `Resistance` window target), `config.toml`, `spu/` (`spu_workloads.c` and the `register.c` of the raw SPU modules; their lifted code is generated here, never committed) |
| [`patches/`](patches/) | The toolkit patch over ps3recomp v0.17.0 |
| [`tools/`](tools/) | `setup_toolkit.sh`, `rebuild.sh`, `play.sh`, `env.sh`, `prepare_r2_job_pm.py`, `prepare_r2_bink.py`, `prepare_r2_rawspu.py`, `regen_r2_jobmods.sh`, `extract_jobmods_elf.py`, `apply_r2_frontend_patch.py`, `r2_input_probe.py`, `compare_r2_logs.py`, `export_toolkit_patch.sh`, `r2_layout.py` |
| [`ROADMAP.md`](ROADMAP.md) | Status and next steps |

## License

Code in this repository: MIT ([LICENSE](LICENSE)). The toolkit is MIT by its
authors. Resistance 2 © Sony Interactive Entertainment LLC, developed by
Insomniac Games; this project is not affiliated with Sony or Insomniac.
