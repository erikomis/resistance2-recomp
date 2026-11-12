#!/usr/bin/env python3
"""Apply the BCUS98120 frontend port-0 compatibility hook after lifting.

The function is the frontend's per-pad input update: it loops over the pads and
skips one whose "enabled" byte is 0. Known builds of the executable:

    01.00 (disc)    func_00764C38, flag through r11, pad counter r23
    01.60 (update)  func_0077DFF4, flag through r25, pad counter r21
"""
from pathlib import Path
import sys

directory = Path(sys.argv[1])
BUILDS = [
    # (file, function, flag register, counter register, loop label, TOC offset)
    ("ppu_recomp_003.cpp", "func_00764C38", 11, 23, "loc_00764C80:", "-0x6A20"),
    ("ppu_recomp_004.cpp", "func_0077DFF4", 25, 21, "loc_0077E044:", "-0x6A28"),
]
marker = "/* R2_FRONTEND_PORT0_COMPAT */"
for name, func, flag, counter, label, toc in BUILDS:
    source = directory / name
    if not source.is_file():
        continue
    text = source.read_text()
    signature = "void %s(ppu_context* ctx) {" % func
    if signature not in text:
        continue
    start = text.index(signature)
    end = text.index("\nvoid func_", start + len(signature))
    body = text[start:end]
    if marker not in body:
        expected = "        ctx->gpr[3] = vm_read8(ctx->gpr[%d] + 0x0);" % flag
        if body.count(expected) != 1 or label not in body or toc not in body:
            raise SystemExit("R2 frontend lift changed; review the port-0 hook before building")
        replacement = expected + "\n" + "\n".join([
            "        " + marker,
            "        /* No local player is registered at the initial save warning.",
            "         * Keep port 0 usable by the frontend while it creates one. */",
            "        if (g_res_frontend_port0 && ctx->gpr[%d] == 0)" % counter,
            "            ctx->gpr[3] = 1;",
        ])
        body = body.replace(expected, replacement, 1)
        declaration = 'extern "C" int g_res_frontend_port0;\n\n'
        source.write_text(text[:start] + declaration + body + text[end:])
        print("Applied R2 frontend port-0 compatibility hook (%s)" % func)
    break
else:
    raise SystemExit("R2 frontend function not found in %s; unknown build of the executable" % directory)
