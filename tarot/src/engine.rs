//! Règles du tarot (Fédération française de tarot), sans graphisme et déterministes à partir de la graine :
//! 3, 4 ou 5 joueurs (à 5, le preneur appelle un roi), enchères (petite, garde, garde sans le chien, garde
//! contre le chien), chien et écart, poignées, excuse, petit au bout, chelem, décompte de la donne et de la
//! partie, journal pour le vérificateur indépendant (`check.py`).
//!
//! Les cartes sont numérotées comme `bistrot::cartes::Carte::tarot` : 0 à 55 les couleurs (14 cartes par
//! couleur, de l'as au roi : pique, cœur, carreau, trèfle), 56 à 76 les atouts 1 à 21, 77 l'excuse. Un
//! ensemble de cartes tient dans un entier de 128 bits (`Cartes`) : mains, plis, chien, écart.

pub type Cartes = u128;

pub const NB_CARTES: u8 = 78;
pub const PETIT: u8 = 56;
pub const VINGT_ET_UN: u8 = 76;
pub const EXCUSE: u8 = 77;
pub const TOUTES: Cartes = (1u128 << 78) - 1;
pub const ATOUTS: Cartes = ((1u128 << 21) - 1) << 56;
pub const BOUTS: Cartes = 1u128 << PETIT | 1u128 << VINGT_ET_UN | 1u128 << EXCUSE;

pub const PH_ENCHERES: u8 = 1;
pub const PH_APPEL: u8 = 2;
pub const PH_ECART: u8 = 3;
pub const PH_JEU: u8 = 4;
pub const PH_FINPLI: u8 = 5;
pub const PH_FINDONNE: u8 = 6;
pub const PH_FINPARTIE: u8 = 7;

/// Contrats : 0 = passe.
pub const CONTRATS: [&str; 5] = ["Passe", "Petite", "Garde", "Garde sans", "Garde contre"];
pub const MULTIPLICATEURS: [i32; 5] = [0, 1, 2, 4, 6];
/// Points à faire selon le nombre de bouts.
pub const CIBLES: [i32; 4] = [56, 51, 41, 36];
pub const POIGNEES: [&str; 4] = ["", "Poignée", "Double poignée", "Triple poignée"];
pub const PRIMES_POIGNEE: [i32; 4] = [0, 20, 30, 40];
/// Noms des couleurs dans l'ordre des numéros de cartes.
pub const COULEURS: [&str; 4] = ["Pique", "Cœur", "Carreau", "Trèfle"];

#[inline]
pub fn bit(c: u8) -> Cartes {
    1u128 << c
}

#[inline]
pub fn nb(m: Cartes) -> u32 {
    m.count_ones()
}

/// Cartes d'une couleur (0 à 3).
#[inline]
pub fn couleur(s: usize) -> Cartes {
    ((1u128 << 14) - 1) << (14 * s)
}

/// Les cartes d'un ensemble, dans l'ordre des numéros.
pub fn liste(mut m: Cartes) -> Vec<u8> {
    let mut v = Vec::with_capacity(nb(m) as usize);
    while m != 0 {
        v.push(m.trailing_zeros() as u8);
        m &= m - 1;
    }
    v
}

/// Famille d'une carte : 0 à 3 les couleurs, 4 les atouts, 5 l'excuse.
#[inline]
pub fn famille(c: u8) -> usize {
    if c < 56 {
        c as usize / 14
    } else if c < 77 {
        4
    } else {
        5
    }
}

/// Force dans sa famille : 0 (as) à 13 (roi) pour les couleurs, 1 à 21 pour les atouts.
#[inline]
pub fn force(c: u8) -> u8 {
    if c < 56 {
        c % 14
    } else if c < 77 {
        c - 55
    } else {
        0
    }
}

pub const ROIS: Cartes = 1u128 << 13 | 1u128 << 27 | 1u128 << 41 | 1u128 << 55;

/// Valeur d'une carte en demi-points (bouts et rois 4,5 ; dames 3,5 ; cavaliers 2,5 ; valets 1,5 ; autres 0,5).
#[inline]
pub fn demi_points(c: u8) -> i32 {
    if bit(c) & BOUTS != 0 {
        return 9;
    }
    if c >= 56 {
        return 1;
    }
    match c % 14 {
        13 => 9,
        12 => 7,
        11 => 5,
        10 => 3,
        _ => 1,
    }
}

