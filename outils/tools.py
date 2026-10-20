"""Outils de test : recadrage PNG agrandi.  python tools.py crop in.png out.png x0 y0 x1 y1 [zoom]
Lit les PNG 8 bits RGB ou RGBA, avec tous les filtres de ligne (ceux des captures F12 des jeux)."""
import sys, struct, zlib


def read_png(path):
    """-> (largeur, hauteur, lignes en RGB)"""
    d = open(path, 'rb').read()
    pos = 8
    idat = b''
    w = h = ctype = 0
    while pos < len(d):
        n = struct.unpack('>I', d[pos:pos + 4])[0]
        typ = d[pos + 4:pos + 8]
        body = d[pos + 8:pos + 8 + n]
        if typ == b'IHDR':
            w, h = struct.unpack('>II', body[:8])
            ctype = body[9]
        elif typ == b'IDAT':
            idat += body
        pos += 12 + n
    raw = zlib.decompress(idat)
    bpp = 4 if ctype == 6 else 3
    stride = w * bpp
    rows, prev = [], bytearray(stride)
    for y in range(h):
        f = raw[y * (stride + 1)]
        cur = bytearray(raw[y * (stride + 1) + 1:(y + 1) * (stride + 1)])
        for i in range(stride):
            a = cur[i - bpp] if i >= bpp else 0
            b = prev[i]
            c = prev[i - bpp] if i >= bpp else 0
            if f == 1:
                cur[i] = (cur[i] + a) & 255
            elif f == 2:
                cur[i] = (cur[i] + b) & 255
            elif f == 3:
                cur[i] = (cur[i] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                cur[i] = (cur[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        prev = cur
        if bpp == 4:
            rgb = bytearray(w * 3)
            rgb[0::3], rgb[1::3], rgb[2::3] = cur[0::4], cur[1::4], cur[2::4]
            rows.append(bytes(rgb))
        else:
            rows.append(bytes(cur))
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
