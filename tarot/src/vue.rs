//! Ce que l'écran montre en mouvement, à côté de la partie (qui, elle, ne dépend jamais de l'affichage) :
//! distribution des 78 cartes (mains et chien), chien qui se retourne puis rejoint le preneur, cartes de la
//! main qui glissent à leur place et se soulèvent (survol, écart), bulles, scores qui défilent.
//! Animations coupées (`bistrot::anim`) : tout est directement à sa place.

use crate::engine::*;
use crate::table::{main_x, Table, CHIEN_MONTRE};
use bistrot::anim::{self, approcher, DUREE_DISTRIB};
use bistrot::table::paquet_donneur;
use std::collections::HashMap;

/// Écart entre deux cartes distribuées (ms).
const CADENCE: f64 = 15.0;
/// Apparition et disparition des bulles (ms).
pub const BULLE_POP: f64 = 170.0;
pub const BULLE_FONDU: f64 = 240.0;
/// Vol des cartes du chien vers le preneur, une fois montrées (ms).
pub const VOL_CHIEN: f64 = 450.0;

/// Une carte en cours de distribution : joueur (5 = chien), carte, rang dans la main ou le chien.
pub struct Vol {
    pub p: usize,
    pub carte: u8,
    pub index: usize,
    pub t0: f64,
    pub src: (f64, f64),
}

#[derive(Default)]
pub struct Vue {
    /// donne dont la distribution a été lancée
    numero: u32,
    pub vols: Vec<Vol>,
    /// cartes adverses arrivées (dessinées dans le fond quand toute la main est là)
    pub stable: [usize; 5],
    /// le chien est posé au centre
    pub chien_pose: bool,
    /// abscisse et levée affichées des cartes de la main du joueur local
    pub x: HashMap<u8, f64>,
    pub levee: HashMap<u8, f64>,
    /// bulles : (texte, fin) -> début d'affichage
    pub bulles: [Option<(String, f64, f64)>; 5],
    pub score: [f64; 5],
    bouge: bool,
}

/// Position (centre) de la carte i du chien de k cartes, au centre de la table.
pub fn place_chien(i: usize, k: usize) -> (f64, f64) {
    (550.0 + (i as f64 - (k as f64 - 1.0) / 2.0) * 34.0, 352.0)
}

impl Vue {
    /// Les cartes du chien montré sont encore au centre (pas encore dans la main du preneur).
    pub fn chien_au_centre(t: &Table) -> bool {
        t.phase() == PH_ECART && t.chien_vu > 0.0 && t.now < t.chien_vu + CHIEN_MONTRE + VOL_CHIEN && anim::actives()
    }

