"""vsel.py IN.bin OUT.bin buf(0-2|all) start stop [step]: slice vertex samples (2920 verts) for pose_fit."""
import sys
inp, out, buf, a, b = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
step = int(sys.argv[6]) if len(sys.argv) > 6 else 1
import os
SZ, N = 0xC000, int(os.environ.get("GAME_VERTS", "2512"))
d = open(inp, 'rb').read(); n = len(d) // (3 * SZ)
with open(out, 'wb') as f:
    for s in range(a, min(b, n), step):
        for k in ([0, 1, 2] if buf == 'all' else [int(buf)]):
            f.write(d[(s * 3 + k) * SZ:(s * 3 + k) * SZ + N * 16])
