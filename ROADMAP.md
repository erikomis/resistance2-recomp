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

## Next

- [ ] (in progress) Performance in the level: lift the SPU code the level
  copies into variable LS addresses (a relocatable lift, as inFAMOUS 2's
  relocatable modules), and take shader compiles off the frame.
- [ ] The shutdown that needs a kill.
- [ ] Play the first level through by hand, with combat; saves, audio in
  play; the levels after it.
- [ ] `RES_LEVEL=<n>`: confirm whether the EBOOT honours `-level`.
- [ ] Find out why the front end leaves port 0 disabled, and replace the
  hook with the real cause's fix.
- [ ] Take the 60 fps and field-of-view community patches for BCUS98120 into
  `port/patches.cpp`, as in the Resistance: Fall of Man port.
- [ ] The desktop launcher, once the game is playable.
- [ ] Send the generic toolkit fixes upstream (per-instance workload tables,
  the lost-wake fix, overlay retirement, `ESRCH`, NV0039, the FIFO `put`
  re-read, sub-rectangle sampling).
