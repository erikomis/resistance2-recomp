/*
 * Resistance 2 port compatibility patches, and the optional game patches.
 *
 *   RES_UNLOCK_FPS=1   campaign (01.00): the frame limiter off and its two
 *                      minimum timesteps (1/30 and 1/75 s) 0.0001 s. The
 *                      community's "Unlock FPS" from RPCS3's patch.yml for
 *                      BCUS98120 01.00 (PPU-08da3b601b335c5b09e6425b27fe00eacdad6e78,
 *                      by Whatcookie). The game then flips once per vblank
 *                      at most: about 60 fps on the frame clock's 62.5 Hz
 *                      (PS3_VBLANK_HZ=120 lets it run past that). Online (the
 *                      01.60 build) is never unlocked: the launcher refuses
 *                      the switch there, and that build has no hook.
 *
 * A patch is two halves. The data half (the timesteps) is written into guest
 * memory here, after the ELF is loaded and before the guest starts. The code
 * half cannot be: a static recompilation never executes the guest's code
 * words, so tools/apply_r2_unlock_fps_patch.py puts the two limiter branches
 * behind g_res_unlock_fps in the lifted code. Each half is applied only when
 * the executable holds the words the patch expects.
 */

#include <cstdio>
#include <cstdlib>
#include <cstring>
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
/* Read by the lifted frame limiter (tools/apply_r2_unlock_fps_patch.py). */
extern "C" { int g_res_unlock_fps = 0; }

/* RES_UNLOCK_FPS: the 01.00 executable's words the patch expects -- the two
 * limiter branches (beq cr7 / blt) and the two timesteps (1/30, 1/75 s). */
static bool r2_unlock_fps_apply(void)
{
    static const struct { uint32_t addr, value; } sig[] = {
        {0x00635D7Cu, 0x41DEFFF8u}, {0x00635D88u, 0x4180FFE0u},
        {0x00D529D8u, 0x3D088889u}, {0x00D69EB8u, 0x3C5A740Eu},
    };
    for (const auto& w : sig)
        if (vm_read32(w.addr) != w.value) return false;
    const float dt = 0.0001f;
    uint32_t bits;
    memcpy(&bits, &dt, 4);
    vm_write32(0x00D529D8u, bits);
    vm_write32(0x00D69EB8u, bits);
    return true;
}

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
    const char* unlock = std::getenv("RES_UNLOCK_FPS");
    if (unlock && *unlock && std::strcmp(unlock, "0") != 0) {
        if (r2_unlock_fps_apply()) {
            g_res_unlock_fps = 1;
            std::fprintf(stderr, "[patch] RES_UNLOCK_FPS: frame limiter off, minimum timestep 0.0001 s (01.00)\n");
        } else {
            std::fprintf(stderr, "[patch] RES_UNLOCK_FPS: this executable is not the 01.00 one the patch is for; "
                                 "left at 30 fps\n");
        }
    }
    const char* setting = std::getenv("RES_FRONTEND_PORT0");
    g_res_frontend_port0 = !(setting && setting[0] == '0');
    std::fprintf(stderr, "[R2] frontend port 0 compatibility: %s\n",
                 g_res_frontend_port0 ? "on" : "off");
}
