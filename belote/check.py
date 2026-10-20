"""Vérificateur indépendant : rejoue sim.log (paquet, enchères, cartes jouées) avec une réimplémentation
des règles en Python, contrôle que chaque coup est légal et que les scores de manche sont identiques.
Usage : python check.py [sim.log]"""
import sys

PT_T = [0, 0, 14, 10, 20, 3, 4, 11]
PT_P = [0, 0, 0, 10, 2, 3, 4, 11]
OR_T = [0, 1, 6, 4, 7, 2, 3, 5]
OR_P = [0, 1, 2, 6, 3, 4, 5, 7]


def suit(c): return c >> 3
def rank(c): return c & 7


class Round:
    def __init__(self, np, dealer, deck):
        self.np, self.dealer, self.deck = np, dealer, deck
        self.hands = [[] for _ in range(np)]
        pos = 0
        counts = [3, 2] if np == 4 else [3, 3]
        for n in counts:
            for k in range(1, np + 1):
                p = (dealer + k) % np
                for _ in range(n):
                    self.hands[p].append(deck[pos]); pos += 1
        self.turned = deck[pos]; pos += 1
        self.pos = pos
        self.trump = None
        self.taker = None
        self.rnd = 1
        self.passes = 0
        self.turn = (dealer + 1) % np
        self.trick = []  # (joueur, carte)
        self.pts = [0, 0]
        self.tricks = [0, 0]
        self.ntricks = 0
        self.bel = [0, 0]
        self.belp = None
        self.void_done = False
        self.redeal = False

    def bid(self, p, choice):
        assert p == self.turn, 'ordre des enchères'
        if choice == -1:
            self.passes += 1
            if self.passes == self.np:
                if self.rnd == 1:
                    self.rnd, self.passes = 2, 0
                    self.turn = (self.dealer + 1) % self.np
                else:
                    self.redeal = True
            else:
                self.turn = (p + 1) % self.np
            return
        if self.rnd == 1:
            assert choice == suit(self.turned), 'tour 1 : couleur de la retourne'
        else:
            assert choice != suit(self.turned), 'tour 2 : autre couleur'
        self.trump, self.taker = choice, p
        self.hands[p].append(self.turned)
        if self.np == 4:
            for k in range(1, 5):
                q = (self.dealer + k) % 4
                for _ in range(2 if q == p else 3):
                    self.hands[q].append(self.deck[self.pos]); self.pos += 1
            for h in self.hands: assert len(h) == 8
        else:
            self.hands[1 - p].append(self.deck[self.pos]); self.pos += 1
        self.turn = (self.dealer + 1) % self.np
        self.compute_ann()

    # --- annonces (4 joueurs) ---
    def player_ann(self, p):
        h = self.hands[p]
        res = []
        for r in (2, 3, 4, 5, 6, 7):
            if all(s * 8 + r in h for s in range(4)):
                val = {4: 200, 2: 150}.get(r, 100)
                order = {4: 8, 2: 7, 7: 6, 3: 5, 6: 4, 5: 3}[r]
                res.append((val, 4000 + order * 10))
        for s in range(4):
            have = [r for r in range(8) if s * 8 + r in h]
            r = 0
            while r < 8:
                if r in have:
                    st = r
                    while r + 1 < 8 and (r + 1) in have: r += 1
                    ln = r - st + 1
                    if ln >= 3:
                        base, val = (3000, 100) if ln >= 5 else ((2000, 50) if ln == 4 else (1000, 20))
                        res.append((val, base + r * 10 + (5 if s == self.trump else 0)))
                r += 1
        return res

    def compute_ann(self):
        self.ann = [0, 0]
        self.annteam = None
        if self.np != 4: return
        self.anns = [self.player_ann(p) for p in range(4)]

    def eval_ann(self):
        if self.np != 4: return
        best = [0, 0]
        for p in range(4):
            for v, k in self.anns[p]:
                best[p & 1] = max(best[p & 1], k)
        if best[0] == best[1] == 0: return
        if best[0] > best[1]: t = 0
        elif best[1] > best[0]: t = 1
        else:
            m = best[0]
            t = None
            for k in range(4):
                p = (self.dealer + 1 + k) % 4
                if any(kk == m for _, kk in self.anns[p]):
                    t = p & 1; break
        self.annteam = t
        self.ann[t] = sum(v for p in range(4) if p & 1 == t for v, _ in self.anns[p])

    # --- jeu ---
    def key(self, c, led):
        if suit(c) == self.trump: return 16 + OR_T[rank(c)]
        if suit(c) == led: return 8 + OR_P[rank(c)]
        return -1

    def legal(self, p):
        h = self.hands[p]
        if not self.trick: return list(h)
        if self.np == 2 and self.pos < 32: return list(h)
        led = suit(self.trick[0][1])
        keys = [self.key(c, led) for _, c in self.trick]
        best = max(range(len(keys)), key=lambda i: keys[i])
        bestp = self.trick[best][0]
        tmax = max([k for (_, c), k in zip(self.trick, keys) if suit(c) == self.trump], default=15)
        trumps = [c for c in h if suit(c) == self.trump]
        over = [c for c in trumps if self.key(c, led) > tmax]
        if led == self.trump:
            if not trumps: return list(h)
            return over if over else trumps
        follow = [c for c in h if suit(c) == led]
        if follow: return follow
        if self.np == 4 and (bestp & 1) == (p & 1): return list(h)
        if not trumps: return list(h)
        return over if over else trumps

    def play(self, p, c):
        assert p == self.turn, 'ordre de jeu'
        assert c in self.hands[p], 'carte pas en main'
        assert c in self.legal(p), 'coup illégal : joueur %d carte %d' % (p, c)
        self.hands[p].remove(c)
        # belote
        if suit(c) == self.trump and rank(c) in (5, 6):
            other = c ^ 3
            if other in self.hands[p]:
                self.belp = p
            elif self.belp == p:
                self.bel[p & 1] += 20
                self.belp = None
        self.trick.append((p, c))
        if len(self.trick) == self.np:
            return self.collect()
        self.turn = (p + 1) % self.np

    def collect(self):
        led = suit(self.trick[0][1])
        keys = [self.key(c, led) for _, c in self.trick]
        wi = max(range(len(keys)), key=lambda i: keys[i])
        w = self.trick[wi][0]
        pts = sum((PT_T if suit(c) == self.trump else PT_P)[rank(c)] for _, c in self.trick)
        last = all(len(h) == 0 for h in self.hands) and (self.np == 4 or self.pos >= 32)
        if last: pts += 10
        self.pts[w & 1] += pts
        self.tricks[w & 1] += 1
        self.ntricks += 1
        if self.ntricks == 1: self.eval_ann()
        self.trick = []
        self.turn = w
        if self.np == 2 and self.pos < 32:
            self.hands[w].append(self.deck[self.pos]); self.pos += 1
            self.hands[1 - w].append(self.deck[self.pos]); self.pos += 1
        return last

    def score(self, pend):
        a = self.taker & 1
        b = 1 - a
        capot = None
        for t in (0, 1):
            if self.tricks[t] == self.ntricks: capot = t
        tot = [self.pts[t] + self.bel[t] + self.ann[t] + (90 if capot == t else 0) for t in (0, 1)]
        add = [0, 0]
        pend_gain = 0
        if tot[a] > tot[b]:
            made = 1
            add[a], add[b] = tot[a], tot[b]
            add[a] += pend; pend_gain = pend; pend = 0
        elif tot[a] < tot[b]:
            made = 0
            add[b] = 162 + (90 if capot == b else 0) + self.ann[0] + self.ann[1] + self.bel[b]
            add[a] = self.bel[a]
            add[b] += pend; pend_gain = pend; pend = 0
        else:
            made = 2
            add[b] = tot[b]
            pend += tot[a]
        return add, made, capot, pend, pend_gain


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else 'sim.log'
    lines = [l.split() for l in open(path)]
    rnd = None
    score = [0, 0]
    pend = 0
    nrounds = 0
    games = 0
    errors = 0
    for ln, f in enumerate(lines, 1):
        try:
            if not f: continue
            if f[0] == 'D':
                np, dealer = int(f[1]), int(f[2])
                deck = [int(x) for x in f[3:35]]
                assert sorted(deck) == list(range(32)), 'paquet invalide'
                if rnd is not None and not rnd.redeal and not getattr(rnd, 'done', False):
                    raise AssertionError('nouvelle donne sans fin de manche')
                # un nouveau match commence quand le dernier S a franchi la cible
                rnd = Round(np, dealer, deck)
            elif f[0] == 'B':
                rnd.bid(int(f[1]), int(f[2]))
            elif f[0] == 'P':
                p, c = int(f[1]), int(f[2])
                rest = sorted(int(x) for x in f[4:]) if len(f) > 3 and f[3] == '|' else None
                if rest is not None:
                    exp = sorted(x for x in rnd.hands[p] if x != c)
                    assert rest == exp, 'main restante asm %s py %s' % (rest, exp)
                last = rnd.play(p, c)
            elif f[0] == 'S':
                assert rnd.ntricks == (8 if rnd.np == 4 else 16), 'nombre de plis %d' % rnd.ntricks
                add, made, capot, pend, pg = rnd.score(pend if True else 0)
                got = [int(x) for x in f[1:8]]
                # got = add0 add1 score0 score1 made capot pend
                assert [got[0], got[1]] == add, 'points de manche : asm %s py %s' % (got[:2], add)
                assert got[4] == made, 'made %d vs %d' % (got[4], made)
                assert got[5] == (-1 if capot is None else capot), 'capot'
                score[0] += add[0]; score[1] += add[1]
                assert [got[2], got[3]] == score, 'cumul : asm %s py %s' % (got[2:4], score)
                assert sum(rnd.pts) == 162, 'somme des points %s' % rnd.pts
                nrounds += 1
                rnd.done = True
                if max(score) >= 1000:
                    games += 1
                    score = [0, 0]
                    pend = 0
        except AssertionError as e:
            errors += 1
            print('ligne %d %s : %s' % (ln, ' '.join(f), e))
            if errors > 10: break
    print('manches vérifiées : %d, parties : %d, erreurs : %d' % (nrounds, games, errors))


main()
