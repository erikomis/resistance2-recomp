# Roadmap

🌐 **English** · [Português (BR)](ROADMAP.pt-BR.md)

Status of the native Resistance 2 (`BCUS98120`) port to macOS/arm64 and what
comes next. Same layout as the Resistance: Fall of Man roadmap.
`[x]` = done and verified; `[ ]` = to do; "(in progress)" = being worked on.

## Status at a glance (2026-10-29)

- **Controllable gameplay in the first level:** boot → menus → New Game →
  intro movies at 30 fps → in-engine cinematic → first-person gameplay with
  the HUD in `iceland`; the player moves. The full-screen image is correct
  (no split, no border) and a 440-second run had no GPU stall.
- **About 15–19 fps in play:** relocated SPU code still interpreted,
  synchronous shader compiles.

## Done

- [x] The toolkit: upstream ps3recomp v0.17.0 plus the Resistance: Fall of
  Man toolkit patch, as one patch with the Resistance 2 changes
  (`patches/ps3recomp-a6d76c8-resistance2.patch`).
- [x] EBOOT decrypted, analysed and lifted (about 41,500 functions, 24
  translation units); builds and links on macOS/arm64 with Metal.
- [x] The embedded SPU ELFs lifted by the build; the raw-SPU loader
  (`igInitRawSpu`) cut out of the EBOOT and lifted with them.
- [x] The Wws_Job SPU policy module (raw bytes at EBOOT `0x00CF9680`) lifted
  and registered by fingerprint; the canonical SPURS r4 layout (the
  "WaitTillReady" freeze).
- [x] Input at the auto-save notice: the front end's port-0 hook
  (`tools/apply_r2_frontend_patch.py`, `RES_FRONTEND_PORT0`).
- [x] `cellHttpClientPollConnections` reports "unavailable" instead of a fake
  success, so the game's HTTP thread waits instead of spinning.
- [x] The three raw-SPU programs lifted (`tools/prepare_r2_rawspu.py`): the
  Bink decoder (movies at 30 fps) and the two level programs.
- [x] "New Game" hang: `ESRCH` for a thread that has exited.
- [x] LFQueue lost wake-up on `WAIT_SIGNAL` (intermittent level-load hang).
- [x] Per-instance SPURS workload IDs (two instances; the 17th workload,
  "triangulate", had no ID).
- [x] The 29 WWS job modules lifted (`tools/regen_r2_jobmods.sh`).
- [x] Stale job-module overlays retired when their LS header is overwritten
  (garbage geometry in play).
- [x] Real `sys_lwcond` notifications by default (the post-movie pause from
  ~5 s to ~1 s).
- [x] The first level (`iceland`) played into: cinematic, HUD, the player
  moves (scripted 420-second run).
