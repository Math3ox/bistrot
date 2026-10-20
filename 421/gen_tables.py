"""Stratégie de référence du bot « difficile » (programmation dynamique, utilité en jetons), en Python.
Les 56 mains triées (a <= b <= c) servent d'états. Le jeu (src/ai.rs) fait le même calcul ;
bottest.py compare les décisions des deux (policy_starter / policy_follower)."""
import itertools, os

H = list(itertools.combinations_with_replacement(range(1, 7), 3))
HID = {h: i for i, h in enumerate(H)}


def rank(d):
    a, b, c = sorted(d, reverse=True)
    if (a, b, c) == (4, 2, 1): return 1000, 8
    if a == 1: return 900, 7
    if a == c: return 800 + 2 * a, a
    if b == 1 and c == 1: return 801 + 2 * a, a
    if a == b + 1 and b == c + 1: return 700 + a, 2
    if (a, b, c) == (2, 2, 1): return 1, 1
    return a * 100 + b * 10 + c, 1


RK = [rank(h)[0] for h in H]
TOK = [rank(h)[1] for h in H]
TOKT = {rank(h)[0]: rank(h)[1] for h in H}


def sid(hand):
    return HID[tuple(sorted(hand))]


def outcomes(hand, mask):
    idx = [i for i in range(3) if mask >> i & 1]
    res = []
    for combo in itertools.product(range(1, 7), repeat=len(idx)):
        h = list(hand)
        for i, v in zip(idx, combo): h[i] = v
        res.append(sid(h))
    return res


OUT = [[outcomes(H[i], m) for m in range(8)] for i in range(56)]
ALL216 = [sid(t) for t in itertools.product(range(1, 7), repeat=3)]


def follower_V(target, target_tok, kmax):
    V0 = [TOK[i] if RK[i] > target else (0.0 if RK[i] == target else -target_tok) for i in range(56)]
    Vs = [V0]
    for k in range(1, kmax + 1):
        prev = Vs[-1]
        cur = []
        for i in range(56):
            best = V0[i]
            for m in range(1, 8):
                o = OUT[i][m]
                e = sum(prev[j] for j in o) / len(o)
                if e > best: best = e
            cur.append(best)
        Vs.append(cur)
    return Vs


def follower_payoff(target, target_tok, t):
    Vs = follower_V(target, target_tok, t - 1)
    return sum(Vs[t - 1][j] for j in ALL216) / 216.0


W = {t: [-follower_payoff(RK[i], TOK[i], t) for i in range(56)] for t in (1, 2, 3)}
VS = {3: W[3][:]}
for t in (2, 1):
    VS[t] = []
    for i in range(56):
        best = W[t][i]
        for m in range(1, 8):
            o = OUT[i][m]
            e = sum(VS[t + 1][j] for j in o) / len(o)
            if e > best: best = e
        VS[t].append(best)


def policy_starter(hand, throws):
    """masque (bits 0-2 = positions de la vraie main) du premier joueur ; 0 = il s'arrête (3 lancers max)"""
    if throws >= 3: return 0
    i = sid(hand)
    best, bm = W[throws][i], 0
    nxt = VS[throws + 1]
    for m in range(1, 8):
        o = outcomes(hand, m)
        e = sum(nxt[j] for j in o) / len(o)
        if e > best + 1e-6: best, bm = e, m
    return bm


_cache = {}


def policy_follower(hand, throws, lim, target, target_tok):
    if throws >= lim: return 0
    L = lim - throws
    key = (target, target_tok, L)
    if key not in _cache: _cache[key] = follower_V(target, target_tok, L)
    Vs = _cache[key]
    i = sid(hand)
    best, bm = Vs[0][i], 0
    for m in range(1, 8):
        o = outcomes(hand, m)
        e = sum(Vs[L - 1][j] for j in o) / len(o)
        if e > best + 1e-6: best, bm = e, m
    return bm

