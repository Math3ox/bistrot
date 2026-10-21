//! Ce que l'écran montre en mouvement, à côté de la partie (qui, elle, ne dépend jamais de l'affichage) :
//! distribution depuis le talon, et chaque carte visible (main du joueur, combinaisons de la table, défausse) qui
//! glisse en douceur vers sa place quand elle change d'endroit ; cartes piochées par les adversaires (dos) ou
//! qu'ils ramassent (défausse, joker repris) qui volent vers leur main ; cartes de la main qui se soulèvent
//! (choisies, survolées, conseillées) ; bulles ; scores qui défilent. Animations coupées (`bistrot::anim`) :
//! tout est directement à sa place.

use crate::engine::*;
use crate::table::{main_x, Table};
use bistrot::anim::{self, approcher, sortie, DUREE_DISTRIB};
use std::collections::HashMap;

/// Talon et défausse, au centre de la table.
pub const TALON: (f64, f64) = (496.0, 232.0);
pub const DEFAUSSE: (f64, f64) = (604.0, 232.0);
/// Échelle des cartes du talon et de la défausse.
pub const ECHELLE_PILE: f64 = PETITE;
/// Écart entre deux cartes distribuées (ms).
const CADENCE: f64 = 22.0;
/// Apparition et disparition des bulles (ms).
pub const BULLE_POP: f64 = 170.0;
pub const BULLE_FONDU: f64 = 240.0;
/// Vol d'une carte vers la main d'un adversaire (ms).
pub const DUREE_ENVOL: f64 = 420.0;
/// Main du joueur local.
pub const MAIN_Y: f64 = 686.0;
pub const ECHELLE_MAIN: f64 = 1.04;

/// Une carte en cours de distribution.
pub struct Vol {
    pub p: usize,
    pub carte: u8,
    pub index: usize,
    pub t0: f64,
}

/// Une carte qui vole vers la main d'un adversaire : départ (x, y, échelle), joueur, départ, face (None = dos).
pub struct Envol {
    pub depart: (f64, f64, f64),
    pub p: usize,
    pub t0: f64,
    pub carte: Option<u8>,
}

/// Place d'une combinaison de la table : centre de la première carte, écart entre les cartes, échelle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rangee {
    pub x0: f64,
    pub y: f64,
    pub pas: f64,
    pub echelle: f64,
}

impl Rangee {
    pub fn carte(&self, i: usize) -> (f64, f64) {
        (self.x0 + i as f64 * self.pas, self.y)
    }

    /// Rectangle (x0, y0, x1, y1) de la combinaison de n cartes.
    pub fn cadre(&self, n: usize) -> (f64, f64, f64, f64) {
        let (w, h) = (50.0 * self.echelle, 70.0 * self.echelle);
        (self.x0 - w, self.y - h, self.x0 + (n.max(1) - 1) as f64 * self.pas + w, self.y + h)
    }

    /// Carte de la combinaison (n cartes) sous l'abscisse x (la partie visible de chaque carte).
    pub fn carte_touchee(&self, n: usize, x: f64) -> Option<usize> {
        let w = 50.0 * self.echelle;
        (0..n).rev().find(|&i| x >= self.x0 + i as f64 * self.pas - w && x <= self.x0 + i as f64 * self.pas + if i + 1 == n { w } else { self.pas - w })
    }
}

/// Échelle des petites cartes (table, piles, mains adverses) : celle des petites images du paquet
/// (0,62 x 1,04), qui sont alors recopiées pixel pour pixel au lieu d'être refiltrées à chaque image.
pub const PETITE: f64 = 0.62 * 1.04;

/// Rangées de la table : y, x min, x max.
const RANGEES: [(f64, f64, f64); 5] = [(330.0, 162.0, 932.0), (424.0, 162.0, 932.0), (518.0, 162.0, 932.0), (232.0, 246.0, 448.0), (232.0, 652.0, 854.0)];
/// Écart entre les cartes d'une combinaison et entre deux combinaisons, du plus lisible au plus serré.
const SERRAGES: [(f64, f64); 4] = [(21.0, 14.0), (17.0, 11.0), (14.0, 8.0), (11.0, 6.0)];

