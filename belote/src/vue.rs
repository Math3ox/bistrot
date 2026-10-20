//! Ce que l'écran montre en mouvement, à côté de la partie (qui, elle, ne dépend jamais de l'affichage) :
//! cartes en cours de distribution, cartes de la main qui glissent à leur place et se soulèvent au survol,
//! retourne qui se retourne, bulles qui apparaissent et s'effacent, score qui défile, chouette qui regarde
//! autour d'elle. Animations coupées (`bistrot::anim`) : tout est directement à sa place.

use crate::table::Table;
use bistrot::ambiance::Chouette;
use bistrot::anim::{self, approcher, DUREE_DISTRIB};
use bistrot::table::{main_x, paquet_donneur};
use std::collections::HashMap;

/// Écart entre deux cartes distribuées (ms).
const CADENCE: f64 = 40.0;
/// Durée du retournement de la retourne (ms).
pub const DUREE_RETOURNE: f64 = 360.0;
/// Apparition et disparition des bulles (ms).
pub const BULLE_POP: f64 = 170.0;
pub const BULLE_FONDU: f64 = 240.0;

/// Une carte en cours de distribution.
pub struct Vol {
    pub p: usize,
    /// carte (main du joueur local) ou rang dans la main (adversaires)
    pub carte: u8,
    pub index: usize,
    pub t0: f64,
    pub src: (f64, f64),
}

#[derive(Default)]
pub struct Vue {
    /// cartes adverses déjà posées (dessinées dans le fond)
    pub stable: [usize; 4],
    pub vols: Vec<Vol>,
    /// abscisse et levée affichées des cartes de la main du joueur local
    pub x: HashMap<u8, f64>,
    pub levee: HashMap<u8, f64>,
    /// retourne et début de son retournement
    pub retourne: (i32, f64),
    /// début d'affichage de chaque bulle (texte, fin) -> début
    pub bulles: [Option<(String, f64, f64)>; 4],
    pub score: [f64; 2],
    /// chouette posée sur la table (images précalculées)
    pub chouette: Chouette,
    connues: [usize; 4],
    ma_main: Vec<u8>,
    bouge: bool,
}

impl Vue {
    /// Fin de la distribution en cours (ms), ou maintenant.
    fn fin_distribution(&self, now: f64) -> f64 {
        self.vols.iter().map(|v| v.t0 + DUREE_DISTRIB).fold(now, f64::max)
    }

    /// Met la vue à jour après un tic de la partie (dt ms) ; `hover` = carte de la main sous la souris.
    pub fn maj(&mut self, t: &Table, hover: Option<usize>, dt: f64) {
        let now = t.now;
        let np = t.np();
        let gs = &t.e.gs;
        let places = t.places();
        // source des cartes distribuées : le talon (à deux) ou le paquet du donneur
        let src = if np == 2 { (300.0, 352.0) } else { paquet_donneur(&places[t.disp_pos(t.e.dealer.max(0) as usize)]) };
        let mut delai = (self.fin_distribution(now) - DUREE_DISTRIB - now).max(0.0);
        for j in 0..np {
            let p = (t.e.dealer.max(0) as usize + 1 + j) % np;
            let n = gs.hn[p] as usize;
            if p == t.me {
                let main: Vec<u8> = gs.hand[p][..n].to_vec();
                for &c in main.iter().filter(|c| !self.ma_main.contains(c)) {
                    self.vols.push(Vol { p, carte: c, index: 0, t0: now + delai, src });
                    delai += CADENCE;
                }
                // cartes parties (jouées, ou nouvelle donne) : on oublie leur position
                self.x.retain(|c, _| main.contains(c));
                self.levee.retain(|c, _| main.contains(c));
                self.vols.retain(|v| v.p != p || main.contains(&v.carte));
                self.ma_main = main;
            } else if n > self.connues[p] {
                for i in self.connues[p]..n {
                    self.vols.push(Vol { p, carte: 0, index: i, t0: now + delai, src });
                    delai += CADENCE;
                }
            } else if n < self.connues[p] {
                self.stable[p] = n.min(self.stable[p]);
                self.vols.retain(|v| v.p != p || v.index < n);
            }
            self.connues[p] = n;
        }
        // cartes arrivées : celles des adversaires rejoignent le fond quand toute leur main est posée
        for p in 0..np {
            if p != t.me && !self.vols.iter().any(|v| v.p == p && now < v.t0 + DUREE_DISTRIB) {
                self.vols.retain(|v| v.p != p);
                self.stable[p] = self.connues[p];
            }
        }
        let mut bouge = !self.vols.is_empty();
        // la main du joueur local : chaque carte glisse vers sa place et se soulève au survol
        let n = self.ma_main.len();
        let legal = t.my_legal();
        for (i, &c) in self.ma_main.clone().iter().enumerate() {
            let en_vol = self.vols.iter().any(|v| v.p == t.me && v.carte == c && now < v.t0 + DUREE_DISTRIB);
            let cible = main_x(i, n);
            if en_vol {
                self.x.insert(c, cible);
                continue;
            }
            self.vols.retain(|v| !(v.p == t.me && v.carte == c));
            let x = self.x.entry(c).or_insert(cible);
            bouge |= approcher(x, cible, dt, 70.0);
            let mut haut = 0.0;
            if hover == Some(i) && legal >> i & 1 == 1 {
                haut += 18.0;
            }
            if t.hint == i as i32 {
                haut += 14.0;
            }
            let l = self.levee.entry(c).or_insert(0.0);
            bouge |= approcher(l, haut, dt, 45.0);
        }
        // retourne : se retourne une fois la distribution finie
        if gs.turned != self.retourne.0 {
            self.retourne = (gs.turned, self.fin_distribution(now));
        }
        bouge |= now < self.retourne.1 + DUREE_RETOURNE;
        // bulles : on note quand chacune apparaît
        for p in 0..4 {
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
        // score qui défile
        for k in 0..2 {
            bouge |= approcher(&mut self.score[k], t.e.score[k] as f64, dt, 160.0);
        }
        self.chouette.avancer(dt);
        if !anim::actives() {
            // animations coupées : cartes distribuées déjà en main, retourne déjà retournée, bulles sans effet
            self.vols.clear();
            for p in 0..np {
                if p != t.me {
                    self.stable[p] = self.connues[p];
                }
            }
            self.retourne.1 = f64::NEG_INFINITY;
            bouge = false;
        }
        self.bouge = bouge;
    }

    /// Une animation est en cours.
    pub fn anime(&self) -> bool {
        self.bouge
    }
}
