/*
 * ps3recomp game project -- entry point template.
 *
 * This is the runner. It boots a lifted title through the toolkit's PPU
 * scaffold: allocate the flat guest VM, load the PPU ELF named on the command
 * line, register the lifted function table and the HLE NID handlers, start a
 * frame clock, and dispatch the entry OPD.
 *
 * The scaffold does the work; nothing below reimplements it. ppu_load_elf,
 * ppu_recomp_register, ppu_hle_init, ppu_sysprx_register, ppu_fs_register,
 * lv2_init_syscalls and ppu_run all come from the toolkit, compiled into this
 * project from PS3RECOMP_DIR (see CMakeLists.txt). What is left here is the
 * part a port owns: which backend to present through, how the frame clock is
 * paced, and whatever diagnostics the title turns out to need.
 *
 * runtime/ppu/tests/boot_main.cpp in the toolkit is the same boot in its
 * fully-instrumented form -- crash filter, hang watchdog, guest-PC sampling
 * profiler. Read it when a boot goes wrong; copy from it what the title
 * needs. It is deliberately not what a fresh project starts with.
 *
 * The one part that IS duplicated from it is the frame clock below, because a
 * port is expected to change its pacing and a shared one would be the wrong
 * shape for that. It was copied faithfully, comments and all. If a pacing bug
 * is fixed in one of the two, look at the other.
 *
 * This builds on Windows, macOS and Linux. CI builds and runs it against the
 * boot smoke title on the last two, and compile-checks it against clang-cl on
 * the first (tools/check_ppu_scaffold.py).
 */

/* The lifter's generated header, which declares ppu_context and the function
 * table. It comes first: everything below is written against that struct. */
#include "ppu_recomp.h"

/* The Win32 names -- Sleep, GetTickCount64, CreateThread, InterlockedIncrement,
 * VirtualAlloc, the scalar typedefs -- from one place. On Windows this is a
 * passthrough to <windows.h>; off Windows the toolkit's shim supplies the same
 * names over pthreads and mmap, so the call sites below are written once. */
#include "win32_compat.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(__APPLE__)
#include <CoreFoundation/CoreFoundation.h>   /* the main thread's run loop */
#include <pthread.h>
#endif

#ifndef _WIN32
#include <sys/mman.h>   /* mmap: the guest arena, see alloc_guest_vm */
#endif

#ifdef _WIN32
/* timeBeginPeriod: <windows.h> arrives with WIN32_LEAN_AND_MEAN set, which
 * excludes the multimedia timer API, so it is asked for by name -- and after
 * windows.h, since timeapi.h uses UINT. No shim exists off Windows because
 * there is nothing to shim: the 15.6 ms default timer granularity this works
 * around is a Windows problem. */
#include <timeapi.h>
#endif

/* ---------------------------------------------------------------------------
 * What the scaffold gives this file
 * -----------------------------------------------------------------------*/
extern "C" {
uint32_t ppu_load_elf(const char* path);      /* ELF -> guest RAM, returns entry OPD */
void     ppu_recomp_register(void);           /* generated: lifted table -> address map */
void     ppu_hle_init(void);                  /* firmware import NID -> HLE handler */
void     ppu_sysprx_register(void);           /* boot-critical CRT (sys_initialize_tls, ...) */
void     ppu_fs_register(void);               /* cellFs over the real game directory */
void     lv2_init_syscalls(void);             /* the lv2 syscall table */
int      ppu_run(uint32_t entry_opd, uint32_t stack_top);
void     cellGame_init_from_paramsfo(const char* sfo_path);

/* This project's own: load whatever real system PRX modules the title needs
 * into guest RAM and register their exports. stubs.cpp has the empty version
 * a game with no lifted PRX wants. */
void     ps3_load_prx_modules(void);

extern const char* ppu_vfs_root;              /* host dir the PS3 mount points map into */

/* sys_ppu_thread.c: the main thread's stack for get_stack_information. */
extern uint32_t g_ppu_main_stack_addr;
extern uint32_t g_ppu_main_stack_size;

/* The status the guest handed to sys_process_exit. A title that exits that way
 * never returns through ppu_run, so this is only read on the path where the
 * entry function unwound instead. */
extern int     g_sys_process_exit_called;
extern int32_t g_sys_process_exit_code;

/* Host-provided symbols the runtime and the HLE libraries link against. */
uint8_t* vm_base = nullptr;
extern uint32_t ppu_vm_size;                  /* ppu_loader.cpp: the OOB guard */

/* Guest-callback dispatch. g_ps3_guest_caller is the hook the HLE runtime
 * calls back into recompiled code through -- cellSysutil events and the GCM
 * vblank/flip handlers. ppu_guest_call does the OPD -> dispatch. */
typedef void (*ps3_guest_caller_fn)(uint32_t, uint64_t, uint64_t, uint64_t, uint64_t,
                                    uint64_t, uint64_t, uint64_t, uint64_t);
extern ps3_guest_caller_fn g_ps3_guest_caller;
uint64_t ppu_guest_call(uint32_t, uint64_t, uint64_t, uint64_t, uint64_t,
                        uint64_t, uint64_t, uint64_t, uint64_t);

/* The GCM half of the frame clock. */
void     cellGcmTickVBlank(void);
void     cellGcmTickFlip(void);
int      cellGcm_take_flip_pending(void);
void     cellGcm_rsx_process_fifo(void);      /* drain get -> put */
unsigned cellGcm_flip_request_count(void);

/* PS3_TELEMETRY: draws in the last frame, and every guest thread's lr. */
uint32_t rsx_draw_engine_guest_draws(void);
void     ppu_report_guest_lrs(void);
}

