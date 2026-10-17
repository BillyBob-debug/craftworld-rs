"""D341 (part 88): write the viewer's --profile-records file from LBP2 01.33 running in RPCS3.

The profile save is encrypted on disc, so this reads the local profile where the game has
loaded it (read-only, ReadProcessMemory; RPCS3 guest base 0x300000000). Windows only.
usage: rpcs3_profile_records.py OUT.txt   (RPCS3_PID picks the process when several run)

Chain (FOUND, 01.33): idx = *0xE50A04; profile = *(0xE4F800 + idx*0x120 + 0x108).
Per-slot records: profile+0x280 data, +0x284 count, 0x80 bytes each, SlotID (type, id) at +0.
A record's items: +0x38 data, +0x3c count, 0x20 bytes each (GUID, SHA1, type +0x18, skip
word +0x1c). Items whose skip word is 0 make the total; the collected count is those the
profile owns: profile+0x78 / +0x7c, a sorted array of pointers to {GUID, SHA1 pointer (0:
zero hash), type} compared by type & 0x7fffffff, GUID and SHA1 (0x171a00).
An item without a GUID is checked elsewhere (*(profile+0x518), not followed): counted as
not owned here.
D342: the profile's colours, *(profile+0x514) words +0x4c..+0x58 (0x18ae48, 0x15894c), go
on a line `colours C0 C1 C2 C3`.
D346: with the Story view (or any planet) up, each planet result's shown badge overlays go
on a line `overlays TYPE ID MASK`. A planet object holds its radius (1750.0) at +0x1c and
its results at +0xf8 (count +0xfc, 0x2c0 bytes each); a result's overlay k (0..12) is
shown while its scale target (+0x68 + 12k, 0x6fba24) is above 0; its SearchResultSlot
*(result+0x30) holds the SlotID at +0x7c / +0x80 (MEASURED: Earth's six groups, 4 22..27).
"""
import ctypes, ctypes.wintypes as wt, os, struct, subprocess, sys

BASE = 0x300000000
k32 = ctypes.WinDLL('kernel32', use_last_error=True)
k32.OpenProcess.restype = wt.HANDLE
k32.ReadProcessMemory.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_void_p,
                                  ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]


def rpcs3_pid():
    if os.environ.get('RPCS3_PID'):
        return int(os.environ['RPCS3_PID'])
    out = subprocess.run(['tasklist', '/FI', 'IMAGENAME eq rpcs3.exe', '/FO', 'CSV', '/NH'],
                         capture_output=True, text=True).stdout
    if 'rpcs3.exe' not in out:
        sys.exit('RPCS3 is not running')
    return int(out.split(',')[1].strip('"'))


def planet_overlays(blk, u32):
    """D346: `overlays` lines from the planet results in guest 0x30000000..0x40000000."""
    lines, step, radius = {}, 1 << 20, struct.pack('>f', 1750.0)
    for base in range(0x30000000, 0x40000000, step):
        try:
            d = blk(base, step)
        except OSError:
            continue
        j = d.find(radius)
        while j >= 0:
            try:
                res, cnt = struct.unpack('>II', blk(base + j - 0x1c + 0xf8, 8))
                ok = j % 4 == 0 and 0 < cnt < 100 and 0x10000 < res < 0x40000000
                for i in range(cnt if ok else 0):
                    r = res + i * 0x2c0
                    scales = [struct.unpack('>f', blk(r + 0x68 + 12 * k, 4))[0] for k in range(13)]
                    slot = u32(r + 0x30)
                    if 0x10000 < slot < 0x40000000:
                        ty, sid = struct.unpack('>ii', blk(slot + 0x7c, 8))
                        mask = sum(1 << k for k in range(1, 13) if scales[k] > 0)
                        lines[(ty, sid)] = f'overlays {ty} {sid} 0x{mask:x}'
            except OSError:
                pass
            j = d.find(radius, j + 1)
    return [lines[k] for k in sorted(lines)]


def main(out):
    h = k32.OpenProcess(0x0010, False, rpcs3_pid())  # PROCESS_VM_READ

    def blk(guest, n):
        buf = ctypes.create_string_buffer(n)
        got = ctypes.c_size_t()
        if not k32.ReadProcessMemory(h, ctypes.c_void_p(BASE + guest), buf, n, ctypes.byref(got)):
            raise OSError(f'read 0x{guest:x}+0x{n:x} failed')
        return buf.raw[:got.value]

    def u32(guest):
        return struct.unpack('>I', blk(guest, 4))[0]

    idx = u32(0xE50A04)
    profile = u32(0xE4F800 + idx * 0x120 + 0x108)
    if not profile:
        sys.exit('no local profile loaded yet')
    owned = set()
    inv, inv_n = u32(profile + 0x78), u32(profile + 0x7c)
    for e in struct.unpack(f'>{inv_n}I', blk(inv, 4 * inv_n)) if inv_n else ():
        guid, sha, ty = struct.unpack('>III', blk(e, 12))
        owned.add((ty & 0x7fffffff, guid, blk(sha, 20) if sha else bytes(20)))
    data, n = u32(profile + 0x280), u32(profile + 0x284)
    lines = ['# LBP2 local profile goodies by SlotID: type id collected total']
    # D342: the profile's colour record *(profile+0x514) (0x18ae48), words +0x4c..+0x58.
    colours = u32(profile + 0x514)
    if colours:
        words = struct.unpack('>4I', blk(colours + 0x4c, 16))
        lines.append('colours ' + ' '.join(f'0x{w:08x}' for w in words))
    for i in range(n):
        rec = blk(data + i * 0x80, 0x80)
        ty, sid = struct.unpack('>ii', rec[:8])
        items, count = struct.unpack('>II', rec[0x38:0x40])
        collected = total = 0
        raw = blk(items, count * 0x20) if count else b''
        for j in range(count):
            it = raw[j * 0x20:(j + 1) * 0x20]
            guid, = struct.unpack('>I', it[:4])
            ity, skip = struct.unpack('>II', it[0x18:0x20])
            if skip == 0:
                total += 1
                collected += (ity & 0x7fffffff, guid, it[4:24]) in owned
        lines.append(f'{ty} {sid} {collected} {total}')
    lines += planet_overlays(blk, u32)
    with open(out, 'w', newline='\n') as f:
        f.write('\n'.join(lines) + '\n')
    print(f'{n} records -> {out}')


if __name__ == '__main__':
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
