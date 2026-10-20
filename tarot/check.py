"""Vérificateur indépendant du tarot : relit sim.log (tarot -sim ... 1) et refait toutes les règles de son côté
(enchères, appel, écart, cartes jouables, poignées, gagnant des plis, excuse, petit au bout, chelem, décompte).
Usage : python tarot/check.py [sim.log]"""
import sys

EXCUSE, PETIT, VINGT_ET_UN = 77, 56, 76
BOUTS = {PETIT, VINGT_ET_UN, EXCUSE}
CIBLES = [56, 51, 41, 36]
MULT = [0, 1, 2, 4, 6]
PRIMES = [0, 20, 30, 40]
SEUILS = {3: [13, 15, 18], 4: [10, 13, 15], 5: [8, 10, 13]}


def fam(c):
    return c // 14 if c < 56 else (4 if c < 77 else 5)


def force(c):
    return c % 14 if c < 56 else (c - 55 if c < 77 else 0)


def demi(c):
    if c in BOUTS:
        return 9
    if c >= 56:
        return 1
    return {13: 9, 12: 7, 11: 5, 10: 3}.get(c % 14, 1)


def roi(c):
    return c < 56 and c % 14 == 13


class Erreur(Exception):
    pass


def verifie(cond, msg):
    if not cond:
        raise Erreur(msg)


def legales(main, pli, np, premier_pli, appel):
    """pli : cartes dans l'ordre joué."""
    if not pli:
        if np == 5 and premier_pli and appel is not None:
            interdites = {c for c in main if fam(c) == fam(appel) and c != appel}
            reste = set(main) - interdites
            if reste:
                return reste
        return set(main)
    dem = next((fam(c) for c in pli if c != EXCUSE), None)
    if dem is None:
        return set(main)
    exc = {EXCUSE} & set(main)
    atouts = {c for c in main if fam(c) == 4}

    def couper():
        if not atouts:
            return set(main)
        mx = max([force(c) for c in pli if fam(c) == 4], default=0)
        dessus = {c for c in atouts if force(c) > mx}
        return (dessus or atouts) | exc

    if dem == 4:
        return couper()
    m = {c for c in main if fam(c) == dem}
    return (m | exc) if m else couper()