/* ---------------------------------------------------------------------------
 * RSX present backend
 * -----------------------------------------------------------------------*/
/* D3D12 on Windows, Metal on Apple, and the null backend's headless software
 * path anywhere else -- the same selection runtime/host/host_posix.c and the
 * toolkit's boot harness make. All three expose the same three entry points,
 * so the frame clock below is backend-agnostic. */
#if defined(_WIN32)
extern "C" int  rsx_d3d12_backend_init(uint32_t w, uint32_t h, const char* title);
extern "C" void rsx_d3d12_backend_present(void);
extern "C" int  rsx_d3d12_backend_pump_messages(void);
#  define rsx_backend_init    rsx_d3d12_backend_init
#  define rsx_backend_present rsx_d3d12_backend_present
#  define rsx_backend_pump    rsx_d3d12_backend_pump_messages
#  define RSX_BACKEND_NAME    "D3D12"
#elif defined(__APPLE__)
extern "C" int  rsx_metal_backend_init(uint32_t w, uint32_t h, const char* title);
extern "C" void rsx_metal_backend_present(void);
extern "C" int  rsx_metal_backend_pump_messages(void);
#  define rsx_backend_init    rsx_metal_backend_init
#  define rsx_backend_present rsx_metal_backend_present
#  define rsx_backend_pump    rsx_metal_backend_pump_messages
#  define RSX_BACKEND_NAME    "Metal"
#else
extern "C" int  rsx_null_backend_init(uint32_t w, uint32_t h, const char* title);
extern "C" void rsx_null_backend_present(void);
extern "C" int  rsx_null_backend_pump_messages(void);
#  define rsx_backend_init    rsx_null_backend_init
#  define rsx_backend_present rsx_null_backend_present
#  define rsx_backend_pump    rsx_null_backend_pump_messages
#  define RSX_BACKEND_NAME    "null (headless software)"
#endif

/* ---------------------------------------------------------------------------
 * Configuration
 * -----------------------------------------------------------------------*/
/* The flat VM must span every region the PS3 memory map uses, not just the
 * game image: sys_ppu_thread_create puts thread stacks at 0xD0000000, so a
 * smaller arena makes every spawned thread's stack access out of bounds. */
#define VM_SIZE       0x100010000ull   /* the full 32-bit guest space + a 64K guard */
#define STACK_TOP     0x0FF00000u      /* main-thread stack, below the 0x10000000 segment */
#define MAIN_STACK_SIZE 0x00100000u    /* what the main thread is told it has */
#define WINDOW_WIDTH  1280
#define WINDOW_HEIGHT 720
static int g_rsx_ready = 0;

/* patches.cpp: the optional game patches (RES_UNLOCK_FPS, RES_FOV,
 * RES_MENU_60FPS), applied to the loaded ELF before the guest starts. */
extern "C" void res_apply_patches(void);
extern "C" double g_res_vblank_hz;   /* patches.cpp: the unlock-fps patch's vblank rate */

