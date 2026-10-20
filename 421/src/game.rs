//! Déroulement d'une partie de 421 : manches, lancers, bot, jetons, messages, et les dés en 3D
//! (orientation, trajectoire du lancer). Aucune dépendance graphique : le même moteur sert à la
//! fenêtre, au mode terminal et aux tests. Tout est déterministe à partir de la graine et de la
//! suite des actions, ce qui permet le jeu en ligne « lockstep ».

use crate::ai;
pub use bistrot::reseau::Frame;
use bistrot::reseau::{Partie, T_ACTION};
use std::collections::VecDeque;
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, PI};

pub const DIE_H: f64 = 0.68; // demi-côté d'un dé
pub const FELT: f64 = 0.10; // hauteur du tapis
const ROW: f64 = 2.6;
const SPOT: [f64; 3] = [-2.3, 0.0, 2.3];
const ROLL_MS: f64 = 1500.0;

/// Repère de chaque face (valeur 1..6) : normale n, axes a et b (a x b = n).
pub const FACES: [[[f64; 3]; 3]; 6] = [
    [[0., 0., 1.], [1., 0., 0.], [0., 1., 0.]],
    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
    [[0., 1., 0.], [0., 0., 1.], [1., 0., 0.]],
    [[0., -1., 0.], [0., 0., 1.], [-1., 0., 0.]],
    [[-1., 0., 0.], [0., 1., 0.], [0., 0., -1.]],
    [[0., 0., -1.], [1., 0., 0.], [0., -1., 0.]],
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Phase {
    PreRoll, // à toi de lancer
    Rolling, // les dés roulent
    Choose,  // choix des dés à relancer
    Bot,     // le bot joue (ou attente de la comparaison)
    Result,  // résultat de la manche
    Over,    // fin de partie
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Act {
    BotRoll,
    BotDone,
    Resolve,
}

/// Mode : seul contre le bot, à deux sur le même PC, ou en ligne (siège local).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mode {
    Solo,
    Duo,
    Online { seat: usize },
}

/// Actions échangées en ligne (trames de 12 octets du moteur, voir `bistrot::reseau`).
pub const NK_MAIN: u8 = T_ACTION;
pub const NK_SECOND: u8 = T_ACTION + 1;
pub const NK_TOGGLE: u8 = T_ACTION + 2;
pub const NK_NEXT: u8 = T_ACTION + 3;
pub const NK_NEW: u8 = T_ACTION + 4;

#[derive(Clone, Copy, Default, Debug)]
pub struct Die {
    pub val: u32,
    pub held: bool,
    pub ax: f64, // position de repos (a = vers la droite)
    pub by: f64, // position de repos (b = vers le joueur)
    sa: f64,     // départ du lancer
    sb: f64,
    axis: [f64; 3],
    ang0: f64,
    amp: f64,
    pub roll: bool,
    wrest: [f64; 9],
    pub w: [f64; 9], // orientation courante
    pub p: [f64; 3], // centre (monde)
    pub depth: f64,
}

fn matmul3(a: &[f64; 9], b: &[f64; 9]) -> [f64; 9] {
    std::array::from_fn(|k| {
        let (i, j) = (k / 3, k % 3);
        a[i * 3] * b[j] + a[i * 3 + 1] * b[3 + j] + a[i * 3 + 2] * b[6 + j]
    })
}

pub fn mv3(m: &[f64; 9], v: &[f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| m[i * 3] * v[0] + m[i * 3 + 1] * v[1] + m[i * 3 + 2] * v[2])
}

fn rodrigues(ax: &[f64; 3], ang: f64) -> [f64; 9] {
    let (s, c) = ang.sin_cos();
    let t = 1.0 - c;
    let [x, y, z] = *ax;
    [
        c + t * x * x,
        t * x * y - s * z,
        t * x * z + s * y,
        t * x * y + s * z,
        c + t * y * y,
        t * y * z - s * x,
        t * x * z - s * y,
        t * y * z + s * x,
        c + t * z * z,
    ]
}

impl Die {
    /// Orientation, position et profondeur au temps t (0..1) du lancer.
    pub fn update(&mut self, t: f64) {
        let u3 = (1.0 - t).powi(3);
        let r = rodrigues(&self.axis, self.ang0 * u3);
        self.w = matmul3(&r, &self.wrest);
        let e = 1.0 - u3;
        let a = self.sa + (self.ax - self.sa) * e;
        let b = self.sb + (self.by - self.sb) * e;
        let bounce = (3.0 * PI * t).sin().abs() * (1.0 - t) * self.amp;
        let mut z = (self.w[6].abs() + self.w[7].abs() + self.w[8].abs()) * DIE_H + bounce + FELT;
        if self.held {
            z += 0.5;
        }
        self.p = [(a - b) * FRAC_1_SQRT_2, -(a + b) * FRAC_1_SQRT_2, z];
        self.depth = self.p[0] + self.p[1] - z;
    }
}

pub struct Game {
    pub rng: u64,
    pub dice: [Die; 6],
    pub phase: Phase,
    pub mode: Mode,
    pub level: usize,
    pub pot_init: u32,
    pub pot: u32,
    pub tok: [u32; 2],
    pub round: u32,
    pub game_id: u32,
    pub wins: u32,
    pub losses: u32,
    pub line1: String,
    pub line2: String,
    pub cur: usize,
    starter: usize,
    throws: [u32; 2],
    lim: u32,
    target: u32,
    target_tok: u32,
    who: usize,
    botmask: u32,
    rank: [u32; 2],
    tokw: [u32; 2],
    cat: [usize; 2],
    wake: Option<(f64, Act)>,
    pub rolling: bool,
    roll_start: f64,
    pub now: f64,
    /// réglages ou scores modifiés : à enregistrer
    pub save_needed: bool,
    /// actions locales à envoyer à l'adversaire (en ligne)
    pub outbox: Vec<Frame>,
    /// actions de l'adversaire en attente (appliquées quand les dés sont posés)
    pub inbox: VecDeque<Frame>,
    /// mode local à retrouver après une partie en ligne
    pub mode_local: Mode,
}

impl Game {
    pub fn new(seed: u64) -> Game {
        let mut g = Game {
            rng: seed | 1,
            dice: [Die::default(); 6],
            phase: Phase::PreRoll,
            mode: Mode::Solo,
            level: 1,
            pot_init: 11,
            pot: 11,
            tok: [0, 0],
            round: 0,
            game_id: 0,
            wins: 0,
            losses: 0,
            line1: String::new(),
            line2: String::new(),
            cur: 0,
            starter: 0,
            throws: [0, 0],
            lim: 3,
            target: 0,
            target_tok: 0,
            who: 0,
            botmask: 0,
            rank: [0; 2],
            tokw: [0; 2],
            cat: [0; 2],
            wake: None,
            rolling: false,
            roll_start: 0.0,
            now: 0.0,
            save_needed: false,
            outbox: Vec::new(),
            inbox: VecDeque::new(),
            mode_local: Mode::Solo,
        };
        g.init_dice();
        g.new_game();
        g
    }

    // ---------- hasard (xorshift64, 32 bits de poids fort) ----------
    fn rng_next(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        (x >> 32) as u32
    }

    fn rng_unit(&mut self) -> f64 {
        self.rng_next() as f64 * (1.0 / 4294967296.0)
    }

    // ---------- dés ----------
    /// Orientation de repos : valeur v au-dessus, lacet aléatoire.
    fn set_rest(&mut self, i: usize, v: u32) {
        let yaw = (self.rng_next() >> 30) as f64 * FRAC_PI_2 + (self.rng_unit() - 0.5) * 0.6;
        let (s, c) = yaw.sin_cos();
        let m2 = [c, -s, 0., s, c, 0., 0., 0., 1.];
        let f = &FACES[v as usize - 1];
        let m1 = [f[1][0], f[1][1], f[1][2], f[2][0], f[2][1], f[2][2], f[0][0], f[0][1], f[0][2]];
        let d = &mut self.dice[i];
        d.val = v;
        d.wrest = matmul3(&m2, &m1);
    }

    fn init_dice(&mut self) {
        const INIT: [u32; 6] = [4, 2, 1, 6, 3, 5];
        for i in 0..6 {
            self.set_rest(i, INIT[i]);
            let d = &mut self.dice[i];
            d.ax = SPOT[i % 3];
            d.sa = d.ax;
            d.by = if i < 3 { ROW } else { -ROW };
            d.sb = d.by;
            d.ang0 = 0.0;
            d.amp = 0.0;
            d.axis = [1.0, 0.0, 0.0];
            d.held = false;
            d.roll = false;
            d.update(1.0);
        }
    }

    /// Lance les dés du masque (bits 0-2 : joueur 1, bits 3-5 : joueur 2 / bot).
    fn start_throw(&mut self, mask: u32) {
        for d in self.dice.iter_mut() {
            d.roll = false;
        }
        for i in 0..6 {
            if mask >> i & 1 == 0 {
                continue;
            }
            let v = ((self.rng_next() as u64 * 6) >> 32) as u32 + 1;
            self.set_rest(i, v);
            let side = if i < 3 { 1.0 } else { -1.0 };
            let by = (self.rng_unit() - 0.5) * 0.6 + side * ROW;
            let sa = (self.rng_unit() - 0.5) * 2.0;
            let mut axis = [self.rng_unit() - 0.5, self.rng_unit() - 0.5, self.rng_unit() - 0.5];
            let n = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt() + 1.0 / 4294967296.0;
            axis.iter_mut().for_each(|a| *a /= n);
            let ang0 = (self.rng_unit() * 6.0 + 5.0) * PI;
            let amp = self.rng_unit() + 1.4;
            let d = &mut self.dice[i];
            d.held = false;
            d.roll = true;
            d.ax = SPOT[i % 3];
            d.by = by;
            d.sa = sa;
            d.sb = 4.0 * side;
            d.axis = axis;
            d.ang0 = ang0;
            d.amp = amp;
        }
        self.roll_start = self.now;
        self.rolling = true;
    }

    fn clear_held(&mut self) {
        for d in self.dice.iter_mut() {
            d.held = false;
        }
    }

    fn refresh_dice(&mut self) {
        for d in self.dice.iter_mut() {
            d.update(1.0);
        }
    }

    pub fn hand(&self, c: usize) -> [u32; 3] {
        [self.dice[c * 3].val, self.dice[c * 3 + 1].val, self.dice[c * 3 + 2].val]
    }

    fn score_hand(&self, c: usize) -> (u32, u32, usize) {
        let h = self.hand(c);
        ai::rank3(h[0], h[1], h[2])
    }

    fn set_wake(&mut self, ms: f64, act: Act) {
        self.wake = Some((self.now + ms, act));
    }

    // ---------- noms et messages ----------
    fn names_base(&self) -> usize {
        match self.mode {
            Mode::Solo => 0,
            Mode::Duo => 1,
            Mode::Online { seat } => seat + 2,
        }
    }

    /// Nom du joueur c (Toi / Bot, Joueur 1 / Joueur 2, Toi / Adversaire).
    pub fn who_name(&self, c: usize) -> &'static str {
        const N: [&str; 8] = ["Toi", "Bot", "Joueur 1", "Joueur 2", "Toi", "Adversaire", "Adversaire", "Toi"];
        N[c + self.names_base() * 2]
    }

    /// Étiquette du joueur c sur le plateau.
    pub fn label(&self, c: usize) -> &'static str {
        const N: [&str; 8] = ["TOI", "BOT", "J1", "J2", "TOI", "ADV", "ADV", "TOI"];
        N[c + self.names_base() * 2]
    }

    fn hand_msg(&self, c: usize, cat: usize) -> String {
        format!("{} : {}", self.who_name(c), ai::CAT_NAMES[cat])
    }

    pub fn myseat(&self) -> usize {
        match self.mode {
            Mode::Online { seat } => seat,
            _ => 0,
        }
    }

    pub fn online(&self) -> bool {
        matches!(self.mode, Mode::Online { .. })
    }

    fn net_turn_msg(&mut self) {
        let who = if self.cur == self.myseat() { "À toi : lance les dés (" } else { "L'adversaire joue (" };
        self.line1 = format!("{who}{} lancer(s) max)...", self.lim);
    }

    // ---------- déroulement ----------
    pub fn new_game(&mut self) {
        self.game_id += 1;
        self.pot = self.pot_init;
        self.tok = [0, 0];
        self.starter = 0;
        self.round = 0;
        self.begin_round();
    }

    fn begin_round(&mut self) {
        self.round += 1;
        self.clear_held();
        self.refresh_dice();
        self.lim = 3;
        self.throws = [0, 0];
        self.target = 0;
        self.target_tok = 0;
        self.cur = 0;
        self.line2.clear();
        if self.starter == 0 {
            self.phase = Phase::PreRoll;
            match self.mode {
                Mode::Solo => self.line1 = "Tu commences ! Lance les dés (3 lancers max).".into(),
                Mode::Online { .. } => self.net_turn_msg(),
                Mode::Duo => self.line1 = format!("{} commence ! Lance les dés (3 lancers max).", self.who_name(0)),
            }
        } else if self.mode != Mode::Solo {
            self.cur = 1;
            self.phase = Phase::PreRoll;
            if self.online() {
                self.net_turn_msg();
            } else {
                self.line1 = format!("{} commence ! Lance les dés (3 lancers max).", self.who_name(1));
            }
        } else {
            self.phase = Phase::Bot;
            self.line1 = "Le bot commence...".into();
            self.botmask = 0x38;
            self.set_wake(1100.0, Act::BotRoll);
        }
    }

    /// Bouton du haut : LANCER / RELANCER / SUIVANT / REJOUER selon la phase (joueur courant).
    fn press_main(&mut self) {
        match self.phase {
            Phase::PreRoll => {
                self.who = self.cur;
                self.throws[self.cur] = 1;
                self.phase = Phase::Rolling;
                self.line1 = "Les dés roulent...".into();
                self.start_throw(7 << (self.cur * 3));
            }
            Phase::Choose => {
                let first = self.cur * 3;
                let mask = (0..3).filter(|&i| self.dice[first + i].held).fold(0, |m, i| m | 1 << i);
                if mask == 0 {
                    self.line1 = "Clique d'abord sur les dés à relancer, ou VALIDER pour garder ta main.".into();
                    return;
                }
                self.clear_held();
                self.throws[self.cur] += 1;
                self.who = self.cur;
                self.phase = Phase::Rolling;
                self.line1 = "Les dés roulent...".into();
                self.start_throw(mask << first);
            }
            Phase::Result => self.begin_round(),
            Phase::Over => self.new_game(),
            _ => {}
        }
    }

    /// VALIDER : garder la main actuelle.
    fn press_second(&mut self) {
        if self.phase == Phase::Choose {
            self.finish_player_turn();
        }
    }

    fn on_roll_done(&mut self) {
        let w = self.who;
        if self.mode == Mode::Solo && w != 0 {
            self.bot_after_roll();
            return;
        }
        if self.throws[w] >= self.lim {
            self.finish_player_turn();
            return;
        }
        self.phase = Phase::Choose;
        let tail = if self.online() && w != self.myseat() {
            " : l'adversaire choisit ses dés..."
        } else {
            " : choisis les dés à relancer, puis RELANCER (ou VALIDER)."
        };
        self.line1 = format!("Lancer {}/{}{}", self.throws[w], self.lim, tail);
        let cat = self.score_hand(w).2;
        self.line2 = self.hand_msg(w, cat);
    }

    fn finish_player_turn(&mut self) {
        self.clear_held();
        self.refresh_dice();
        let c = self.cur;
        let (rank, tokw, cat) = self.score_hand(c);
        self.rank[c] = rank;
        self.tokw[c] = tokw;
        self.cat[c] = cat;
        self.line2 = self.hand_msg(c, cat);
        self.phase = Phase::Bot;
        if self.starter != c {
            self.set_wake(600.0, Act::Resolve);
            return;
        }
        self.target = rank;
        self.target_tok = tokw;
        self.lim = self.throws[c];
        if self.mode == Mode::Solo {
            self.throws[1] = 0;
            self.botmask = 0x38;
            self.line1 = format!("Le bot joue ({} lancer(s) max)...", self.lim);
            self.set_wake(900.0, Act::BotRoll);
        } else {
            self.cur ^= 1;
            self.throws[self.cur] = 0;
            self.phase = Phase::PreRoll;
            if self.online() {
                self.net_turn_msg();
            } else {
                self.line1 = format!("{} : lance les dés ({} lancer(s) max).", self.who_name(self.cur), self.lim);
            }
        }
    }

    fn bot_choose(&mut self) -> u32 {
        let hand = self.hand(1);
        if self.level == 2 {
            return ai::hard_choose(hand, self.throws[1], self.lim, self.target, self.target_tok);
        }
        // facile : 35 % de décisions au hasard
        if self.level == 0 && self.rng_next() % 100 < 35 {
            return if self.rng_next() & 1 == 0 { 0 } else { self.rng_next() % 7 + 1 };
        }
        ai::normal_choose(hand, self.target)
    }

    fn bot_after_roll(&mut self) {
        self.throws[1] += 1;
        let (rank, tokw, cat) = self.score_hand(1);
        self.rank[1] = rank;
        self.tokw[1] = tokw;
        self.cat[1] = cat;
        self.line2 = self.hand_msg(1, cat);
        let m = if self.throws[1] >= self.lim { 0 } else { self.bot_choose() };
        if m == 0 {
            self.set_wake(900.0, Act::BotDone);
        } else {
            self.botmask = m << 3;
            self.line1 = format!("Le bot relance {} dé(s)...", m.count_ones());
            self.set_wake(1000.0, Act::BotRoll);
        }
    }

    fn do_action(&mut self, act: Act) {
        match act {
            Act::BotRoll => {
                self.who = 1;
                self.phase = Phase::Bot;
                if self.throws[1] == 0 {
                    self.line1 = "Le bot lance les dés...".into();
                }
                self.start_throw(self.botmask);
            }
            Act::BotDone if self.starter == 1 => {
                self.lim = self.throws[1];
                self.target = self.rank[1];
                self.target_tok = self.tokw[1];
                self.cur = 0;
                self.throws[0] = 0;
                self.phase = Phase::PreRoll;
                self.line1 = format!(
                    "Le bot s'arrête après {} lancer(s). À toi, {} lancer(s) max !",
                    self.throws[1], self.lim
                );
            }
            Act::BotDone | Act::Resolve => self.resolve_round(),
        }
    }

    /// Le gagnant w prend v jetons : dans le pot s'il en reste (donnés au perdant), sinon les siens.
    fn transfer(&mut self, w: usize, v: u32) -> u32 {
        let l = w ^ 1;
        let n = if self.pot > 0 {
            let n = v.min(self.pot);
            self.pot -= n;
            n
        } else {
            let n = v.min(self.tok[w]);
            self.tok[w] -= n;
            n
        };
        self.tok[l] += n;
        n
    }

    fn resolve_round(&mut self) {
        for c in 0..2 {
            let (r, t, k) = self.score_hand(c);
            self.rank[c] = r;
            self.tokw[c] = t;
            self.cat[c] = k;
        }
        self.line2 = format!("{}        {}", self.hand_msg(0, self.cat[0]), self.hand_msg(1, self.cat[1]));
        self.phase = Phase::Result;
        if self.rank[0] == self.rank[1] {
            self.line1 = "Égalité ! On rejoue la manche.".into();
            return;
        }
        let w = if self.rank[0] > self.rank[1] { 0 } else { 1 };
        let n = self.transfer(w, self.tokw[w]);
        self.starter = w ^ 1;
        let head = match self.mode {
            Mode::Solo if w == 0 => "Tu gagnes la manche ! Le bot encaisse ".to_string(),
            Mode::Solo => "Le bot gagne la manche. Tu encaisses ".to_string(),
            Mode::Online { seat } if w == seat => "Tu gagnes la manche ! L'adversaire encaisse ".to_string(),
            Mode::Online { .. } => "L'adversaire gagne la manche. Tu encaisses ".to_string(),
            Mode::Duo => format!("{} gagne la manche ! {} encaisse ", self.who_name(w), self.who_name(w ^ 1)),
        };
        self.line1 = format!("{head}{n} jeton(s).");
        if self.pot != 0 {
            return;
        }
        // fin de partie : le premier sans jeton gagne
        let winner = if self.tok[0] == 0 {
            0
        } else if self.tok[1] == 0 {
            1
        } else {
            return;
        };
        self.line1 = match self.mode {
            Mode::Solo if winner == 0 => {
                self.wins += 1;
                self.save_needed = true;
                "VICTOIRE ! Tu as battu le bot.".into()
            }
            Mode::Solo => {
                self.losses += 1;
                self.save_needed = true;
                "Le bot gagne la partie.".into()
            }
            Mode::Online { seat } if winner == seat => "VICTOIRE ! Tu as battu ton adversaire.".into(),
            Mode::Online { .. } => "Ton adversaire gagne la partie.".into(),
            Mode::Duo => format!("{} gagne la partie !", self.who_name(winner)),
        };
        self.phase = Phase::Over;
    }

    // ---------- temps ----------
    /// Avance l'animation et les actions différées de dt millisecondes.
    pub fn tick(&mut self, dt: f64) {
        self.now += dt;
        if self.rolling {
            let t = ((self.now - self.roll_start) / ROLL_MS).min(1.0);
            // animations coupées (processeur léger) : les dés sont tout de suite à leur place
            let vu = if bistrot::anim::actives() { t } else { 1.0 };
            for d in self.dice.iter_mut() {
                d.update(if d.roll { vu } else { 1.0 });
            }
            if t >= 1.0 {
                self.rolling = false;
                for d in self.dice.iter_mut() {
                    d.roll = false;
                }
                self.on_roll_done();
            }
        }
        if let Some((when, act)) = self.wake {
            if self.now >= when {
                self.wake = None;
                self.do_action(act);
            }
        }
        self.pump();
    }

    /// Plus rien ne bouge et aucune action n'est programmée.
    pub fn idle(&self) -> bool {
        !self.rolling && self.wake.is_none()
    }

    /// Temps restant avant la fin de l'animation ou de l'action programmée (ms).
    pub fn pending_ms(&self) -> Option<f64> {
        if self.rolling {
            return Some((self.roll_start + ROLL_MS - self.now).max(0.0));
        }
        self.wake.map(|(w, _)| (w - self.now).max(0.0))
    }

    // ---------- en ligne ----------
    /// Le joueur local peut agir (toujours vrai hors ligne).
    pub fn can_act(&self) -> bool {
        !self.online() || self.phase >= Phase::Result || self.cur == self.myseat()
    }

    fn send(&mut self, kind: u8, b: u16, c: u32) {
        if self.online() {
            self.outbox.push(Frame { kind, b1: 0, b, c, d: 0 });
        }
    }

    /// Démarre une partie en ligne (identique des deux côtés) avec la graine et le pot de l'hôte.
    pub fn start_online(&mut self, seat: usize, seed: u64, pot: u32) {
        if !self.online() {
            self.mode_local = self.mode;
        }
        self.rng = seed;
        self.pot_init = pot;
        self.mode = Mode::Online { seat };
        self.inbox.clear();
        self.outbox.clear();
        self.game_id = 0;
        self.wake = None;
        self.rolling = false;
        self.init_dice();
        self.new_game();
    }

    /// Applique dans l'ordre les actions de l'adversaire quand ce PC est posé.
    fn pump(&mut self) {
        while self.online() && self.idle() {
            let Some(f) = self.inbox.pop_front() else { break };
            match f.kind {
                NK_NEXT if self.phase == Phase::Result && f.c == self.round => self.begin_round(),
                NK_NEW if self.phase == Phase::Over && f.c == self.game_id => self.new_game(),
                _ if self.cur == self.myseat() => {}
                NK_MAIN => self.press_main(),
                NK_SECOND => self.press_second(),
                NK_TOGGLE if self.phase == Phase::Choose && f.b <= 2 => {
                    let d = &mut self.dice[self.cur * 3 + f.b as usize];
                    d.held = !d.held;
                    d.update(1.0);
                }
                _ => {}
            }
        }
    }

    // ---------- actions du joueur local ----------
    pub fn ui_main(&mut self) {
        if self.online() {
            match self.phase {
                Phase::PreRoll | Phase::Choose => {
                    if self.cur != self.myseat() {
                        return;
                    }
                    self.send(NK_MAIN, 0, 0);
                }
                Phase::Result => self.send(NK_NEXT, 0, self.round),
                Phase::Over => self.send(NK_NEW, 0, self.game_id),
                _ => return,
            }
        }
        self.press_main();
    }

    pub fn ui_second(&mut self) {
        if self.online() {
            if self.phase != Phase::Choose || self.cur != self.myseat() {
                return;
            }
            self.send(NK_SECOND, 0, 0);
        }
        self.press_second();
    }

    /// Coche / décoche le dé i (0..2) du joueur courant.
    pub fn ui_toggle(&mut self, i: usize) {
        if self.phase != Phase::Choose || !self.can_act() {
            return;
        }
        let d = &mut self.dice[self.cur * 3 + i];
        d.held = !d.held;
        d.update(1.0);
        self.send(NK_TOGGLE, i as u16, 0);
    }

    /// Les boutons et les dés réagissent-ils aux clics ?
    pub fn accepts_input(&self) -> bool {
        !matches!(self.phase, Phase::Rolling | Phase::Bot) && self.can_act()
    }

    // ---------- aide et réglages ----------
    /// Touche H : probabilités de la sélection et conseil de la stratégie optimale.
    pub fn help(&mut self) {
        if self.phase != Phase::Choose {
            return;
        }
        let c = self.cur;
        let hand = self.hand(c);
        let sel = (0..3).filter(|&i| self.dice[c * 3 + i].held).fold(0, |m, i| m | 1 << i);
        let adv = ai::hard_choose(hand, self.throws[c], self.lim, self.target, self.target_tok);
        let mut s = String::new();
        if sel != 0 {
            let (n421, better, total) = ai::stats(hand, sel);
            let pct = |n: u32| (n * 100 + total / 2) / total;
            s = format!("Ta sélection : {} % de 421, {} % de mieux. ", pct(n421), pct(better));
        }
        s.push_str("Conseil : ");
        if adv == 0 {
            s.push_str("garde ta main.");
        } else {
            s.push_str("relance les dés ");
            for i in 0..3 {
                if adv >> i & 1 == 1 {
                    s.push_str(&format!("{} ", i + 1));
                }
            }
        }
        self.line2 = s;
    }

    /// Touche J : 1 joueur contre le bot <-> 2 joueurs sur le même PC.
    pub fn toggle_duo(&mut self) {
        if self.online() {
            return;
        }
        self.mode = if self.mode == Mode::Solo { Mode::Duo } else { Mode::Solo };
        self.wake = None;
        self.rolling = false;
        self.new_game();
        self.save_needed = true;
    }

    /// Touche L : niveau du bot.
    pub fn cycle_level(&mut self) {
        self.level = (self.level + 1) % 3;
        self.line2 = format!("Niveau du bot : {}", ai::LEVEL_NAMES[self.level]);
        self.save_needed = true;
    }

    /// Touche P : pot de départ 7 / 11 / 15 / 21 (prochaine partie).
    pub fn cycle_pot(&mut self) {
        self.pot_init = match self.pot_init {
            7 => 11,
            11 => 15,
            15 => 21,
            _ => 7,
        };
        self.line2 = format!("Pot de départ : {} jetons (prochaine partie).", self.pot_init);
        self.save_needed = true;
    }

    /// Repasse en partie locale (fin du jeu en ligne), dans le mode d'avant.
    pub fn leave_online(&mut self) {
        self.mode = self.mode_local;
        self.inbox.clear();
        self.outbox.clear();
        self.wake = None;
        self.rolling = false;
        self.new_game();
    }
}

