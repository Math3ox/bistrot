"""Compare les décisions du bot « difficile » (Rust : test bot_dump -> 421_bot.bin) à la stratégie Python (gen_tables.py)."""
import os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen_tables as G

OUT = os.path.join(HERE, '421_bot.bin')
if os.path.exists(OUT):
    os.remove(OUT)
subprocess.run(['cargo', 'test', '--release', '--quiet', '--', '--ignored', 'bot_dump'], cwd=HERE, check=True)
data = open(OUT, 'rb').read()
print('octets recus :', len(data), '(attendu 5616)')

scen = [(0, 0, 1, 3), (0, 0, 2, 3)]
for tr, tk in [(1000, 8), (900, 7), (813, 6), (806, 3), (706, 2), (665, 1), (541, 1), (1, 1)]:
    for thr, lim in ((1, 2), (1, 3), (2, 3)):
        scen.append((tr, tk, thr, lim))
bad = 0
pos = 0
for tr, tk, thr, lim in scen:
    diff = 0
    for n in range(216):
        hand = (n // 36 + 1, (n // 6) % 6 + 1, n % 6 + 1)
        if tr == 0:
            exp = G.policy_starter(hand, thr)
        else:
            exp = G.policy_follower(hand, thr, lim, tr, tk)
        got = data[pos] if pos < len(data) else -1
        pos += 1
        if got != exp:
            diff += 1
            if diff <= 2:
                print('  ecart : cible %d/%d lancers %d/%d main %s : rust=%s python=%s' % (tr, tk, thr, lim, hand, got, exp))
    bad += diff
    if diff:
        print('scenario cible=%d lancers=%d/%d : %d ecarts sur 216' % (tr, thr, lim, diff))
print('TOTAL : %d ecarts sur %d decisions' % (bad, pos))
sys.exit(1 if bad else 0)