pub fn demi_points_de(m: Cartes) -> i32 {
    liste(m).into_iter().map(demi_points).sum()
}

/// Clé de tri d'une main à l'écran : pique, cœur, trèfle, carreau (couleurs alternées), excuse, atouts.
pub fn cle_affichage(c: u8) -> u8 {
    if c < 56 {
        [0u8, 1, 3, 2][c as usize / 14] * 14 + c % 14
    } else if c == EXCUSE {
        56
    } else {
        57 + (c - 56)
    }
}

/// Main triée pour l'affichage.
pub fn trier(m: Cartes) -> Vec<u8> {
    let mut v = liste(m);
    v.sort_by_key(|&c| cle_affichage(c));
    v
}

/// Seuils des poignées (simple, double, triple) selon le nombre de joueurs.
pub fn seuils_poignee(np: usize) -> [u32; 3] {
    match np {
        3 => [13, 15, 18],
        4 => [10, 13, 15],
        _ => [8, 10, 13],
    }
}

/// Bilan d'une donne.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Bilan {
    pub preneur: i8,
    pub partenaire: i8,
    pub contrat: u8,
    pub bouts: u8,
    /// points à faire, points faits par l'attaque (en demi-points)
    pub cible: i32,
    pub demi: i32,
    pub fait: bool,
    /// écart avec la cible, arrondi (points)
    pub ecart: i32,
    /// petit au bout : 1 pour l'attaque, -1 pour la défense, 0 sinon
    pub petit_bout: i8,
    /// primes, vues de l'attaque
    pub poignee: i32,
    pub chelem: i32,
    /// valeur d'une part, vue de l'attaque : ((25 + écart) * signe + 10 * petit au bout) * multiplicateur + primes
    pub unite: i32,
    pub gains: [i32; 5],
}

/// État d'une donne (copiable : les bots s'en servent pour leurs simulations).
#[derive(Clone, Copy, Debug)]
pub struct Donne {
    pub phase: u8,
    /// joueur qui doit parler, appeler, écarter ou jouer
    pub tour: u8,
    pub mains: [Cartes; 5],
    pub chien: Cartes,
    pub ecart: Cartes,
    /// enchère de chaque joueur (-1 : n'a pas encore parlé)
    pub encheres: [i8; 5],
    pub contrat: u8,
    pub preneur: i8,
    /// carte appelée (5 joueurs), -1 sinon
    pub appel: i8,
    /// détenteur de la carte appelée (-1 : le preneur joue seul)
    pub partenaire: i8,
    /// la carte appelée est tombée : tout le monde connaît le partenaire
    pub revele: bool,
    /// pli en cours : cartes dans l'ordre joué, à partir de l'entame
    pub pli: [u8; 5],
    pub n_pli: u8,
    pub entame: u8,
    pub plis: u8,
    /// gagnant du pli terminé (phase PH_FINPLI)
    pub gagnant: u8,
    /// cartes ramassées par chaque joueur (sans l'excuse, comptée à part)
    pub piles: [Cartes; 5],
    pub plis_gagnes: [u8; 5],
    /// excuse : (qui l'a jouée, gagnant du pli, jouée au dernier pli) ; -1 si pas encore jouée
    pub excuse: (i8, i8, bool),
    /// petit au bout : gagnant du dernier pli s'il contenait le petit, -1 sinon
    pub petit_bout: i8,
    /// familles où le joueur a montré qu'il n'avait plus de carte (bits 0 à 3 couleurs, 4 atouts)
    pub vides: [u8; 5],
    pub joues: Cartes,
    pub poignees: [u8; 5],
    /// joueurs ayant déjà joué une carte (les poignées se montrent avec la première carte)
    pub premiere: u8,
    /// le dernier pli ramassé (cartes dans l'ordre joué, entame) pour l'affichage
    pub dernier: ([u8; 5], u8),
}

