//! Règles du rami 51, sans graphisme et déterministes à partir de la graine : 2 à 5 joueurs, deux jeux de 52
//! cartes et 4 jokers, 13 cartes chacun, pioche au talon ou à la défausse, suites et brelans (au plus un
//! joker par combinaison), ouverture à 51 points, ajouts sur toutes les combinaisons de la table, échange
//! des jokers, défausse, rami, décompte des pénalités et fin de partie à la limite de points ; journal pour
//! le vérificateur indépendant (`check.py`).
//!
//! Les cartes sont numérotées de 0 à 107 : 0 à 103 les deux jeux (`c % 52` comme
//! `bistrot::cartes::Carte::jeu52` : 13 cartes par couleur, du 2 à l'as, pique, cœur, carreau, trèfle),
//! 104 à 107 les jokers. Chaque carte a son numéro : un ensemble de cartes tient dans un entier de 128 bits.

pub type Cartes = u128;

pub const NB_CARTES: u8 = 108;
pub const PREMIER_JOKER: u8 = 104;
pub const JOKERS: Cartes = 0xF << 104;
/// Points à poser en une fois pour ouvrir.
pub const OUVERTURE: i32 = 51;
pub const TAILLE_MAIN: usize = 13;
/// Pénalité d'un joueur qui n'a pas ouvert (au moins).
pub const SANS_OUVRIR: i32 = 100;
/// Talon refait au plus deux fois avec la défausse ; ensuite la manche est nulle.
pub const RECYCLAGES: u8 = 2;

pub const PH_PIOCHE: u8 = 1;
pub const PH_JEU: u8 = 2;
pub const PH_FINMANCHE: u8 = 3;
pub const PH_FINPARTIE: u8 = 4;

/// Limites de points de la partie (touche P).
pub const LIMITES: [i32; 4] = [100, 200, 300, 500];

#[inline]
pub fn bit(c: u8) -> Cartes {
    1u128 << c
}

#[inline]
pub fn nb(m: Cartes) -> u32 {
    m.count_ones()
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

pub fn ensemble(v: &[u8]) -> Cartes {
    v.iter().fold(0, |a, &c| a | bit(c))
}

#[inline]
pub fn est_joker(c: u8) -> bool {
    c >= PREMIER_JOKER
}

/// Couleur (0 à 3) d'une carte qui n'est pas un joker.
#[inline]
pub fn couleur(c: u8) -> u8 {
    c % 52 / 13
}

/// Rang : 0 (le 2) à 12 (l'as).
#[inline]
pub fn rang(c: u8) -> u8 {
    c % 52 % 13
}

/// Hauteur dans une suite : 2 à 14 (as haut) ; l'as peut aussi valoir 1 (as, 2, 3).
#[inline]
pub fn hauteur(c: u8) -> u8 {
    rang(c) + 2
}

/// Les deux exemplaires d'une même carte ont le même nom (couleur et rang).
#[inline]
pub fn nom(c: u8) -> u8 {
    if est_joker(c) {
        52
    } else {
        c % 52
    }
}

/// Points d'ouverture d'une carte selon la hauteur qu'elle tient : as bas 1, 2 à 10, figures 10, as haut 11.
pub fn valeur_hauteur(h: u8) -> i32 {
    match h {
        1 => 1,
        14 => 11,
        11..=13 => 10,
        h => h as i32,
    }
}

/// Pénalité d'une carte restée en main : joker 20, as 11, figures 10, les autres leur valeur.
pub fn penalite(c: u8) -> i32 {
    if est_joker(c) {
        20
    } else {
        valeur_hauteur(hauteur(c))
    }
}

pub fn penalites(m: Cartes) -> i32 {
    liste(m).into_iter().map(penalite).sum()
}

/// Couleurs écrites (les polices n'ont pas les symboles ♠ ♥ ♦ ♣, que le moteur dessine en formes).
pub const COULEURS: [&str; 4] = ["pique", "cœur", "carreau", "trèfle"];

/// Nom d'une hauteur (1 et 14 = as).
fn nom_hauteur(h: u8) -> &'static str {
    ["as", "2", "3", "4", "5", "6", "7", "8", "9", "10", "valet", "dame", "roi", "as"][h as usize - 1]
}

