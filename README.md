# Resistance 2 — static recompilation for PC (macOS)

🌐 **English** · [Português (BR)](README.pt-BR.md)

A native port of **Resistance 2** (PS3, disc `BCUS98120`) to **macOS/arm64**,
built with static recompilation: the game's PowerPC and SPU code is
translated to C/C++ and compiled into a native executable, with graphics on
Metal. It is not an emulator.

**It reaches controllable gameplay in the first level.** The game runs
from the boot and the auto-save notice through the intro movies (at their
30 fps) and the menus to New Game, plays the level's in-engine cinematic and
hands over to first-person gameplay with the HUD in the first level
(`iceland`), with the full-screen image correct: the player moves. It runs
at about 15–19 fps in play; see [Status](#status).

Built on [ps3recomp](https://github.com/sp00nznet/ps3recomp), with the
toolkit, the layout and the tools of the author's
[Resistance: Fall of Man port](https://github.com/erikomis/resistance-recomp).
See [CREDITS.md](CREDITS.md).

> [!WARNING]
> **Early work in progress.** The first level can be entered and walked
> in, but at about 15–19 fps, and nothing past it has been tested; it is not
> a finished way to play the game. There are no releases: you build the game
> yourself from your own disc. Apple-silicon macOS only, with a controller.
> What is missing is in [ROADMAP.md](ROADMAP.md).

> **This repository contains no game data, no firmware, no Sony keys and
> nothing translated from them.** Everyone generates all of that locally from
> their own legitimate copy. A fan project, not affiliated with Sony
> Interactive Entertainment or Insomniac Games.

## Status

As of 2026-10-30: boot → intro movies → menus → New Game → the first level
(`iceland`) played through by hand, with combat, dialogue and checkpoints,
into the second level (`bay area`). Normal gameplay runs at **28–30 fps**
(was 14.5 on 2026-10-29); heavy combat can still drop lower (see known
issues). Sessions of 70+ minutes without a hang.

Since 2026-10-29:

- **Neon surfaces and a world that stopped drawing:** the Metal backend kept
  at most 4096 pipeline states, and Resistance 2 keeps creating them as a
  level goes on (4600+ in a 15-minute session). Once the table was full,
  every new pipeline failed and its draws were dropped for the rest of the
  session, which left flat neon colours and missing geometry minutes into a
  level, worse when aiming. The pipeline tables are now 65536 entries
  (`PS3RECOMP_METAL_MAX_PIPES`, `PS3RECOMP_ENG_MAX_PIPELINES`).
- **Frame rate, 14.5 → ~28 fps:** the RSX thread was CPU-bound (~68 ms a
  frame; the GPU worked ~8 ms). Shadow maps and NV3089 depth downsamples
  stay on the GPU instead of being read back every frame
  (`PS3RECOMP_ENG_ZETA16_GPU`); attribute-major vertex fetch (upstream #458,
  `PS3RECOMP_ENG_FETCH_MANY`); a vertex cache that no longer starts over and
  hashes resident geometry sparsely (`PS3RECOMP_ENG_VCACHE_KEEP2`,
  `PS3RECOMP_ENG_VCACHE_SPARSE`); faster swizzled texture decode (upstream
  #578).
- **Shader hitches:** 99% of a pipeline build was HLSL→MSL translation, so
  translations are cached on disk across sessions and builds
  (`PS3_SHADER_CACHE`), new pipelines build in the background, four at a
  time (`PS3_SHADER_ASYNC`, `PS3_SHADER_ASYNC_JOBS`), and one whose
  translation is already cached builds at once (`PS3_SHADER_ASYNC_SYNC_HIT`),
  which removes the brief flash when aiming.
- **Saves:** the game saves every checkpoint through
  `cellSaveDataUserListAutoSave`, and the menu through `UserListSave` /
  `UserListLoad`; none was registered, so the unresolved-NID default
  returned OK without writing anything. They now run the game's callbacks,
  and saves go to the HDD folder the game lists them from.
- **HDD install:** `cellGameDataCheckCreate2` reported the game-data folder
  as already installed, so the game never copied its caches and streamed
  dialogue to the HDD and every dialogue line failed to open. It now reports
  a new install until the files exist; the game copies ~354 MB on first
  boot (`PS3_GAMEDATA_ISNEW=0/1` forces it).
- **Linear textures with mips** keep the base pitch on every level
  (`PS3RECOMP_TEX_LN_PITCH`); a recursion guard that halted an SPU job now
  unwinds and resumes it.

What it took, with the cause of each:

- **SPURS r4 layout:** the Wws_Job policy module takes the canonical r4
  layout (word 1 = the job list). Putting the list in word 0 made it read
  the list as an audit buffer, write `FFFFFFFF` over it and exit without
  running a job: the "render db spu timeout in WaitTillReady" freeze ~90 s
  in. The port no longer sets `SPURS_PM_R4_WORD0`.
- **`ESRCH` for exited threads:** the save system waits for its "Check
  Thread" to go away by polling `sys_ppu_thread_get_priority` until it gets
  `ESRCH`; the runtime answered OK for exited threads and "New Game" hung.
- **Raw-SPU programs lifted:** the Bink decoder and the two programs the
  levels run every frame are raw SPU bytes (a `DEADDEAD` header, not ELFs)
  that `igInitRawSpu` streams into LS `0x200`. The image scanner never saw
  them and they ran in the interpreter (movies at ~3 fps).
  `tools/prepare_r2_rawspu.py` lifts all three; they register as resident
  code regions by source address. The Wws_Job policy module (also raw
  bytes) is lifted by `tools/prepare_r2_job_pm.py`.
- **LFQueue lost wake-up:** an intermittent hang loading the level, between
  the PPU's push on a lock-free queue and the zlib task's `WAIT_SIGNAL`. The
  task now registers as parked before it re-checks the queue, with fences on
  both sides.
- **Per-instance workload IDs:** the game runs two SPURS instances; the
  runtime numbered workloads with one global counter of 16, so the 17th
  ("triangulate") got no ID and the job API asserted. `cellSpurs.c`'s
  workload tables are now per instance.
- **29 lifted job modules:** the WWS job modules (`0xC0DEC0DE` headers in
  the EBOOT, loaded at LS `0x4000`) are lifted by `tools/regen_r2_jobmods.sh`
  and registered by signature.
- **Stale overlay retirement:** the job manager also loads bodies that are
  not among those 29 at `0x4000`; the runtime kept the previous module
  "resident" and ran its lifted code over the new bytes (garbage geometry in
  play). An overlay is now retired when its header in LS is overwritten
  (A/B switches: `SPU_OVL_SIG_OFF=1`, `SPU_OVL_SIG_ONLY=<ids>`).
- **lwcond events on by default:** real `sys_lwcond` notifications
  (`PS3_LWCOND_EVENTS=1`, now the default in `port/main.cpp`) cut the pause
  after the Insomniac movie from ~5 s to ~1 s.
- **The RSX FIFO race (5 s stalls, exploded geometry, neon colours):** the
  game links a side buffer whose JUMP back sits at `put`, and reuses that
  memory for vertex data right after moving `put` on. The emulated GPU woke
  every 1 ms and read the vertex data where the JUMP had been ("FIFO JUMP to
  unmapped IO", the game's 5-second "GPU sit and spin timeout", state lost in
  the resync). The frame clock now wakes every 50 µs (`port/main.cpp`;
  `PS3_FRAMECLOCK_US` still sets it), and the FIFO reader re-reads `put`
  every 8 words and after each JUMP/CALL/RET. A 440-second run: no bad
  jumps.
- **Split / duplicated left half:** the scene renders into a 2560×704
  (2×1 AA) target and the final pass reads 1280×704 of it with unnormalised
  coordinates, but the draw engine handed over the whole target. It now
  copies a sub-rectangle of the declared size on the GPU. With these
  fixes the light border around the play image is gone too.
- **NV0039 memory-to-memory copies:** the copy engine is implemented on
  subchannel 1, where libgcm binds it; its methods used to land in the 3D
  engine as state.
- **SPU PUT order:** SPU PUT, PUTLLC and PUTLLUC write their first 128 bytes
  last, so a reader polling a header never sees it before the payload (a
  port of upstream ps3recomp
  [#632](https://github.com/sp00nznet/ps3recomp/pull/632)).
- **Input at the first screen:** the front end's input dispatcher read
  port 0 as disabled before a local player existed, so X did nothing at the
  auto-save notice. A narrowly scoped hook in the lifted dispatcher
  (`tools/apply_r2_frontend_patch.py`, applied at configure time; switch:
  `RES_FRONTEND_PORT0=0`) keeps port 0's events flowing. It injects nothing.

Known issues, being worked on:

- **Heavy combat drops to 4–15 fps** for tens of seconds: SPU-built
  geometry alternating between two buffers defeats the vertex cache (being
  tested: `PS3RECOMP_ENG_VCACHE_CHURN`).
- **Effects:** black squares where a steam/water effect should be, odd
  electric sparks, a water-like distortion that follows the camera in some
  places, and the pause menu background (a GPU-side depth downsample was
  read as the blurred scene; `PS3RECOMP_ENG_BLIT_GPU` is off by default
  again while that is verified).
- The fog reads a little darker than on a PS3.
- The SPU code levels copy into variable LS addresses still runs in the
  interpreter (it needs a relocatable lift); it is not the frame limiter.

## How it works

Same pipeline as the Resistance: Fall of Man port:

1. **Decrypt** your disc's `EBOOT.BIN` with RPCS3 into `game/BCUS98120/EBOOT.elf`.
2. **Analyse and lift** it with the toolkit's `ppu_loader.py` and
   `ppu_lifter.py`: about 41,500 PowerPC functions become C++ in
   `recompiled/BCUS98120/` (24 translation units).
3. **SPU code:** the SPU ELFs embedded in the EBOOT are found and lifted by
   the build (`PORT_SPU_INPUTS`). Resistance 2 also keeps SPU code as raw
   bytes, which no scanner finds: the Wws_Job policy module (EBOOT VA
   `0x00CF9680`, LS `0xA00`), the three raw-SPU programs (the Bink decoder
   at `0x00EB6580` and two level programs at `0x00E9B400` and `0x00EA2580`,
   LS `0x200`), the raw-SPU loader `igInitRawSpu` (`0x00ED2330`, LS `0x10`)
   and the 29 WWS job modules. The `tools/prepare_r2_*.py` scripts and
   `tools/regen_r2_jobmods.sh` cut each one out of your EBOOT, check its
   fingerprint against the tested executable and lift it.
4. **Build** with CMake/Ninja against the patched toolkit: the runtime (LV2
   syscalls, HLE firmware libraries, SPURS, the Metal RSX backend, FFmpeg for
   video) plus the lifted code, into `build/BCUS98120/ResistanceRecomp`, and
   the small `Resistance` program that owns the window.

The toolkit is upstream ps3recomp v0.17.0 (`a6d76c8`) plus one patch,
[`patches/ps3recomp-a6d76c8-resistance2.patch`](patches/): the whole
Resistance: Fall of Man toolkit patch plus the Resistance 2 changes (the
SPURS lost-wake fix, per-instance SPURS workload tables, stale SPU overlay
retirement, the NV0039 copy engine, `put` re-read in the RSX FIFO reader,
sub-rectangle sampling of a larger colour target, SPU PUTs with their head
written last, `ESRCH` for exited threads, real `sys_lwcond` notifications, a
timebase anchored at first read, and input logging with `PS3_PAD_LOG=1`).

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
| `spu` | `prepare_r2_job_pm.py`, `prepare_r2_rawspu.py`, `regen_r2_jobmods.sh` → `port/spu/`; `prepare_r2_rawspu_loader.py` → `game/BCUS98120/spu_images/` |
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
that); the last 10 session logs move to `logs/`, and every log line starts
with the seconds since launch. The window title shows the frame rate. Close
the window, or press Ctrl+C in the terminal, to quit. The first boot copies
~354 MB of game data to the HDD folder, as the PS3 does.

### Environment

| Variable | Effect |
| --- | --- |
| `PS3_VFS_ROOT` | the folder holding `PS3_GAME/` |
| `RES_DATA` | saves, HDD folders and the log |
| `RES_FRONTEND_PORT0=0` | turns off the front-end input hook (for comparison) |
| `RES_HTTP_POLL_COMPAT=0` | `cellHttpClientPollConnections` back to the toolkit's default (it now reports "unavailable" so the game's HTTP thread does not spin) |
| `RES_LEVEL=<n>` | passes `-level <n>` to the game; not confirmed to work yet |
| `PS3_PAD_LOG=0` | turns off the controller button log (on in `play.sh`) |
| `PS3_LWCOND_EVENTS=0` | `sys_lwcond` back to sleep-and-retry polling (real notifications are the default) |
| `SPURS_LFQ_NOWAKE=1` | turns off the SPURS lost-wake fix (for comparison) |
| `SPU_OVL_SIG_OFF=1` / `SPU_OVL_SIG_ONLY=<ids>` | run the job modules interpreted / only these lifted (bisection) |
| `PS3_FRAMECLOCK_US=<µs>` | the frame clock / RSX wake-up period (default 50; 1000 brings back the FIFO race) |
| `GCM_PUT_LIVE=0` | the FIFO reader stops re-reading `put` mid-walk |
| `GCM_SUBCH1_M2MF=0` | subchannel 1 back to the 3D engine instead of the NV0039 copy engine |
| `GCM_FIFO_STRADDLE=1` | opt-in guard for a FIFO walk that straddles `put` (diagnostic) |
| `PS3RECOMP_ENG_SUBRECT=0` | turns off sub-rectangle sampling of a larger colour target (brings back the split image) |
| `PS3_TELEMETRY=0` | turns off the per-second `[tele]` line (frames, draws) and the 3-second stall report (on in `play.sh`) |
| `RSX_DRAIN_SPIKE_MS=<ms>` | reports every FIFO pass at least this long with what it was spent on (100 in `play.sh`) |
| `PS3_SHADER_CACHE=<dir>` / `PS3_SHADER_ASYNC=0` | the shader translation cache (in `RES_DATA/shadercache`) / background pipeline builds (both on in `play.sh`) |
| `PS3_SHADER_ASYNC_JOBS=<n>`, `PS3_SHADER_ASYNC_SYNC_HIT=0/1/2` | parallel background builds (4); build at once when the translation is cached (2) |
| `PS3RECOMP_METAL_MAX_PIPES=<n>`, `PS3RECOMP_ENG_MAX_PIPELINES=<n>` | pipeline table sizes (65536; 4096/8192 bring back the neon) |
| `PS3RECOMP_ENG_ZETA16_GPU=0`, `PS3RECOMP_ENG_FETCH_MANY=0`, `PS3RECOMP_ENG_VCACHE_SPARSE=0` | turn off the performance changes above, one at a time |
| `PS3RECOMP_ENG_BLIT_GPU=1`, `PS3RECOMP_ENG_VCACHE_CHURN=<frames>`, `PS3RECOMP_ENG_PLANAR=1` | being tested: GPU depth downsample, resting churning vertex buffers, one vertex stream per attribute |
| `PS3_GAMEDATA_ISNEW=0/1` | forces what the game is told about its HDD install |

### Diagnostics

`tools/r2_input_probe.py` runs the build headless on a throwaway copy of
your HDD folders, presses buttons from a script (`--buttons`), saves frame
dumps every five seconds and writes `probe.json` and `summary.json` in
`build/r2-probe-<label>-*/` (`--help` lists the probes: SPU dumps, CPU
sampling, SPURS statistics, memory watches; `--env KEY=VALUE` adds any
variable). Into the first level and walking, as tested:

```sh
B="12:0x4000,20:0x4000"; for t in $(seq 30 8 110); do B="$B,$t:0x4008"; done
python3 tools/r2_input_probe.py --label level --buttons "$B" --seconds 320
```

`tools/compare_r2_logs.py` lines a probe log up against an RPCS3 log of the
same title.

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
| [`port/`](port/) | The port: `main.cpp` (boot, guest thread), `stubs.cpp`, `patches.cpp` (the front-end input hook switch, `cellHttpClientPollConnections`), `CMakeLists.txt` (with the `Resistance` window target), `config.toml`, `spu/` (`spu_workloads.c` and the policy module's `register.c`; the lifted code of the raw SPU modules and job modules, and the raw-SPU programs' `register.c`, are generated here and never committed) |
| [`patches/`](patches/) | The toolkit patch over ps3recomp v0.17.0 |
| [`tools/`](tools/) | `setup_toolkit.sh`, `rebuild.sh`, `play.sh`, `env.sh`, `prepare_r2_job_pm.py`, `prepare_r2_rawspu.py`, `prepare_r2_rawspu_loader.py`, `regen_r2_jobmods.sh`, `extract_jobmods_elf.py`, `apply_r2_frontend_patch.py`, `r2_input_probe.py`, `compare_r2_logs.py`, `export_toolkit_patch.sh`, `r2_layout.py` |
| [`ROADMAP.md`](ROADMAP.md) | Status and next steps |

## License

Code in this repository: MIT ([LICENSE](LICENSE)). The toolkit is MIT by its
authors. Resistance 2 © Sony Interactive Entertainment LLC, developed by
Insomniac Games; this project is not affiliated with Sony or Insomniac.