/* ---------------------------------------------------------------------------
 * Frame clock
 * -----------------------------------------------------------------------*/
/* On real hardware the RSX raises a vblank interrupt about 60 times a second
 * and that is what drives the game's frame loop. With no RSX it is synthesized
 * here: a host thread calls cellGcmTickVBlank/TickFlip, which invoke the
 * handlers the guest registered, and drains the GCM FIFO. Without this the
 * title initialises, registers its handlers and then waits forever.
 *
 * The ticks are driven off real elapsed time rather than off how long present()
 * took. A hidden or occluded window makes present block hard, and pacing the
 * ticks behind it paces the whole game behind it. */
static volatile LONG g_frames_presented = 0;

/* Frames handed to the backend at a guest FLIP boundary -- one per frame the
 * guest actually finished. The presents made before the guest's first flip, so
 * a fresh window is not left blank through a long boot, carry no guest frame
 * and are deliberately not counted. */
extern "C" unsigned ppu_boot_frames_presented(void)
{
    return (unsigned)g_frames_presented;
}

static void present_guest_frame(void)
{
    rsx_backend_present();
    /* This thread increments and guest threads read, so interlocked rather
     * than a volatile ++, which on arm64 is neither atomic nor a fence. */
    InterlockedIncrement(&g_frames_presented);
}