/// Texte d'une carte (« 7 de cœur », « roi de pique », « joker »).
pub fn texte(c: u8) -> String {
    if est_joker(c) {
        return "joker".into();
    }
    format!("{} de {}", nom_hauteur(hauteur(c)), COULEURS[couleur(c) as usize])
}

/// La carte avec son article (« le 7 de cœur », « l'as de pique », « la dame de trèfle », « le joker »).
pub fn la_carte(c: u8) -> String {
    let t = texte(c);
    if !est_joker(c) && rang(c) == 12 {
        format!("l'{t}")
    } else if !est_joker(c) && rang(c) == 10 {
        format!("la {t}")
    } else {
        format!("le {t}")
    }
}

/// Une combinaison posée : suite (au moins 3 cartes de la même couleur qui se suivent, l'as en bas ou en haut
/// mais pas les deux) ou brelan / carré (3 ou 4 cartes du même rang, de couleurs différentes). Au plus un joker,
/// qui garde la place qu'il a reçue en arrivant sur la table.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Combi {
    pub suite: bool,
    /// suite : de la plus basse à la plus haute, le joker à la place de la carte qu'il remplace ;
    /// brelan : les cartes par couleur, puis le joker
    pub cartes: Vec<u8>,
    /// suite : hauteur de la première carte (1 = as bas) ; brelan : rang
    pub base: u8,
    /// couleur d'une suite
    pub coul: u8,
}

impl Combi {
    pub fn ensemble(&self) -> Cartes {
        ensemble(&self.cartes)
    }

    /// Le joker de la combinaison : (place, carte).
    pub fn joker(&self) -> Option<(usize, u8)> {
        self.cartes.iter().position(|&c| est_joker(c)).map(|i| (i, self.cartes[i]))
    }

    /// Points d'ouverture (le joker vaut la carte qu'il remplace).
    pub fn points(&self) -> i32 {
        if self.suite {
            (0..self.cartes.len()).map(|i| valeur_hauteur(self.base + i as u8)).sum()
        } else {
            self.cartes.len() as i32 * valeur_hauteur(self.base + 2)
        }
    }

    /// Hauteur la plus haute d'une suite.
    pub fn haut(&self) -> u8 {
        self.base + self.cartes.len() as u8 - 1
    }

    /// Couleurs déjà présentes dans un brelan (bits 0 à 3), sans le joker.
    fn couleurs(&self) -> u8 {
        self.cartes.iter().filter(|&&c| !est_joker(c)).fold(0, |a, &c| a | 1 << couleur(c))
    }

    /// Description avec l'article (« la suite du 5 au 8 de cœur », « le brelan de 7 »).
    pub fn la_combi(&self) -> String {
        format!("{} {}", if self.suite { "la" } else { "le" }, self.decrire())
    }

    /// Description courte (« suite du 5 au 8 de cœur », « brelan de 7 »).
    pub fn decrire(&self) -> String {
        if self.suite {
            format!("suite du {} au {} de {}", nom_hauteur(self.base), nom_hauteur(self.haut()), COULEURS[self.coul as usize])
        } else {
            let r = ["2", "3", "4", "5", "6", "7", "8", "9", "10", "valets", "dames", "rois", "as"][self.base as usize];
            format!("{} de {r}", if self.cartes.len() == 4 { "carré" } else { "brelan" })
        }
    }
}

