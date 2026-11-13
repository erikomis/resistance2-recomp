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
- [x] DualSense adaptive triggers, from the Resistance 1 port (2026-11-11):
  R2 clicks like a gun's trigger and fires at the click (about half way),
  buzzes while it is held and the game rumbles; L2 has a light resistance.
  Other controllers keep free triggers. `PS3_PAD_ADAPTIVE=0` turns it off,
  `PS3_PAD_FIRE_AT` moves the press point; a switch in the launcher, and a
  Fire with R2 (PS5 style) button on its Controls page that swaps L1/R1 with
  L2/R2. Not yet felt on a real DualSense.
- [x] MetalFX frame generation, from the inFAMOUS 2 port (2026-11-12,
  `PS3_FRAMEGEN=1`, a launcher switch, off by default, macOS 26+): one
  generated frame between every two the game renders, the HUD put back
  untouched. The camera is the world view-projection in vertex constants
  c[8..11] (the most common one among the frame's 3D draws, so shadow
  cascades and the first-person weapon's own projection are left out); the
  depth is copied where the scene is composited; the weapon (depth under
  0.2) moves with the camera. Headless self-test while walking and turning:
  34–41 dB against 24–28 for a 50/50 blend, motion scale 1 best, a static
  camera exactly zero motion, ~2 ms GPU per generated frame. Wired through
  the window host too (the launcher's Play). Not yet tried in a window by a
  player.

- [x] More of the DualSense (2026-11-13): the light bar in the game's
  orange (or another colour, or off) glowing red with hits and explosions,
  the player lights, and optional gyro aim (turn and tilt the controller
  while aiming, L1 on whatever the controls bind it to, or always). Adaptive triggers did nothing
  on the player's DualSense because Steam (Steam Input) held it and kept
  rewriting its output; with Steam closed they work. The port now clears the
  controller's power-save bits when it opens it and logs a line when Steam
  is running. Gyro aim not yet tried by a player.

- [x] From the other Resistance ports (2026-11-13): the Resistance 3 port's
  RSX-thread work, as the Resistance 1 port took it in (texture lookups
  through a hash index, only changed vertex constants copied, an address
  filter before the surface scans, no legacy state shadow, 2048 texture
  views with LRU eviction, a direct path for plain FIFO words), and its
  timebase-poll sleep for a thread that only reads the clock. Headless A/B
  in the first level, two pairs in opposite order, the game at its 30 fps:
  the RSX thread 450 -> 399 ms busy per second (-11%), the whole process
  276% -> 217% CPU (cooler Mac, more room before throttling). The picture is
  unchanged. Each part has an off switch (README, Environment).
  Also the fixes from Psyykosis's Resistance PC port (MIT) that the
  Resistance 1 port took in: the lock-free `cellSpursReadyCountStore` (8% of
  the main thread in heavy scenes there), the SPU channel wake under the
  waiter's lock and a short spin before a raw SPU sleeps, a same-value
  `stwcx.` keeping the SPUs' reservations, word-aligned SPU indirect
  branches, the vertex program's ARL destination and the alpha test on the
  stored 8-bit alpha. Gameplay checked headless: same picture, no SPU
  timeouts. The Resistance 1 port's BLIT_NEST was not needed: Resistance 2
  declines none of those copies.

- [x] Supersampling (2026-11-13, from the Resistance 1 port): an internal
  resolution above the window's is now reduced into the window with a
  Lanczos filter, every pixel counted, instead of the layer sampling it, so
  it removes jaggies and shimmer; a 5K (4×) option in the launcher.
  `PS3_DOWNSCALE=0` turns it off. Checked with the window host's own test (a
  one-pixel checkerboard comes out an even grey; a white picture stays white
  to its edges); not yet seen in a window here. The sharpening (CAS) now
  runs after the reduction, in the window, instead of before it where the
  reduction averaged it away (`PS3_CAS_AFTER=0` reverts); frame
  generation's real frame carries it too. From the Resistance 1 port's
  audit of the window's present path: a frame whose reduction or MetalFX
  fails is drawn with a filtered copy instead of dropped, GPU failures are
  counted and turn the failing path off, the reduction writes straight into
  the drawable when there is no sharpening (one copy less), a screenshot of
  what is shown (Cmd+Shift+S), and an experimental linear-light reduction.

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
- [ ] RecompLauncher: the profile (`recomplauncher.json`), the launcher's
  headless `--prepare` and `scripts/package-recomplauncher.sh` are in; a
  release with that zip attached, a full prepare on a clean Mac, and the
  listing on recomplauncher.com are still to do (launcher README,
  "RecompLauncher"). Online from RecompLauncher (2026-11-12): the setup
  builds the 01.60 version after the campaign (never failing it), an
  Online tile runs `--play-online` without a window at the game's own
  30 fps, the online ID is set in the launcher's window (the profile has no
  text setting), exit codes 47-51; tested with fake inputs only, a real
  setup and an online start from RecompLauncher are still to do.
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
  cover both ways. First try (2026-11-11): with those switches the disc
  version (01.00) boots, reports itself online, gets its ticket and reaches
  the Cooperative menu, but its own sign-in logic then answers "You've been
  signed out of the PlayStation®Network" and never contacts the server. One
  port bug found on the way is fixed (the game binds UDP 1900 to look for a
  UPnP router; with another program on that port the bind failed). The RPCS3
  netplay list reports this game working on PS Rewired with update 01.60,
  and the Resistance: Fall of Man server only accepted that game's last
  update. So online now runs from a second build, the 01.60 update's,
  keeping 01.00 for the campaign (2026-11-12): `tools/build_online.sh`
  downloads the two official update packages, checks and extracts them and
  builds the 01.60 executable (decrypted by `tools/decrypt_self.py`
  without RPCS3 since 2026-11-12: a free NPDRM licence; the result has the
  known SHA-256), and the launcher's **Play online** runs it in
  its own folder (its own HDD folders: an empty `dev_hdd1`, since the 01.00
  install cache keeps it looping at boot, and the update's data patches,
  which it only mounts when told its HDD install exists) at the game's own
  30 fps, as the server's rules require. The 01.60 build runs, signs in and
  reaches PS Rewired's user agreement as of 2026-11-12. Still untested: the
  lobby, the game list, a match. The campaign comes first.
