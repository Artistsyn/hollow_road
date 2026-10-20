"""Cut the flat background out of a generated sprite: pixels connected to the image border whose
colour is within `tol` of the border's own colour become transparent. Pure Python (zlib only).

    python3 -I tools/cutout.py in.png [out.png] [--tol 28]
"""
import struct, sys, zlib
from collections import deque

def read_png(path):
    data = open(path, 'rb').read()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', 'not a PNG'
    pos, idat, w = 8, b'', 0
    while pos < len(data):
        n, kind = struct.unpack('>I4s', data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        if kind == b'IHDR':
            w, h, depth, ctype, _, _, inter = struct.unpack('>IIBBBBB', body)
            assert depth == 8 and ctype in (2, 6) and inter == 0, f'unsupported PNG ({depth}, {ctype}, {inter})'
            bpp = 4 if ctype == 6 else 3
        elif kind == b'IDAT':
            idat += body
        pos += 12 + n
    raw, stride = zlib.decompress(idat), w * bpp
    out, prev = bytearray(), bytearray(stride)
    for y in range(h):
        f, line = raw[y * (stride + 1)], bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = line[i - bpp] if i >= bpp else 0
            b, c = prev[i], prev[i - bpp] if i >= bpp else 0
            if f == 1: line[i] = (line[i] + a) & 255
            elif f == 2: line[i] = (line[i] + b) & 255
            elif f == 3: line[i] = (line[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c; pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[i] = (line[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        out += line; prev = line
    px = [list(out[i:i + bpp]) + ([255] if bpp == 3 else []) for i in range(0, len(out), bpp)]
    return w, h, px

def write_png(path, w, h, px):
    raw = b''.join(b'\x00' + bytes(v for p in px[y * w:(y + 1) * w] for v in p) for y in range(h))
    chunk = lambda k, b: struct.pack('>I', len(b)) + k + b + struct.pack('>I', zlib.crc32(k + b) & 0xffffffff)
    open(path, 'wb').write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
                           + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))

def cutout(w, h, px, tol):
    border = [px[x] for x in range(w)] + [px[(h - 1) * w + x] for x in range(w)] + [px[y * w] for y in range(h)] + [px[y * w + w - 1] for y in range(h)]
    bg = [sorted(p[k] for p in border)[len(border) // 2] for k in range(3)]
    near = lambda p: p[3] < 16 or sum((p[k] - bg[k]) ** 2 for k in range(3)) <= tol * tol
    seen, q = bytearray(w * h), deque()
    for i in [x for x in range(w)] + [(h - 1) * w + x for x in range(w)] + [y * w for y in range(h)] + [y * w + w - 1 for y in range(h)]:
        if not seen[i] and near(px[i]): seen[i] = 1; q.append(i)
    while q:
        i = q.popleft(); x, y = i % w, i // w
        px[i] = [0, 0, 0, 0]
        for j in ((i - 1) if x > 0 else -1, (i + 1) if x < w - 1 else -1, (i - w) if y > 0 else -1, (i + w) if y < h - 1 else -1):
            if j >= 0 and not seen[j] and near(px[j]): seen[j] = 1; q.append(j)
    return bg

if __name__ == '__main__':
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    tol = float(sys.argv[sys.argv.index('--tol') + 1]) if '--tol' in sys.argv else 28.0
    if '--tol' in sys.argv: args.remove(sys.argv[sys.argv.index('--tol') + 1])
    w, h, px = read_png(args[0])
    bg = cutout(w, h, px, tol)
    write_png(args[1] if len(args) > 1 else args[0], w, h, px)
    print(f'{args[0]}: background {bg} cut, {sum(1 for p in px if p[3] == 0)} of {w * h} pixels clear')