static DWORD WINAPI frame_clock(LPVOID)
{
    fprintf(stderr, "[rsx] frame clock thread started\n");
    fflush(stderr);
    int rsx_ok = g_rsx_ready;

    /* PS3_FRAMECLOCK_US=<us>: the sleep between two passes of this loop.
     * The FIFO has no doorbell: a put the guest writes, a flip it asks for,
     * waits here until the next pass, so this is also the latency of every
     * one of those. It was a fixed 4 ms, which left the RSX thread idle up to
     * 4 ms after each flip request. Default 50 (below); 4000 is the old pacing. */
    /* Resistance 2 needs the RSX to react to a put in microseconds: it links
     * a side buffer whose JUMP back sits at put, and reuses that memory for
     * dynamic vertex data right after moving put on. At 1 ms the walker read
     * the vertex data there instead of the JUMP ("FIFO JUMP to unmapped IO",
     * the title's 5 s "GPU sit and spin timeout", corrupt geometry after the
     * resync); at 50 us none in a 440 s run into gameplay. */
    int clock_us = 50;
    { const char* e = getenv("PS3_FRAMECLOCK_US"); if (e && atoi(e) > 0) clock_us = atoi(e); }
    /* PS3_VBLANK_HZ=<hz> (10..1000): the vblank rate, which also bounds the
     * flip rate. Default one tick per 16 ms (62.5 Hz). */
    ULONGLONG vblank_us = 16000;
    const char* vhz = getenv("PS3_VBLANK_HZ");
    if (vhz || g_res_vblank_hz > 0) {
        const double hz = vhz ? atof(vhz) : g_res_vblank_hz;
        if (hz >= 10 && hz <= 1000) {
            vblank_us = (ULONGLONG)(1e6 / hz + 0.5);
            fprintf(stderr, "[rsx] vblank %.2f Hz (%llu us)\n", hz,
                    (unsigned long long)vblank_us);
        }
    }
    ULONGLONG next_tick = GetTickCount64() * 1000;   /* us */
    ULONGLONG last_pump = 0;
    /* PS3_TELEMETRY=1 (jogar.sh turns it on): one cheap line a second with
     * the guest frames and the draws in the last frame -- a scene that keeps
     * flipping with almost no draws is a world that stopped rendering -- and,
     * once per stall, where every guest thread is when no frame has come for
     * 3 s, so a freeze in a player's session explains itself in the log. */
    const char* tele_env = getenv("PS3_TELEMETRY");
    const bool telemetry = tele_env && tele_env[0] == '1';
    ULONGLONG tele_time = GetTickCount64(), tele_last_flip_time = tele_time;
    uint32_t tele_flips = cellGcm_flip_request_count();
    bool tele_stalled = false;
    const char* stats_env = getenv("PS3RECOMP_FRAME_STATS");
    const bool frame_stats = stats_env && stats_env[0] == '1';
    ULONGLONG stats_time = GetTickCount64();
    const ULONGLONG stats_start = stats_time;
    uint32_t stats_flips = cellGcm_flip_request_count();

    for (;;) {
#ifdef _WIN32
        Sleep((DWORD)((clock_us + 999) / 1000));
#else
        usleep((useconds_t)clock_us);
#endif
        ULONGLONG now = GetTickCount64();
        const ULONGLONG now_us = now * 1000;
        if (frame_stats && now - stats_time >= 1000) {
            uint32_t flips = cellGcm_flip_request_count();
            fprintf(stderr, "[frame-stats] t=%.2f guest=%.2f flips=%u\n",
                    (now - stats_start) / 1000.0,
                    (flips - stats_flips) * 1000.0 / (now - stats_time), flips);
            stats_time = now; stats_flips = flips;
        }
        if (telemetry && now - tele_time >= 1000) {
            const uint32_t flips = cellGcm_flip_request_count();
            if (flips != tele_flips) {
                if (tele_stalled)
                    fprintf(stderr, "[tele] frames resumed after %.1f s without one\n",
                            (now - tele_last_flip_time) / 1000.0);
                tele_stalled = false;
                tele_last_flip_time = now;
            } else if (!tele_stalled && now - tele_last_flip_time >= 3000) {
                tele_stalled = true;
                fprintf(stderr, "[tele] FREEZE: no guest frame for %.1f s; guest threads:\n",
                        (now - tele_last_flip_time) / 1000.0);
                ppu_report_guest_lrs();
            }
            fprintf(stderr, "[tele] frames=%u draws=%u\n", flips - tele_flips,
                    rsx_draw_engine_guest_draws());
            tele_time = now; tele_flips = flips;
        }

        int fired = 0;
        while ((long long)(now_us - next_tick) >= 0 && fired < 240) {
            cellGcmTickVBlank();
            cellGcmTickFlip();
            /* Present a pending flip BEFORE draining any further. The flip
             * fires at a get==put frame boundary on the guest thread, so the
             * batch held right now is exactly the completed frame; presenting
             * after the drain races the guest's next-frame writes and shows a
             * mixed one. */
            if (rsx_ok && cellGcm_take_flip_pending())
                present_guest_frame();
            /* Drain the FIFO every tick. This is what writes the RSX sync-fence
             * labels the game's per-frame logic blocks on, so it has to keep
             * advancing at 60 Hz even while present() throttles. */
            if (rsx_ok) cellGcm_rsx_process_fifo();
            next_tick += vblank_us;      /* ~60 Hz by default */
            fired++;
        }
        if (fired >= 240) next_tick = now_us;   /* fell too far behind -- resync */

        /* Drain at the outer cadence too, not only on the 16 ms tick: titles
         * fence every render pass on a label the drain writes, and one wave of
         * those at 16 ms apiece paces the guest into single-figure frame rates.
         * The real RSX writes them in microseconds. */
        if (rsx_ok) {
            if (cellGcm_take_flip_pending())
                present_guest_frame();
            cellGcm_rsx_process_fifo();

            /* Window events at most every 4 ms, as before PS3_FRAMECLOCK_US:
             * a windowed pump is a dispatch_sync to the main thread, and this
             * loop now runs every 1 ms. */
            const int pump_due = now - last_pump >= 4 || now < last_pump;
            if (pump_due) {
                last_pump = now;
                if (rsx_backend_pump() != 0) {
                    rsx_ok = 0;              /* window closed */
                    continue;
                }
            }
            /* Present a flip the drain just reached, now: the drain stops at
             * the flip, so the batch is exactly the finished frame. Consuming
             * the pending flag here matters -- this used to present on the
             * flip count changing and leave the flag set, so the check above
             * re-showed the same frame on the next pass with no draws: every
             * frame was presented twice. Before the first flip present at the
             * pump's cadence, so the window is not blank during boot. */
            if (cellGcm_take_flip_pending())
                present_guest_frame();
            else if (pump_due && cellGcm_flip_request_count() == 0)
                rsx_backend_present();
        }
    }
    return 0;
}

/* ---------------------------------------------------------------------------
 * VFS root
 * -----------------------------------------------------------------------*/
/* Derive the directory holding PS3_GAME from the EBOOT path
 * <root>/PS3_GAME/USRDIR/EBOOT.elf -> <root>. $PS3_VFS_ROOT overrides. */