impl Default for Donne {
    fn default() -> Donne {
        Donne {
            phase: PH_ENCHERES,
            tour: 0,
            mains: [0; 5],
            chien: 0,
            ecart: 0,
            encheres: [-1; 5],
            contrat: 0,
            preneur: -1,
            appel: -1,
            partenaire: -1,
            revele: false,
            pli: [0xFF; 5],
            n_pli: 0,
            entame: 0,
            plis: 0,
            gagnant: 0,
            piles: [0; 5],
            plis_gagnes: [0; 5],
            excuse: (-1, -1, false),
            petit_bout: -1,
            vides: [0; 5],
            joues: 0,
            poignees: [0; 5],
            premiere: 0,
            dernier: ([0xFF; 5], 0),
        }
    }
}

#[derive(Clone)]
pub struct Moteur {
    pub d: Donne,
    pub np: usize,
    pub donneur: usize,
    pub rng: u64,
    /// niveau de chaque siège : -1 humain, 0 facile, 1 normal, 2 difficile
    pub niveaux: [i8; 5],
    pub scores: [i32; 5],
    /// donnes jouées et nombre de donnes de la partie
    pub donne: u32,
    pub donnes: u32,
    pub bilan: Option<Bilan>,
    /// donnes blanches de suite (tout le monde a passé)
    pub blanches: u32,
    pub journal: Vec<u8>,
    pub journal_on: bool,
    /// simulation en cours (bots difficiles) : pas de journal
    pub simule: bool,
    /// tirages des bots difficiles par décision
    pub mc_tirages: u32,
    /// valeur de main (barème des bots) à partir de laquelle ils demandent petite, garde, garde sans, garde contre,
    /// à 3, 4 et 5 joueurs (réglés au banc d'essai : chaque contrat est demandé là où il rapporte le plus)
    pub seuils: [[i32; 4]; 3],
}

impl Default for Moteur {
    fn default() -> Moteur {
        Moteur::new()
    }
}

impl Moteur {
    pub fn new() -> Moteur {
        Moteur {
            d: Donne::default(),
            np: 4,
            donneur: 0,
            rng: 0x2545_F491_4F6C_DD1D,
            niveaux: [-1, 1, 1, 1, 1],
            scores: [0; 5],
            donne: 0,
            donnes: 10,
            bilan: None,
            blanches: 0,
            journal: Vec::new(),
            journal_on: false,
            simule: false,
            mc_tirages: 24,
            seuils: [[33, 38, 47, 51], [38, 46, 60, 68], [31, 38, 50, 58]],
        }
    }

    // ---------- aléatoire ----------
    #[inline]
    pub fn rng_next(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        (x >> 32) as u32
    }

    #[inline]
    pub fn rng_mod(&mut self, n: u32) -> u32 {
        self.rng_next() % n.max(1)
    }

    // ---------- journal ----------
    fn ecrire(&mut self, ligne: impl FnOnce() -> String) {
        if self.journal_on && !self.simule {
            let l = ligne();
            self.journal.extend_from_slice(l.as_bytes());
            self.journal.push(b'\n');
        }
    }

    fn ecrire_cartes(m: Cartes) -> String {
        liste(m).iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
    }

    // ---------- tailles ----------
    pub fn taille_main(&self) -> usize {
        match self.np {
            3 => 24,
            4 => 18,
            _ => 15,
        }
    }

    pub fn taille_chien(&self) -> usize {
        if self.np == 5 {
            3
        } else {
            6
        }
    }

    // ---------- partie et donnes ----------
    pub fn nouvelle_partie(&mut self) {
        self.scores = [0; 5];
        self.donne = 0;
        self.bilan = None;
        self.donneur = self.rng_mod(self.np as u32) as usize;
        self.ecrire(|| "G".into());
        self.nouvelle_donne();
    }

