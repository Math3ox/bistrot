"""Vérificateur indépendant du rami 51 : relit sim.log (rami -sim ... 1) et refait toutes les règles de son côté
(distribution, pioche, talon refait avec la défausse, validité des suites et des brelans, joker qui garde sa place,
ouverture à 51 points, ajouts et échanges seulement après avoir ouvert, carte prise à la défausse et joker repris à
poser dans le tour, fin de manche, pénalités, rami, limite de la partie).
Usage : python rami/check.py [sim.log]"""
import sys

JOKER = 104
OUVERTURE = 51


class Erreur(Exception):
    pass


def verifie(cond, msg):
    if not cond:
        raise Erreur(msg)


def joker(c):
    return c >= JOKER


def coul(c):
    return (c % 52) // 13


def rg(c):
    return (c % 52) % 13  # 0 = le 2, 12 = l'as


def val_h(h):
    """points d'ouverture d'une hauteur 1 (as bas) à 14 (as haut)"""
    if h == 1:
        return 1
    if h == 14:
        return 11
    return 10 if h >= 11 else h


def penalite(c):
    return 20 if joker(c) else val_h(rg(c) + 2)


def hauteurs(c):
    """hauteurs possibles d'une carte dans une suite"""
    return {1, 14} if rg(c) == 12 else {rg(c) + 2}


def analyse(cartes):
    """('suite', base) ou ('brelan', rang) pour une combinaison donnée dans l'ordre, sinon Erreur."""
    verifie(len(cartes) >= 3, f"combinaison trop courte : {cartes}")
    verifie(len(set(cartes)) == len(cartes), f"carte en double : {cartes}")
    verifie(sum(joker(c) for c in cartes) <= 1, f"plus d'un joker : {cartes}")
    nat = [c for c in cartes if not joker(c)]
    if all(rg(c) == rg(nat[0]) for c in nat):
        verifie(len(cartes) <= 4, f"brelan de plus de 4 cartes : {cartes}")
        verifie(len({coul(c) for c in nat}) == len(nat), f"brelan avec deux fois la même couleur : {cartes}")
        return ('brelan', rg(nat[0]))
    verifie(len({coul(c) for c in nat}) == 1, f"suite de plusieurs couleurs : {cartes}")
    bases = None
    for i, c in enumerate(cartes):
        if joker(c):
            continue
        b = {h - i for h in hauteurs(c)}
        bases = b if bases is None else bases & b
    bases = {b for b in bases if b >= 1 and b + len(cartes) - 1 <= 14}
    verifie(len(bases) == 1, f"suite mal ordonnée : {cartes}")
    verifie(len(cartes) <= 13, f"suite trop longue : {cartes}")
    return ('suite', bases.pop())


def points(cartes):
    sorte, b = analyse(cartes)
    if sorte == 'suite':
        return sum(val_h(b + i) for i in range(len(cartes)))
    return len(cartes) * val_h(b + 2)


def hauteur_joker(cartes):
    sorte, b = analyse(cartes)
    if sorte != 'suite':
        return None
    for i, c in enumerate(cartes):
        if joker(c):
            return b + i
    return None


