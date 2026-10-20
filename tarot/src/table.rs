//! Déroulement d'une partie de tarot autour des règles : tours des bots, délais (le chien reste montré, les
//! cartes volent, le pli est ramassé), bulles, conseils, écart choisi à la souris, poignée, « suite » à la fin
//! des donnes, actions en ligne. Sans graphisme : le même code sert à la fenêtre et aux tests. Tout ce qui
//! touche à la partie est déterministe (graine + suite des actions) : jeu en ligne « lockstep ».

use crate::engine::*;
pub use bistrot::reseau::Frame;
use bistrot::reseau::{Partie, T_ACTION};
use bistrot::table::{relative, Place, BAS, DROITE, GAUCHE, HAUT, HAUT_DROITE, HAUT_GAUCHE};
use std::collections::VecDeque;

pub const NT_BID: u8 = T_ACTION;
pub const NT_APPEL: u8 = T_ACTION + 1;
pub const NT_ECART: u8 = T_ACTION + 2;
pub const NT_PLAY: u8 = T_ACTION + 3;
pub const NT_NEXT: u8 = T_ACTION + 4;

pub const LEVEL_NAMES: [&str; 3] = ["facile", "normal", "difficile"];
/// Nombre de donnes d'une partie (touche P).
pub const DONNES: [u32; 4] = [5, 10, 15, 20];
/// Les habitués de la taverne, autour de la table (dans l'ordre du jeu à partir du joueur local).
const NOMS: [&str; 5] = ["TOI", "GASTON", "MARGOT", "ERNEST", "LUCIE"];
/// Temps pendant lequel le chien reste montré (ms).
pub const CHIEN_MONTRE: f64 = 2300.0;

pub struct Table {
    pub m: Moteur,
    /// siège du joueur de cet ordinateur
    pub me: usize,
    pub botlvl: i8,
    pub help: bool,
    /// l'ordinateur joue à la place du joueur local (démonstration, tests)
    pub auto: bool,
    /// masque des sièges humains
    pub humans: u32,
    /// sièges qui ont appuyé sur SUITE (en ligne)
    pub nextmask: u32,
    pub online: bool,
    pub now: f64,
    wake: f64,
    /// début du ramassage du pli (0 = pas en cours)
    pub collect: f64,
    /// carte conseillée (aide), -1 = aucune
    pub hint: i32,
    /// départ de la carte jouée par chaque joueur : instant et position
    pub ast0: [f64; 5],
    pub asx: [f64; 5],
    pub asy: [f64; 5],
    /// bulles : texte et fin d'affichage
    pub bub: [Option<(String, f64)>; 5],
    /// victoires / défaites du joueur local (parties locales)
    pub wins: [u32; 2],
    pub save_needed: bool,
    pub outbox: Vec<Frame>,
    pub inbox: VecDeque<Frame>,
    /// écart en préparation par le joueur local
    pub selection: Cartes,
    /// le joueur local montrera sa poignée avec sa première carte
    pub poignee: bool,
    /// instant où le chien a été retourné (petite et garde)
    pub chien_vu: f64,
    /// numéro de la donne affichée (change à chaque distribution)
    pub numero: u32,
}

impl Table {
    pub fn new(seed: u64) -> Table {
        let mut m = Moteur::new();
        m.rng = seed | 1;
        let mut t = Table {
            m,
            me: 0,
            botlvl: 1,
            help: false,
            auto: false,
            humans: 1,
            nextmask: 0,
            online: false,
            now: 10_000.0,
            wake: 0.0,
            collect: 0.0,
            hint: -1,
            ast0: [0.0; 5],
            asx: [0.0; 5],
            asy: [0.0; 5],
            bub: Default::default(),
            wins: [0, 0],
            save_needed: false,
            outbox: Vec::new(),
            inbox: VecDeque::new(),
            selection: 0,
            poignee: false,
            chien_vu: 0.0,
            numero: 0,
        };
        t.new_match();
        t
    }

    pub fn np(&self) -> usize {
        self.m.np
    }

    pub fn phase(&self) -> u8 {
        self.m.d.phase
    }

    pub fn turn(&self) -> usize {
        self.m.d.tour as usize
    }

    /// Place d'affichage du joueur p autour de la table (0 = en bas, puis dans le sens du jeu).
    pub fn disp_pos(&self, p: usize) -> usize {
        relative(p, self.me, self.np())
    }

