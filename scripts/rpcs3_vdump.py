"""Record RPCS3's CPU-skinned Sackboy vertex buffers (guest 0x32990000 / 0x39690000 /
0x39f90000 this boot) while keys are pressed. usage: vdump.py OUT.bin DURATION 't:key:ms' ...
Writes OUT.bin (per sample: 3 x 0xC000 bytes) and OUT.json (times, key events)."""
import sys, time, json, threading
import os; sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import memscan, pad
BUFS = [0x32990000, 0x39690000, 0x39f90000]
SZ = 0xC000
out, dur = sys.argv[1], float(sys.argv[2])
events = []
for tok in sys.argv[3:]:
    t, k, ms = tok.split(':'); events.append((float(t), k, int(ms)))
h = memscan.open_proc()
t0 = time.time()
def keys():
    for t, k, ms in events:
        while time.time() - t0 < t: time.sleep(0.002)
        pad.hold(k, ms)
th = threading.Thread(target=keys, daemon=True); th.start()
last = None; times = []
with open(out, 'wb') as f:
    while time.time() - t0 < dur:
        d = [memscan.read(h, b, SZ) for b in BUFS]
        if d != last and all(x is not None for x in d):
            for x in d: f.write(x)
            times.append(round(time.time() - t0, 4)); last = d
json.dump({'times': times, 'events': events, 'bufs': BUFS, 'size': SZ}, open(out + '.json', 'w'))
print('samples', len(times))
