"""Capture l'écran principal en PNG (test du plein écran). Usage : python screencap.py sortie.png [reduction]"""
import ctypes, struct, sys, zlib
from ctypes import wintypes

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
user32.SetProcessDPIAware()
W, H = user32.GetSystemMetrics(0), user32.GetSystemMetrics(1)
hdc = user32.GetDC(0)
mdc = gdi32.CreateCompatibleDC(hdc)
bmp = gdi32.CreateCompatibleBitmap(hdc, W, H)
gdi32.SelectObject(mdc, bmp)
gdi32.BitBlt(mdc, 0, 0, W, H, hdc, 0, 0, 0x00CC0020)


class BIH(ctypes.Structure):
    _fields_ = [('biSize', wintypes.DWORD), ('biWidth', wintypes.LONG), ('biHeight', wintypes.LONG),
                ('biPlanes', wintypes.WORD), ('biBitCount', wintypes.WORD), ('biCompression', wintypes.DWORD),
                ('biSizeImage', wintypes.DWORD), ('x', wintypes.LONG), ('y', wintypes.LONG),
                ('c', wintypes.DWORD), ('i', wintypes.DWORD)]


bi = BIH(40, W, -H, 1, 32, 0, W * H * 4, 0, 0, 0, 0)
buf = (ctypes.c_ubyte * (W * H * 4))()
gdi32.GetDIBits(mdc, bmp, 0, H, buf, ctypes.byref(bi), 0)
data = bytes(buf)
k = int(sys.argv[2]) if len(sys.argv) > 2 else 1
rows = []
for y in range(0, H, k):
    row = data[y * W * 4:(y + 1) * W * 4]
    rgb = bytearray((W // k) * 3)
    px = row[0::4 * k], row[1::4 * k], row[2::4 * k]
    n = W // k
    rgb[0::3] = px[2][:n]; rgb[1::3] = px[1][:n]; rgb[2::3] = px[0][:n]
    rows.append(b'\x00' + bytes(rgb))
raw = b''.join(rows)


def chunk(t, c):
    x = struct.pack('>I', len(c)) + t + c
    return x + struct.pack('>I', zlib.crc32(t + c) & 0xffffffff)


png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', W // k, len(rows), 8, 2, 0, 0, 0)) \
    + chunk(b'IDAT', zlib.compress(raw, 6)) + chunk(b'IEND', b'')
open(sys.argv[1], 'wb').write(png)
print('ecran %dx%d -> %s' % (W, H, sys.argv[1]))
