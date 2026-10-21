//! Déroulement d'une partie de rami autour des règles : tours des bots (une action à la fois, le temps de la
//! voir), sélection des cartes à la souris, actions du joueur local (avec la raison quand elles sont refusées),
//! bulles, conseils, « suite » à la fin des manches, actions en ligne. Sans graphisme : le même code sert à la
//! fenêtre et aux tests. Tout ce qui touche à la partie est déterministe (graine + suite des actions) : jeu en
//! ligne « lockstep ».

use crate::bots::Action;
use crate::engine::*;
pub use bistrot::reseau::Frame;
use bistrot::reseau::{Partie, T_ACTION};
use bistrot::table::{relative, Place, BAS, DROITE, GAUCHE, HAUT, HAUT_DROITE, HAUT_GAUCHE};
use std::collections::VecDeque;

pub const NT_PIOCHE: u8 = T_ACTION;
pub const NT_POSER: u8 = T_ACTION + 1;
pub const NT_AJOUTER: u8 = T_ACTION + 2;
pub const NT_ECHANGER: u8 = T_ACTION + 3;
pub const NT_DEFAUSSE: u8 = T_ACTION + 4;
pub const NT_ANNULER: u8 = T_ACTION + 5;
pub const NT_NEXT: u8 = T_ACTION + 6;