    /// Mélange et distribue (on redonne si un joueur a le petit sec : le petit seul, sans autre atout ni excuse).
    pub fn nouvelle_donne(&mut self) {
        let np = self.np;
        let tm = self.taille_main();
        loop {
            let mut jeu: [u8; 78] = std::array::from_fn(|i| i as u8);
            for i in (1..78).rev() {
                let j = self.rng_mod(i as u32 + 1) as usize;
                jeu.swap(i, j);
            }
            let mut d = Donne::default();
            for p in 0..np {
                for &c in &jeu[p * tm..(p + 1) * tm] {
                    d.mains[p] |= bit(c);
                }
            }
            for &c in &jeu[np * tm..] {
                d.chien |= bit(c);
            }
            let sec = (0..np).any(|p| d.mains[p] & (ATOUTS | bit(EXCUSE)) == bit(PETIT));
            if !sec {
                d.tour = ((self.donneur + 1) % np) as u8;
                d.entame = d.tour;
                self.d = d;
                break;
            }
        }
        self.bilan = None;
        let (np, donneur) = (self.np, self.donneur);
        self.ecrire(|| format!("N {np} {donneur}"));
        for p in 0..np {
            let m = self.d.mains[p];
            self.ecrire(|| format!("M {p} {}", Self::ecrire_cartes(m)));
        }
        let chien = self.d.chien;
        self.ecrire(|| format!("C {}", Self::ecrire_cartes(chien)));
    }

    /// Donne suivante (après le bilan) : le donneur suivant distribue.
    pub fn donne_suivante(&mut self) {
        self.donneur = (self.donneur + 1) % self.np;
        self.nouvelle_donne();
    }

    // ---------- enchères ----------
    pub fn enchere_max(&self) -> u8 {
        self.d.encheres[..self.np].iter().copied().max().unwrap_or(-1).max(0) as u8
    }

    /// Le contrat c est permis au joueur dont c'est le tour (0 = passer, toujours permis).
    pub fn enchere_permise(&self, c: u8) -> bool {
        self.d.phase == PH_ENCHERES && (c == 0 || (c <= 4 && c > self.enchere_max()))
    }

    pub fn encherir(&mut self, c: u8) -> bool {
        if !self.enchere_permise(c) {
            return false;
        }
        let p = self.d.tour as usize;
        self.d.encheres[p] = c as i8;
        self.ecrire(|| format!("B {p} {c}"));
        let parle = self.d.encheres[..self.np].iter().filter(|&&e| e >= 0).count();
        if parle < self.np {
            self.d.tour = ((p + 1) % self.np) as u8;
            return true;
        }
        let max = self.enchere_max();
        if max == 0 {
            // donne blanche : le donneur suivant redonne
            self.blanches += 1;
            self.ecrire(|| "X".into());
            self.donne_suivante();
            return true;
        }
        self.blanches = 0;
        let preneur = (0..self.np).find(|&q| self.d.encheres[q] == max as i8).unwrap_or(0);
        self.d.preneur = preneur as i8;
        self.d.contrat = max;
        if self.np == 5 {
            self.d.phase = PH_APPEL;
            self.d.tour = preneur as u8;
        } else {
            self.apres_appel();
        }
        true
    }

    // ---------- appel du roi (5 joueurs) ----------
    /// Cartes que le preneur peut appeler : un roi ; s'il a les quatre rois, une dame ; et ainsi de suite.
    pub fn appels_possibles(&self) -> Cartes {
        if self.d.preneur < 0 {
            return 0;
        }
        let h = self.d.mains[self.d.preneur as usize];
        for rang in [13u8, 12, 11, 10] {
            let m: Cartes = (0..4).map(|s| bit(s * 14 + rang)).fold(0, |a, b| a | b);
            if h & m != m {
                return m;
            }
        }
        0
    }

    pub fn appeler(&mut self, c: u8) -> bool {
        if self.d.phase != PH_APPEL || self.appels_possibles() & bit(c) == 0 {
            return false;
        }
        self.d.appel = c as i8;
        let preneur = self.d.preneur as usize;
        self.d.partenaire = (0..self.np).find(|&q| q != preneur && self.d.mains[q] & bit(c) != 0).map_or(-1, |q| q as i8);
        // carte dans le chien montré : tout le monde sait que le preneur joue seul
        if self.d.contrat <= 2 && self.d.chien & bit(c) != 0 {
            self.d.revele = true;
        }
        let partenaire = self.d.partenaire;
        self.ecrire(|| format!("A {c} {partenaire}"));
        self.apres_appel();
        true
    }

    fn apres_appel(&mut self) {
        let preneur = self.d.preneur as usize;
        if self.d.contrat <= 2 {
            // petite et garde : le chien est montré, le preneur le prend et écarte
            self.d.mains[preneur] |= self.d.chien;
            self.d.phase = PH_ECART;
            self.d.tour = preneur as u8;
        } else {
            self.commencer_jeu();
        }
    }