/// Place de chaque combinaison de la table (dans l'ordre où elles ont été posées, rangée après rangée).
pub fn disposer(table: &[Combi]) -> Vec<Rangee> {
    let w = 100.0 * PETITE;
    for (k, &(pas, marge)) in SERRAGES.iter().enumerate() {
        let dernier = k + 1 == SERRAGES.len();
        let mut out = Vec::with_capacity(table.len());
        let mut r = 0;
        let mut x = RANGEES[0].1;
        let mut ok = true;
        for c in table {
            let large = w + (c.cartes.len() as f64 - 1.0) * pas;
            while r < RANGEES.len() && x + large > RANGEES[r].2 {
                r += 1;
                if r < RANGEES.len() {
                    x = RANGEES[r].1;
                }
            }
            if r >= RANGEES.len() {
                ok = false;
                if !dernier {
                    break;
                }
                // plus de place du tout : on empile sur la dernière rangée
                r = RANGEES.len() - 1;
                x = RANGEES[r].1;
            }
            out.push(Rangee { x0: x + w / 2.0, y: RANGEES[r].0, pas, echelle: PETITE });
            x += large + marge;
        }
        if ok || dernier {
            return out;
        }
    }
    Vec::new()
}

#[derive(Default)]
pub struct Vue {
    /// manche dont la distribution a été lancée
    numero: u32,
    pub vols: Vec<Vol>,
    /// cartes arrivées dans chaque main pendant la distribution
    pub stable: [usize; 5],
    /// position affichée de chaque carte visible : (x, y, échelle)
    pub pos: HashMap<u8, (f64, f64, f64)>,
    /// sa place
    pub cible: HashMap<u8, (f64, f64, f64)>,
    /// place des combinaisons de la table
    pub rangees: Vec<Rangee>,
    pub envols: Vec<Envol>,
    pub levee: HashMap<u8, f64>,
    /// bulles : (texte, fin) -> début d'affichage
    pub bulles: [Option<(String, f64, f64)>; 5],
    pub score: [f64; 5],
    vu_pioche: f64,
    bouge: bool,
}

impl Vue {
    /// Met la vue à jour après un tic de la partie (dt ms) ; `hover` = carte de la main sous la souris.
    pub fn maj(&mut self, t: &Table, hover: Option<u8>, dt: f64) {
        let now = t.now;
        let np = t.np();
        let d = &t.m.d;
        if t.numero != self.numero {
            self.distribuer(t);
        }
        if !self.vols.iter().any(|v| now < v.t0 + DUREE_DISTRIB) {
            self.vols.clear();
        }
        if self.vols.is_empty() {
            for p in 0..np {
                self.stable[p] = nb(d.mains[p]) as usize;
            }
        }
        let mut bouge = !self.vols.is_empty();
        // places des cartes visibles
        let mut cible = HashMap::with_capacity(self.cible.len().max(32));
        let main = t.ma_main();
        let n = main.len();
        for (i, &c) in main.iter().enumerate() {
            cible.insert(c, (main_x(i, n), MAIN_Y, ECHELLE_MAIN));
        }
        self.rangees = disposer(&d.table);
        for (k, cb) in d.table.iter().enumerate() {
            let r = self.rangees[k];
            for (i, &c) in cb.cartes.iter().enumerate() {
                let (x, y) = r.carte(i);
                cible.insert(c, (x, y, r.echelle));
            }
        }
        let nd = d.defausse.len();
        for (k, &c) in d.defausse.iter().enumerate().skip(nd.saturating_sub(3)) {
            let e = (nd - 1 - k) as f64;
            cible.insert(c, (DEFAUSSE.0 - 3.0 * e, DEFAUSSE.1 - 2.0 * e, ECHELLE_PILE));
        }
        // cartes qui ne sont plus visibles : ramassées par un adversaire (défausse, joker repris)
        let p = t.turn();
        for (&c, &(x, y, s)) in &self.pos {
            if !cible.contains_key(&c) && p != t.me && d.mains[p] & bit(c) != 0 && anim::actives() {
                self.envols.push(Envol { depart: (x, y, s), p, t0: now, carte: Some(c) });
            }
        }
        // pioche au talon d'un adversaire
        if t.pioche.0 != self.vu_pioche {
            self.vu_pioche = t.pioche.0;
            let (_, q, talon) = t.pioche;
            if talon && q != t.me && anim::actives() {
                self.envols.push(Envol { depart: (TALON.0, TALON.1, ECHELLE_PILE), p: q, t0: now, carte: None });
            }
        }
        self.pos.retain(|c, _| cible.contains_key(c));
        let debut = d.tours == 0 && d.phase == PH_PIOCHE;
        for (&c, &(x, y, s)) in &cible {
            let en_vol = self.vols.iter().any(|v| v.p == t.me && v.carte == c);
            let pos = self.pos.entry(c).or_insert_with(|| {
                if en_vol || main.contains(&c) {
                    (TALON.0, TALON.1, ECHELLE_PILE)
                } else if p != t.me && !debut {
                    let pl = t.places()[t.disp_pos(p)];
                    (pl.main.0, pl.main.1, PETITE)
                } else {
                    (TALON.0, TALON.1, ECHELLE_PILE)
                }
            });
            if en_vol {
                *pos = (x, y, s);
                continue;
            }
            if !self.vols.is_empty() && !main.contains(&c) {
                continue; // la défausse se retourne quand la distribution est finie
            }
            bouge |= approcher(&mut pos.0, x, dt, 75.0);
            bouge |= approcher(&mut pos.1, y, dt, 75.0);
            let mut e = pos.2 * 100.0;
            bouge |= approcher(&mut e, s * 100.0, dt, 75.0);
            pos.2 = e / 100.0;
        }
        self.cible = cible;
        // levée des cartes de la main
        self.levee.retain(|c, _| main.contains(c));
        for &c in &main {
            let mut haut = 0.0;
            if t.selection & bit(c) != 0 {
                haut += 30.0;
            } else if hover == Some(c) {
                haut += 16.0;
            }
            if t.hint & bit(c) != 0 {
                haut += 12.0;
            }
            let l = self.levee.entry(c).or_insert(0.0);
            bouge |= approcher(l, haut, dt, 45.0);
        }
        self.envols.retain(|e| now < e.t0 + DUREE_ENVOL);
        bouge |= !self.envols.is_empty();
        // bulles : on note quand chacune apparaît
        for q in 0..5 {
            let b = t.bub[q].clone();
            let meme = match (&self.bulles[q], &b) {
                (Some((s, fin, _)), Some((s2, fin2))) => s == s2 && fin == fin2,
                (None, None) => true,
                _ => false,
            };
            if !meme {
                self.bulles[q] = b.map(|(s, fin)| (s, fin, now));
            }
            if let Some((_, fin, debut)) = &self.bulles[q] {
                bouge |= now < debut + BULLE_POP || (now < *fin && now > fin - BULLE_FONDU);
            }
        }
        for q in 0..5 {
            bouge |= approcher(&mut self.score[q], t.m.scores[q] as f64, dt, 160.0);
        }
        if !anim::actives() {
            self.vols.clear();
            self.envols.clear();
            for q in 0..np {
                self.stable[q] = nb(d.mains[q]) as usize;
            }
            for (c, pos) in self.pos.iter_mut() {
                if let Some(&cb) = self.cible.get(c) {
                    *pos = cb;
                }
            }
            bouge = false;
        }
        self.bouge = bouge;
    }