/// Suite faite des cartes `nat` (même couleur) et de `nj` jokers (0 ou 1), si possible.
fn faire_suite(nat: &[u8], joker: Option<u8>) -> Option<Combi> {
    let coul = couleur(*nat.first()?);
    for as_bas in [false, true] {
        let mut h: Vec<(u8, u8)> = nat.iter().map(|&c| (if rang(c) == 12 && as_bas { 1 } else { hauteur(c) }, c)).collect();
        h.sort();
        if h.windows(2).any(|w| w[0].0 == w[1].0) {
            return None;
        }
        let (mut lo, mut hi) = (h[0].0, h[h.len() - 1].0);
        let trous = (hi - lo + 1) as usize - h.len();
        let nj = joker.is_some() as usize;
        if trous > nj {
            continue;
        }
        if nj > trous {
            // joker en plus : en haut, ou en bas si la suite finit à l'as
            if hi < 14 {
                hi += 1;
            } else if lo > 1 {
                lo -= 1;
            } else {
                continue;
            }
        }
        if hi - lo + 1 < 3 || hi - lo + 1 > 13 {
            continue;
        }
        let cartes = (lo..=hi).map(|v| h.iter().find(|x| x.0 == v).map_or_else(|| joker.unwrap_or(0), |x| x.1)).collect();
        return Some(Combi { suite: true, cartes, base: lo, coul });
    }
    None
}

/// La combinaison formée par ces cartes (au moins trois), ou None.
pub fn former(m: Cartes) -> Option<Combi> {
    if nb(m) < 3 || nb(m & JOKERS) > 1 {
        return None;
    }
    let joker = (m & JOKERS != 0).then(|| (m & JOKERS).trailing_zeros() as u8);
    let mut nat = liste(m & !JOKERS);
    if nat.iter().all(|&c| rang(c) == rang(nat[0])) {
        nat.sort_by_key(|&c| couleur(c));
        if nat.windows(2).any(|w| couleur(w[0]) == couleur(w[1])) || nb(m) > 4 {
            return None;
        }
        let base = rang(nat[0]);
        nat.extend(joker);
        return Some(Combi { suite: false, cartes: nat, base, coul: 0 });
    }
    if nat.iter().any(|&c| couleur(c) != couleur(nat[0])) {
        return None;
    }
    faire_suite(&nat, joker)
}

/// La combinaison c complétée par les cartes m (le joker déjà posé garde sa place), ou None.
pub fn completer(c: &Combi, m: Cartes) -> Option<Combi> {
    if m == 0 || m & c.ensemble() != 0 || nb((m | c.ensemble()) & JOKERS) > 1 {
        return None;
    }
    if !c.suite {
        let r = former(m | c.ensemble())?;
        return (!r.suite && r.base == c.base).then_some(r);
    }
    let joker = (m & JOKERS != 0).then(|| (m & JOKERS).trailing_zeros() as u8);
    let nat = liste(m & !JOKERS);
    if nat.iter().any(|&x| couleur(x) != c.coul) {
        return None;
    }
    for as_bas in [false, true] {
        let mut v: Vec<(u8, u8)> = c.cartes.iter().enumerate().map(|(i, &x)| (c.base + i as u8, x)).collect();
        for &x in &nat {
            v.push((if rang(x) == 12 && as_bas { 1 } else { hauteur(x) }, x));
        }
        v.sort();
        if v.windows(2).any(|w| w[0].0 == w[1].0) {
            continue;
        }
        let (mut lo, mut hi) = (v[0].0, v[v.len() - 1].0);
        let trous = (hi - lo + 1) as usize - v.len();
        let nj = joker.is_some() as usize;
        if trous > nj {
            continue;
        }
        if nj > trous {
            if hi < 14 {
                hi += 1;
            } else if lo > 1 {
                lo -= 1;
            } else {
                continue;
            }
        }
        if hi - lo + 1 > 13 {
            continue;
        }
        let cartes = (lo..=hi).map(|h| v.iter().find(|x| x.0 == h).map_or_else(|| joker.unwrap_or(0), |x| x.1)).collect();
        return Some(Combi { suite: true, cartes, base: lo, coul: c.coul });
    }
    None
}

