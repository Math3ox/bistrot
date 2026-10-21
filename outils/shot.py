"""Pilote un jeu (target/release/<jeu>.exe) pour les tests : lance, envoie des touches/clics, capture (F12 -> PNG).
Seule la fenêtre du processus lancé est pilotée (jamais un jeu déjà ouvert). Les réglages et captures sont dans
target/release, pas à côté des exe distribués.
Usage : python outils/shot.py <belote|421|tarot|rami> sortie.png [actions...]
  actions : wait:<s>  key:<code hex>  char:<lettres>  click:<x>,<y> (pixels de la fenêtre)  shot:<fichier.png>
  (ex. char:G wait:1 shot:galerie.png)"""
import ctypes, os, shutil, subprocess, sys, time

user32 = ctypes.windll.user32
user32.SetProcessDPIAware()
JEU = sys.argv[1]
HERE = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'target', 'release')
EXE = os.path.join(HERE, JEU + '.exe')
SHOT = os.path.join(HERE, JEU + '_shot.png')
TITLE = {'belote': 'Belote - jeu du bistro en 3D isométrique', '421': '421 - jeu du bistro en 3D isométrique',
         'tarot': 'Tarot - jeu du bistro en 3D isométrique', 'rami': 'Rami - jeu du bistro en 3D isométrique'}[JEU]
sys.argv = sys.argv[1:]


def own_window(pid):
    """Fenêtre principale du processus lancé (jamais celle d'un jeu déjà ouvert par l'utilisateur)."""
    found = []
    PROC = ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_void_p)

    def cb(h, _):
        owner = ctypes.c_ulong()
        user32.GetWindowThreadProcessId(ctypes.c_void_p(h), ctypes.byref(owner))
        buf = ctypes.create_unicode_buffer(200)
        user32.GetWindowTextW(ctypes.c_void_p(h), buf, 200)
        if owner.value == pid and buf.value == TITLE and user32.IsWindowVisible(ctypes.c_void_p(h)):
            found.append(h)
        return True
    user32.EnumWindows(PROC(cb), 0)
    return found[0] if found else 0


def key(hwnd, vk):
    # la fenêtre lit le code de touche physique (scan code) dans lParam
    sc = user32.MapVirtualKeyW(vk, 0)
    user32.PostMessageW(hwnd, 0x100, vk, (sc << 16) | 1)
    time.sleep(0.05)
    user32.PostMessageW(hwnd, 0x101, vk, (sc << 16) | 0xC0000001)


def capture(hwnd, dst):
    if os.path.exists(SHOT):
        os.remove(SHOT)
    key(hwnd, 0x7B)
    for _ in range(30):
        time.sleep(0.1)
        if os.path.exists(SHOT) and os.path.getsize(SHOT) > 0:
            time.sleep(0.2)
            shutil.move(SHOT, dst)
            print('ok', dst)
            return
    print('pas de capture', dst)


def main():
    out = sys.argv[1]
    acts = sys.argv[2:] or ['wait:1.0']
    p = subprocess.Popen([EXE], cwd=HERE)
    hwnd = 0
    for _ in range(80):
        hwnd = own_window(p.pid)
        if hwnd:
            break
        time.sleep(0.1)
    if not hwnd:
        print('fenetre introuvable'); p.kill(); return
    time.sleep(0.8)
    for a in acts:
        k, v = a.split(':', 1)
        if k == 'wait':
            time.sleep(float(v))
        elif k == 'key':
            key(hwnd, int(v, 16))
        elif k == 'char':
            for ch in v:
                key(hwnd, user32.VkKeyScanW(ord(ch)) & 0xFF)  # touche réelle : la fenêtre en déduit le caractère
                time.sleep(0.05)
        elif k == 'shot':
            capture(hwnd, v)
        elif k == 'click':
            x, y = [int(t) for t in v.split(',')]
            user32.PostMessageW(hwnd, 0x200, 0, (y << 16) | x)
            user32.PostMessageW(hwnd, 0x201, 1, (y << 16) | x)
            time.sleep(0.05)
            user32.PostMessageW(hwnd, 0x202, 0, (y << 16) | x)
    time.sleep(0.2)
    capture(hwnd, out)
    user32.PostMessageW(hwnd, 0x10, 0, 0)
    try:
        p.wait(timeout=3)
    except Exception:
        p.kill()


main()