    pub fn places(&self) -> &'static [Place] {
        disposition(self.np())
    }
}

/// Le joueur local : étiquette au-dessus de la gauche de sa main (une main de tarot est large).
const MOI: Place = Place { etiquette: (112.0, 572.0), donneur_a_gauche: true, ..BAS };
/// À gauche et à droite, les mains de 15 à 24 cartes sont hautes : l'étiquette passe sur la table, sous la chandelle.
const GAUCHE_T: Place = Place { etiquette: (232.0, 232.0), ..GAUCHE };
const DROITE_T: Place = Place { etiquette: (868.0, 232.0), ..DROITE };
/// À cinq, les deux mains du haut sont resserrées pour ne pas se chevaucher ni passer sous les panneaux.
const HAUT_GAUCHE_T: Place = Place { main: (440.0, 96.0), largeur: 170.0, pas: 20.0, etiquette: (440.0, 24.0), bulle: (440.0, 172.0), ..HAUT_GAUCHE };
const HAUT_DROITE_T: Place = Place { main: (660.0, 96.0), largeur: 170.0, pas: 20.0, etiquette: (660.0, 24.0), bulle: (660.0, 172.0), ..HAUT_DROITE };

/// Places du tarot pour 3, 4 ou 5 joueurs, à partir du joueur local.
pub fn disposition(n: usize) -> &'static [Place] {
    match n {
        3 => &[MOI, GAUCHE_T, DROITE_T],
        5 => &[MOI, GAUCHE_T, HAUT_GAUCHE_T, HAUT_DROITE_T, DROITE_T],
        _ => &[MOI, GAUCHE_T, HAUT, DROITE_T],
    }
}

impl Table {

    /// Le joueur p est-il le joueur de cet ordinateur (et pas joué par l'ordinateur) ?
    pub fn is_human(&self, p: usize) -> bool {
        !self.auto && p == self.me && self.m.niveaux[p] == -1
    }

    fn is_bot(&self, p: usize) -> bool {
        self.m.niveaux[p] >= 0 || (self.auto && p == self.me)
    }

    pub fn player_name(&self, p: usize) -> String {
        let rel = self.disp_pos(p);
        if rel == 0 {
            "TOI".into()
        } else if self.m.niveaux[p] == -1 {
            format!("JOUEUR {}", p + 1)
        } else {
            NOMS[rel].into()
        }
    }

    /// Le joueur local sait que p est dans son camp (couleur de son étiquette).
    pub fn ally_of_me(&self, p: usize) -> bool {
        if self.m.d.preneur < 0 {
            return p == self.me;
        }
        self.m.allie(self.me, p)
    }

    fn set_bubble(&mut self, p: usize, text: String, ms: f64) {
        self.bub[p] = Some((text, self.now + ms));
    }

    // ---------- déroulement ----------
    pub fn set_levels(&mut self) {
        for p in 0..5 {
            self.m.niveaux[p] = if self.humans >> p & 1 == 1 { -1 } else { self.botlvl };
        }
    }

    pub fn new_match(&mut self) {
        self.set_levels();
        self.nextmask = 0;
        self.m.nouvelle_partie();
        self.after_deal();
    }

    /// Après chaque distribution (nouvelle donne, donne blanche) : le temps de distribuer à l'écran.
    fn after_deal(&mut self) {
        self.bub = Default::default();
        self.hint = -1;
        self.ast0 = [self.now - 2000.0; 5];
        self.collect = 0.0;
        self.selection = 0;
        self.poignee = false;
        self.chien_vu = 0.0;
        self.numero += 1;
        self.wake = self.now + 1700.0;
    }

    fn apply_bid(&mut self, c: u8) {
        let p = self.turn();
        let blanches = self.m.blanches;
        if !self.m.encherir(c) {
            return;
        }
        self.set_bubble(p, CONTRATS[c as usize].into(), if c == 0 { 1400.0 } else { 2000.0 });
        self.wake = self.now + 900.0;
        self.hint = -1;
        if self.m.blanches > blanches {
            // tout le monde a passé : on redonne
            self.after_deal();
            self.bub[p] = Some(("Passe".into(), self.now + 1400.0));
            return;
        }
        self.after_contract();
    }

    /// Le contrat est connu (et la carte appelée à cinq) : chien montré pour la petite et la garde.
    fn after_contract(&mut self) {
        if self.phase() == PH_ECART {
            self.chien_vu = self.now + 500.0;
            self.wake = self.chien_vu + CHIEN_MONTRE + 400.0;
        }
    }