class Partie:
    def __init__(self):
        self.np = 0
        self.limite = 0
        self.scores = []
        self.manches = 0
        self.parties = 0
        self.actions = 0
        self.ramis = 0
        self.nulles = 0
        self.fini = True

    def nouvelle_manche(self, donneur):
        self.donneur = donneur
        self.mains = [set() for _ in range(self.np)]
        self.talon = []
        self.defausse = []
        self.table = []
        self.ouvert = [False] * self.np
        self.tour = (donneur + 1) % self.np
        self.recyclages = 0
        self.phase = 'distribution'

    def debut_tour(self):
        self.phase = 'pioche'
        self.pris = None
        self.jokers_pris = set()
        self.pose_tour = 0
        self.ouvert_avant = self.ouvert[self.tour]

    def ligne(self, mots):
        t = mots[0]
        a = [int(x) for x in mots[1:] if x not in ('|', 'T', 'D')]
        if t == 'G':
            verifie(self.fini, "nouvelle partie avant la fin de la précédente")
            self.np, self.limite = a
            self.scores = [0] * self.np
            self.fini = False
            self.parties += 1
            self.phase = 'manche'
            return
        if t == 'N':
            verifie(self.phase in ('manche', 'fin'), "distribution au milieu d'une manche")
            if self.phase == 'fin':
                verifie(a[0] == (self.donneur + 1) % self.np, "mauvais donneur")
            self.nouvelle_manche(a[0])
            self.vues = set()
            return
        if t == 'M':
            p = a[0]
            verifie(len(a) - 1 == 13, "main de 13 cartes")
            self.mains[p] = set(a[1:])
            self.vues |= set(a[1:])
            return
        if t == 'T':
            self.talon = a
            return
        if t == 'D':
            self.defausse = [a[0]]
            tout = self.vues | set(self.talon) | {a[0]}
            verifie(len(tout) == 108 and len(self.talon) + 1 + 13 * self.np == 108, "jeu incomplet")
            self.debut_tour()
            self.verifie_recyclage()
            return
        if t == 'R':
            verifie(self.phase == 'recyclage', "talon refait sans raison")
            dessus = self.defausse[-1]
            verifie(sorted(a) == sorted(self.defausse[:-1]), "talon refait avec d'autres cartes que la défausse")
            self.talon = a
            self.defausse = [dessus]
            self.recyclages += 1
            self.phase = 'pioche'
            return
        if t == 'F':
            self.fin(a)
            return
        p = a[0]
        verifie(p == self.tour, f"{t} joué par {p} au lieu de {self.tour}")
        verifie(self.phase in ('pioche', 'jeu'), f"action {t} hors d'un tour")
        self.actions += 1
        main = self.mains[p]
        if t == 'P':
            verifie(self.phase == 'pioche', "deuxième pioche")
            c = a[1]
            if mots[2] == 'T':
                verifie(self.talon and self.talon[-1] == c, "carte du talon fausse")
                self.talon.pop()
            else:
                verifie(self.defausse and self.defausse[-1] == c, "carte de la défausse fausse")
                self.defausse.pop()
                self.pris = c
            main.add(c)
            self.phase = 'jeu'
            return
        verifie(self.phase == 'jeu', f"{t} avant d'avoir pioché")
        if t == 'C':
            cartes = a[1:]
            verifie(set(cartes) <= main, "pose de cartes absentes de la main")
            pts = points(cartes)
            if not self.ouvert_avant:
                self.pose_tour += pts
            reste = main - set(cartes)
            if not self.ouvert[p]:
                verifie(reste or self.pose_tour >= OUVERTURE, "main vidée sans ouvrir")
                if self.pose_tour >= OUVERTURE:
                    self.ouvert[p] = True
            self.mains[p] = reste
            self.table.append(cartes)
            self.peut_finir()
            return
        if t == 'A':
            i, cartes = a[1], a[2:]
            verifie(self.ouvert[p], "ajout avant d'avoir ouvert")
            verifie(0 <= i < len(self.table), "combinaison inconnue")
            ancien = self.table[i]
            ajout = set(cartes) - set(ancien)
            verifie(set(ancien) <= set(cartes) and ajout and ajout <= main, "ajout incohérent")
            analyse(cartes)
            if analyse(ancien)[0] == 'suite':
                verifie(analyse(cartes)[0] == 'suite', "suite devenue brelan")
                hj = hauteur_joker(ancien)
                if hj is not None:
                    jk = next(c for c in ancien if joker(c))
                    b = analyse(cartes)[1]
                    verifie(b + cartes.index(jk) == hj, "le joker a changé de place")
            else:
                verifie(analyse(cartes)[0] == 'brelan', "brelan devenu suite")
            self.mains[p] = main - ajout
            self.table[i] = cartes
            self.peut_finir()
            return
        if t == 'E':
            i, x, j, cartes = a[1], a[2], a[3], a[4:]
            verifie(self.ouvert[p], "échange avant d'avoir ouvert")
            ancien = self.table[i]
            verifie(j in ancien and joker(j) and x in main and not joker(x), "échange incohérent")
            if analyse(ancien)[0] == 'suite':
                attendu = [x if c == j else c for c in ancien]
                verifie(cartes == attendu, "le remplaçant n'est pas à la place du joker")
                verifie(analyse(cartes)[1] + cartes.index(x) == hauteur_joker(ancien), "mauvais remplaçant")
            else:
                verifie(sorted(cartes) == sorted([c for c in ancien if c != j] + [x]), "échange de brelan incohérent")
            analyse(cartes)
            main.discard(x)
            main.add(j)
            self.jokers_pris.add(j)
            self.table[i] = cartes
            return
        if t == 'X':
            c = a[1]
            verifie(c in main, "défausse d'une carte absente")
            verifie(self.pris is None or self.pris not in main, "carte de la défausse pas posée")
            verifie(not (self.jokers_pris & main), "joker repris pas reposé")
            verifie(self.ouvert[p] or self.pose_tour == 0, "pose de moins de 51 points")
            main.discard(c)
            self.defausse.append(c)
            if not main:
                self.phase = 'attente_fin'
                self.gagnant = p
                return
            self.tour = (p + 1) % self.np
            self.debut_tour()
            self.verifie_recyclage()
            return
        raise Erreur(f"ligne inconnue : {' '.join(mots)}")

    def verifie_recyclage(self):
        if not self.talon:
            if self.recyclages >= 2 or len(self.defausse) < 2:
                self.phase = 'attente_nulle'
            else:
                self.phase = 'recyclage'

    def peut_finir(self):
        p = self.tour
        if not self.mains[p]:
            verifie(self.ouvert[p], "main vide sans avoir ouvert")
            self.phase = 'attente_fin'
            self.gagnant = p

    def fin(self, a):
        g, rami = a[0], a[1]
        pen, sc = a[2:2 + self.np], a[2 + self.np:]
        if g < 0:
            verifie(self.phase == 'attente_nulle', "manche nulle sans raison")
            attendu = [0] * self.np
            self.nulles += 1
        else:
            verifie(self.phase == 'attente_fin' and g == self.gagnant, "fin de manche inattendue")
            verifie(bool(rami) == (not self.ouvert_avant), "rami mal compté")
            self.ramis += rami
            attendu = []
            for q in range(self.np):
                if q == g:
                    attendu.append(0)
                    continue
                v = sum(penalite(c) for c in self.mains[q])
                if not self.ouvert[q]:
                    v = max(v, 100)
                attendu.append(v * (2 if rami else 1))
        verifie(pen == attendu, f"pénalités {pen} au lieu de {attendu}")
        for q in range(self.np):
            self.scores[q] += attendu[q]
        verifie(sc == self.scores, f"scores {sc} au lieu de {self.scores}")
        self.manches += 1
        self.phase = 'fin'
        if max(self.scores) >= self.limite:
            self.fini = True
            self.phase = 'manche'


def main():
    nom = sys.argv[1] if len(sys.argv) > 1 else 'sim.log'
    p = Partie()
    erreurs = 0
    with open(nom, encoding='utf-8') as f:
        for n, l in enumerate(f, 1):
            mots = l.split()
            if not mots:
                continue
            try:
                p.ligne(mots)
            except Erreur as e:
                erreurs += 1
                if erreurs <= 10:
                    print(f"ligne {n} : {e}")
                if mots[0] == 'G':
                    p.fini = True
    print(f"{p.parties} parties, {p.manches} manches ({p.nulles} nulles, {p.ramis} ramis), {p.actions} actions : "
          f"{erreurs} erreur(s)")
    sys.exit(1 if erreurs else 0)


if __name__ == '__main__':
    main()