    /// Lance l'animation de la distribution : une carte à chacun en tournant, depuis le talon.
    fn distribuer(&mut self, t: &Table) {
        self.numero = t.numero;
        self.vols.clear();
        self.envols.clear();
        self.stable = [0; 5];
        self.pos.clear();
        self.levee.clear();
        let np = t.np();
        let d = &t.m.d;
        let mut restes: Vec<Vec<u8>> = (0..np).map(|p| if p == t.me { t.ma_main() } else { liste(d.mains[p]) }).collect();
        let mut index = [0usize; 5];
        let mut t0 = t.now + 200.0;
        while restes.iter().any(|r| !r.is_empty()) {
            for j in 0..np {
                let p = (t.m.donneur + 1 + j) % np;
                if !restes[p].is_empty() {
                    let c = restes[p].remove(0);
                    self.vols.push(Vol { p, carte: c, index: index[p], t0 });
                    index[p] += 1;
                    t0 += CADENCE;
                }
            }
        }
        // la défausse se retourne après la distribution
        self.vu_pioche = t.pioche.0;
    }

    /// La carte c bouge encore (pas à sa place) : elle est dessinée par-dessus les autres.
    pub fn en_mouvement(&self, c: u8) -> bool {
        match (self.pos.get(&c), self.cible.get(&c)) {
            (Some(a), Some(b)) => (a.0 - b.0).abs() + (a.1 - b.1).abs() > 1.5,
            _ => false,
        }
    }

    /// Avancement d'un envol (0 à 1, avec sa hauteur d'arc) pour l'instant now.
    pub fn avancement(e: &Envol, now: f64) -> (f64, f64) {
        let t = ((now - e.t0) / DUREE_ENVOL).clamp(0.0, 1.0);
        (sortie(t), (std::f64::consts::PI * t).sin())
    }

    /// Une animation est en cours.
    pub fn anime(&self) -> bool {
        self.bouge
    }
}
