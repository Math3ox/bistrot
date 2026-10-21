//! Bots du rami. Ils ne trichent pas : ils ne voient que leur main, la table, la défausse (et ce que chacun y a
//! pris ou jeté), le nombre de cartes des autres.
//!
//! - Recherche exacte du meilleur ensemble de combinaisons disjointes d'une main (points d'ouverture, ou
//!   pénalités évitées une fois ouvert), avec jokers : à chaque étape, la plus petite carte restante est laissée
//!   de côté ou mise dans une des combinaisons possibles avec elle ; les restes déjà vus sont mémorisés.
//! - Tour d'un bot : il ouvre dès qu'il a 51 points ; ouvert, il pose tout ce qu'il peut, complète les
//!   combinaisons de la table, récupère les jokers qu'il peut reposer aussitôt, ne garde un joker que s'il lui
//!   reste plus de 3 cartes. Il ne prend la défausse que s'il peut la poser dans le tour (vérifié en jouant le
//!   tour sur une copie).
//! - Défausse : la carte la moins utile (paires, voisines de la même couleur, combinaisons cachées en attendant
//!   d'ouvrir), les grosses cartes d'abord quand un adversaire approche de la fin, jamais une carte que le
//!   joueur suivant pourrait poser sur la table s'il a ouvert.
//! - facile : ne prend la défausse que pour ouvrir, ne reprend pas les jokers, défausse parfois au hasard ;
//!   normal : tout ce qui précède ; difficile : compte en plus les cartes encore en jeu (une paire dont les deux
//!   cartes manquantes sont sorties ne vaut plus rien) et se méfie de ce que le joueur suivant ramasse.

use crate::engine::*;
use std::collections::HashMap;

/// Une action du joueur dont c'est le tour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// vrai : talon ; faux : défausse
    Pioche(bool),
    Poser(Cartes),
    Ajouter(Cartes, usize),
    Echanger(u8, usize),
    Defausser(u8),
}

/// Ce que vaut une combinaison dans la recherche.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Objectif {
    /// points d'ouverture
    Points,
    /// pénalités évitées (le joker compte 20)
    Penalites,
}

/// Carte de couleur `s` et de hauteur h (1 ou 14 = as), premier exemplaire.
fn carte_de(s: u8, h: u8) -> u8 {
    s * 13 + if h == 1 { 12 } else { h - 2 }
}

/// Un exemplaire de la carte n (0 à 51) présent dans m.
fn exemplaire(m: Cartes, n: u8) -> Option<u8> {
    [n, n + 52].into_iter().find(|&x| m & bit(x) != 0)
}

/// Combinaisons possibles contenant la carte c (pas un joker) avec des cartes de m.
fn combis_avec(c: u8, m: Cartes, out: &mut Vec<Cartes>) {
    let joker = (m & JOKERS != 0).then(|| (m & JOKERS).trailing_zeros() as u8);
    let (r, s) = (rang(c), couleur(c));
    // brelans et carrés
    let autres: Vec<u8> = (0..4).filter(|&t| t != s).filter_map(|t| exemplaire(m, t * 13 + r)).collect();
    for k in 1u32..(1 << autres.len()) {
        let sub = (0..autres.len()).filter(|&i| k >> i & 1 == 1).fold(0, |a, i| a | bit(autres[i]));
        let n = k.count_ones();
        if n >= 2 {
            out.push(bit(c) | sub);
        }
        if let Some(j) = joker {
            if n <= 2 {
                out.push(bit(c) | sub | bit(j));
            }
        }
    }
    // suites : toutes les plages qui contiennent c, avec au plus une carte manquante remplacée par le joker
    let hauteurs: &[u8] = if r == 12 { &[14, 1] } else { &[r + 2] };
    for &hc in hauteurs {
        for lo in hc.saturating_sub(12).max(1)..=hc {
            for hi in hc.max(lo + 2)..=(lo + 12).min(14) {
                let mut sel = bit(c);
                let mut manque = 0;
                for h in lo..=hi {
                    if h == hc {
                        continue;
                    }
                    match exemplaire(m & !bit(c), carte_de(s, h)) {
                        Some(x) => sel |= bit(x),
                        None => manque += 1,
                    }
                    if manque > 1 {
                        break;
                    }
                }
                if manque == 0 {
                    out.push(sel);
                } else if manque == 1 {
                    if let Some(j) = joker {
                        out.push(sel | bit(j));
                    }
                }
            }
        }
    }
}