def main():
    nom = sys.argv[1] if len(sys.argv) > 1 else 'sim.log'
    lignes = [l.split() for l in open(nom, encoding='utf8') if l.strip()]
    i = 0
    donnes = cartes_jouees = 0
    scores = None
    parties = 0
    while i < len(lignes):
        l = lignes[i]
        if l[0] == 'G':
            scores = None
            parties += 1
            i += 1
            continue
        verifie(l[0] == 'N', f'ligne {i + 1} : N attendu, {l}')
        np, donneur = int(l[1]), int(l[2])
        if scores is None:
            scores = [0] * np
        tm = {3: 24, 4: 18, 5: 15}[np]
        k = 3 if np == 5 else 6
        mains = []
        for p in range(np):
            l = lignes[i + 1 + p]
            verifie(l[0] == 'M' and int(l[1]) == p, f'main {p} attendue')
            mains.append(set(map(int, l[2:])))
            verifie(len(mains[p]) == tm, 'taille de main')
            atouts = {c for c in mains[p] if fam(c) >= 4}
            verifie(atouts != {PETIT}, 'petit sec non redonné')
        l = lignes[i + 1 + np]
        verifie(l[0] == 'C', 'chien attendu')
        chien = set(map(int, l[1:]))
        tout = set().union(*mains) | chien
        verifie(len(chien) == k and tout == set(range(78)), 'distribution incomplète')
        i += 2 + np
        # enchères
        encheres = [-1] * np
        tour = (donneur + 1) % np
        for _ in range(np):
            l = lignes[i]
            verifie(l[0] == 'B' and int(l[1]) == tour, f'ligne {i + 1} : enchère de {tour} attendue, {l}')
            c = int(l[2])
            verifie(c == 0 or c > max(max(encheres), 0), f'ligne {i + 1} : enchère trop basse')
            encheres[tour] = c
            tour = (tour + 1) % np
            i += 1
        if max(encheres) == 0:
            verifie(lignes[i][0] == 'X', 'donne blanche non signalée')
            i += 1
            continue
        contrat = max(encheres)
        preneur = encheres.index(contrat)
        appel, partenaire = None, -1
        if np == 5:
            l = lignes[i]
            verifie(l[0] == 'A', 'appel attendu')
            appel = int(l[1])
            h = mains[preneur]
            for rang in (13, 12, 11, 10):
                m = {s * 14 + rang for s in range(4)}
                if not m <= h:
                    break
            verifie(appel in m, f'ligne {i + 1} : carte appelée interdite')
            porteurs = [q for q in range(np) if q != preneur and appel in mains[q]]
            partenaire = porteurs[0] if porteurs else -1
            verifie(int(l[2]) == partenaire, 'partenaire faux')
            i += 1
        ecart = set()
        if contrat <= 2:
            mains[preneur] |= chien
            l = lignes[i]
            verifie(l[0] == 'E', 'écart attendu')
            ecart = set(map(int, l[1:]))
            h = mains[preneur]
            normales = {c for c in h if fam(c) < 4 and not roi(c)}
            verifie(len(ecart) == k and ecart <= h, 'écart : taille ou cartes absentes')
            verifie(not any(roi(c) or c in BOUTS for c in ecart), 'écart : roi ou bout')
            if any(fam(c) == 4 for c in ecart):
                verifie(len(normales) < k and normales <= ecart, 'écart : atout sans nécessité')
            mains[preneur] -= ecart
            i += 1
        att = lambda p: p == preneur or p == partenaire
        # jeu
        entame = (donneur + 1) % np
        piles = [set() for _ in range(np)]
        plis_gagnes = [0] * np
        poignees = [0] * np
        excuse = None
        petit_bout = None
        premiere = set()
        for n_pli in range(tm):
            pli = []
            for j in range(np):
                p = (entame + j) % np
                l = lignes[i]
                verifie(l[0] == 'P' and int(l[1]) == p, f'ligne {i + 1} : carte de {p} attendue, {l}')
                c, pg = int(l[2]), int(l[3])
                verifie(c in legales(mains[p], pli, np, n_pli == 0, appel), f'ligne {i + 1} : carte {c} interdite')
                if p not in premiere:
                    premiere.add(p)
                    n = sum(1 for x in mains[p] if fam(x) >= 4)
                    possible = sum(1 for s in SEUILS[np] if n >= s)
                    verifie(pg <= possible, f'ligne {i + 1} : poignée impossible')
                    poignees[p] = pg
                else:
                    verifie(pg == 0, 'poignée hors de la première carte')
                mains[p].discard(c)
                pli.append(c)
                cartes_jouees += 1
                i += 1
            # gagnant
            dem = next((fam(c) for c in pli if c != EXCUSE), None)

            def cle(c):
                if c == EXCUSE:
                    return -2
                if fam(c) == 4:
                    return 100 + force(c)
                return force(c) if fam(c) == dem else -1
            js = max(range(np), key=lambda j: cle(pli[j]))
            g = (entame + js) % np
            dernier = n_pli == tm - 1
            if EXCUSE in pli:
                o = (entame + pli.index(EXCUSE)) % np
                autres = [q for q in range(np) if att(q) != att(o)]
                if dernier and n_pli > 0 and all(plis_gagnes[q] == 0 for q in autres):
                    g = o
                excuse = (o, g, dernier)
            piles[g] |= set(pli) - {EXCUSE}
            plis_gagnes[g] += 1
            if dernier and PETIT in pli:
                petit_bout = g
            entame = g
        # décompte
        cartes = set()
        for p in range(np):
            if att(p):
                cartes |= piles[p]
        if contrat <= 2:
            cartes |= ecart
        elif contrat == 3:
            cartes |= chien
        echange = 0
        if excuse:
            o, g, dernier = excuse
            if att(o) == att(g) or not dernier:
                if att(o):
                    cartes.add(EXCUSE)
                if att(o) != att(g):
                    echange = -1 if att(o) else 1
            elif att(g):
                cartes.add(EXCUSE)
        d_att = sum(demi(c) for c in cartes) + echange
        nbouts = len(cartes & BOUTS)
        diff = d_att - 2 * CIBLES[nbouts]
        signe = 1 if diff >= 0 else -1
        ecart_pts = (abs(diff) + 1) // 2
        pab = 0 if petit_bout is None else (1 if att(petit_bout) else -1)
        prime = sum(PRIMES[x] for x in poignees) * signe
        plis_att = sum(plis_gagnes[p] for p in range(np) if att(p))
        chelem = 200 if plis_att == tm else (-200 if plis_att == 0 else 0)
        unite = ((25 + ecart_pts) * signe + 10 * pab) * MULT[contrat] + prime + chelem
        seul = partenaire < 0
        gains = []
        for p in range(np):
            if p == preneur:
                gains.append(unite * {3: 2, 4: 3, 5: 4 if seul else 2}[np])
            else:
                gains.append(unite if att(p) else -unite)
        verifie(sum(gains) == 0, 'gains non équilibrés')
        for p in range(np):
            scores[p] += gains[p]
        l = lignes[i]
        verifie(l[0] == 'S', f'ligne {i + 1} : bilan attendu')
        sep = l.index('|')
        sep2 = l.index('|', sep + 1)
        attendu = [contrat, preneur, partenaire, d_att, nbouts, unite]
        verifie(list(map(int, l[1:sep])) == attendu, f'ligne {i + 1} : bilan {l[1:sep]} au lieu de {attendu}')
        verifie(list(map(int, l[sep + 1:sep2])) == gains, f'ligne {i + 1} : gains {l[sep + 1:sep2]} au lieu de {gains}')
        verifie(list(map(int, l[sep2 + 1:])) == scores, f'ligne {i + 1} : scores')
        i += 1
        donnes += 1
    print(f'ok : {parties} parties, {donnes} donnes, {cartes_jouees} cartes vérifiées')


if __name__ == '__main__':
    try:
        main()
    except Erreur as e:
        print('ERREUR :', e)
        sys.exit(1)
