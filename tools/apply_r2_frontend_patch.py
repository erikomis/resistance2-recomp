#!/usr/bin/env python3
"""Apply the BCUS98120 frontend port-0 compatibility hook after lifting."""
from pathlib import Path
import sys

directory = Path(sys.argv[1])
source = directory / "ppu_recomp_003.cpp"
text = source.read_text()
signature = "void func_00764C38(ppu_context* ctx) {"
marker = "/* R2_FRONTEND_PORT0_COMPAT */"
start = text.index(signature)
end = text.index("\nvoid func_", start + len(signature))
body = text[start:end]
if marker not in body:
    expected = "        ctx->gpr[3] = vm_read8(ctx->gpr[11] + 0x0);"
    if body.count(expected) != 1 or "loc_00764C80:" not in body or "-0x6A20" not in body:
        raise SystemExit("R2 frontend lift changed; review the port-0 hook before building")
    replacement = expected + "\n" + "\n".join([
        "        " + marker,
        "        /* No local player is registered at the initial save warning.",
        "         * Keep port 0 usable by the frontend while it creates one. */",
        "        if (g_res_frontend_port0 && ctx->gpr[23] == 0)",
        "            ctx->gpr[3] = 1;",
    ])
    body = body.replace(expected, replacement, 1)
    declaration = 'extern "C" int g_res_frontend_port0;\n\n'
    source.write_text(text[:start] + declaration + body + text[end:])
    print("Applied R2 frontend port-0 compatibility hook")