/// La carte x peut prendre la place du joker de la combinaison : la combinaison sans joker, et le joker.
pub fn remplacer_joker(c: &Combi, x: u8) -> Option<(Combi, u8)> {
    let (i, j) = c.joker()?;
    if est_joker(x) {
        return None;
    }
    if c.suite {
        let h = c.base + i as u8;
        let ok = couleur(x) == c.coul && (hauteur(x) == h || (h == 1 && rang(x) == 12));
        if !ok {
            return None;
        }
        let mut n = c.clone();
        n.cartes[i] = x;
        return Some((n, j));
    }
    if rang(x) != c.base || c.couleurs() >> couleur(x) & 1 == 1 {
        return None;
    }
    let r = former(c.ensemble() & !bit(j) | bit(x))?;
    Some((r, j))
}

/// Bilan d'une manche.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Bilan {
    /// -1 : manche nulle (talon épuisé)
    pub gagnant: i8,
    /// le gagnant a tout posé d'un coup sans avoir ouvert avant : pénalités doublées
    pub rami: bool,
    pub penalites: [i32; 5],
    /// cartes restées en main
    pub restes: [Cartes; 5],
    pub ouverts: [bool; 5],
}

/// État d'une manche (les bots s'en servent pour leurs essais).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Manche {
    pub phase: u8,
    /// joueur dont c'est le tour
    pub tour: u8,
    pub mains: [Cartes; 5],
    /// talon et défausse : la carte du dessus est la dernière
    pub talon: Vec<u8>,
    pub defausse: Vec<u8>,
    pub table: Vec<Combi>,
    /// qui a posé chaque combinaison
    pub auteurs: Vec<u8>,
    pub ouvert: [bool; 5],
    /// carte prise à la défausse ce tour-ci (à poser avant la fin du tour), -1 sinon
    pub pris: i8,
    /// jokers repris sur la table ce tour-ci (à reposer avant la fin du tour)
    pub jokers_pris: Cartes,
    /// points posés ce tour-ci par un joueur qui n'avait pas encore ouvert
    pub pose_tour: i32,
    /// le joueur avait ouvert au début de son tour (sinon, finir d'un coup est un rami)
    pub ouvert_avant: bool,
    pub recyclages: u8,
    /// tours joués dans la manche
    pub tours: u32,
    /// cartes prises à la défausse par chaque joueur (tout le monde les a vues)
    pub ramassees: [Cartes; 5],
    /// cartes défaussées par chaque joueur
    pub jetees: [Cartes; 5],
}

#[derive(Clone)]
pub struct Moteur {
    pub d: Manche,
    /// état du début du tour (après la pioche au talon) : ANNULER y revient
    pub sauvegarde: Option<Manche>,
    pub np: usize,
    pub donneur: usize,
    pub rng: u64,
    /// niveau de chaque siège : -1 humain, 0 facile, 1 normal, 2 difficile
    pub niveaux: [i8; 5],
    /// pénalités cumulées
    pub scores: [i32; 5],
    /// la partie s'arrête quand un joueur atteint la limite
    pub limite: i32,
    pub manche: u32,
    pub bilan: Option<Bilan>,
    pub journal: Vec<u8>,
    pub journal_on: bool,
    /// essai d'un bot : pas de journal
    pub simule: bool,
}

impl Default for Moteur {
    fn default() -> Moteur {
        Moteur::new()
    }
}