    fn apply_appel(&mut self, c: u8) {
        let p = self.turn();
        if !self.m.appeler(c) {
            return;
        }
        let r = ["Roi", "Dame", "Cavalier", "Valet"][(13 - c % 14) as usize];
        self.set_bubble(p, format!("J'appelle le {r} de {}", COULEURS[c as usize / 14]), 2600.0);
        self.wake = self.now + 1300.0;
        self.after_contract();
    }

    fn apply_ecart(&mut self, e: Cartes) {
        if self.m.ecarter(e) {
            self.selection = 0;
            self.wake = self.now + 900.0;
            let atouts = nb(e & ATOUTS);
            if atouts > 0 {
                let p = self.m.d.preneur as usize;
                self.set_bubble(p, format!("{atouts} atout(s) écarté(s)"), 2600.0);
            }
        }
    }

    /// Carte jouée par le joueur dont c'est le tour (avec sa poignée éventuelle).
    fn apply_play(&mut self, c: u8, poignee: u8) {
        let p = self.turn();
        let dp = self.disp_pos(p);
        let pl = self.places()[dp];
        let (x, y) = if dp == 0 {
            let main = trier(self.m.d.mains[p]);
            let i = main.iter().position(|&x| x == c).unwrap_or(0);
            (main_x(i, main.len()), pl.main.1)
        } else {
            pl.main
        };
        if !self.m.jouer(c, poignee) {
            return;
        }
        self.asx[p] = x;
        self.asy[p] = y;
        self.ast0[p] = self.now;
        if poignee > 0 {
            let n = seuils_poignee(self.np())[poignee as usize - 1];
            self.set_bubble(p, format!("{} ({n} atouts) !", POIGNEES[poignee as usize]), 2800.0);
        }
        if c as i8 == self.m.d.appel && self.m.d.partenaire == p as i8 {
            self.set_bubble(p, "C'est moi le partenaire !".into(), 2400.0);
        }
        if p == self.me {
            self.poignee = false;
        }
        self.wake = self.now + if self.phase() == PH_FINPLI { 1250.0 } else { 700.0 };
        self.hint = -1;
    }

    /// Le pli a fini de glisser vers son gagnant : on le range.
    fn collect_done(&mut self) {
        self.collect = 0.0;
        self.m.ramasser();
        if self.phase() == PH_FINPARTIE && !self.online {
            let premier = self.m.classement()[0];
            self.wins[(premier != self.me) as usize] += 1;
            self.save_needed = true;
        }
        self.wake = self.now + 500.0;
    }

    /// Avance le temps de dt millisecondes : bots, chien, ramassage des plis, actions reçues.
    pub fn tick(&mut self, dt: f64) {
        self.now += dt;
        if self.online {
            self.pump();
        }
        if self.help && self.hint == -1 && self.phase() == PH_JEU {
            self.update_hint();
        }
        if self.now < self.wake {
            return;
        }
        let p = self.turn();
        match self.phase() {
            PH_ENCHERES if self.is_bot(p) => {
                let c = self.m.bot_enchere(p);
                if self.m.niveaux[p] == -1 {
                    self.human_bid(c); // siège humain joué par l'ordinateur (U) : action diffusée
                } else {
                    self.apply_bid(c);
                }
            }
            PH_APPEL if self.is_bot(p) => {
                let c = self.m.bot_appel(p);
                if self.m.niveaux[p] == -1 {
                    self.human_appel(c);
                } else {
                    self.apply_appel(c);
                }
            }
            PH_ECART if self.is_bot(p) => {
                let e = self.m.bot_ecart(p);
                if self.m.niveaux[p] == -1 {
                    self.human_ecart(e);
                } else {
                    self.apply_ecart(e);
                }
            }
            PH_JEU if self.is_bot(p) => {
                let (c, pg) = self.m.bot_carte(p);
                if self.m.niveaux[p] == -1 {
                    self.send(NT_PLAY, c as u16, pg as u32, 0);
                }
                self.apply_play(c, pg);
            }
            PH_FINPLI => {
                if self.collect == 0.0 {
                    // le pli reste visible, puis glisse vers le gagnant
                    self.collect = self.now;
                    self.wake = self.now + 320.0;
                } else {
                    self.collect_done();
                }
            }
            PH_FINDONNE | PH_FINPARTIE if self.auto && self.nextmask >> self.me & 1 == 0 => self.human_next(),
            _ => {}
        }
    }