- [x] The RSX FIFO race: the frame clock / RSX wake-up at 50 µs (was 1 ms)
  and `put` re-read mid-walk ("FIFO JUMP to unmapped IO", the 5-second "GPU
  sit and spin timeout", corrupt geometry); no bad jumps in 440 s.
- [x] The split / duplicated left half: sub-rectangle sampling of the
  2560×704 AA target; the light border in play gone.
- [x] The NV0039 memory-to-memory copy engine on subchannel 1.
- [x] SPU PUTs write their first 128 bytes last (upstream #632, ported).
- [x] The first level played through by hand, with combat and dialogue,
  into the second level (`bay area`); 70-minute sessions without a hang.
- [x] Neon surfaces and missing geometry minutes into a level: the 4096-entry
  pipeline table filled and new pipelines' draws were dropped; now 65536.
- [x] Normal gameplay from ~15 to 28–30 fps: shadow maps and depth
  downsamples on the GPU, attribute-major vertex fetch (upstream #458), a
  vertex cache that keeps and samples its geometry, faster swizzled decode
  (upstream #578).
- [x] Shader hitches: translations cached across sessions and builds, four
  background builds, cached ones built at once (no flash when aiming).
- [x] Saves: `cellSaveDataUserListAutoSave` / `UserListSave` /
  `UserListLoad` implemented (nothing was written before).
- [x] HDD install: the game copies its caches and streamed dialogue on first
  boot (`cellGameDataCheckCreate2` reports a new install).
- [x] Session logs: timestamped lines, the last 10 kept, a per-second
  telemetry line, a 3-second stall report and long-pass breakdowns.
- [x] Pause menu background: CPU-side NV3089 copies into GPU targets now
  reach the GPU (`PS3RECOMP_ENG_GUEST_WRITE`); GPU depth downsample back on.
- [x] Same-frame vertex reuse for SPU-built geometry drawn twice
  (`PS3RECOMP_ENG_VCACHE_FRAMEREUSE`).
- [x] Measured and rejected: one vertex stream per attribute
  (`PS3RECOMP_ENG_PLANAR`), a recycled shadow-target pool.
- [x] The aim flash (a CPU scene copy over a shadow map's memory).
- [x] 3-second stalls entering new areas (Metal compiles on the RSX thread).
- [x] Memory growth (an autorelease pool on the frame-clock thread).
- [x] Shader function table of 65536; stereo output with a peak limiter (no
  more clipping); GPU scene copies; a faster SPU interpreter loop.
- [x] Clean shutdown on window close and SIGTERM; HDD paths for cellFs and
  trophies; no dropped draws past 8192 records.

## Next

- [ ] (in progress) Heavy combat below 30 fps: SPU-built geometry at new
  addresses every frame misses the vertex cache.
- [ ] (in progress) Confirm in play: the Moonpool effects (black squares for
  steam/water, odd sparks, a water-like distortion following the camera)
  after `PS3RECOMP_FP_UNNORM_VOL` and `PS3RECOMP_ENG_GUEST_WRITE`; fog a
  little darker than on PS3.
- [x] Saves across sessions: checkpoints are written, and Continue Game loads
  the last one (`cellSaveDataUserListAutoLoad`).
- [x] Relocatable SPU lifts on by default (`tools/lift_r2_relocmods.py`).
- [x] Bay area shader explosion: the texel-coordinate scale goes in the
  constant buffer (`PS3RECOMP_FP_UNNORM_UNIFORM`).
- [ ] Measure the relocatable lifts on a hot machine (thermal throttling).
- [ ] Validate the ps3recomp v0.19 ports (#670, #693, #635, #666, #659) and
  rebase the toolkit patch on v0.19.
- [ ] The levels after `bay area`.
- [ ] `RES_LEVEL=<n>`: confirm whether the EBOOT honours `-level`.
- [ ] Find out why the front end leaves port 0 disabled, and replace the
  hook with the real cause's fix.
- [ ] A constant 30 fps through the whole game, with no hangs. As of
  2026-10-30 normal gameplay holds 28–30 fps (was 14.5) and frames of 100 ms
  or more are down from 27 to about 3 a minute; heavy combat is what is
  left.
- [ ] Then 60 fps: take the community's frame-rate unlock and field-of-view
  patches for BCUS98120 into `port/patches.cpp` and run the vblank clock at
  120 Hz through `g_res_vblank_hz` (already in `port/patches.cpp`), as the
  Resistance: Fall of Man port does with `RES_UNLOCK_FPS=1`.
- [x] The desktop launcher (`launcher/`, adapted from the Resistance: Fall of
  Man port's), with a one-click install and decryption without RPCS3.
- [ ] Test the one-click install end to end on a clean Mac; a release build
  of the launcher.
- [ ] Send the generic toolkit fixes upstream (per-instance workload tables,
  the lost-wake fix, overlay retirement, `ESRCH`, NV0039, the FIFO `put`
  re-read, sub-rectangle sampling).

## Later

- [ ] Online multiplayer (competitive and the 8-player co-op). Sony closed
  the Resistance servers in April 2014. Community projects are bringing them
  back for real consoles and RPCS3: PS Rewired is reported to run Resistance 2
  co-op and ranked play (reached by changing the console's DNS), and VGFury
  showed a first alpha of a separate restoration in September 2026. The
  Resistance: Fall of Man port already has the groundwork in its toolkit
  patch (2026-11-11): real sockets (`PS3_NET_ONLINE=1`), lookups through a
  chosen DNS (`PS3_NET_DNS`), a locally made PSN sign-in and ticket
  (`PS3_NP_SIGNIN=1`), and with them that game signs in to PS Rewired. For
  this port those changes are in the toolkit patch since 2026-11-11, all off
  by default (offline play is unchanged; their unit tests pass here). PS
  Rewired's RPCS3 guide lists Resistance 2 with DNS `67.222.156.250` or the
  host switch `resistance2.ps3.online.scea.com=67.222.156.250`; that is the
  server name in this game's executable, and `PS3_NET_DNS` / `PS3_NET_HOSTS`
  cover both ways. Next:
  see how far Resistance 2 gets with them, find what it asks for beyond them
  (`cellHttp`/`cellSsl`, `sceNp` matching, its own server protocol), and
  follow the community server's rules. The campaign comes first.