    // ---------- écart ----------
    /// Cartes que le preneur a le droit d'écarter : ni roi ni bout ; des atouts seulement s'il ne peut pas faire
    /// autrement (ils sont alors montrés).
    pub fn ecart_possible(&self) -> Cartes {
        let h = self.d.mains[self.d.preneur.max(0) as usize];
        let normales = h & !ATOUTS & !ROIS & !BOUTS;
        if nb(normales) as usize >= self.taille_chien() {
            normales
        } else {
            normales | (h & ATOUTS & !BOUTS)
        }
    }

    pub fn ecart_valide(&self, m: Cartes) -> bool {
        if self.d.phase != PH_ECART || nb(m) as usize != self.taille_chien() || m & !self.ecart_possible() != 0 {
            return false;
        }
        // des atouts seulement après toutes les cartes ordinaires possibles
        let h = self.d.mains[self.d.preneur as usize];
        let normales = h & !ATOUTS & !ROIS & !BOUTS;
        m & ATOUTS == 0 || normales & !m == 0
    }

    pub fn ecarter(&mut self, m: Cartes) -> bool {
        if !self.ecart_valide(m) {
            return false;
        }
        let preneur = self.d.preneur as usize;
        self.d.mains[preneur] &= !m;
        self.d.ecart = m;
        self.ecrire(|| format!("E {}", Self::ecrire_cartes(m)));
        self.commencer_jeu();
        true
    }

    fn commencer_jeu(&mut self) {
        self.d.phase = PH_JEU;
        self.d.tour = ((self.donneur + 1) % self.np) as u8;
        self.d.entame = self.d.tour;
        self.d.n_pli = 0;
    }

    // ---------- jeu de la carte ----------
    /// Attaquant (preneur ou partenaire, d'après la vraie donne : sert au décompte).
    #[inline]
    pub fn attaque(&self, p: usize) -> bool {
        p as i8 == self.d.preneur || p as i8 == self.d.partenaire
    }

    /// Famille demandée dans le pli en cours (la première carte qui n'est pas l'excuse).
    pub fn famille_demandee(&self) -> Option<usize> {
        self.d.pli[..self.d.n_pli as usize].iter().map(|&c| famille(c)).find(|&f| f != 5)
    }

    /// Plus fort atout du pli en cours (0 s'il n'y en a pas).
    pub fn atout_max(&self) -> u8 {
        self.d.pli[..self.d.n_pli as usize].iter().filter(|&&c| famille(c) == 4).map(|&c| force(c)).max().unwrap_or(0)
    }

    /// Atouts de la main strictement plus forts que l'atout n.
    #[inline]
    fn atouts_au_dessus(h: Cartes, n: u8) -> Cartes {
        h & ATOUTS & !((1u128 << (56 + n)) - 1)
    }

    /// Cartes que le joueur p peut jouer : fournir la couleur demandée, sinon couper, en montant sur le plus fort
    /// atout du pli si possible ; l'excuse se joue toujours. À cinq, on n'entame pas le premier pli dans la
    /// couleur du roi appelé, sauf avec le roi lui-même.
    pub fn legales(&self, p: usize) -> Cartes {
        let d = &self.d;
        let h = d.mains[p];
        if d.n_pli == 0 {
            if self.np == 5 && d.plis == 0 && d.appel >= 0 {
                let c = d.appel as u8;
                let interdites = couleur(famille(c)) & !bit(c);
                let m = h & !interdites;
                if m != 0 {
                    return m;
                }
            }
            return h;
        }
        let exc = h & bit(EXCUSE);
        let atouts = |h: Cartes| -> Cartes {
            let a = h & ATOUTS;
            if a == 0 {
                return h;
            }
            let dessus = Self::atouts_au_dessus(a, self.atout_max());
            (if dessus != 0 { dessus } else { a }) | exc
        };
        match self.famille_demandee() {
            None => h,
            Some(4) => atouts(h),
            Some(s) => {
                let c = h & couleur(s);
                if c != 0 {
                    c | exc
                } else {
                    atouts(h)
                }
            }
        }
    }