pub const LEVEL_NAMES: [&str; 3] = ["facile", "normal", "difficile"];
/// Les habitués de la taverne, autour de la table (dans l'ordre du jeu à partir du joueur local).
const NOMS: [&str; 5] = ["TOI", "GASTON", "MARGOT", "ERNEST", "LUCIE"];
/// Délais des bots (ms) : avant de piocher, entre deux poses, avant de défausser.
const D_PIOCHE: f64 = 750.0;
const D_ACTION: f64 = 800.0;
/// Durée d'affichage d'un message d'erreur (ms).
const D_ALERTE: f64 = 3500.0;

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
    /// actions prévues par le bot dont c'est le tour
    plan: VecDeque<Action>,
    /// cartes conseillées (aide) : combinaison à poser, ou carte à défausser ; pioche conseillée (1 talon, 0 défausse)
    pub hint: Cartes,
    pub hint_pioche: i8,
    /// texte du conseil
    hint_texte: String,
    hint_pour: Option<Manche>,
    /// bulles : texte et fin d'affichage
    pub bub: [Option<(String, f64)>; 5],
    /// message d'erreur passager : texte et fin d'affichage
    pub alerte: Option<(String, f64)>,
    /// victoires / défaites du joueur local (parties locales)
    pub wins: [u32; 2],
    pub save_needed: bool,
    pub outbox: Vec<Frame>,
    pub inbox: VecDeque<Frame>,
    /// cartes choisies par le joueur local
    pub selection: Cartes,
    /// main triée par valeur (sinon par couleur)
    pub tri_valeur: bool,
    /// numéro de la manche affichée (change à chaque distribution)
    pub numero: u32,
    /// dernière pioche : instant, joueur, talon (pour l'animation des cartes cachées)
    pub pioche: (f64, usize, bool),
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
            plan: VecDeque::new(),
            hint: 0,
            hint_pioche: -1,
            hint_texte: String::new(),
            hint_pour: None,
            bub: Default::default(),
            alerte: None,
            wins: [0, 0],
            save_needed: false,
            outbox: Vec::new(),
            inbox: VecDeque::new(),
            selection: 0,
            tri_valeur: false,
            numero: 0,
            pioche: (0.0, 0, true),
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

    fn set_bubble(&mut self, p: usize, text: String, ms: f64) {
        self.bub[p] = Some((text, self.now + ms));
    }

    fn alerter(&mut self, s: impl Into<String>) {
        self.alerte = Some((s.into(), self.now + D_ALERTE));
    }

    /// Main du joueur local, dans l'ordre d'affichage.
    pub fn ma_main(&self) -> Vec<u8> {
        trier(self.m.d.mains[self.me], self.tri_valeur)
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

    /// Après chaque distribution : le temps de distribuer à l'écran.
    fn after_deal(&mut self) {
        self.bub = Default::default();
        self.alerte = None;
        self.plan.clear();
        self.selection = 0;
        self.hint = 0;
        self.hint_pioche = -1;
        self.hint_pour = None;
        self.numero += 1;
        self.wake = self.now + 2300.0;
    }

    /// Applique une action du joueur dont c'est le tour (déjà vérifiée ou venant du réseau) : bulles, délais.
    fn apply(&mut self, a: Action) -> bool {
        let p = self.turn();
        let ouvert = self.m.d.ouvert[p];
        let dessus = self.m.d.defausse.last().copied();
        let manche = self.m.manche;
        if !self.m.appliquer(a) {
            return false;
        }
        match a {
            Action::Pioche(talon) => {
                self.pioche = (self.now, p, talon);
                if !talon {
                    if let Some(c) = dessus {
                        self.set_bubble(p, format!("Je prends {}", la_carte(c)), 1600.0);
                    }
                }
            }
            Action::Echanger(..) => {
                self.set_bubble(p, "Je reprends le joker".into(), 1600.0);
            }
            _ => {}
        }
        if !ouvert && self.m.d.ouvert[p] {
            let pts = self.m.d.pose_tour;
            self.set_bubble(p, format!("J'ouvre : {pts} points"), 2400.0);
        }
        if self.m.manche != manche {
            // fin de la manche
            if let Some(b) = self.m.bilan {
                if b.gagnant >= 0 {
                    let g = b.gagnant as usize;
                    self.set_bubble(g, if b.rami { "Rami !".into() } else { "Fini !".into() }, 3000.0);
                }
            }
            if self.phase() == PH_FINPARTIE && !self.online {
                let premier = self.m.classement()[0];
                self.wins[(premier != self.me) as usize] += 1;
                self.save_needed = true;
            }
            self.plan.clear();
        }
        self.selection &= self.m.d.mains[self.me];
        self.wake = self.now + if matches!(a, Action::Defausser(_)) { D_PIOCHE } else { D_ACTION };
        self.hint = 0;
        self.hint_pioche = -1;
        self.hint_pour = None;
        true
    }

    /// Avance le temps de dt millisecondes : bots, actions reçues.
    pub fn tick(&mut self, dt: f64) {
        self.now += dt;
        if self.online {
            self.pump();
        }
        if self.alerte.as_ref().is_some_and(|a| a.1 < self.now) {
            self.alerte = None;
        }
        if self.help && self.hint_pour.as_ref() != Some(&self.m.d) {
            self.update_hint();
        }
        if self.now < self.wake {
            return;
        }
        let p = self.turn();
        match self.phase() {
            PH_PIOCHE if self.is_bot(p) => {
                let talon = self.m.bot_pioche(p);
                self.bot_action(p, Action::Pioche(talon));
                self.plan = self.m.bot_jeu(p).into();
            }
            PH_JEU if self.is_bot(p) => match self.plan.pop_front() {
                Some(a) => {
                    if !self.bot_action(p, a) {
                        self.secours(p);
                    }
                }
                None => self.secours(p),
            },
            PH_FINMANCHE | PH_FINPARTIE if self.auto && self.nextmask >> self.me & 1 == 0 => self.human_next(),
            _ => {}
        }
    }

    /// Action d'un bot ; celle d'un siège humain joué par l'ordinateur (U) est diffusée en ligne.
    fn bot_action(&mut self, p: usize, a: Action) -> bool {
        if self.m.niveaux[p] == -1 {
            self.send_action(p, a);
        }
        self.apply(a)
    }

    /// Plan d'un bot épuisé ou refusé (ne devrait pas arriver) : on revient au début du tour, on pioche au talon
    /// et on défausse sans rien poser.
    fn secours(&mut self, p: usize) {
        self.plan.clear();
        if self.m.annulable() {
            if self.m.niveaux[p] == -1 {
                self.send(NT_ANNULER, 0, 0, 0);
            }
            self.m.annuler();
        }
        if self.phase() == PH_PIOCHE {
            self.bot_action(p, Action::Pioche(true));
        }
        if self.phase() == PH_JEU {
            let lvl = self.m.niveau(p).min(1);
            let c = self.m.clone().choisir_defausse(p, lvl);
            self.bot_action(p, Action::Defausser(c));
        }
    }

    // ---------- actions du joueur local ----------
    fn send(&mut self, kind: u8, b: u16, c: u32, d: u32) {
        if self.online {
            self.outbox.push(Frame { kind, b1: self.me as u8, b, c, d });
        }
    }

    /// Cartes m de la main du joueur p en masque de positions (main par numéros croissants).
    fn masque(&self, p: usize, m: Cartes) -> u32 {
        liste(self.m.d.mains[p]).iter().enumerate().filter(|(_, &c)| m & bit(c) != 0).fold(0, |a, (i, _)| a | 1 << i)
    }

    fn depuis_masque(&self, p: usize, k: u32) -> Cartes {
        liste(self.m.d.mains[p]).iter().enumerate().filter(|(i, _)| *i < 32 && k >> i & 1 == 1).fold(0, |a, (_, &c)| a | bit(c))
    }

    fn send_action(&mut self, p: usize, a: Action) {
        match a {
            Action::Pioche(t) => self.send(NT_PIOCHE, t as u16, 0, 0),
            Action::Poser(m) => {
                let k = self.masque(p, m);
                self.send(NT_POSER, 0, k, 0)
            }
            Action::Ajouter(m, i) => {
                let k = self.masque(p, m);
                self.send(NT_AJOUTER, 0, k, i as u32)
            }
            Action::Echanger(x, i) => self.send(NT_ECHANGER, x as u16, 0, i as u32),
            Action::Defausser(c) => self.send(NT_DEFAUSSE, c as u16, 0, 0),
        }
    }

    /// Le joueur local peut agir maintenant.
    pub fn my_turn(&self) -> bool {
        self.is_human(self.turn()) && matches!(self.phase(), PH_PIOCHE | PH_JEU)
    }

    /// Action du joueur local : essayée sur une copie, refusée avec sa raison, ou jouée et diffusée.
    fn human(&mut self, a: Action) -> bool {
        if !self.my_turn() {
            return false;
        }
        let mut essai = self.m.clone();
        essai.simule = true;
        if !essai.appliquer(a) {
            let raison = self.raison(a);
            self.alerter(raison);
            return false;
        }
        self.send_action(self.me, a);
        self.alerte = None;
        self.apply(a)
    }

    /// Pourquoi l'action a est refusée.
    fn raison(&self, a: Action) -> String {
        let d = &self.m.d;
        let me = self.me;
        if d.phase == PH_PIOCHE && !matches!(a, Action::Pioche(_)) {
            return "Pioche d'abord : clique sur le talon ou sur la défausse.".into();
        }
        match a {
            Action::Pioche(_) => "Tu as déjà pioché.".into(),
            Action::Poser(m) => {
                if nb(m) < 3 {
                    "Une combinaison a au moins 3 cartes.".into()
                } else if nb(m & JOKERS) > 1 {
                    "Un seul joker par combinaison.".into()
                } else if former(m).is_none() {
                    "Ces cartes ne forment ni une suite (même couleur, à la suite) ni un brelan (même valeur).".into()
                } else {
                    format!("Pour finir sans avoir ouvert, il faut poser {OUVERTURE} points.")
                }
            }
            Action::Ajouter(m, i) => {
                if !d.ouvert[me] {
                    format!("Il faut d'abord ouvrir ({OUVERTURE} points posés dans le tour).")
                } else {
                    format!("Ces cartes ne complètent pas {}.", d.table.get(i).map_or(String::new(), |c| c.la_combi()))
                        + if nb(m & JOKERS) > 0 && d.table.get(i).is_some_and(|c| c.joker().is_some()) { " (un seul joker)" } else { "" }
                }
            }
            Action::Echanger(x, _) => {
                if !d.ouvert[me] {
                    format!("Il faut d'abord ouvrir pour reprendre un joker ({OUVERTURE} points).")
                } else {
                    format!("Le joker ne remplace pas {} ici.", la_carte(x))
                }
            }
            Action::Defausser(_) => self.m.empechement().unwrap_or_else(|| "Choisis une seule carte à défausser.".into()),
        }
    }

    pub fn human_pioche(&mut self, talon: bool) {
        self.human(Action::Pioche(talon));
    }

    /// POSER : la sélection devient une combinaison.
    pub fn human_poser(&mut self) {
        let s = self.selection;
        if s == 0 {
            self.alerter("Choisis d'abord les cartes de la combinaison (clic sur les cartes).");
            return;
        }
        self.human(Action::Poser(s));
    }

    /// Clic sur la combinaison i de la table : la compléter avec la sélection, ou échanger son joker
    /// (une carte choisie, clic sur le joker).
    pub fn human_table(&mut self, i: usize, sur_joker: bool) {
        let s = self.selection;
        if s == 0 {
            if self.my_turn() {
                self.alerter("Choisis d'abord les cartes à ajouter, puis clique sur la combinaison.");
            }
            return;
        }
        let combi = self.m.d.table.get(i);
        let echange = nb(s) == 1 && combi.is_some_and(|c| c.joker().is_some() && remplacer_joker(c, s.trailing_zeros() as u8).is_some());
        let ajout = combi.is_some_and(|c| completer(c, s).is_some());
        if echange && (sur_joker || !ajout) {
            self.human(Action::Echanger(s.trailing_zeros() as u8, i));
        } else {
            self.human(Action::Ajouter(s, i));
        }
    }

    pub fn human_defausser(&mut self) {
        let s = self.selection;
        if nb(s) != 1 {
            if self.my_turn() {
                self.alerter(if self.phase() == PH_PIOCHE {
                    "Pioche d'abord : clique sur le talon, ou sur la défausse pour prendre sa carte."
                } else {
                    "Choisis une seule carte, puis clique sur la défausse (ou DÉFAUSSER)."
                });
            }
            return;
        }
        self.human(Action::Defausser(s.trailing_zeros() as u8));
    }

    /// ANNULER : retour au début du tour (ou juste après la pioche au talon).
    pub fn human_annuler(&mut self) {
        if !self.my_turn() || !self.m.annulable() {
            return;
        }
        self.send(NT_ANNULER, 0, 0, 0);
        self.m.annuler();
        self.selection = 0;
        self.alerte = None;
        self.hint_pour = None;
    }

    /// Clic sur une carte de la main : la choisir ou la remettre.
    pub fn toggle_selection(&mut self, c: u8) {
        if self.m.d.mains[self.me] & bit(c) == 0 {
            return;
        }
        self.selection ^= bit(c);
    }

    pub fn toggle_tri(&mut self) {
        self.tri_valeur = !self.tri_valeur;
        self.save_needed = true;
    }

    /// SUITE (fin de manche / de partie).
    pub fn human_next(&mut self) {
        if !matches!(self.phase(), PH_FINMANCHE | PH_FINPARTIE) {
            return;
        }
        if self.online {
            if self.nextmask >> self.me & 1 == 1 {
                return; // déjà envoyé : un second SUITE bloquerait les autres à la manche suivante
            }
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
            PH_FINMANCHE => {
                self.m.manche_suivante();
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
            let a_lui = seat == self.turn() && matches!(ph, PH_PIOCHE | PH_JEU);
            let action = match f.kind {
                NT_PIOCHE => Some(Action::Pioche(f.b != 0)),
                NT_POSER => Some(Action::Poser(self.depuis_masque(seat, f.c))),
                NT_AJOUTER => Some(Action::Ajouter(self.depuis_masque(seat, f.c), f.d as usize)),
                NT_ECHANGER => Some(Action::Echanger(f.b as u8, f.d as usize)),
                NT_DEFAUSSE => Some(Action::Defausser(f.b as u8)),
                _ => None,
            };
            match f.kind {
                NT_PIOCHE | NT_POSER | NT_AJOUTER | NT_ECHANGER | NT_DEFAUSSE | NT_ANNULER if !a_lui => return, // pas encore son tour
                NT_ANNULER => {
                    self.inbox.pop_front();
                    self.m.annuler();
                }
                NT_NEXT if ph == PH_FINMANCHE || ph == PH_FINPARTIE => {
                    self.inbox.pop_front();
                    self.mark_next(seat);
                }
                NT_NEXT => return,
                _ => {
                    self.inbox.pop_front();
                    if let Some(a) = action {
                        self.apply(a);
                    }
                }
            }
        }
    }

    /// Démarre une partie en ligne. `humans` : sièges humains ; `config` : joueurs - 2 (bits 0-2), niveau des
    /// bots (bits 3-4), limite de points (bits 5-6).
    pub fn net_begin(&mut self, me: usize, humans: u8, config: u16, seed: u64) {
        self.me = me;
        self.humans = (humans & 31) as u32;
        self.m.np = 2 + (config & 7).min(3) as usize;
        self.botlvl = ((config >> 3 & 3) as i8).min(2);
        self.m.limite = LIMITES[(config >> 5 & 3) as usize];
        self.m.rng = seed;
        self.online = true;
        self.inbox.clear();
        self.outbox.clear();
        self.nextmask = 0;
        self.new_match();
    }

    pub fn net_config(&self) -> u16 {
        let l = LIMITES.iter().position(|&n| n == self.m.limite).unwrap_or(1) as u16;
        (self.np() as u16 - 2) | (self.botlvl as u16) << 3 | l << 5
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
    /// Conseil au joueur local (calculé sur une copie : la partie n'est pas touchée) : pioche, puis combinaison à
    /// poser, cartes à ajouter ou carte à défausser.
    fn update_hint(&mut self) {
        self.hint_pour = Some(self.m.d.clone());
        self.hint = 0;
        self.hint_pioche = -1;
        let p = self.turn();
        if !self.help || !self.is_human(p) {
            return;
        }
        let mut m = self.m.clone();
        m.niveaux[p] = 1;
        match self.phase() {
            PH_PIOCHE => self.hint_pioche = m.bot_pioche(p) as i8,
            PH_JEU if self.m.empechement().is_none() || self.m.d.pose_tour == 0 => {
                let plan = m.bot_jeu(p);
                let (h, txt) = match plan.first() {
                    Some(&Action::Poser(x)) => {
                        let pts: i32 = plan.iter().filter_map(|a| if let Action::Poser(y) = a { former(*y).map(|c| c.points()) } else { None }).sum();
                        let ouvrir = if self.m.d.ouvert[p] { String::new() } else { format!(" (tu ouvres avec {pts} points)") };
                        (x, format!("Conseil : pose les cartes cerclées d'or{ouvrir}."))
                    }
                    Some(&Action::Ajouter(x, i)) => (x, format!("Conseil : ajoute les cartes cerclées d'or {}.", contraction("à", &self.m.d.table[i]))),
                    Some(&Action::Echanger(x, i)) => (bit(x), format!("Conseil : échange {} contre le joker {}.", la_carte(x), contraction("de", &self.m.d.table[i]))),
                    Some(&Action::Defausser(x)) => (bit(x), format!("Conseil : défausse {}.", la_carte(x))),
                    _ => (0, String::new()),
                };
                self.hint = h;
                self.hint_texte = txt;
            }
            _ => {}
        }
    }

    pub fn toggle_help(&mut self) {
        self.help = !self.help;
        self.hint_pour = None;
        self.hint = 0;
        self.hint_pioche = -1;
        self.save_needed = true;
    }

    /// Message du bas de l'écran.
    pub fn message(&self) -> String {
        if let Some((a, _)) = &self.alerte {
            return a.clone();
        }
        let p = self.turn();
        let human = self.is_human(p);
        let d = &self.m.d;
        match self.phase() {
            PH_PIOCHE if human && self.hint_pioche == 0 => "Conseil : prends la carte de la défausse.".into(),
            PH_PIOCHE if human && self.hint_pioche == 1 => "Conseil : pioche au talon.".into(),
            PH_PIOCHE if human => "À toi : pioche au talon, ou prends la défausse (elle devra être posée ce tour-ci).".into(),
            PH_JEU if human => {
                if let Some(e) = self.m.empechement().filter(|_| d.pris >= 0 || d.jokers_pris != 0 || d.pose_tour > 0) {
                    return e;
                }
                if self.help && self.hint != 0 {
                    self.hint_texte.clone()
                } else if !d.ouvert[p] {
                    format!("Pose tes combinaisons ({OUVERTURE} points d'un coup pour ouvrir), puis défausse une carte.")
                } else {
                    "Pose, complète une combinaison (choisis les cartes puis clique dessus), puis défausse.".into()
                }
            }
            PH_FINMANCHE | PH_FINPARTIE if self.nextmask >> self.me & 1 == 1 => "En attente des autres joueurs...".into(),
            PH_FINMANCHE | PH_FINPARTIE => "Espace : suite".into(),
            _ => format!("{} réfléchit...", self.player_name(p)),
        }
    }

    // ---------- réglages ----------
    pub fn cycle_np(&mut self) {
        if self.online {
            return;
        }
        self.m.np = if self.m.np >= 5 { 2 } else { self.m.np + 1 };
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

    pub fn cycle_limite(&mut self) {
        if self.online {
            return;
        }
        let i = LIMITES.iter().position(|&n| n == self.m.limite).unwrap_or(1);
        self.m.limite = LIMITES[(i + 1) % LIMITES.len()];
        self.save_needed = true;
    }
}

/// « à la suite... », « au brelan... », « de la suite... », « du brelan... ».
fn contraction(prep: &str, c: &Combi) -> String {
    match (prep, c.suite) {
        (_, true) => format!("{prep} {}", c.la_combi()),
        ("à", false) => format!("au {}", c.decrire()),
        (_, false) => format!("du {}", c.decrire()),
    }
}

/// Main triée pour l'affichage : par couleur (pique, cœur, trèfle, carreau : rouges et noires alternées, du 2 à
/// l'as) ou par valeur (du 2 à l'as, puis par couleur) ; les jokers à la fin.
pub fn trier(m: Cartes, par_valeur: bool) -> Vec<u8> {
    let mut v = liste(m);
    v.sort_by_key(|&c| {
        if est_joker(c) {
            return 1000 + c as u32;
        }
        let s = [0u32, 1, 3, 2][couleur(c) as usize];
        let r = rang(c) as u32;
        (if par_valeur { r * 8 + s * 2 } else { s * 32 + r * 2 }) + (c >= 52) as u32
    });
    v
}

/// Le joueur local : étiquette au-dessus de la gauche de sa main.
const MOI: Place = Place { etiquette: (112.0, 572.0), donneur_a_gauche: true, ..BAS };
/// À gauche et à droite, les mains sont hautes : l'étiquette passe au-dessus.
const GAUCHE_R: Place = Place { etiquette: (92.0, 138.0), largeur: 290.0, ..GAUCHE };
const DROITE_R: Place = Place { etiquette: (1008.0, 138.0), largeur: 290.0, ..DROITE };
/// En haut, la main est un peu resserrée pour laisser la place au panneau des scores.
const HAUT_R: Place = Place { largeur: 360.0, bulle: (550.0, 166.0), ..HAUT };
/// À cinq, les deux mains du haut sont resserrées pour ne pas se chevaucher.
const HAUT_GAUCHE_R: Place = Place { main: (440.0, 96.0), largeur: 150.0, pas: 20.0, etiquette: (440.0, 24.0), bulle: (440.0, 172.0), ..HAUT_GAUCHE };
const HAUT_DROITE_R: Place = Place { main: (660.0, 96.0), largeur: 150.0, pas: 20.0, etiquette: (660.0, 24.0), bulle: (660.0, 172.0), ..HAUT_DROITE };

/// Places du rami pour 2 à 5 joueurs, à partir du joueur local.
pub fn disposition(n: usize) -> &'static [Place] {
    match n {
        2 => &[MOI, HAUT_R],
        3 => &[MOI, GAUCHE_R, DROITE_R],
        5 => &[MOI, GAUCHE_R, HAUT_GAUCHE_R, HAUT_DROITE_R, DROITE_R],
        _ => &[MOI, GAUCHE_R, HAUT_R, DROITE_R],
    }
}

/// Abscisse de la carte i (sur n) de la main du bas (entre les boutons de droite et le bord gauche).
pub fn main_x(i: usize, n: usize) -> f64 {
    let pas = if n > 1 { 72.0f64.min(690.0 / (n - 1) as f64) } else { 72.0 };
    545.0 + (i as f64 - (n as f64 - 1.0) / 2.0) * pas
}

/// Carte de la main du bas (n cartes) sous le point (x, y), ou None.
pub fn main_touchee(n: usize, x: f32, y: f32) -> Option<usize> {
    if !(578.0..=764.0).contains(&y) {
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
        0x5241_4D01 // « RAM », protocole 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Joue tout seul (mode U) des parties entières de 2 à 5 joueurs.
    #[test]
    fn parties_automatiques() {
        for np in 2usize..=5 {
            let mut t = Table::new(4242 + np as u64);
            t.m.np = np;
            t.m.limite = 100;
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
        for np in 2usize..=5 {
            let seed = 0x1234_5678_9ABC_DEF1;
            let humans = (1u8 << np) - 1;
            let mut ts: Vec<Table> = (0..np).map(|k| Table::new(k as u64 + 7)).collect();
            for (k, t) in ts.iter_mut().enumerate() {
                t.m.np = np;
                t.m.limite = 100;
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
                    let end = ts[k].phase() == PH_FINMANCHE || ts[k].phase() == PH_FINPARTIE;
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
                if hist.iter().all(|h| h.len() >= 5) {
                    break;
                }
            }
            assert!(hist.iter().all(|h| h.len() >= 5), "np={np} : manches jouées {:?}", hist.iter().map(|h| h.len()).collect::<Vec<_>>());
            for h in &hist {
                assert_eq!(h[..5], hist[0][..5], "np={np} : machines désynchronisées");
            }
        }
    }

    /// Un humain en ligne qui pose, ajoute et annule : les autres machines suivent (masques de la main).
    #[test]
    fn annuler_en_ligne() {
        let seed = 77;
        let mut a = Table::new(1);
        let mut b = Table::new(2);
        a.m.np = 2;
        b.m.np = 2;
        let cfg = a.net_config();
        a.net_begin(0, 3, cfg, seed);
        b.net_begin(1, 3, cfg, seed);
        b.auto = true;
        for _ in 0..20_000 {
            // le joueur de a : pioche, essaie de poser, annule parfois, puis défausse comme un bot
            if a.my_turn() {
                match a.phase() {
                    PH_PIOCHE => a.human_pioche(a.m.d.tours % 3 != 0),
                    _ => {
                        let plan = a.m.bot_jeu(0);
                        if a.m.d.tours % 4 == 1 {
                            if let Some(Action::Poser(m)) = plan.first() {
                                a.selection = *m;
                                a.human_poser();
                                a.human_annuler();
                            }
                        }
                        for act in a.m.bot_jeu(0) {
                            match act {
                                Action::Poser(m) => {
                                    a.selection = m;
                                    a.human_poser();
                                }
                                Action::Ajouter(m, i) => {
                                    a.selection = m;
                                    a.human_table(i, false);
                                }
                                Action::Echanger(x, i) => {
                                    a.selection = bit(x);
                                    a.human_table(i, true);
                                }
                                Action::Defausser(c) => {
                                    a.selection = bit(c);
                                    a.human_defausser();
                                }
                                Action::Pioche(_) => {}
                            }
                        }
                        if a.phase() == PH_JEU {
                            a.human_annuler();
                            if a.phase() == PH_PIOCHE {
                                a.human_pioche(true);
                            }
                            let c = a.m.clone().choisir_defausse(0, 1);
                            a.selection = bit(c);
                            a.human_defausser();
                        }
                    }
                }
            }
            if matches!(a.phase(), PH_FINMANCHE | PH_FINPARTIE) {
                a.human_next();
            }
            a.tick(40.0);
            b.tick(40.0);
            for f in a.outbox.drain(..) {
                b.inbox.push_back(f);
            }
            for f in b.outbox.drain(..) {
                a.inbox.push_back(f);
            }
            assert!(a.m.d.table == b.m.d.table || a.inbox.len() + b.inbox.len() > 0 || a.m.d.tours != b.m.d.tours);
            if a.m.manche >= 3 && b.m.manche >= 3 {
                break;
            }
        }
        assert!(a.m.manche >= 3, "manches : {} {} / b {} ; phases {} {} ; tours {} {} ; tour {} {} ; next {} {} ; boîtes {} {} ; alerte {:?}", a.m.manche, a.m.d.tours, b.m.manche, a.phase(), b.phase(), a.m.d.tours, b.m.d.tours, a.turn(), b.turn(), a.nextmask, b.nextmask, a.inbox.len(), b.inbox.len(), a.alerte);
        assert_eq!(a.m.scores, b.m.scores);
    }
}