    /// Met la vue à jour après un tic de la partie (dt ms) ; `hover` = carte de la main sous la souris.
    pub fn maj(&mut self, t: &Table, hover: Option<usize>, dt: f64) {
        let now = t.now;
        let np = t.np();
        let d = &t.m.d;
        if t.numero != self.numero {
            self.distribuer(t);
        }
        // distribution finie : les mains adverses rejoignent le fond (jusque-là, les cartes arrivées restent
        // dessinées à leur place avec celles qui volent)
        if !self.vols.iter().any(|v| now < v.t0 + DUREE_DISTRIB) {
            self.vols.clear();
        }
        if self.vols.is_empty() {
            for p in 0..np {
                self.stable[p] = nb(d.mains[p]) as usize;
            }
        }
        self.chien_pose = self.vols.is_empty();
        let mut bouge = !self.vols.is_empty();
        // la main du joueur local : chaque carte glisse vers sa place et se soulève (survol, écart)
        let main = self.main_visible(t);
        let n = main.len();
        let legal = t.my_legal();
        self.x.retain(|c, _| main.contains(c));
        self.levee.retain(|c, _| main.contains(c));
        for (i, &c) in main.iter().enumerate() {
            let cible = main_x(i, n);
            if self.vols.iter().any(|v| v.p == t.me && v.carte == c) {
                self.x.insert(c, cible);
                continue;
            }
            let x = self.x.entry(c).or_insert(cible);
            bouge |= approcher(x, cible, dt, 70.0);
            let mut haut = 0.0;
            if t.selection & bit(c) != 0 {
                haut += 34.0;
            } else if hover == Some(i) && (legal & bit(c) != 0 || d.phase == PH_ECART) {
                haut += 18.0;
            }
            if t.hint == c as i32 {
                haut += 14.0;
            }
            let l = self.levee.entry(c).or_insert(0.0);
            bouge |= approcher(l, haut, dt, 45.0);
        }
        // chien montré puis envoyé au preneur
        if d.phase == PH_ECART && t.chien_vu > 0.0 {
            bouge |= now < t.chien_vu + CHIEN_MONTRE + VOL_CHIEN;
        }
        // bulles : on note quand chacune apparaît
        for p in 0..5 {
            let b = t.bub[p].clone();
            let meme = match (&self.bulles[p], &b) {
                (Some((s, fin, _)), Some((s2, fin2))) => s == s2 && fin == fin2,
                (None, None) => true,
                _ => false,
            };
            if !meme {
                self.bulles[p] = b.map(|(s, fin)| (s, fin, now));
            }
            if let Some((_, fin, debut)) = &self.bulles[p] {
                bouge |= now < debut + BULLE_POP || (now < *fin && now > fin - BULLE_FONDU);
            }
        }
        // scores qui défilent
        for p in 0..5 {
            bouge |= approcher(&mut self.score[p], t.m.scores[p] as f64, dt, 160.0);
        }
        if !anim::actives() {
            self.vols.clear();
            for p in 0..np {
                self.stable[p] = nb(d.mains[p]) as usize;
            }
            self.chien_pose = true;
            bouge = false;
        }
        self.bouge = bouge;
    }

    /// Lance l'animation de la distribution : par paquets de trois cartes en tournant, une carte au chien de
    /// temps en temps, depuis le paquet du donneur.
    fn distribuer(&mut self, t: &Table) {
        self.numero = t.numero;
        self.vols.clear();
        self.stable = [0; 5];
        self.x.clear();
        self.levee.clear();
        let np = t.np();
        let d = &t.m.d;
        let src = paquet_donneur(&t.places()[t.disp_pos(t.m.donneur)]);
        let mut restes: Vec<Vec<u8>> = (0..np).map(|p| if p == t.me { trier(d.mains[p]) } else { liste(d.mains[p]) }).collect();
        let mut chien = liste(d.chien);
        let mut index = [0usize; 6];
        let mut t0 = t.now + 150.0;
        let mut tourne = 0;
        while restes.iter().any(|r| !r.is_empty()) || !chien.is_empty() {
            for j in 0..np {
                let p = (t.m.donneur + 1 + j) % np;
                for _ in 0..3 {
                    if let Some(c) = (!restes[p].is_empty()).then(|| restes[p].remove(0)) {
                        self.vols.push(Vol { p, carte: c, index: index[p], t0, src });
                        index[p] += 1;
                        t0 += CADENCE;
                    }
                }
                tourne += 1;
                if tourne % 3 == 0 {
                    if let Some(c) = (!chien.is_empty()).then(|| chien.remove(0)) {
                        self.vols.push(Vol { p: 5, carte: c, index: index[5], t0, src });
                        index[5] += 1;
                        t0 += CADENCE;
                    }
                }
            }
        }
    }

    /// Main du joueur local telle qu'elle s'affiche (sans les cartes du chien encore au centre).
    pub fn main_visible(&self, t: &Table) -> Vec<u8> {
        let mut m = t.m.d.mains[t.me];
        if t.m.d.preneur == t.me as i8 && Self::chien_au_centre(t) {
            m &= !t.m.d.chien;
        }
        trier(m)
    }

    /// Une animation est en cours.
    pub fn anime(&self) -> bool {
        self.bouge
    }
}
