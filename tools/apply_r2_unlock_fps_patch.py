#!/usr/bin/env python3
"""Apply the code half of the "Unlock FPS" patch to the lifted 01.00 executable.

RPCS3's patch.yml ships "Unlock FPS" for Resistance 2 01.00 (BCUS98120,
PPU-08da3b601b335c5b09e6425b27fe00eacdad6e78, by Whatcookie): two branches of
the campaign's frame limiter become no-ops, and its two minimum timesteps
become 0.0001 s. A static recompilation never executes the guest's code words,
so the branches are patched here, in the lifted code, behind a run-time switch
(g_res_unlock_fps, set by port/patches.cpp from RES_UNLOCK_FPS); the
timesteps are written to guest memory by port/patches.cpp. Off, the lifted
code is the game's own.

The limiter (func_00635D28) sleeps 50 us (sys_timer_usleep) and reads the
timebase until the next frame's time:

    0x00635D7C  beq  cr7, 0x00635D74   (the timebase read 0: read again)
    0x00635D88  blt  0x00635D68         (not yet the frame's time: sleep again)

The 01.60 build (online) is left alone: online play stays at the game's 30 fps.
"""
from pathlib import Path
import sys

directory = Path(sys.argv[1])
NAME, FUNC = "ppu_recomp_003.cpp", "func_00635D28"
marker = "/* R2_UNLOCK_FPS */"
BRANCHES = [
    "        if (((ctx->cr >> 1) & 1)) goto loc_00635D74;",
    "        if (((ctx->cr >> 31) & 1)) goto loc_00635D68;",
]
source = directory / NAME
text = source.read_text() if source.is_file() else ""
signature = "void %s(ppu_context* ctx) {" % FUNC
if signature not in text:
    print("R2 unlock-FPS hook: %s not in %s (not the 01.00 build); skipped" % (FUNC, directory))
    sys.exit(0)
start = text.index(signature)
end = text.index("\nvoid func_", start + len(signature))
body = text[start:end]
if marker in body:
    sys.exit(0)
for b in BRANCHES:
    if body.count(b) != 1:
        raise SystemExit("R2 frame-limiter lift changed; review the unlock-FPS hook before building")
for b in BRANCHES:
    body = body.replace(b, b.replace("if (((", "if (!g_res_unlock_fps && (((").replace(")) goto", "))) goto"), 1)
body = body.replace(signature, signature + "\n        " + marker, 1)
declaration = 'extern "C" int g_res_unlock_fps;\n\n'
source.write_text(text[:start] + declaration + body + text[end:])
print("Applied R2 unlock-FPS hook (%s)" % FUNC)