/// Jeu en ligne : duel à deux, l'invité est le joueur 2 ; le pot de l'hôte est imposé.
impl Partie for Game {
    fn sieges_invites(&self) -> Vec<usize> {
        vec![1]
    }

    fn configuration(&self, _humains: u32) -> (u8, u16) {
        (self.pot_init as u8, 0)
    }

    fn demarrer(&mut self, siege: usize, b1: u8, _b: u16, graine: u64) {
        self.start_online(siege, graine, b1 as u32);
    }

    fn quitter(&mut self) {
        self.leave_online();
    }

    fn en_ligne(&self) -> bool {
        self.online()
    }

    fn sortantes(&mut self) -> Vec<Frame> {
        self.outbox.drain(..).collect()
    }

    fn recue(&mut self, f: Frame) {
        if self.online() && self.inbox.len() < 64 && (NK_MAIN..=NK_NEW).contains(&f.kind) {
            self.inbox.push_back(f);
        }
    }

    fn graine_locale(&self) -> u64 {
        self.rng
    }

    fn jeu(&self) -> u32 {
        0x3432_3103 // « 421 », protocole 3 (moteur bistrot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Joue jusqu'à ce que plus rien ne bouge.
    fn settle(g: &mut Game) {
        for _ in 0..1000 {
            g.tick(100.0);
            if g.idle() && g.inbox.is_empty() {
                return;
            }
        }
    }

    #[test]
    fn partie_contre_le_bot_se_termine() {
        for level in 0..3 {
            let mut g = Game::new(0x1234_5678 + level as u64);
            g.level = level;
            let mut steps = 0;
            while g.phase != Phase::Over {
                settle(&mut g);
                match g.phase {
                    Phase::PreRoll | Phase::Result => g.ui_main(),
                    Phase::Choose => g.ui_second(),
                    _ => {}
                }
                steps += 1;
                assert!(steps < 5000, "partie interminable");
            }
            assert_eq!(g.pot, 0);
            assert!(g.tok[0] == 0 || g.tok[1] == 0);
            assert_eq!(g.wins + g.losses, 1);
        }
    }

    #[test]
    fn jetons_conserves() {
        let mut g = Game::new(42);
        g.mode = Mode::Duo;
        g.new_game();
        for _ in 0..400 {
            settle(&mut g);
            assert_eq!(g.pot + g.tok[0] + g.tok[1], g.pot_init);
            match g.phase {
                Phase::PreRoll | Phase::Result | Phase::Over => g.ui_main(),
                Phase::Choose => {
                    g.ui_toggle(0);
                    g.ui_main();
                }
                _ => {}
            }
        }
    }

    /// Deux PC en lockstep : seules les actions circulent, les deux parties restent identiques.
    #[test]
    fn lockstep_deux_machines() {
        let seed = 0xDEAD_BEEF_1234_5678;
        let mut gs = [Game::new(1), Game::new(2)];
        gs[0].start_online(0, seed, 11);
        gs[1].start_online(1, seed, 11);
        let mut step = 0u32;
        for _ in 0..3000 {
            // la machine B est plus lente : ses animations avancent moins vite
            gs[0].tick(100.0);
            gs[1].tick(37.0);
            for k in 0..2 {
                let [a, b] = &mut gs;
                let (me, other) = if k == 0 { (a, b) } else { (b, a) };
                if me.idle() && me.inbox.is_empty() && me.can_act() {
                    step += 1;
                    match me.phase {
                        Phase::PreRoll | Phase::Result | Phase::Over => me.ui_main(),
                        Phase::Choose if step % 3 == 0 => me.ui_second(),
                        Phase::Choose => {
                            me.ui_toggle((step % 3) as usize);
                            me.ui_main();
                        }
                        _ => {}
                    }
                }
                let out: Vec<Frame> = me.outbox.drain(..).collect();
                other.inbox.extend(out);
            }
            let [a, b] = &gs;
            if a.idle() && b.idle() && a.inbox.is_empty() && b.inbox.is_empty() {
                assert_eq!(a.round, b.round);
                assert_eq!(a.phase, b.phase);
                assert_eq!((a.pot, a.tok), (b.pot, b.tok));
                assert_eq!(a.rng, b.rng);
                assert_eq!(a.hand(0), b.hand(0));
                assert_eq!(a.hand(1), b.hand(1));
            }
        }
        assert!(gs[0].round > 10 || gs[0].game_id > 1, "la partie en ligne n'a pas avancé");
    }
}