    // ---------- actions du joueur local ----------
    fn send(&mut self, kind: u8, b: u16, c: u32, d: u32) {
        if self.online {
            self.outbox.push(Frame { kind, b1: self.me as u8, b, c, d });
        }
    }

    /// Le joueur local peut agir maintenant (sa décision, et le chien a fini d'être montré).
    pub fn my_turn(&self) -> bool {
        self.is_human(self.turn()) && (self.phase() != PH_ECART || self.now >= self.chien_vu + CHIEN_MONTRE)
    }

    pub fn human_bid(&mut self, c: u8) {
        if !self.m.enchere_permise(c) {
            return;
        }
        self.send(NT_BID, c as u16, 0, 0);
        self.apply_bid(c);
    }

    pub fn human_appel(&mut self, c: u8) {
        if self.m.appels_possibles() & bit(c) == 0 {
            return;
        }
        self.send(NT_APPEL, c as u16, 0, 0);
        self.apply_appel(c);
    }

    pub fn human_ecart(&mut self, e: Cartes) {
        if !self.m.ecart_valide(e) {
            return;
        }
        self.send(NT_ECART, (e >> 64) as u16, e as u32, (e >> 32) as u32);
        self.apply_ecart(e);
    }

    /// Clic sur une carte de la main pendant l'écart : la choisir ou la remettre.
    pub fn toggle_selection(&mut self, c: u8) {
        if self.phase() != PH_ECART || !self.is_human(self.turn()) {
            return;
        }
        if self.selection & bit(c) != 0 {
            self.selection &= !bit(c);
        } else if self.m.ecart_possible() & bit(c) != 0 && (nb(self.selection) as usize) < self.m.taille_chien() {
            self.selection |= bit(c);
        }
    }

    pub fn human_play(&mut self, c: u8) {
        if self.phase() != PH_JEU || self.m.legales(self.me) & bit(c) == 0 {
            return;
        }
        let pg = if self.poignee { self.m.poignee_possible(self.me) } else { 0 };
        self.send(NT_PLAY, c as u16, pg as u32, 0);
        self.apply_play(c, pg);
    }

    /// SUITE (fin de donne / de partie).
    pub fn human_next(&mut self) {
        if self.online {
            self.send(NT_NEXT, 0, 0, 0);
            self.mark_next(self.me);
        } else {
            self.do_next_step();
        }
    }

    fn mark_next(&mut self, seat: usize) {
        self.nextmask |= 1 << seat;
        if self.nextmask & self.humans == self.humans {
            self.nextmask = 0;
            self.do_next_step();
        }
    }

    fn do_next_step(&mut self) {
        match self.phase() {
            PH_FINPARTIE => self.new_match(),
            PH_FINDONNE => {
                self.m.donne_suivante();
                self.after_deal();
            }
            _ => {}
        }
    }

    /// Applique dans l'ordre les actions reçues, chacune quand c'est bien le tour de son auteur.
    fn pump(&mut self) {
        while let Some(f) = self.inbox.front().copied() {
            let seat = f.b1 as usize;
            let ph = self.phase();
            let a_lui = seat == self.turn();
            match f.kind {
                NT_BID if ph == PH_ENCHERES && a_lui => {
                    self.inbox.pop_front();
                    self.apply_bid(f.b as u8);
                }
                NT_APPEL if ph == PH_APPEL && a_lui => {
                    self.inbox.pop_front();
                    self.apply_appel(f.b as u8);
                }
                NT_ECART if ph == PH_ECART && a_lui => {
                    self.inbox.pop_front();
                    self.apply_ecart((f.b as u128) << 64 | (f.d as u128) << 32 | f.c as u128);
                }
                NT_PLAY if ph == PH_JEU && a_lui => {
                    self.inbox.pop_front();
                    self.apply_play(f.b as u8, f.c as u8);
                }
                NT_NEXT if ph == PH_FINDONNE || ph == PH_FINPARTIE => {
                    self.inbox.pop_front();
                    self.mark_next(seat);
                }
                NT_BID | NT_APPEL | NT_ECART | NT_PLAY | NT_NEXT => return, // pas encore son tour
                _ => {
                    self.inbox.pop_front();
                }
            }
        }
    }

