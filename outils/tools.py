"""Outils de test : recadrage PNG agrandi.  python tools.py crop in.png out.png x0 y0 x1 y1 [zoom]"""
import sys, struct, zlib


def read_png(path):
    d = open(path, 'rb').read()
    pos = 8
    idat = b''
    w = h = 0
    while pos < len(d):
        n = struct.unpack('>I', d[pos:pos + 4])[0]
        typ = d[pos + 4:pos + 8]
        body = d[pos + 8:pos + 8 + n]
        if typ == b'IHDR':
            w, h = struct.unpack('>II', body[:8])
        elif typ == b'IDAT':
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    stride = w * 3 + 1
    rows = [raw[i * stride + 1:(i + 1) * stride] for i in range(h)]
    return w, h, rows


def write_png(path, w, h, rows):
    raw = b''.join(b'\x00' + r for r in rows)

    def chunk(t, c):
        x = struct.pack('>I', len(c)) + t + c
        return x + struct.pack('>I', zlib.crc32(t + c) & 0xffffffff)
    open(path, 'wb').write(b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0))
                           + chunk(b'IDAT', zlib.compress(raw, 6)) + chunk(b'IEND', b''))


if sys.argv[1] == 'crop':
    src, dst = sys.argv[2], sys.argv[3]
    x0, y0, x1, y1 = [int(v) for v in sys.argv[4:8]]
    z = int(sys.argv[8]) if len(sys.argv) > 8 else 2
    w, h, rows = read_png(src)
    out = []
    for y in range(y0, y1):
        r = rows[y][x0 * 3:x1 * 3]
        row = b''.join(r[i:i + 3] * z for i in range(0, len(r), 3))
        out.extend([row] * z)
    write_png(dst, (x1 - x0) * z, (y1 - y0) * z, out)
    print('ok', dst)
