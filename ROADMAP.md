# Roadmap

🌐 **English** · [Português (BR)](ROADMAP.pt-BR.md)

Status of the native Resistance 2 (`BCUS98120`) port to macOS/arm64 and what
comes next. Same layout as the Resistance: Fall of Man roadmap.
`[x]` = done and verified; `[ ]` = to do; "(in progress)" = being worked on.

## Status at a glance (2026-10-29)

- **Boots to the first level:** boot, auto-save notice, intro movies, menus,
  New Game, and the first level rendering in the engine.
- **Movies at 30 fps:** the raw-SPU Bink decoder is lifted (it ran in the
  SPU interpreter at ~3 fps).
- **Not playable yet:** about 5 fps inside the level, a white border around
  the view, and intermittent problems.

## Done

- [x] The toolkit: upstream ps3recomp v0.17.0 plus the Resistance: Fall of
  Man toolkit patch, as one patch with the Resistance 2 changes
  (`patches/ps3recomp-a6d76c8-resistance2.patch`).
- [x] EBOOT decrypted, analysed and lifted (about 41,500 functions, 24
  translation units); builds and links on macOS/arm64 with Metal.
- [x] The embedded SPU ELFs lifted by the build; the raw-SPU loader
  (`igInitRawSpu`) cut out of the EBOOT and lifted with them.
- [x] The Wws_Job SPU policy module (raw bytes at EBOOT `0x00CF9680`) lifted
  and registered by fingerprint.
- [x] Input at the auto-save notice: the front end's port-0 hook
  (`tools/apply_r2_frontend_patch.py`, `RES_FRONTEND_PORT0`).
- [x] `cellHttpClientPollConnections` reports "unavailable" instead of a fake
  success, so the game's HTTP thread waits instead of spinning.
- [x] The raw-SPU Bink decoder lifted: movies at 30 fps.
- [x] SPURS lost wake-up on `WAIT_SIGNAL` (level-load freeze) fixed.
- [x] "New Game" hang: `ESRCH` for a thread that has exited.
- [x] First level renders in the engine.

## Next

- [ ] (in progress) Lift the 29 WWS job modules (`tools/regen_r2_jobmods.sh`),
  check the ones with unsupported instructions, and measure the level's
  frame rate (about 5 fps now).
- [ ] The white border around the view in the level (viewport / scissor /
  surface size).
- [ ] The intermittent stalls and the shutdown that needs a kill.
- [ ] Play the first level through with a controller; saves, audio in play.
- [ ] `RES_LEVEL=<n>`: confirm whether the EBOOT honours `-level`.
- [ ] Find out why the front end leaves port 0 disabled, and replace the
  hook with the real cause's fix.
- [ ] Take the 60 fps and field-of-view community patches for BCUS98120 into
  `port/patches.cpp`, as in the Resistance: Fall of Man port.
- [ ] The desktop launcher, once the game is playable.
- [ ] Send the generic toolkit fixes upstream.
