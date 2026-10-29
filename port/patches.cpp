/*
 * Resistance 2 port compatibility patches.
 *
 * Resistance 1's port carried the community's FPS/FOV patches here
 * (patches.cpp defining res_apply_patches and g_res_vblank_hz, which
 * main.cpp's frame clock reads). Resistance 2 starts with no patches; if the
 * community's RPCS3 patch.yml entries for BCUS98120 are wanted, they go in
 * this file in the same two-half shape: the data half written into guest
 * memory here, the code half lifted in both forms via
 * ppu_lifter.py --insn-patches.
 */

#include <cstdio>
#include <cstdlib>
#include "ppu_recomp.h"

extern "C" void ps3_hle_register_ctx(uint32_t, const char*, void (*)(ppu_context*));

/* This import is otherwise unresolved and returns fake success without a
 * transaction ID. R2's HTTP service then polls it at full CPU speed. Until
 * asynchronous HTTP polling is implemented, report unavailable so the game's
 * existing retry/error path sleeps. No guest output is valid on failure. */
static void r2_http_poll_unavailable(ppu_context* ctx)
{
    ctx->gpr[3] = (uint64_t)(int64_t)(int32_t)0x80710026u;
}

/* BCUS98120: the frontend's enabled-port list is empty before its first local
 * player is created. The generated input dispatcher still needs to deliver
 * port 0's real key events at the initial save warning. This does not inject
 * buttons or advance the UI: it enables that port's event path only.
 * tools/apply_r2_frontend_patch.py installs the hook after a fresh lift.
 * RES_FRONTEND_PORT0=0 restores the original dispatcher for comparison. */
extern "C" { int g_res_frontend_port0 = 1; }

/* main.cpp's frame clock reads this: 0 = the runtime's default vblank rate
 * (62.5 Hz), RES_UNLOCK_FPS-style patches would set it higher. */
extern "C" double g_res_vblank_hz = 0;

extern "C" void res_apply_patches(void)
{
    // The Wws_Job policy module takes the canonical SPURS r4 layout: word 1 is
    // the job list EA, word 0 an optional audit buffer EA. SPURS_PM_R4_WORD0=1
    // put the list EA in word 0, so the module treated the list as an audit
    // header, wrote FFFFFFFF over its per-SPU rows and exited without running
    // a job ("render db spu timeout in WaitTillReady" ~90 s in). Leave unset.
    const char* http = std::getenv("RES_HTTP_POLL_COMPAT");
    if (!(http && http[0] == '0'))
        ps3_hle_register_ctx(0xADC0A4B2u, "cellHttpClientPollConnections (unavailable)",
                             r2_http_poll_unavailable);
    const char* setting = std::getenv("RES_FRONTEND_PORT0");
    g_res_frontend_port0 = !(setting && setting[0] == '0');
    std::fprintf(stderr, "[R2] frontend port 0 compatibility: %s\n",
                 g_res_frontend_port0 ? "on" : "off");
}