static char s_vfs_root[1024];

static void derive_vfs_root(const char* eboot)
{
    const char* env = getenv("PS3_VFS_ROOT");
    if (env && *env) { ppu_vfs_root = env; return; }

    strncpy(s_vfs_root, eboot, sizeof s_vfs_root - 1);
    for (char* p = s_vfs_root; *p; p++) if (*p == '\\') *p = '/';
    /* strip EBOOT.elf, USRDIR and PS3_GAME */
    for (int i = 0; i < 3; i++) { char* s = strrchr(s_vfs_root, '/'); if (s) *s = 0; }
    if (!s_vfs_root[0]) strcpy(s_vfs_root, ".");
    ppu_vfs_root = s_vfs_root;
}

/* ---------------------------------------------------------------------------
 * Guest VM
 * -----------------------------------------------------------------------*/
#ifdef _WIN32
/* Demand-paging for the flat VM: reserve the whole 4 GB guest space, which
 * costs no commit, and commit each 64 KB page on first access. Every 32-bit
 * guest offset is then valid, so a garbage guest pointer reads zero instead of
 * killing the process. Faults outside the arena fall through untouched. */
static LONG WINAPI vm_commit_veh(EXCEPTION_POINTERS* ep)
{
    if (ep->ExceptionRecord->ExceptionCode == EXCEPTION_ACCESS_VIOLATION) {
        ULONG_PTR fault = ep->ExceptionRecord->ExceptionInformation[1];
        uintptr_t base  = (uintptr_t)vm_base;
        if (vm_base && fault >= base && fault < base + VM_SIZE) {
            void* page = (void*)(fault & ~(uintptr_t)0xFFFF);
            if (VirtualAlloc(page, 0x10000, MEM_COMMIT, PAGE_READWRITE))
                return EXCEPTION_CONTINUE_EXECUTION;
        }
    }
    return EXCEPTION_CONTINUE_SEARCH;
}
#endif

