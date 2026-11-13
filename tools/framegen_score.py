#!/usr/bin/env python3
"""Score a PS3_FRAMEGEN_TEST dump folder (needs numpy).

    python3 tools/framegen_score.py <dir>

One line per dumped triple: how far the eye moved over the two frames, the
motion vectors' size in pixels, and PSNR (dB) against the real middle frame
N+1 of: the interpolation from N and N+2 with this port's motion vectors
(interp), the same with zero motion (interp0), a 50/50 blend, N and N+2
themselves, and each PS3_FRAMEGEN_TEST_SCALES variant (S<scale>). Working
motion: a static camera has exactly zero, interp beats interp0 and the
blend, and scale 1 beats the others.
"""
import sys, glob, os, re, numpy as np
d = sys.argv[1]
def load(k, name):
    f = glob.glob(f"{d}/t{k:03d}_{name}_*.raw")
    if not f: return None
    w, h = map(int, re.search(r'_(\d+)x(\d+)\.raw', f[0]).groups())
    raw = np.fromfile(f[0], dtype=np.uint8)
    if name == 'mv': return np.frombuffer(raw.tobytes(), dtype=np.float16).reshape(h, w, 2).astype(np.float32)
    if name == 'depth': return np.frombuffer(raw.tobytes(), dtype=np.float32).reshape(h, w)
    return raw.reshape(h, w, 4)[..., :3].astype(np.float32)
def psnr(a, b):
    m = np.mean((a - b) ** 2); return 99.0 if m == 0 else 10 * np.log10(255 ** 2 / m)
rows = []
for meta in sorted(glob.glob(f"{d}/t*_meta.txt")):
    k = int(os.path.basename(meta)[1:4])
    md = dict(l.split(' ', 1) for l in open(meta).read().strip().split('\n'))
    c0, c1, c2, it, it0 = (load(k, n) for n in ('c0', 'c1', 'c2', 'interp', 'interp0'))
    mv, z = load(k, 'mv'), load(k, 'depth')
    if c1 is None: continue
    blend = (c0 + c2) / 2
    es = np.array(list(map(float, md['eye_step2'].split())))
    mvpx = np.abs(mv) * np.array([mv.shape[1], mv.shape[0]])
    print(f"t{k:03d} run {md['run']:>5} eye2 {np.linalg.norm(es):7.3f} mv max {mvpx.max():8.2f}px mean {mvpx.mean():6.2f}px nz {np.mean(mvpx.max(-1)>0)*100:5.1f}%"
          f" | interp {psnr(it, c1):5.1f} interp0 {psnr(it0, c1):5.1f} blend {psnr(blend, c1):5.1f} c0 {psnr(c0, c1):5.1f} c2 {psnr(c2, c1):5.1f}"
          f" | z {z.min():.3f}..{z.max():.3f}"
          + ''.join(f" S{sc} {psnr(load(k, 'interpS'+sc), c1):5.1f}" for sc in sorted({re.search(r'interpS([-0-9.]+)_', f).group(1) for f in glob.glob(f"{d}/t{k:03d}_interpS*")}, key=float)))