    /// Poignée que le joueur p peut montrer avec sa première carte (0 : aucune). L'excuse compte comme un atout.
    pub fn poignee_possible(&self, p: usize) -> u8 {
        if self.d.premiere >> p & 1 == 1 || self.d.phase != PH_JEU {
            return 0;
        }
        let n = nb(self.d.mains[p] & (ATOUTS | bit(EXCUSE)));
        seuils_poignee(self.np).iter().filter(|&&s| n >= s).count() as u8
    }

    /// Le joueur dont c'est le tour joue la carte c, en montrant éventuellement une poignée (niveau 1 à 3).
    pub fn jouer(&mut self, c: u8, poignee: u8) -> bool {
        if self.d.phase != PH_JEU || c >= NB_CARTES {
            return false;
        }
        let p = self.d.tour as usize;
        if self.legales(p) & bit(c) == 0 || poignee > self.poignee_possible(p) {
            return false;
        }
        if self.d.premiere >> p & 1 == 0 {
            self.d.premiere |= 1 << p;
            self.d.poignees[p] = poignee;
        }
        // ce que la carte révèle : une famille manquante
        if let Some(s) = self.famille_demandee() {
            let f = famille(c);
            if f != s && f != 5 {
                self.d.vides[p] |= 1 << s;
                if f != 4 {
                    self.d.vides[p] |= 1 << 4;
                }
            }
        }
        self.ecrire(|| format!("P {p} {c} {poignee}"));
        let d = &mut self.d;
        d.mains[p] &= !bit(c);
        d.joues |= bit(c);
        d.pli[d.n_pli as usize] = c;
        d.n_pli += 1;
        if c as i8 == d.appel {
            d.revele = true;
        }
        if d.n_pli as usize == self.np {
            let g = self.gagnant_du_pli();
            self.d.gagnant = g as u8;
            self.d.tour = g as u8;
            self.d.phase = PH_FINPLI;
        } else {
            self.d.tour = ((p + 1) % self.np) as u8;
        }
        true
    }

    /// Joueur qui remporte le pli complet en cours.
    pub fn gagnant_du_pli(&self) -> usize {
        let d = &self.d;
        let np = self.np;
        let dem = self.famille_demandee();
        let mut meilleur = (i32::MIN, d.entame as usize);
        let mut excuse = None;
        for j in 0..d.n_pli as usize {
            let p = (d.entame as usize + j) % np;
            let c = d.pli[j];
            let cle = match famille(c) {
                5 => {
                    excuse = Some(p);
                    -2
                }
                4 => 100 + force(c) as i32,
                f if Some(f) == dem => force(c) as i32,
                _ => -1,
            };
            if cle > meilleur.0 {
                meilleur = (cle, p);
            }
        }
        // chelem : l'excuse jouée au dernier pli par le camp qui a fait tous les autres plis le remporte
        if let Some(o) = excuse {
            let dernier = d.plis as usize + 1 == self.taille_main();
            let camp = self.attaque(o);
            let tous = (0..np).filter(|&q| self.attaque(q) != camp).all(|q| d.plis_gagnes[q] == 0);
            if dernier && d.plis > 0 && tous {
                return o;
            }
        }
        meilleur.1
    }

    /// Ramasse le pli terminé ; à la fin de la donne, fait le décompte.
    pub fn ramasser(&mut self) {
        if self.d.phase != PH_FINPLI {
            return;
        }
        let np = self.np;
        let g = self.d.gagnant as usize;
        let dernier = self.d.plis as usize + 1 == self.taille_main();
        let mut cartes: Cartes = 0;
        for j in 0..np {
            let c = self.d.pli[j];
            let p = (self.d.entame as usize + j) % np;
            if c == EXCUSE {
                self.d.excuse = (p as i8, g as i8, dernier);
            } else {
                cartes |= bit(c);
            }
        }
        if dernier && cartes & bit(PETIT) != 0 {
            self.d.petit_bout = g as i8;
        }
        self.d.piles[g] |= cartes;
        self.d.plis_gagnes[g] += 1;
        self.d.dernier = (self.d.pli, self.d.entame);
        self.d.plis += 1;
        self.d.pli = [0xFF; 5];
        self.d.n_pli = 0;
        if self.d.plis as usize == self.taille_main() {
            if self.simule {
                self.d.phase = PH_FINDONNE; // simulation d'un bot : on regarde seulement le bilan
            } else {
                self.compter();
            }
        } else {
            self.d.phase = PH_JEU;
            self.d.entame = g as u8;
            self.d.tour = g as u8;
        }
    }