static bool alloc_guest_vm(void)
{
#ifdef _WIN32
    AddVectoredExceptionHandler(1, vm_commit_veh);
    vm_base = (uint8_t*)VirtualAlloc(NULL, VM_SIZE, MEM_RESERVE, PAGE_READWRITE);
    ppu_vm_size = 0;              /* the whole space is backed; no OOB guard needed */
#else
    /* No vectored exception handlers off Windows, so the arena is mapped up
     * front and lazily backed by the OS: only pages the title touches cost
     * anything. It spans the whole 32-bit space plus the guard, as on Windows:
     * Resistance's game.self starts a raw SPU after the intro movie, and the
     * raw SPU windows (local store and problem-state registers, spu_raw.c)
     * live at 0xE0000000+. An arena that stopped there turned every MFC proxy
     * DMA register store into an "OOB access" and the level never loaded.
     *
     * mmap, not calloc: vm_commit() hands vm_base + addr to mprotect, which
     * takes page-aligned addresses only. glibc serves an allocation this big
     * from its own mmap chunk and returns a pointer 16 bytes past the page
     * start, so every commit failed with EINVAL and sys_memory_allocate
     * answered ENOMEM to every call. mmap's base is page-aligned and zeroed. */
    void* p = mmap(NULL, VM_SIZE, PROT_READ | PROT_WRITE,
                   MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
    vm_base = (p == MAP_FAILED) ? nullptr : (uint8_t*)p;
    ppu_vm_size = 0;              /* the whole space is backed; no OOB guard needed */
#endif
    return vm_base != nullptr;
}

/* ---------------------------------------------------------------------------
 * Main
 * -----------------------------------------------------------------------*/
static void harness_guest_caller(uint32_t opd, uint64_t a0, uint64_t a1,
                                 uint64_t a2, uint64_t a3, uint64_t a4,
                                 uint64_t a5, uint64_t a6, uint64_t a7)
{
    ppu_guest_call(opd, a0, a1, a2, a3, a4, a5, a6, a7);
}

int main(int argc, char** argv)
{
    if (argc < 2) {
        printf("usage: %s <PPU ELF>\n", argv[0]);
        return 2;
    }

#ifdef _WIN32
#pragma comment(lib, "winmm.lib")
    /* 1 ms timer resolution. The default granularity is about 15.6 ms, which
     * inflates every shorter wait the title makes and throttles the whole
     * thing. POSIX timers are already fine-grained. */
    timeBeginPeriod(1);
    setvbuf(stdout, NULL, _IONBF, 0);   /* unbuffered: do not lose prints on a kill */
#endif

    printf("=== ps3recomp game runner ===\n");

    /* The toolkit's SPURS kernel-style workload selection is opt-in there;
     * this game needs it. With the old round-robin selection every workload
     * but wid 0 yielded at its first poll, and the gameplay jobs
     * (igGeomCullClip, SpuBucketerDrawSort) timed out. SPURS_RPCS3_SCHED=0
     * in the environment still turns it off. */
    {
        const char* s = getenv("SPURS_RPCS3_SCHED");
#ifdef _WIN32
        if (!s) _putenv_s("SPURS_RPCS3_SCHED", "1");
        else if (strcmp(s, "0") == 0) _putenv_s("SPURS_RPCS3_SCHED", "");
#else
        if (!s) setenv("SPURS_RPCS3_SCHED", "1", 1);
        else if (strcmp(s, "0") == 0) unsetenv("SPURS_RPCS3_SCHED");
#endif
    }
    /* Real sys_lwcond notifications instead of the 1 ms sleep-and-retry
     * fallback. R2's job/loader worker pools (func_00B11A80 x14,
     * func_007D9EB8 x8) hand work over through lwconds; at ~1.2 ms per
     * handoff the front-end load after the Insomniac movie froze the guest
     * for 5-6 s, with events ~1 s (A/B on one binary). PS3_LWCOND_EVENTS=0
     * restores the fallback. ppu_lwcond_events.h reads it once, lazily. */
    {
        const char* s = getenv("PS3_LWCOND_EVENTS");
#ifdef _WIN32
        if (!s) _putenv_s("PS3_LWCOND_EVENTS", "1");
#else
        if (!s) setenv("PS3_LWCOND_EVENTS", "1", 1);
#endif
    }

    const char* title = getenv("PS3_TITLE");
    if (!title || !*title) title = "Resistance 2";
    g_rsx_ready = (rsx_backend_init(WINDOW_WIDTH, WINDOW_HEIGHT, title) == 0);
    fprintf(stderr, "[rsx] %s backend init %s\n", RSX_BACKEND_NAME,
            g_rsx_ready ? "OK -- window open" : "FAILED");

    if (!alloc_guest_vm()) {
        fprintf(stderr, "ERROR: could not allocate the guest address space\n");
        return 1;
    }

    uint32_t entry = ppu_load_elf(argv[1]);
    if (!entry) {
        fprintf(stderr, "ERROR: could not load %s\n", argv[1]);
        return 1;
    }

    res_apply_patches();

    derive_vfs_root(argv[1]);
    printf("[boot] VFS root: %s\n", ppu_vfs_root);
    char sfo_path[1024];
    snprintf(sfo_path, sizeof(sfo_path), "%s/PS3_GAME/PARAM.SFO", ppu_vfs_root);
    cellGame_init_from_paramsfo(sfo_path);

    ppu_recomp_register();   /* lifted function table -> address map */
    ps3_load_prx_modules();  /* this game's lifted system PRX, if it has any */
    ppu_hle_init();          /* firmware import NID -> HLE handlers */
    ppu_sysprx_register();   /* boot-critical CRT */
    ppu_fs_register();       /* cellFs over the game directory */
    lv2_init_syscalls();     /* the lv2 syscall table */

    /* Install the guest-callback hook, then start the frame clock. It no-ops
     * until the title registers its vblank and flip handlers during init. */
    g_ps3_guest_caller = harness_guest_caller;
    CreateThread(NULL, 4u * 1024 * 1024, frame_clock, NULL, 0, NULL);

    printf("\n[boot] dispatching entry OPD 0x%08X (stack top 0x%08X)\n\n",
           entry, STACK_TOP);

    /* The main thread's stack, as sys_ppu_thread_get_stack_information reports
     * it: the 1 MB below STACK_TOP the guest really runs on. */
    g_ppu_main_stack_addr = STACK_TOP - MAIN_STACK_SIZE;
    g_ppu_main_stack_size = MAIN_STACK_SIZE;

    /* A process started by sys_game_process_exitspawn (stubs.cpp) gets the
     * parent's data block where lv2 leaves it: Resistance's game.self looks for
     * it at (stack addr + size - 0x1000) and accepts it when it starts with
     * "ELFtastic!" (func_001E96A4). The guest stack starts below it. */
    uint32_t run_stack_top = STACK_TOP;
    if (const char* df = getenv("RES_SPAWN_DATA")) {
        if (FILE* f = fopen(df, "rb")) {
            uint8_t buf[0x1000];
            size_t n = fread(buf, 1, sizeof buf, f);
            fclose(f);
            /* RES_LEVEL=<n>: start the campaign in level n (the disc's
             * packed/levels/level<n>: 20-22, 30-32, ... 110-112, 120-121).
             * The front end passes the level to start in the block itself: a
             * flag byte at +0x10 and a big-endian int at +0x11, read by
             * func_001EA984 into the variables "-level" sets (0 = the first
             * level of a new campaign). game.self's own "-level" argument is
             * overwritten by the block, so the block is the place to set it. */
            const char* lv = getenv("RES_LEVEL");
            if (lv && *lv && n >= 0x15 && memcmp(buf, "ELFtastic!", 10) == 0) {
                const uint32_t level = (uint32_t)strtoul(lv, nullptr, 10);
                buf[0x11] = (uint8_t)(level >> 24); buf[0x12] = (uint8_t)(level >> 16);
                buf[0x13] = (uint8_t)(level >> 8);  buf[0x14] = (uint8_t)level;
                printf("[boot] RES_LEVEL: starting in level %u\n", level);
            }
            memcpy(vm_base + STACK_TOP - 0x1000, buf, n);
            run_stack_top = STACK_TOP - 0x1000;
            printf("[boot] exitspawn data: %zu bytes at 0x%08X\n", n, STACK_TOP - 0x1000);
        }
        /* RES_SPAWN_KEEP=1 leaves the file for a rerun of the child alone. */
        if (!getenv("RES_SPAWN_KEEP")) remove(df);
        unsetenv("RES_SPAWN_DATA");
    }

#if defined(__APPLE__)
    /* AppKit only runs on the process's main thread: the Metal backend pumps
     * the window's events with dispatch_sync onto the main queue. With the
     * guest on the main thread nothing ever serviced that queue, so the frame
     * clock blocked in its first event pump, the FIFO never drained and the
     * title spun for ever in its "get == put" wait (func_000A00DC). The guest
     * runs on its own thread instead (the 256 MB host stack every guest thread
     * gets) and the main thread turns the run loop. Same fix as the inFAMOUS 2
     * port. */
    int rc;
    {
        struct boot_args { uint32_t entry; uint32_t stack_top; int rc; volatile int done; };
        static boot_args ba;
        ba.entry = entry; ba.stack_top = run_stack_top; ba.rc = 0; ba.done = 0;
        pthread_attr_t attr;
        pthread_attr_init(&attr);
        pthread_attr_setstacksize(&attr, 256u * 1024u * 1024u);
        pthread_t th;
        auto body = [](void* a) -> void* {
            boot_args* b = (boot_args*)a;
            b->rc = ppu_run(b->entry, b->stack_top);
            __atomic_store_n(&b->done, 1, __ATOMIC_RELEASE);
            CFRunLoopStop(CFRunLoopGetMain());
            return nullptr;
        };
        if (pthread_create(&th, &attr, body, &ba) != 0) {
            fprintf(stderr, "[boot] could not start the guest thread\n");
            return 1;
        }
        pthread_attr_destroy(&attr);
        while (!__atomic_load_n(&ba.done, __ATOMIC_ACQUIRE))
            CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.05, false);
        pthread_join(th, nullptr);
        rc = ba.rc;
    }
#else
    int rc = ppu_run(entry, run_stack_top);
#endif
    printf("\n[boot] ppu_run returned %d (entry function unwound)\n", rc);

    /* A title that called sys_process_exit never reaches this line: that path
     * ends in the host exit(). Getting here means the entry function returned
     * instead, so hand back the status the guest published if it published
     * one. Returning a hardcoded 0 would report success for a run that never
     * got anywhere. */
    return g_sys_process_exit_called ? (int)g_sys_process_exit_code : 0;
}