fn valeur(m: Cartes, c: &Combi, obj: Objectif) -> i32 {
    let v = match obj {
        Objectif::Points => c.points(),
        Objectif::Penalites => penalites(m),
    };
    v * 16 + nb(m) as i32
}

fn chercher(reste: Cartes, obj: Objectif, memo: &mut HashMap<Cartes, (i32, Vec<Cartes>)>) -> (i32, Vec<Cartes>) {
    let nat = reste & !JOKERS;
    if nat == 0 || nb(reste) < 3 {
        return (0, Vec::new());
    }
    if let Some(r) = memo.get(&reste) {
        return r.clone();
    }
    let c = nat.trailing_zeros() as u8;
    let mut best = chercher(reste & !bit(c), obj, memo);
    let mut cands = Vec::new();
    combis_avec(c, reste, &mut cands);
    cands.sort_unstable();
    cands.dedup();
    for m in cands {
        let Some(cb) = former(m) else { continue };
        let (v, l) = chercher(reste & !m, obj, memo);
        let v = v + valeur(m, &cb, obj);
        if v > best.0 {
            let mut l = l;
            l.push(m);
            best = (v, l);
        }
    }
    memo.insert(reste, best.clone());
    best
}

/// Meilleur ensemble de combinaisons disjointes tirées de la main.
pub fn meilleures(main: Cartes, obj: Objectif) -> Vec<Cartes> {
    let mut memo = HashMap::new();
    chercher(main, obj, &mut memo).1
}

/// Comme `meilleures`, avec une mémoire gardée d'un appel à l'autre (mains voisines : les restes se retrouvent).
pub type Memo = HashMap<Cartes, (i32, Vec<Cartes>)>;

fn meilleures_memo(main: Cartes, obj: Objectif, memo: &mut Memo) -> Vec<Cartes> {
    chercher(main, obj, memo).1
}

/// Meilleur ensemble de combinaisons dont une contient la carte c (s'il en existe).
pub fn meilleures_avec(main: Cartes, c: u8, obj: Objectif) -> Vec<Cartes> {
    let mut memo = HashMap::new();
    let mut cands = Vec::new();
    if est_joker(c) {
        for x in liste(main & !JOKERS) {
            combis_avec(x, main, &mut cands);
        }
        cands.retain(|&m| m & bit(c) != 0);
    } else {
        combis_avec(c, main, &mut cands);
    }
    let mut best: (i32, Vec<Cartes>) = (i32::MIN, Vec::new());
    for m in cands {
        let Some(cb) = former(m) else { continue };
        let (v, mut l) = chercher(main & !m, obj, &mut memo);
        let v = v + valeur(m, &cb, obj);
        if v > best.0 {
            l.push(m);
            best = (v, l);
        }
    }
    if best.1.is_empty() {
        return meilleures(main, obj);
    }
    best.1
}

/// Points d'ouverture d'un ensemble de combinaisons.
pub fn points(combis: &[Cartes]) -> i32 {
    combis.iter().filter_map(|&m| former(m)).map(|c| c.points()).sum()
}

impl Moteur {
    pub fn niveau(&self, p: usize) -> i8 {
        if self.niveaux[p] < 0 {
            1
        } else {
            self.niveaux[p].min(2)
        }
    }

    pub fn appliquer(&mut self, a: Action) -> bool {
        match a {
            Action::Pioche(t) => self.piocher(t),
            Action::Poser(m) => self.poser(m),
            Action::Ajouter(m, i) => self.ajouter(m, i),
            Action::Echanger(x, i) => self.echanger(x, i),
            Action::Defausser(c) => self.defausser(c),
        }
    }