    // ---------- décompte ----------
    /// Bilan de la donne finie (sans toucher aux scores).
    pub fn calculer_bilan(&self) -> Bilan {
        let d = &self.d;
        let np = self.np;
        let att = |p: usize| self.attaque(p);
        let mut cartes: Cartes = 0;
        for p in 0..np {
            if att(p) {
                cartes |= d.piles[p];
            }
        }
        match d.contrat {
            1 | 2 => cartes |= d.ecart,
            3 => cartes |= d.chien,
            _ => {}
        }
        // excuse : reste à son camp, qui donne une demi-carte en échange ; jouée au dernier pli, elle est perdue
        let mut echange = 0;
        let (o, g, dernier) = d.excuse;
        if o >= 0 {
            let (so, sg) = (att(o as usize), att(g as usize));
            if so == sg || !dernier {
                if so {
                    cartes |= bit(EXCUSE);
                }
                if so != sg {
                    echange = if so { -1 } else { 1 };
                }
            } else if sg {
                cartes |= bit(EXCUSE);
            }
        }
        let demi = demi_points_de(cartes) + echange;
        let bouts = nb(cartes & BOUTS) as u8;
        let cible = CIBLES[bouts as usize];
        let diff = demi - 2 * cible;
        let fait = diff >= 0;
        let ecart = (diff.abs() + 1) / 2;
        let petit_bout = if d.petit_bout < 0 {
            0
        } else if att(d.petit_bout as usize) {
            1
        } else {
            -1
        };
        let signe = if fait { 1 } else { -1 };
        let mult = MULTIPLICATEURS[d.contrat as usize];
        let poignee: i32 = (0..np).map(|p| PRIMES_POIGNEE[d.poignees[p] as usize]).sum::<i32>() * signe;
        let plis_att: u32 = (0..np).filter(|&p| att(p)).map(|p| d.plis_gagnes[p] as u32).sum();
        let chelem = if plis_att as usize == self.taille_main() {
            200
        } else if plis_att == 0 {
            -200
        } else {
            0
        };
        let unite = ((25 + ecart) * signe + 10 * petit_bout as i32) * mult + poignee + chelem;
        let mut gains = [0i32; 5];
        let preneur = d.preneur as usize;
        let seul = d.partenaire < 0 || d.partenaire as usize == preneur;
        for p in 0..np {
            gains[p] = if p == preneur {
                unite * match np {
                    3 => 2,
                    4 => 3,
                    _ if seul => 4,
                    _ => 2,
                }
            } else if att(p) {
                unite
            } else {
                -unite
            };
        }
        Bilan {
            preneur: d.preneur,
            partenaire: if seul { -1 } else { d.partenaire },
            contrat: d.contrat,
            bouts,
            cible,
            demi,
            fait,
            ecart,
            petit_bout,
            poignee,
            chelem,
            unite,
            gains,
        }
    }

    fn compter(&mut self) {
        let b = self.calculer_bilan();
        for p in 0..self.np {
            self.scores[p] += b.gains[p];
        }
        self.bilan = Some(b);
        self.donne += 1;
        self.d.phase = if self.donne >= self.donnes { PH_FINPARTIE } else { PH_FINDONNE };
        let np = self.np;
        let s = self.scores;
        self.ecrire(|| {
            let g: Vec<String> = b.gains[..np].iter().map(|v| v.to_string()).collect();
            let t: Vec<String> = s[..np].iter().map(|v| v.to_string()).collect();
            format!("S {} {} {} {} {} {} | {} | {}", b.contrat, b.preneur, b.partenaire, b.demi, b.bouts, b.unite, g.join(" "), t.join(" "))
        });
    }

    /// Classement à la fin de la partie : joueurs du meilleur score au moins bon.
    pub fn classement(&self) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.np).collect();
        v.sort_by_key(|&p| -self.scores[p]);
        v
    }
}