impl Moteur {
    pub fn new() -> Moteur {
        Moteur {
            d: Manche::default(),
            sauvegarde: None,
            np: 4,
            donneur: 0,
            rng: 0x2545_F491_4F6C_DD1D,
            niveaux: [-1, 1, 1, 1, 1],
            scores: [0; 5],
            limite: 200,
            manche: 0,
            bilan: None,
            journal: Vec::new(),
            journal_on: false,
            simule: false,
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

    fn melanger(&mut self, v: &mut [u8]) {
        for i in (1..v.len()).rev() {
            let j = self.rng_mod(i as u32 + 1) as usize;
            v.swap(i, j);
        }
    }

    // ---------- journal ----------
    fn ecrire(&mut self, ligne: impl FnOnce() -> String) {
        if self.journal_on && !self.simule {
            let l = ligne();
            self.journal.extend_from_slice(l.as_bytes());
            self.journal.push(b'\n');
        }
    }

    fn ecrire_liste(v: &[u8]) -> String {
        v.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(" ")
    }

    // ---------- partie et manches ----------
    pub fn nouvelle_partie(&mut self) {
        self.scores = [0; 5];
        self.manche = 0;
        self.bilan = None;
        self.donneur = self.rng_mod(self.np as u32) as usize;
        let (np, limite) = (self.np, self.limite);
        self.ecrire(|| format!("G {np} {limite}"));
        self.nouvelle_manche();
    }

    /// Mélange et distribue 13 cartes à chacun ; une carte retournée commence la défausse.
    pub fn nouvelle_manche(&mut self) {
        let mut jeu: Vec<u8> = (0..NB_CARTES).collect();
        self.melanger(&mut jeu);
        let mut d = Manche { phase: PH_PIOCHE, pris: -1, ..Manche::default() };
        let np = self.np;
        for k in 0..TAILLE_MAIN * np {
            let p = (self.donneur + 1 + k) % np;
            d.mains[p] |= bit(jeu.pop().unwrap_or(0));
        }
        d.defausse.push(jeu.pop().unwrap_or(0));
        d.talon = jeu;
        d.tour = ((self.donneur + 1) % np) as u8;
        self.d = d;
        self.bilan = None;
        let donneur = self.donneur;
        self.ecrire(|| format!("N {donneur}"));
        for p in 0..np {
            let m = self.d.mains[p];
            self.ecrire(|| format!("M {p} {}", Self::ecrire_liste(&liste(m))));
        }
        let (t, f) = (self.d.talon.clone(), self.d.defausse[0]);
        self.ecrire(|| format!("T {}", Self::ecrire_liste(&t)));
        self.ecrire(|| format!("D {f}"));
        self.debut_tour();
    }

    /// Manche suivante (après le bilan) : le donneur suivant distribue.
    pub fn manche_suivante(&mut self) {
        self.donneur = (self.donneur + 1) % self.np;
        self.nouvelle_manche();
    }

    /// Début du tour du joueur `tour` : talon refait avec la défausse s'il est vide, état mémorisé pour ANNULER.
    fn debut_tour(&mut self) {
        let p = self.d.tour as usize;
        self.d.phase = PH_PIOCHE;
        self.d.pris = -1;
        self.d.jokers_pris = 0;
        self.d.pose_tour = 0;
        self.d.ouvert_avant = self.d.ouvert[p];
        if self.d.talon.is_empty() {
            if self.d.recyclages >= RECYCLAGES || self.d.defausse.len() < 2 {
                self.fin_manche(None);
                return;
            }
            let dessus = self.d.defausse.pop().unwrap_or(0);
            let mut t = std::mem::take(&mut self.d.defausse);
            self.melanger(&mut t);
            self.d.talon = t;
            self.d.defausse.push(dessus);
            self.d.recyclages += 1;
            let t = self.d.talon.clone();
            self.ecrire(|| format!("R {}", Self::ecrire_liste(&t)));
        }
        self.sauvegarde = Some(self.d.clone());
    }

    pub fn joueur(&self) -> usize {
        self.d.tour as usize
    }

    // ---------- actions du joueur dont c'est le tour ----------
    /// Pioche au talon, ou prend la carte du dessus de la défausse (elle devra être posée ce tour-ci).
    pub fn piocher(&mut self, talon: bool) -> bool {
        if self.d.phase != PH_PIOCHE {
            return false;
        }
        let p = self.joueur();
        let c = if talon { self.d.talon.pop() } else { self.d.defausse.pop() };
        let Some(c) = c else { return false };
        self.d.mains[p] |= bit(c);
        self.d.phase = PH_JEU;
        if talon {
            // ANNULER ne remet pas la carte piochée : on repart d'ici
            self.sauvegarde = Some(self.d.clone());
            self.ecrire(|| format!("P {p} T {c}"));
        } else {
            self.d.pris = c as i8;
            self.d.ramassees[p] |= bit(c);
            self.ecrire(|| format!("P {p} D {c}"));
        }
        true
    }

    /// Pose une nouvelle combinaison faite des cartes m de la main. Avant d'avoir ouvert, les points posés dans
    /// le tour s'additionnent : le joueur ouvre en atteignant 51 ; il ne peut pas vider sa main sans ouvrir.
    pub fn poser(&mut self, m: Cartes) -> bool {
        let p = self.joueur();
        if self.d.phase != PH_JEU || m & !self.d.mains[p] != 0 {
            return false;
        }
        let Some(c) = former(m) else { return false };
        let reste = self.d.mains[p] & !m;
        if !self.d.ouvert[p] && reste == 0 && self.d.pose_tour + c.points() < OUVERTURE {
            return false;
        }
        if !self.d.ouvert_avant {
            self.d.pose_tour += c.points();
        }
        if !self.d.ouvert[p] && self.d.pose_tour >= OUVERTURE {
            self.d.ouvert[p] = true;
        }
        self.d.mains[p] = reste;
        let v = c.cartes.clone();
        self.d.table.push(c);
        self.d.auteurs.push(p as u8);
        self.ecrire(|| format!("C {p} {}", Self::ecrire_liste(&v)));
        self.verifier_fin();
        true
    }

    /// Le joueur dont c'est le tour peut toucher aux combinaisons de la table (il a ouvert).
    pub fn peut_completer(&self) -> bool {
        self.d.phase == PH_JEU && self.d.ouvert[self.joueur()]
    }

    /// Complète la combinaison i de la table avec les cartes m (après avoir ouvert).
    pub fn ajouter(&mut self, m: Cartes, i: usize) -> bool {
        let p = self.joueur();
        if !self.peut_completer() || m & !self.d.mains[p] != 0 || i >= self.d.table.len() {
            return false;
        }
        let Some(c) = completer(&self.d.table[i], m) else { return false };
        self.d.mains[p] &= !m;
        let v = c.cartes.clone();
        self.d.table[i] = c;
        self.ecrire(|| format!("A {p} {i} {}", Self::ecrire_liste(&v)));
        self.verifier_fin();
        true
    }

    /// Échange la carte x contre le joker de la combinaison i (après avoir ouvert) ; le joker devra être reposé
    /// avant la fin du tour.
    pub fn echanger(&mut self, x: u8, i: usize) -> bool {
        let p = self.joueur();
        if !self.peut_completer() || self.d.mains[p] & bit(x) == 0 || i >= self.d.table.len() {
            return false;
        }
        let Some((c, j)) = remplacer_joker(&self.d.table[i], x) else { return false };
        self.d.mains[p] = self.d.mains[p] & !bit(x) | bit(j);
        self.d.jokers_pris |= bit(j);
        let v = c.cartes.clone();
        self.d.table[i] = c;
        self.ecrire(|| format!("E {p} {i} {x} {j} {}", Self::ecrire_liste(&v)));
        true
    }

    /// Ce qui empêche de finir le tour (défausser), ou None.
    pub fn empechement(&self) -> Option<String> {
        if self.d.phase != PH_JEU {
            return Some("Il faut d'abord piocher.".into());
        }
        let p = self.joueur();
        let main = self.d.mains[p];
        if self.d.pris >= 0 && main & bit(self.d.pris as u8) != 0 {
            return Some(format!("La carte prise à la défausse ({}) doit être posée ce tour-ci.", texte(self.d.pris as u8)));
        }
        if main & self.d.jokers_pris != 0 {
            return Some("Le joker repris doit être reposé ce tour-ci.".into());
        }
        if !self.d.ouvert[p] && self.d.pose_tour > 0 {
            return Some(format!("Il faut {OUVERTURE} points pour ouvrir ({} posés) : complète, ou ANNULER.", self.d.pose_tour));
        }
        None
    }

    /// Défausse la carte c et finit le tour (la manche si c'était la dernière carte).
    pub fn defausser(&mut self, c: u8) -> bool {
        let p = self.joueur();
        if self.d.mains[p] & bit(c) == 0 || self.empechement().is_some() {
            return false;
        }
        self.d.mains[p] &= !bit(c);
        self.d.defausse.push(c);
        self.d.jetees[p] |= bit(c);
        self.ecrire(|| format!("X {p} {c}"));
        if self.d.mains[p] == 0 {
            self.fin_manche(Some(p));
            return true;
        }
        self.d.tours += 1;
        self.d.tour = ((p + 1) % self.np) as u8;
        self.debut_tour();
        true
    }

    /// Le tour a changé depuis la sauvegarde (ANNULER a un effet).
    pub fn annulable(&self) -> bool {
        matches!(self.d.phase, PH_PIOCHE | PH_JEU) && self.sauvegarde.as_ref().is_some_and(|s| *s != self.d)
    }

    /// Revient au début du tour (ou juste après la pioche au talon).
    pub fn annuler(&mut self) -> bool {
        if !self.annulable() {
            return false;
        }
        if let Some(s) = self.sauvegarde.clone() {
            self.d = s;
        }
        let p = self.joueur();
        self.ecrire(|| format!("U {p}"));
        true
    }

    /// Main vide après une pose : la manche est finie.
    fn verifier_fin(&mut self) {
        let p = self.joueur();
        if self.d.mains[p] == 0 && self.empechement().is_none() {
            self.fin_manche(Some(p));
        }
    }

    /// Fin de la manche : chacun compte ses cartes (au moins 100 s'il n'a pas ouvert), double au rami.
    fn fin_manche(&mut self, gagnant: Option<usize>) {
        let mut b = Bilan { gagnant: gagnant.map_or(-1, |g| g as i8), ..Bilan::default() };
        b.rami = gagnant.is_some() && !self.d.ouvert_avant;
        for q in 0..self.np {
            b.restes[q] = self.d.mains[q];
            b.ouverts[q] = self.d.ouvert[q];
            if gagnant.is_none() || gagnant == Some(q) {
                continue;
            }
            let mut pen = penalites(self.d.mains[q]);
            if !self.d.ouvert[q] {
                pen = pen.max(SANS_OUVRIR);
            }
            if b.rami {
                pen *= 2;
            }
            b.penalites[q] = pen;
            self.scores[q] += pen;
        }
        self.manche += 1;
        self.bilan = Some(b);
        let fin = self.scores[..self.np].iter().any(|&s| s >= self.limite);
        self.d.phase = if fin { PH_FINPARTIE } else { PH_FINMANCHE };
        let (g, r, pen) = (b.gagnant, b.rami as u8, b.penalites);
        let sc = self.scores;
        let np = self.np;
        self.ecrire(|| {
            let l = |v: &[i32]| v[..np].iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ");
            format!("F {g} {r} {} | {}", l(&pen), l(&sc))
        });
    }

    /// Classement : le moins pénalisé d'abord.
    pub fn classement(&self) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.np).collect();
        v.sort_by_key(|&p| self.scores[p]);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Carte (premier jeu) : couleur 0 à 3, hauteur 2 à 14.
    fn k(coul: u8, h: u8) -> u8 {
        coul * 13 + h - 2
    }

    #[test]
    fn combinaisons() {
        let j = PREMIER_JOKER;
        // suites
        let s = former(bit(k(1, 5)) | bit(k(1, 6)) | bit(k(1, 7))).unwrap();
        assert!(s.suite && s.base == 5 && s.points() == 18);
        assert!(former(bit(k(1, 5)) | bit(k(1, 6)) | bit(k(2, 7))).is_none());
        assert!(former(bit(k(1, 5)) | bit(k(1, 7)) | bit(k(1, 8))).is_none());
        let s = former(bit(k(1, 5)) | bit(k(1, 7)) | bit(j)).unwrap();
        assert_eq!(s.cartes, vec![k(1, 5), j, k(1, 7)]);
        let s = former(bit(k(0, 13)) | bit(k(0, 14)) | bit(j)).unwrap();
        assert_eq!((s.base, s.points()), (12, 31), "joker en bas sous l'as");
        let s = former(bit(k(0, 14)) | bit(k(0, 2)) | bit(k(0, 3))).unwrap();
        assert_eq!((s.base, s.points()), (1, 6), "as bas");
        assert!(former(bit(k(0, 13)) | bit(k(0, 14)) | bit(k(0, 2))).is_none(), "pas de suite tournante");
        assert!(former(bit(k(1, 5)) | bit(k(1, 6)) | bit(j) | bit(j + 1)).is_none(), "un seul joker");
        // brelans
        let b = former(bit(k(0, 9)) | bit(k(1, 9)) | bit(k(3, 9))).unwrap();
        assert!(!b.suite && b.points() == 27);
        assert!(former(bit(k(0, 9)) | bit(52 + k(0, 9)) | bit(k(3, 9))).is_none(), "deux fois la même couleur");
        let b = former(bit(k(0, 14)) | bit(k(1, 14)) | bit(j)).unwrap();
        assert_eq!(b.points(), 33);
        // compléter
        let s = former(bit(k(2, 5)) | bit(k(2, 6)) | bit(j)).unwrap();
        assert_eq!(s.cartes[2], j, "joker = 7");
        let s2 = completer(&s, bit(k(2, 8)) | bit(k(2, 9))).unwrap();
        assert_eq!(s2.cartes, vec![k(2, 5), k(2, 6), j, k(2, 8), k(2, 9)]);
        assert!(completer(&s, bit(k(2, 7))).is_none(), "le joker garde sa place");
        let s3 = completer(&s, bit(k(2, 4))).unwrap();
        assert_eq!(s3.base, 4);
        // échanger
        let (s4, jj) = remplacer_joker(&s, k(2, 7)).unwrap();
        assert_eq!((s4.cartes.clone(), jj), (vec![k(2, 5), k(2, 6), k(2, 7)], j));
        assert!(remplacer_joker(&s, k(2, 8)).is_none());
        let b = former(bit(k(0, 9)) | bit(k(1, 9)) | bit(j)).unwrap();
        assert!(remplacer_joker(&b, k(2, 9)).is_some());
        assert!(remplacer_joker(&b, k(1, 9) + 52).is_none(), "couleur déjà là");
        let c = completer(&b, bit(k(3, 9))).unwrap();
        assert_eq!(c.cartes.len(), 4);
        assert!(completer(&c, bit(k(2, 9))).is_none(), "carré complet");
    }

    #[test]
    fn manche_et_annuler() {
        let mut m = Moteur::new();
        m.rng = 99;
        m.nouvelle_partie();
        let p = m.joueur();
        assert_eq!(nb(m.d.mains[p]), 13);
        assert_eq!(m.d.talon.len() + m.d.defausse.len() + 13 * m.np, 108);
        let dessus = *m.d.defausse.last().unwrap();
        assert!(m.piocher(false));
        assert!(m.empechement().is_some(), "carte de la défausse à poser");
        let c = liste(m.d.mains[p] & !bit(dessus))[0];
        assert!(!m.defausser(c));
        assert!(m.annuler());
        assert_eq!(m.d.phase, PH_PIOCHE);
        assert!(m.piocher(true));
        assert!(!m.annuler(), "pas d'annulation de la pioche au talon");
        let c = liste(m.d.mains[p])[0];
        assert!(m.defausser(c));
        assert_eq!(m.joueur(), (p + 1) % m.np);
    }
}