    /// Démarre une partie en ligne. `humans` : sièges humains ; `config` : joueurs - 3 (bits 0-1), niveau des
    /// bots (bits 2-3), nombre de donnes (bits 4-5).
    pub fn net_begin(&mut self, me: usize, humans: u8, config: u16, seed: u64) {
        self.me = me;
        self.humans = (humans & 31) as u32;
        self.m.np = 3 + (config & 3).min(2) as usize;
        self.botlvl = ((config >> 2 & 3) as i8).min(2);
        self.m.donnes = DONNES[(config >> 4 & 3) as usize];
        self.m.rng = seed;
        self.online = true;
        self.inbox.clear();
        self.outbox.clear();
        self.nextmask = 0;
        self.new_match();
    }

    pub fn net_config(&self) -> u16 {
        let d = DONNES.iter().position(|&n| n == self.m.donnes).unwrap_or(1) as u16;
        (self.np() as u16 - 3) | (self.botlvl as u16) << 2 | d << 4
    }

    /// Fin du jeu en ligne : retour à une partie locale.
    pub fn leave_online(&mut self) {
        self.online = false;
        self.humans = 1;
        self.me = 0;
        self.inbox.clear();
        self.outbox.clear();
        self.new_match();
    }

    // ---------- aide ----------
    /// Carte conseillée au joueur local (calculée sur une copie : la partie n'est pas touchée).
    fn update_hint(&mut self) {
        self.hint = -1;
        let p = self.turn();
        if self.help && self.phase() == PH_JEU && self.is_human(p) {
            let leg = self.m.legales(p);
            self.hint = if nb(leg) == 1 { leg.trailing_zeros() as i32 } else { self.m.heuristique(p, leg) as i32 };
        }
    }

    pub fn toggle_help(&mut self) {
        self.help = !self.help;
        self.update_hint();
        self.save_needed = true;
    }

    /// Message du bas de l'écran.
    pub fn message(&self) -> String {
        let p = self.turn();
        let human = self.is_human(p);
        let k = self.m.taille_chien();
        match self.phase() {
            PH_ENCHERES if human && self.help => {
                let mut c = self.m.clone();
                format!("Conseil : {}", CONTRATS[c.bot_enchere(p) as usize])
            }
            PH_ENCHERES if human => "À toi de parler : choisis un contrat, ou passe.".into(),
            PH_APPEL if human => "Appelle un roi : son détenteur sera ton partenaire (en secret).".into(),
            PH_ECART if human && self.now < self.chien_vu + CHIEN_MONTRE => "Voici le chien...".into(),
            PH_ECART if human => format!("Écart : choisis {k} cartes ({} / {k}) ; ni roi ni bout.", nb(self.selection)),
            PH_JEU if human && self.m.poignee_possible(p) > 0 && !self.poignee => {
                "Tu peux montrer une poignée avec ta première carte (bouton Poignée).".into()
            }
            PH_JEU if human && self.m.d.n_pli == 0 => "Tu entames : joue la carte de ton choix.".into(),
            PH_JEU if human => "À toi de jouer : clique sur une carte.".into(),
            PH_FINDONNE | PH_FINPARTIE if self.nextmask >> self.me & 1 == 1 => "En attente des autres joueurs...".into(),
            PH_FINDONNE | PH_FINPARTIE => "Espace : suite".into(),
            PH_ECART => "Le preneur fait son écart...".into(),
            _ => "Les autres réfléchissent...".into(),
        }
    }

    /// Cartes jouables du joueur local (toutes si ce n'est pas à lui de jouer).
    pub fn my_legal(&self) -> Cartes {
        if self.phase() == PH_JEU && self.turn() == self.me {
            self.m.legales(self.me)
        } else {
            TOUTES
        }
    }

    // ---------- réglages ----------
    pub fn cycle_np(&mut self) {
        if self.online {
            return;
        }
        self.m.np = if self.m.np >= 5 { 3 } else { self.m.np + 1 };
        self.new_match();
        self.save_needed = true;
    }

    pub fn cycle_level(&mut self) {
        if self.online {
            return;
        }
        self.botlvl = (self.botlvl + 1) % 3;
        self.set_levels();
        self.save_needed = true;
    }

    pub fn cycle_donnes(&mut self) {
        if self.online {
            return;
        }
        let i = DONNES.iter().position(|&n| n == self.m.donnes).unwrap_or(1);
        self.m.donnes = DONNES[(i + 1) % DONNES.len()];
        self.save_needed = true;
    }
}