    fn copie(&self, p: usize) -> Moteur {
        let mut m = self.clone();
        m.simule = true;
        m.journal = Vec::new();
        // hasard propre à ce tour (sinon une copie repartirait toujours du même tirage)
        m.rng ^= (self.d.tours as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (p as u64) << 40 ^ nb(self.d.mains[p]) as u64;
        m.rng |= 1;
        m
    }

    /// Pioche du bot p : vrai = talon. Il prend la défausse s'il peut la poser dans le tour (essai sur une copie).
    pub fn bot_pioche(&self, p: usize) -> bool {
        if self.d.defausse.is_empty() {
            return true;
        }
        let mut m = self.copie(p);
        if !m.piocher(false) {
            return true;
        }
        let ouvert = m.d.ouvert[p];
        m.jouer_bot(p, true);
        if m.d.phase == PH_JEU {
            return true; // la carte ne servirait pas ce tour-ci
        }
        // facile : seulement pour ouvrir ou finir
        self.niveau(p) == 0 && ouvert && m.d.phase == PH_PIOCHE
    }

    /// Tour du bot p après la pioche : les actions à jouer, dans l'ordre (la défausse en dernier).
    pub fn bot_jeu(&self, p: usize) -> Vec<Action> {
        self.copie(p).jouer_bot(p, false)
    }

    /// Joue le tour du bot sur cette copie (après la pioche) et renvoie les actions jouées ; `essai` : défausse
    /// choisie vite (seule compte la validité du tour).
    fn jouer_bot(&mut self, p: usize, essai: bool) -> Vec<Action> {
        let lvl = self.niveau(p);
        let mut acts = Vec::new();
        let faire = |m: &mut Moteur, a: Action, acts: &mut Vec<Action>| -> bool {
            let ok = m.appliquer(a);
            if ok {
                acts.push(a);
            }
            ok
        };
        let pris = (self.d.pris >= 0).then_some(self.d.pris as u8);
        if !self.d.ouvert[p] {
            let combis = match pris {
                Some(c) => meilleures_avec(self.d.mains[p], c, Objectif::Points),
                None => meilleures(self.d.mains[p], Objectif::Points),
            };
            if points(&combis) >= OUVERTURE {
                // les plus grosses d'abord : la dernière, celle qui vide peut-être la main, arrive après les 51
                let mut combis = combis;
                combis.sort_by_key(|&m| -former(m).map_or(0, |c| c.points()));
                for m in combis {
                    faire(self, Action::Poser(m), &mut acts);
                }
            }
        }
        while self.d.ouvert[p] && self.d.phase == PH_JEU {
            let mut change = false;
            let main = self.d.mains[p];
            let combis = match pris.filter(|&c| main & bit(c) != 0) {
                Some(c) => meilleures_avec(main, c, Objectif::Penalites),
                None => meilleures(main, Objectif::Penalites),
            };
            for m in combis {
                change |= faire(self, Action::Poser(m), &mut acts);
            }
            if self.d.phase != PH_JEU {
                break;
            }
            for c in liste(self.d.mains[p]) {
                if est_joker(c) && nb(self.d.mains[p]) > 3 {
                    continue;
                }
                if self.d.mains[p] & bit(c) == 0 {
                    continue;
                }
                for i in 0..self.d.table.len() {
                    if faire(self, Action::Ajouter(bit(c), i), &mut acts) {
                        change = true;
                        break;
                    }
                }
                if self.d.phase != PH_JEU {
                    break;
                }
            }
            if self.d.phase != PH_JEU {
                break;
            }
            if !change && lvl >= 1 {
                change = self.reprendre_joker(p, &mut acts);
            }
            if !change {
                break;
            }
        }
        if self.d.phase == PH_JEU {
            let c = self.choisir_defausse(p, if essai { lvl.min(1) } else { lvl });
            faire(self, Action::Defausser(c), &mut acts);
        }
        acts
    }

    /// Échange une carte contre un joker de la table s'il peut le reposer aussitôt (combinaison nouvelle, ou
    /// ajout ailleurs) ; renvoie vrai si c'est fait.
    fn reprendre_joker(&mut self, p: usize, acts: &mut Vec<Action>) -> bool {
        for i in 0..self.d.table.len() {
            if self.d.table[i].joker().is_none() {
                continue;
            }
            for x in liste(self.d.mains[p] & !JOKERS) {
                let Some((_, j)) = remplacer_joker(&self.d.table[i], x) else { continue };
                let mut essai = self.clone();
                if !essai.echanger(x, i) {
                    continue;
                }
                let suite = meilleures(essai.d.mains[p], Objectif::Penalites)
                    .into_iter()
                    .find(|&m| m & bit(j) != 0)
                    .map(Action::Poser)
                    .or_else(|| (0..essai.d.table.len()).filter(|&k| k != i).find(|&k| completer(&essai.d.table[k], bit(j)).is_some()).map(|k| Action::Ajouter(bit(j), k)));
                if let Some(a) = suite {
                    if self.echanger(x, i) {
                        acts.push(Action::Echanger(x, i));
                        if self.appliquer(a) {
                            acts.push(a);
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Cartes vues par le joueur p : sa main, la table, la défausse, et ce que chacun a ramassé.
    fn vues(&self, p: usize) -> Cartes {
        let mut v = self.d.mains[p] | ensemble(&self.d.defausse);
        for c in &self.d.table {
            v |= c.ensemble();
        }
        for q in 0..self.np {
            v |= self.d.ramassees[q];
        }
        v
    }

    /// Exemplaires de la carte n (0 à 51) que p n'a pas vus (0, 1 ou 2).
    fn vivantes(vues: Cartes, n: u8) -> u32 {
        (vues & bit(n) == 0) as u32 + (vues & bit(n + 52) == 0) as u32
    }

    /// Pénalité qui resterait dans la main h après avoir posé ce qui peut l'être (combinaisons, ajouts d'une carte
    /// aux combinaisons de la table).
    fn residuel(&self, h: Cartes, memo: &mut Memo) -> (i32, u32) {
        let pose = meilleures_memo(h, Objectif::Penalites, memo).iter().fold(0, |a, &m| a | m);
        let mut reste = h & !pose;
        for c in liste(reste & !JOKERS) {
            if self.d.table.iter().any(|t| completer(t, bit(c)).is_some()) {
                reste &= !bit(c);
            }
        }
        (penalites(reste), nb(reste))
    }

    /// Ce que vaut la main h pour la suite (difficile) : moyenne sur toutes les cartes que p peut encore piocher
    /// (selon ce qu'il a vu) de ce qu'il pourrait faire au tour suivant. Plus c'est haut, mieux c'est.
    fn anticipation(&self, p: usize, h: Cartes, vues: Cartes, memo: &mut Memo) -> f32 {
        let mut tirages: Vec<(u8, u32)> = (0..52u8)
            .filter_map(|n| {
                let w = Self::vivantes(vues, n);
                (w > 0).then(|| (if h & bit(n) != 0 { n + 52 } else { n }, w))
            })
            .collect();
        let jokers = 4 - nb(vues & JOKERS);
        if let Some(j) = (PREMIER_JOKER..NB_CARTES).find(|&j| (h | vues) & bit(j) == 0) {
            tirages.push((j, jokers));
        }
        let total: u32 = tirages.iter().map(|t| t.1).sum();
        if total == 0 {
            return 0.0;
        }
        let mut somme = 0.0f32;
        for &(x, w) in &tirages {
            let hx = h | bit(x);
            let v = if self.d.ouvert[p] {
                // ce qui resterait en main ; finir au tour suivant (une carte à défausser au plus) vaut cher
                let (pen, n) = self.residuel(hx, memo);
                -(pen as f32) - 10.0 * n as f32 + if n <= 1 { 60.0 } else { 0.0 }
            } else {
                let pts = points(&meilleures_memo(hx, Objectif::Points, memo));
                if pts >= OUVERTURE {
                    60.0
                } else {
                    0.4 * pts as f32
                }
            };
            somme += w as f32 * v;
        }
        somme / total as f32
    }

    /// La carte à défausser.
    pub fn choisir_defausse(&mut self, p: usize, lvl: i8) -> u8 {
        let main = self.d.mains[p];
        let interdites = self.d.jokers_pris | if self.d.pris >= 0 { bit(self.d.pris as u8) } else { 0 };
        let mut cands = liste(main & !JOKERS & !interdites);
        if cands.is_empty() {
            cands = liste(main);
        }
        let proteges = if self.d.ouvert[p] { 0 } else { meilleures(main, Objectif::Points).iter().fold(0, |a, &m| a | m) };
        if lvl == 0 && self.rng_mod(100) < 30 {
            let libres: Vec<u8> = cands.iter().copied().filter(|&c| proteges & bit(c) == 0).collect();
            if !libres.is_empty() {
                return libres[self.rng_mod(libres.len() as u32) as usize];
            }
        }
        let np = self.np;
        let suivant = (p + 1) % np;
        let presse = (0..np).any(|q| q != p && nb(self.d.mains[q]) <= 4);
        let vues = self.vues(p);
        let mut memo = Memo::new();
        let mut meilleur = (f32::MAX, cands[0]);
        for &c in &cands {
            let mut v = 0.0f32;
            if lvl >= 2 {
                v -= self.anticipation(p, main & !bit(c), vues, &mut memo);
            } else if proteges & bit(c) != 0 {
                v += 100.0;
            }
            if lvl >= 2 {
                // la pénalité est déjà dans l'anticipation
            } else if !est_joker(c) {
                let reste = main & !bit(c) & !JOKERS;
                let (r, s) = (rang(c), couleur(c));
                // brelan en vue : même rang, autres couleurs
                let mut paires = 0.0;
                for t in (0..4).filter(|&t| t != s) {
                    if exemplaire(reste, t * 13 + r).is_some() {
                        // difficile : il faut encore une carte de ce rang dans une couleur absente
                        let utile = lvl < 2 || (0..4).filter(|&u| u != s && u != t).any(|u| Self::vivantes(vues, u * 13 + r) > 0);
                        if utile {
                            paires += 1.0;
                        }
                    }
                }
                v += 9.0 * f32::min(paires, 2.0);
                // suite en vue : voisines de la même couleur
                let hs: &[u8] = if r == 12 { &[14, 1] } else { &[r + 2] };
                for &h in hs {
                    for (d, poids) in [(1i8, 10.0f32), (2, 5.0)] {
                        for sens in [-1i8, 1] {
                            let hv = h as i8 + d * sens;
                            if !(1..=14).contains(&hv) {
                                continue;
                            }
                            if exemplaire(reste, carte_de(s, hv as u8)).is_some() {
                                let entre = h as i8 + sens;
                                let utile = lvl < 2 || d == 1 || Self::vivantes(vues, carte_de(s, entre as u8)) > 0;
                                if utile {
                                    v += poids;
                                }
                            }
                        }
                    }
                }
                if exemplaire(reste, nom(c)).is_some() {
                    v -= 3.0;
                }
            } else {
                v += 60.0;
            }
            if lvl < 2 {
                v -= penalite(c) as f32 * if presse { 0.9 } else { 0.35 };
            }
            let pose_table = self.d.table.iter().any(|cb| completer(cb, bit(c)).is_some() || remplacer_joker(cb, c).is_some());
            if pose_table {
                if self.d.ouvert[suivant] {
                    v += if lvl >= 2 { 45.0 } else { 25.0 };
                }
                if !self.d.ouvert[p] {
                    v += 3.0;
                }
            }
            if lvl >= 2 && !est_joker(c) {
                let pris = self.d.ramassees[suivant];
                let lie = liste(pris & !JOKERS).iter().any(|&x| rang(x) == rang(c) || (couleur(x) == couleur(c) && (hauteur(x) as i8 - hauteur(c) as i8).abs() <= 2));
                if lie {
                    v += 8.0;
                }
            }
            if v < meilleur.0 {
                meilleur = (v, c);
            }
        }
        meilleur.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(coul: u8, h: u8) -> u8 {
        carte_de(coul, h)
    }

    #[test]
    fn recherche() {
        // 5♥ 6♥ 7♥ 8♥, 9♠ 9♦ 9♣, joker, 2♣ : suite de 4 + brelan + le joker quelque part
        let main = [k(1, 5), k(1, 6), k(1, 7), k(1, 8), k(0, 9), k(2, 9), k(3, 9), PREMIER_JOKER, k(3, 2)];
        let combis = meilleures(ensemble(&main), Objectif::Points);
        let pose: Cartes = combis.iter().fold(0, |a, &m| a | m);
        assert_eq!(pose & bit(k(3, 2)), 0);
        assert_ne!(pose & bit(PREMIER_JOKER), 0, "le joker sert");
        assert!(points(&combis) >= 26 + 27 + 9, "points : {}", points(&combis));
        // deux exemplaires : deux suites identiques
        let main = [k(2, 10), k(2, 11), k(2, 12), k(2, 10) + 52, k(2, 11) + 52, k(2, 12) + 52];
        assert_eq!(meilleures(ensemble(&main), Objectif::Points).len(), 2);
    }

    #[test]
    fn tours_de_bots() {
        // parties entières entre bots de tous niveaux : aucune action refusée, la partie finit
        for np in 2..=5 {
            let mut m = Moteur::new();
            m.np = np;
            m.rng = 1234 + np as u64;
            m.niveaux = [0, 1, 2, 1, 0];
            m.limite = 100;
            m.nouvelle_partie();
            let mut tours = 0;
            while m.d.phase != PH_FINPARTIE {
                match m.d.phase {
                    PH_PIOCHE => {
                        let p = m.joueur();
                        let t = m.bot_pioche(p);
                        assert!(m.piocher(t));
                        for a in m.bot_jeu(p) {
                            assert!(m.appliquer(a), "action refusée : {a:?}");
                        }
                        assert_ne!(m.d.phase, PH_JEU, "tour non fini");
                        tours += 1;
                    }
                    PH_FINMANCHE => m.manche_suivante(),
                    _ => unreachable!(),
                }
                assert!(tours < 20_000);
            }
        }
    }
}
