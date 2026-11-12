#!/usr/bin/env python3
"""lift_r2_relocmods.py [EBOOT.elf] [--out port/spu/relocmods]

The position-independent SPU code Resistance 2 runs inside its WWS jobs. The
EBOOT carries 142 of these blobs: a 0x30-byte header (4 ila words that identify
the blob, then small fields, 0xFFFFFF6F or 0xFFFFFFEF, and DEADDEAD padding) and
the first function at +0x30. A job loads one wherever it has room in local store
and calls into it, so a fixed-address lift cannot match them and they ran in the
SPU interpreter: 50-80 M instructions a second in gameplay, two host threads.

Each blob is wrapped as an SPU ELF at LS 0 and lifted in --relocatable mode
(every address derived from the code's own position adds spu_context.code_base).
relocmods_register.c registers it with spu_overlay_register_region_reloc at its
guest address: the GET that loads it makes the lift resident wherever it landed.
PS3_RELOCMODS=0 leaves them to the interpreter.

Generated code from the game: regenerate it, never commit it.
"""
import argparse, os, re, struct, subprocess, sys

here = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, here)
import r2_layout  # noqa: E402
# The toolkit's lifter and helpers (PS3RECOMP_DIR or ./ps3recomp).
TK_TOOLS = os.path.join(str(r2_layout.toolkit()), 'tools')
sys.path.insert(0, TK_TOOLS)
import lift_jobmods as lj          # manual_link_targets, switch_table_targets
from wrap_spu_elf import wrap

# Second PT_LOAD: file offset + this = guest address.
VA_OFFSET = 0x10000
# Set in main() from r2_layout.EBOOT_BUILDS for the executable being read: the
# data segment's file range, and the raw SPU programs tools/prepare_r2_rawspu.py
# lifts, as file offsets.
DATA_LO, DATA_HI = 0, 0
RAW = []
FIRST_IMAGE_ID = 300
HEADER = 0x30


def find_blobs(d):
    starts = []
    for o in range(DATA_LO, DATA_HI - 32, 16):
        w = struct.unpack_from('>8I', d, o)
        if not all((x >> 24) in (0x40, 0x41, 0x42, 0x43) for x in w[:4]):
            continue
        if w[5] or w[6] or w[4] >= 0x40000 or w[7] >= 0x40000:
            continue
        if d[o + 0x28:o + 0x30] != b'\xDE\xAD\xDE\xAD\xDE\xAD\xDE\xAD':
            continue
        if any(lo <= o < lo + n for lo, n in RAW):
            continue
        starts.append(o)
    out = []
    for n, s in enumerate(starts):
        end = starts[n + 1] if n + 1 < len(starts) else DATA_HI
        # The last blob of a run ends where 256 zero bytes begin.
        z = d.find(b'\x00' * 256, s, end)
        if z > 0:
            end = z
        for lo, sz in RAW:
            if s < lo < end:
                end = lo
        span = min((end - s + 15) & ~15, 0x3C000)
        out.append((s, span))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('eboot', nargs='?', default=str(r2_layout.eboot()))
    ap.add_argument('--out', default=str(r2_layout.SPU_DIR / 'relocmods'))
    ap.add_argument('--extra', default='', help='comma-separated "0xEA:0xOFF" extra entries inside a blob')
    a = ap.parse_args()
    d = open(a.eboot, 'rb').read()
    global DATA_LO, DATA_HI, RAW
    build = r2_layout.eboot_build(a.eboot)
    DATA_LO, DATA_HI = build["data"]
    RAW = [(va - VA_OFFSET, size) for va, size, _ in build["rawspu"].values()]
    blobs = find_blobs(d)
    if not blobs:
        sys.exit('no relocatable blobs found')
    extra = {}
    for tok in filter(None, (t.strip() for t in a.extra.split(','))):
        ea, off = (int(x, 0) for x in tok.split(':'))
        extra.setdefault(ea, set()).add(off)
    lifter = os.path.join(TK_TOOLS, 'spu_lifter.py')
    regs = []
    os.makedirs(a.out, exist_ok=True)
    for n, (s, span) in enumerate(blobs):
        ea = s + VA_OFFSET
        raw = d[s:s + span]
        name = f'relocmod_{ea:08x}'
        prefix = name + '_'
        outsub = os.path.join(a.out, name)
        os.makedirs(outsub, exist_ok=True)
        tmp = os.path.join(a.out, f'_tmp_{name}.elf')
        with open(tmp, 'wb') as f:
            f.write(wrap(raw, base=0, entry=HEADER))
        extras = {hex(HEADER)}
        extras |= {hex(x) for x in lj.manual_link_targets(raw, base=0)}
        extras |= {hex(x) for x in lj.switch_table_targets(raw, base=0)}
        extras |= {hex(x) for x in extra.get(ea, ())}
        cmd = [sys.executable, lifter, '--auto-functions', tmp, '--base', '0', '--relocatable',
               '--symbol-prefix', prefix, '-o', outsub, '--source-name', 'spu_recomp.c',
               '--header-name', 'spu_recomp.h', '--code-end', hex(span),
               '--extra-funcs', ','.join(sorted(extras, key=lambda x: int(x, 16)))]
        r = subprocess.run(cmd, capture_output=True, text=True)
        os.remove(tmp)
        if r.returncode != 0:
            sys.exit(f'{name}: lift failed\n{r.stderr[-3000:]}')
        funcs = next((l.strip() for l in r.stdout.splitlines() if l.strip().endswith('function(s) lifted')), '?')
        print(f'  {name}: span 0x{span:X} -> image {FIRST_IMAGE_ID + n}, {funcs}')
        regs.append((prefix, ea, span, FIRST_IMAGE_ID + n, name))
    L = ['/* relocmods_register.c -- GENERATED by tools/lift_r2_relocmods.py.',
         ' * Position-independent SPU code a job loads anywhere in local store, lifted at',
         ' * LS 0 and registered at its guest address: the GET that loads one makes its',
         ' * lift resident wherever it landed. */',
         '#include <stdint.h>', '#include <stdlib.h>', '',
         'extern void spu_begin_image(int image_id);',
         'extern void spu_overlay_register_region_reloc(uint32_t content_ea, uint32_t span, int image_id);']
    L += [f'extern void {p}spu_recomp_register(void);' for p, *_ in regs]
    L += ['', '__attribute__((constructor)) static void r2_relocmods_register(void)', '{',
          '    /* Default on (A/B of 07/11/2026, cool machine, back to back: interpreted SPU',
          '     * steps 7.9 -> 1.5 M/s, same 29 FPS and CPU, image identical; a hot machine',
          '     * interprets 26-33 M/s). PS3_RELOCMODS=0 leaves them to the interpreter. */',
          '    { const char* e = getenv("PS3_RELOCMODS"); if (e && e[0] == \'0\') return; }']
    for p, ea, span, img, name in regs:
        L.append(f'    spu_begin_image({img}); {p}spu_recomp_register();')
        L.append(f'    spu_overlay_register_region_reloc(0x{ea:08X}u, 0x{span:X}u, {img});   /* {name} */')
    L += ['    spu_begin_image(0);', '}', '']
    with open(os.path.join(a.out, 'relocmods_register.c'), 'w') as f:
        f.write('\n'.join(L))
    print(f'wrote {len(regs)} modules to {a.out}')


if __name__ == '__main__':
    main()