/// Abscisse de la carte i (sur n) de la main du bas : plus large que la belote (jusqu'à 30 cartes à l'écart).
pub fn main_x(i: usize, n: usize) -> f64 {
    let pas = if n > 1 { 76.0f64.min(960.0 / (n - 1) as f64) } else { 76.0 };
    550.0 + (i as f64 - (n as f64 - 1.0) / 2.0) * pas
}

/// Carte de la main du bas (n cartes) sous le point (x, y), ou None.
pub fn main_touchee(n: usize, x: f32, y: f32) -> Option<usize> {
    if !(590.0..=764.0).contains(&y) {
        return None;
    }
    (0..n).rev().find(|&i| (x as f64 - main_x(i, n)).abs() <= 52.0)
}

/// Jeu en ligne : sièges des invités, configuration de départ, actions.
impl Partie for Table {
    fn sieges_invites(&self) -> Vec<usize> {
        (1..self.np()).collect()
    }

    fn configuration(&self, humains: u32) -> (u8, u16) {
        (humains as u8, self.net_config())
    }

    fn demarrer(&mut self, siege: usize, b1: u8, b: u16, graine: u64) {
        self.net_begin(siege, b1, b, graine);
    }

    fn quitter(&mut self) {
        self.leave_online();
    }

    fn en_ligne(&self) -> bool {
        self.online
    }

    fn sortantes(&mut self) -> Vec<Frame> {
        self.outbox.drain(..).collect()
    }

    fn recue(&mut self, f: Frame) {
        if self.online && self.inbox.len() < 256 {
            self.inbox.push_back(f);
        }
    }

    fn graine_locale(&self) -> u64 {
        self.m.rng
    }

    fn jeu(&self) -> u32 {
        0x5441_5201 // « TAR », protocole 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Joue tout seul (mode U) des parties entières à 3, 4 et 5 joueurs.
    #[test]
    fn parties_automatiques() {
        for np in [3usize, 4, 5] {
            let mut t = Table::new(4242 + np as u64);
            t.m.np = np;
            t.m.donnes = 5;
            t.new_match();
            t.auto = true;
            let mut parties = 0;
            for _ in 0..400_000 {
                let avant = t.phase();
                t.tick(50.0);
                if avant == PH_FINPARTIE && t.phase() != PH_FINPARTIE {
                    parties += 1;
                    if parties == 2 {
                        break;
                    }
                }
            }
            assert_eq!(parties, 2, "np={np} : parties non terminées");
        }
    }

    /// Plusieurs machines en lockstep, à des vitesses différentes : les parties restent identiques.
    #[test]
    fn lockstep_machines() {
        for np in [3usize, 4, 5] {
            let seed = 0x1234_5678_9ABC_DEF1;
            let humans = (1u8 << np) - 1;
            let mut ts: Vec<Table> = (0..np).map(|k| Table::new(k as u64 + 7)).collect();
            for (k, t) in ts.iter_mut().enumerate() {
                t.m.np = np;
                t.m.donnes = 5;
                let cfg = t.net_config();
                t.net_begin(k, humans, cfg, seed);
                t.auto = true;
            }
            let speeds = [50.0, 31.0, 77.0, 43.0, 64.0];
            let mut hist: Vec<Vec<([i32; 5], u64)>> = vec![Vec::new(); np];
            let mut was_end = [false; 5];
            for _ in 0..400_000 {
                for k in 0..np {
                    ts[k].tick(speeds[k]);
                    let end = ts[k].phase() == PH_FINDONNE || ts[k].phase() == PH_FINPARTIE;
                    if end && !was_end[k] {
                        hist[k].push((ts[k].m.scores, ts[k].m.rng));
                    }
                    was_end[k] = end;
                    let out: Vec<Frame> = ts[k].outbox.drain(..).collect();
                    for f in out {
                        for j in 0..np {
                            if j != k {
                                ts[j].inbox.push_back(f);
                            }
                        }
                    }
                }
                if hist.iter().all(|h| h.len() >= 6) {
                    break;
                }
            }
            assert!(hist.iter().all(|h| h.len() >= 6), "np={np} : donnes jouées {:?}", hist.iter().map(|h| h.len()).collect::<Vec<_>>());
            for h in &hist {
                assert_eq!(h[..6], hist[0][..6], "np={np} : machines désynchronisées");
            }
        }
    }
}
