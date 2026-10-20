//! Moteur de belote : règles (4 joueurs et belote à deux), annonces, décompte, bots facile / normal /
//! difficile (Monte-Carlo) et banc d'essai `-sim`. Sans graphisme et déterministe à partir de la graine.
//! Banc d'essai : belote -sim <parties> <graine> <joueurs> <niveau0> <niveau1> <journal>
//!                [seuil1 seuil2 seuil1' seuil2' mcn mcnb mcthr]  -> sim.log (vérifié par check.py)

use std::io::Write;
use std::time::Instant;

pub const PH_BID1: i32 = 1;
pub const PH_BID2: i32 = 2;
pub const PH_PLAY: i32 = 3;
pub const PH_TRICKEND: i32 = 4;
pub const PH_ROUNDEND: i32 = 5;
pub const PH_MATCHEND: i32 = 6;

const PTS_T: [i32; 8] = [0, 0, 14, 10, 20, 3, 4, 11];
const PTS_P: [i32; 8] = [0, 0, 0, 10, 2, 3, 4, 11];
const ORD_T: [i32; 8] = [0, 1, 6, 4, 7, 2, 3, 5];
const ORD_P: [i32; 8] = [0, 1, 2, 6, 3, 4, 5, 7];
const DISP: [i32; 4] = [0, 1, 3, 2];
const CARORD: [i32; 8] = [0, 0, 7, 5, 8, 3, 4, 6];
const CARVAL: [i32; 8] = [0, 0, 150, 100, 200, 100, 100, 100];

// Bloc d'état de la donne (copiable pour les simulations)
#[derive(Clone, Copy, Default)]
pub struct Gs {
    pub phase: i32,
    pub turn: i32,
    pub trump: i32,
    pub taker: i32,
    pub turned: i32,
    pub bidrnd: i32,
    pub passes: i32,
    pub dpos: i32,
    pub tn: i32,
    pub tlead: i32,
    pub led: i32,
    pub trickno: i32,
    pub lastwin: i32,
    pub belp: i32,
    pub seenmask: u32,
    pub annteam: i32,
    pub ev: i32,
    pub evp: i32,
    pub rpts: [i32; 2],
    pub rtr: [i32; 2],
    pub bel: [i32; 2],
    pub ann: [i32; 2],
    pub hn: [i32; 4],
    pub void: [u32; 4],
    pub anv: [i32; 4],
    pub ank: [i32; 4],
    pub ann_n: [i32; 4],
    pub deck: [u8; 32],
    pub hand: [[u8; 8]; 4],
    pub tcard: [u8; 8],
    /// annonces de chaque joueur : 1 tierce, 2 cinquante, 3 cent, 4 carré
    pub anl: [[u8; 4]; 4],
}

#[derive(Clone)]
pub struct Engine {
    pub gs: Gs,
    pub np: i32,
    pub target: i32,
    pub optann: i32,
    pub rng: u64,
    pub lvl: [i32; 4],
    pub dealer: i32,
    pub score: [i32; 2],
    pub pend: i32,
    pub logon: bool,
    pub rs_add: [i32; 2],
    pub rs_tot: [i32; 2],
    pub rs_made: i32,
    pub rs_capot: i32,
    pub rs_pend: i32,
    pub rs_npend: i32,
    pub mcflag: bool,
    pub rs_winner: i32,
    pub bmine: u32,
    pub thr1: [i32; 3],
    pub thr2: [i32; 3],
    pub mcn: i32,
    pub mcnb: i32,
    pub mcthr: i32,
    pub log: Vec<u8>,
    pub mcu: [u8; 40],
    pub mcm: usize,
    pub snap0: Gs,
    pub snap1: Gs,
    pub mcsum: [i32; 8],
}

#[inline]
fn bit(m: u32, i: i32) -> bool {
    (m >> i) & 1 != 0
}

impl Engine {
    pub fn new() -> Engine {
        Engine {
            gs: Gs::default(),
            np: 4,
            target: 1000,
            optann: 1,
            rng: 0x2545F4914F6CDD1D,
            lvl: [-1, 1, 1, 1],
            dealer: 0,
            score: [0, 0],
            pend: 0,
            logon: false,
            rs_add: [0; 2],
            rs_tot: [0; 2],
            rs_made: 0,
            rs_capot: -1,
            rs_pend: 0,
            rs_npend: 0,
            mcflag: false,
            rs_winner: -1,
            bmine: 0,
            thr1: [44, 44, 44],
            thr2: [42, 42, 42],
            mcn: 60,
            mcnb: 40,
            mcthr: 12,
            log: Vec::new(),
            mcu: [0; 40],
            mcm: 0,
            snap0: Gs::default(),
            snap1: Gs::default(),
            mcsum: [0; 8],
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
        self.rng_next() % n
    }

    // ---------- journal ----------
    pub fn lg_num(&mut self, v: i32) {
        let s = v.to_string();
        self.log.extend_from_slice(s.as_bytes());
        self.log.push(b' ');
    }
    pub fn log_deck(&mut self) {
        if !self.logon {
            return;
        }
        self.log.extend_from_slice(b"D ");
        self.lg_num(self.np);
        self.lg_num(self.dealer);
        for i in 0..32 {
            self.lg_num(self.gs.deck[i] as i32);
        }
        self.log.push(b'\n');
    }
    pub fn log_bid(&mut self, p: i32, ch: i32) {
        if !self.logon {
            return;
        }
        self.log.extend_from_slice(b"B ");
        self.lg_num(p);
        self.lg_num(ch);
        self.log.push(b'\n');
    }
    pub fn log_play(&mut self, p: i32, card: i32) {
        if !self.logon {
            return;
        }
        self.log.extend_from_slice(b"P ");
        self.lg_num(p);
        self.lg_num(card);
        self.log.extend_from_slice(b"| ");
        for i in 0..self.gs.hn[p as usize] as usize {
            self.lg_num(self.gs.hand[p as usize][i] as i32);
        }
        self.log.push(b'\n');
    }
    pub fn log_score(&mut self) {
        if !self.logon {
            return;
        }
        self.log.extend_from_slice(b"S ");
        self.lg_num(self.rs_add[0]);
        self.lg_num(self.rs_add[1]);
        self.lg_num(self.score[0]);
        self.lg_num(self.score[1]);
        self.lg_num(self.rs_made);
        self.lg_num(self.rs_capot);
        self.lg_num(self.rs_pend);
        self.log.push(b'\n');
    }

    // ---------- cartes ----------
    #[inline]
    pub fn card_key(&self, c: u8) -> i32 {
        let s = (c >> 3) as i32;
        let r = (c & 7) as usize;
        if s == self.gs.trump {
            16 + ORD_T[r]
        } else if s == self.gs.led {
            8 + ORD_P[r]
        } else {
            -1
        }
    }
    #[inline]
    pub fn card_pts(&self, c: u8) -> i32 {
        let s = (c >> 3) as i32;
        let r = (c & 7) as usize;
        if s == self.gs.trump {
            PTS_T[r]
        } else {
            PTS_P[r]
        }
    }

    // ---------- mains ----------
    pub fn hand_add(&mut self, p: usize, card: u8) {
        let n = self.gs.hn[p] as usize;
        self.gs.hand[p][n] = card;
        self.gs.hn[p] += 1;
    }
    pub fn hand_remove(&mut self, p: usize, pos: usize) -> u8 {
        let card = self.gs.hand[p][pos];
        let n = (self.gs.hn[p] - 1) as usize;
        self.gs.hn[p] = n as i32;
        let mut i = pos;
        while i < n {
            self.gs.hand[p][i] = self.gs.hand[p][i + 1];
            i += 1;
        }
        card
    }
    pub fn hand_key(&self, c: u8) -> i32 {
        let s = (c >> 3) as i32;
        let r = (c & 7) as usize;
        if s == self.gs.trump {
            7 - ORD_T[r]
        } else {
            (DISP[s as usize] + 1) * 8 + 7 - ORD_P[r]
        }
    }
    pub fn hand_sort(&mut self, p: usize) {
        let n = self.gs.hn[p] as usize;
        for i in 1..n {
            let card = self.gs.hand[p][i];
            let key = self.hand_key(card);
            let mut j = i;
            while j > 0 {
                let prev = self.gs.hand[p][j - 1];
                if self.hand_key(prev) <= key {
                    break;
                }
                self.gs.hand[p][j] = prev;
                j -= 1;
            }
            self.gs.hand[p][j] = card;
        }
    }
    pub fn sort_all(&mut self) {
        for p in 0..self.np as usize {
            self.hand_sort(p);
        }
    }
    pub fn deck_draw_to(&mut self, p: usize) {
        let c = self.gs.deck[self.gs.dpos as usize];
        self.gs.dpos += 1;
        self.hand_add(p, c);
    }
    pub fn hand_mask(&self, p: usize) -> u32 {
        let mut m = 0u32;
        for i in 0..self.gs.hn[p] as usize {
            m |= 1 << self.gs.hand[p][i];
        }
        m
    }
    pub fn hand_has(&self, p: usize, card: u8) -> bool {
        (0..self.gs.hn[p] as usize).any(|i| self.gs.hand[p][i] == card)
    }

    pub fn shuffle(&mut self) {
        for i in 0..32 {
            self.gs.deck[i] = i as u8;
        }
        let mut i = 31usize;
        while i > 0 {
            let j = self.rng_mod(i as u32 + 1) as usize;
            self.gs.deck.swap(j, i);
            i -= 1;
        }
    }
    pub fn deal_each(&mut self, count: i32) {
        for k in 1..=self.np {
            let p = ((self.dealer + k) % self.np) as usize;
            for _ in 0..count {
                self.deck_draw_to(p);
            }
        }
    }

    // ---------- donne ----------
    pub fn new_round(&mut self) {
        self.gs = Gs::default();
        self.gs.trump = -1;
        self.gs.taker = -1;
        self.gs.belp = -1;
        self.gs.annteam = -1;
        self.gs.bidrnd = 1;
        self.gs.tcard = [0xFF; 8];
        self.shuffle();
        if self.np == 4 {
            self.deal_each(3);
            self.deal_each(2);
        } else {
            self.deal_each(3);
            self.deal_each(3);
        }
        self.gs.turned = self.gs.deck[self.gs.dpos as usize] as i32;
        self.gs.dpos += 1;
        self.sort_all();
        self.gs.phase = PH_BID1;
        self.gs.turn = (self.dealer + 1) % self.np;
        self.log_deck();
    }
    pub fn next_round(&mut self) {
        self.dealer = (self.dealer + 1) % self.np;
        self.new_round();
    }
    pub fn new_match(&mut self) {
        self.score = [0, 0];
        self.pend = 0;
        self.rs_winner = -1;
    }

    // ---------- enchères ----------
    pub fn take_cards(&mut self) {
        let taker = self.gs.taker as usize;
        let turned = self.gs.turned as u8;
        self.hand_add(taker, turned);
        if self.np == 4 {
            for k in 1..=4 {
                let q = ((self.dealer + k) % 4) as usize;
                let cnt = if q == taker { 2 } else { 3 };
                for _ in 0..cnt {
                    self.deck_draw_to(q);
                }
            }
        } else {
            let opp = (taker + 1) & 1;
            self.deck_draw_to(opp);
        }
    }

    pub fn do_bid(&mut self, choice: i32) {
        let p = self.gs.turn;
        self.log_bid(p, choice);
        if choice != -1 {
            self.gs.trump = choice;
            self.gs.taker = p;
            self.take_cards();
            self.sort_all();
            self.gs.phase = PH_PLAY;
            self.gs.turn = (self.dealer + 1) % self.np;
            if self.np == 4 {
                self.compute_annonces();
            }
            return;
        }
        self.gs.passes += 1;
        if self.gs.passes >= self.np {
            if self.gs.bidrnd == 1 {
                self.gs.bidrnd = 2;
                self.gs.phase = PH_BID2;
                self.gs.passes = 0;
                self.gs.turn = (self.dealer + 1) % self.np;
            } else {
                self.next_round();
            }
        } else {
            self.gs.turn = (p + 1) % self.np;
        }
    }

    // ---------- annonces ----------
    pub fn add_annonce(&mut self, p: usize, kind: u8, value: i32, key: i32) {
        if self.gs.ann_n[p] >= 4 {
            return;
        }
        self.gs.anl[p][self.gs.ann_n[p] as usize] = kind;
        self.gs.ann_n[p] += 1;
        self.gs.anv[p] += value;
        if key > self.gs.ank[p] {
            self.gs.ank[p] = key;
        }
    }

    pub fn compute_annonces(&mut self) {
        for p in 0..self.np as usize {
            let mut m = [0u32; 4];
            for i in 0..self.gs.hn[p] as usize {
                let c = self.gs.hand[p][i];
                m[(c >> 3) as usize] |= 1 << (c & 7);
            }
            for r in 2..8usize {
                let b = 1u32 << r;
                if m[0] & b != 0 && m[1] & b != 0 && m[2] & b != 0 && m[3] & b != 0 {
                    self.add_annonce(p, 4, CARVAL[r], 4000 + CARORD[r] * 10);
                }
            }
            for s in 0..4usize {
                let mut b = 0usize;
                while b < 8 {
                    if m[s] >> b & 1 == 0 {
                        b += 1;
                        continue;
                    }
                    let start = b;
                    loop {
                        b += 1;
                        if b >= 8 || m[s] >> b & 1 == 0 {
                            break;
                        }
                    }
                    let len = b - start;
                    if len < 3 {
                        continue;
                    }
                    let mut key = (b as i32 - 1) * 10;
                    if s as i32 == self.gs.trump {
                        key += 5;
                    }
                    let (kind, value, base) = if len >= 5 {
                        (3, 100, 3000)
                    } else if len == 4 {
                        (2, 50, 2000)
                    } else {
                        (1, 20, 1000)
                    };
                    self.add_annonce(p, kind, value, key + base);
                }
            }
        }
    }

    pub fn eval_annonces(&mut self) {
        let mut k0 = 0;
        let mut k1 = 0;
        self.gs.annteam = -1;
        for p in 0..4usize {
            let k = self.gs.ank[p];
            if p & 1 == 0 {
                if k > k0 {
                    k0 = k;
                }
            } else if k > k1 {
                k1 = k;
            }
        }
        let team;
        if k0 > k1 {
            team = 0;
        } else if k1 > k0 {
            team = 1;
        } else {
            if k0 == 0 {
                return;
            }
            let mut j = 0;
            loop {
                let q = ((self.dealer + 1 + j) % self.np) as usize;
                if self.gs.ank[q] == k0 {
                    team = (q & 1) as i32;
                    break;
                }
                j += 1;
            }
        }
        self.gs.annteam = team;
        let mut sum = 0;
        for p in 0..4usize {
            if (p & 1) as i32 == team {
                sum += self.gs.anv[p];
            }
        }
        self.gs.ann[team as usize] = sum;
    }

    // ---------- jeu ----------
    pub fn trick_best(&self) -> (i32, i32) {
        let mut best = -2;
        let mut bp = 0;
        for q in 0..4usize {
            let c = self.gs.tcard[q];
            if c == 0xFF {
                continue;
            }
            let k = self.card_key(c);
            if k > best {
                best = k;
                bp = q as i32;
            }
        }
        (bp, best)
    }

    pub fn legal_mask(&self, p: usize) -> u32 {
        let n = self.gs.hn[p];
        let all = (1u32 << n) - 1;
        if self.gs.tn == 0 {
            return all;
        }
        if self.np == 2 && self.gs.dpos < 32 {
            return all;
        }
        let mut tmax = 15;
        for q in 0..4usize {
            let c = self.gs.tcard[q];
            if c == 0xFF {
                continue;
            }
            if (c >> 3) as i32 == self.gs.trump {
                let k = self.card_key(c);
                if k > tmax {
                    tmax = k;
                }
            }
        }
        let mut mled = 0u32;
        let mut mtr = 0u32;
        let mut mover = 0u32;
        for i in 0..n as usize {
            let c = self.gs.hand[p][i];
            let s = (c >> 3) as i32;
            if s == self.gs.led {
                mled |= 1 << i;
            }
            if s == self.gs.trump {
                mtr |= 1 << i;
                if self.card_key(c) > tmax {
                    mover |= 1 << i;
                }
            }
        }
        if self.gs.led == self.gs.trump {
            if mtr == 0 {
                return all;
            }
            return if mover != 0 { mover } else { mtr };
        }
        if mled != 0 {
            return mled;
        }
        if self.np == 4 {
            let (b, _) = self.trick_best();
            if (b ^ p as i32) & 1 == 0 {
                return all;
            }
        }
        if mtr == 0 {
            return all;
        }
        if mover != 0 {
            mover
        } else {
            mtr
        }
    }

    pub fn do_play(&mut self, pos: usize) {
        let p = self.gs.turn as usize;
        let card = self.hand_remove(p, pos);
        self.log_play(p as i32, card as i32);
        let s = (card >> 3) as i32;
        if self.gs.tn == 0 {
            self.gs.led = s;
            self.gs.tlead = p as i32;
        } else if s != self.gs.led {
            self.gs.void[p] |= 1 << self.gs.led;
        }
        self.gs.tcard[p] = card;
        self.gs.tn += 1;
        self.gs.seenmask |= 1 << card;
        if s == self.gs.trump {
            let r = card & 7;
            if r == 5 || r == 6 {
                let other = card ^ 3;
                if self.hand_has(p, other) {
                    self.gs.belp = p as i32;
                    self.gs.ev = 1;
                    self.gs.evp = p as i32;
                } else if self.gs.belp == p as i32 {
                    self.gs.bel[p & 1] = 20;
                    self.gs.belp = -1;
                    self.gs.ev = 2;
                    self.gs.evp = p as i32;
                }
            }
        }
        if self.gs.tn >= self.np {
            self.gs.phase = PH_TRICKEND;
        } else {
            self.gs.turn = (p as i32 + 1) % self.np;
        }
    }

    pub fn trick_collect(&mut self) {
        let (w, _) = self.trick_best();
        let mut pts = 0;
        for q in 0..4usize {
            let c = self.gs.tcard[q];
            if c != 0xFF {
                pts += self.card_pts(c);
            }
        }
        let empty = self.gs.hn.iter().all(|&h| h == 0);
        if empty && (self.np == 4 || self.gs.dpos >= 32) {
            pts += 10;
        }
        let t = (w & 1) as usize;
        self.gs.rpts[t] += pts;
        self.gs.rtr[t] += 1;
        self.gs.lastwin = w;
        self.gs.trickno += 1;
        if self.gs.trickno == 1 && self.np == 4 && self.optann != 0 {
            self.eval_annonces();
        }
        self.gs.tcard = [0xFF; 8];
        self.gs.tn = 0;
        self.gs.turn = w;
        if self.np == 2 && self.gs.dpos < 32 {
            self.deck_draw_to(w as usize);
            self.deck_draw_to(((w + 1) & 1) as usize);
            self.hand_sort(0);
            self.hand_sort(1);
        }
        if self.gs.hn.iter().all(|&h| h == 0) {
            if self.mcflag {
                self.gs.phase = PH_ROUNDEND;
            } else {
                self.score_round();
            }
        } else {
            self.gs.phase = PH_PLAY;
        }
    }

    // ---------- décompte ----------
    pub fn calc_round(&mut self) {
        let a = (self.gs.taker & 1) as usize;
        let b = a ^ 1;
        self.rs_capot = -1;
        if self.gs.rtr[0] == self.gs.trickno {
            self.rs_capot = 0;
        }
        if self.gs.rtr[1] == self.gs.trickno {
            self.rs_capot = 1;
        }
        for t in 0..2usize {
            let mut v = self.gs.rpts[t] + self.gs.bel[t] + self.gs.ann[t];
            if self.rs_capot == t as i32 {
                v += 90;
            }
            self.rs_tot[t] = v;
        }
        let ta = self.rs_tot[a];
        let tb = self.rs_tot[b];
        self.rs_pend = 0;
        self.rs_npend = 0;
        if ta == tb {
            self.rs_made = 2;
            self.rs_add[b] = tb;
            self.rs_add[a] = 0;
            self.rs_npend = self.pend + ta;
            return;
        }
        let winner;
        if ta > tb {
            self.rs_made = 1;
            self.rs_add[a] = ta;
            self.rs_add[b] = tb;
            winner = a;
        } else {
            self.rs_made = 0;
            let mut v = 162;
            if self.rs_capot == b as i32 {
                v += 90;
            }
            v += self.gs.ann[0] + self.gs.ann[1] + self.gs.bel[b];
            self.rs_add[b] = v;
            self.rs_add[a] = self.gs.bel[a];
            winner = b;
        }
        self.rs_pend = self.pend;
        self.rs_add[winner] += self.pend;
    }

    pub fn score_round(&mut self) {
        self.calc_round();
        self.pend = self.rs_npend;
        self.score[0] += self.rs_add[0];
        self.score[1] += self.rs_add[1];
        self.gs.phase = PH_ROUNDEND;
        if self.score[0] >= self.target || self.score[1] >= self.target {
            self.gs.phase = PH_MATCHEND;
            self.rs_winner = if self.score[1] > self.score[0] { 1 } else { 0 };
        }
        self.log_score();
    }

    // ---------- bots : enchères ----------
    pub fn clamp_lvl(&self, p: usize) -> i32 {
        if self.mcflag {
            return 1;
        }
        let l = self.lvl[p];
        if l < 0 {
            1
        } else if l > 2 {
            2
        } else {
            l
        }
    }

    pub fn eval_suit(&self, p: usize, s: i32, with_turned: bool) -> i32 {
        let mut m = self.hand_mask(p);
        if with_turned {
            m |= 1 << self.gs.turned;
        }
        let mut v = 0;
        let mut ntr = 0;
        for c in 0..32i32 {
            if !bit(m, c) {
                continue;
            }
            let su = c >> 3;
            let r = (c & 7) as usize;
            if su == s {
                v += PTS_T[r];
                ntr += 1;
                if r == 4 {
                    v += 6;
                }
                if r == 2 {
                    v += 3;
                }
            } else if r == 7 {
                v += 9;
            } else if r == 3 && bit(m, su * 8 + 7) {
                v += 4;
            }
        }
        if ntr >= 3 {
            v += (ntr - 2) * 7;
        }
        if bit(m, s * 8 + 5) && bit(m, s * 8 + 6) {
            v += 10;
        }
        v
    }

    pub fn bot_bid(&mut self, p: usize) -> i32 {
        let lvl = self.clamp_lvl(p);
        if lvl == 2 {
            return self.mc_bid(p);
        }
        if self.gs.bidrnd == 1 {
            let s = self.gs.turned >> 3;
            let mut v = self.eval_suit(p, s, true);
            if lvl == 0 {
                v = v + self.rng_mod(31) as i32 - 15;
            }
            return if v >= self.thr1[lvl as usize] { s } else { -1 };
        }
        let mut best = -1;
        let mut bv = -1000;
        for s in 0..4 {
            if self.gs.turned >> 3 == s {
                continue;
            }
            let mut v = self.eval_suit(p, s, true);
            if lvl == 0 {
                v = v + self.rng_mod(31) as i32 - 15;
            }
            if v > bv {
                bv = v;
                best = s;
            }
        }
        let mut thr = self.thr2[lvl as usize];
        if p as i32 == self.dealer {
            thr -= 8;
        }
        if bv >= thr {
            best
        } else {
            -1
        }
    }

    // ---------- bots : jeu ----------
    pub fn is_master(&self, c: u8) -> bool {
        let s = (c >> 3) as i32;
        let r = (c & 7) as usize;
        let ord = if s == self.gs.trump { &ORD_T } else { &ORD_P };
        let mine = self.gs.seenmask | self.bmine;
        let my = ord[r];
        for r2 in 0..8usize {
            if ord[r2] > my && !bit(mine, s * 8 + r2 as i32) {
                return false;
            }
        }
        true
    }

    pub fn bot_lead(&self, p: usize, mask: u32) -> usize {
        let n = self.gs.hn[p] as usize;
        let mut bsc = [0i32; 4];
        for i in 0..n {
            bsc[(self.gs.hand[p][i] >> 3) as usize] += 1;
        }
        let mut best = -1000;
        let mut bpos = 0usize;
        for i in 0..n {
            if !bit(mask, i as i32) {
                continue;
            }
            let c = self.gs.hand[p][i];
            let master = self.is_master(c);
            let s = (c >> 3) as i32;
            let r = (c & 7) as usize;
            let attacker = ((self.gs.taker ^ p as i32) & 1) == 0;
            let sc;
            if s == self.gs.trump {
                let strength = ORD_T[r];
                if attacker {
                    if r == 4 {
                        sc = 100;
                    } else if master {
                        sc = 80;
                    } else {
                        let base = if bsc[s as usize] >= 3 { 45 } else { 5 };
                        sc = base - strength;
                    }
                } else if master {
                    sc = 40;
                } else {
                    sc = -10 - strength;
                }
            } else {
                let pts = PTS_P[r];
                if master {
                    sc = 60 + (pts >> 1);
                } else {
                    sc = bsc[s as usize] * 2 + 20 - 2 * pts;
                }
            }
            if sc > best {
                best = sc;
                bpos = i;
            }
        }
        bpos
    }

    pub fn bot_follow(&self, p: usize, mask: u32) -> usize {
        let (bestp, bestkey) = self.trick_best();
        let n = self.gs.hn[p] as usize;
        let mut bwk = 1000;
        let mut bwp: i32 = -1;
        let mut bmv = -1;
        let mut bmp = 0usize;
        let mut bdv = 100000;
        let mut bdp = 0usize;
        for i in 0..n {
            if !bit(mask, i as i32) {
                continue;
            }
            let c = self.gs.hand[p][i];
            let k = self.card_key(c);
            if k > bestkey && k < bwk {
                bwk = k;
                bwp = i as i32;
            }
            let pts = self.card_pts(c);
            let r = (c & 7) as usize;
            let (give, disc);
            if (c >> 3) as i32 == self.gs.trump {
                give = pts * 2;
                disc = pts * 8 + ORD_T[r] + 400;
            } else {
                give = pts * 4;
                disc = pts * 8 + ORD_P[r];
            }
            if give > bmv {
                bmv = give;
                bmp = i;
            }
            if disc < bdv {
                bdv = disc;
                bdp = i;
            }
        }
        if self.np == 4 && ((bestp ^ p as i32) & 1) == 0 {
            if self.gs.tn == self.np - 1 {
                return bmp;
            }
            let wc = self.gs.tcard[bestp as usize];
            if !self.is_master(wc) {
                return bdp;
            }
            if (wc >> 3) as i32 == self.gs.trump {
                return bmp;
            }
            if self.gs.led == self.gs.trump {
                return bmp;
            }
            let mut j = 1;
            loop {
                let q = ((p as i32 + j) % self.np) as usize;
                if q as i32 == self.gs.tlead {
                    return bmp;
                }
                if bit(self.gs.void[q], self.gs.led) {
                    return bdp;
                }
                j += 1;
            }
        }
        if bwp >= 0 {
            bwp as usize
        } else {
            bdp
        }
    }

    pub fn bot_card(&mut self, p: usize) -> usize {
        let mask = self.legal_mask(p);
        if mask.count_ones() == 1 {
            return mask.trailing_zeros() as usize;
        }
        let lvl = self.clamp_lvl(p);
        if lvl == 2 {
            return self.mc_choose(p);
        }
        if lvl == 0 {
            let r = self.rng_mod(100);
            if r < 35 {
                let n = mask.count_ones();
                let mut k = self.rng_mod(n);
                let mut m = mask;
                loop {
                    let b = m.trailing_zeros();
                    if k == 0 {
                        return b as usize;
                    }
                    m &= !(1 << b);
                    k -= 1;
                }
            }
        }
        self.bmine = self.hand_mask(p);
        if self.gs.tn == 0 {
            self.bot_lead(p, mask)
        } else {
            self.bot_follow(p, mask)
        }
    }

    // ---------- Monte-Carlo ----------
    pub fn mc_begin(&mut self) -> (bool, i32) {
        let sv = (self.logon, self.optann);
        self.logon = false;
        self.optann = 0;
        self.mcflag = true;
        sv
    }
    pub fn mc_end(&mut self, sv: (bool, i32)) {
        self.logon = sv.0;
        self.optann = sv.1;
        self.mcflag = false;
        self.gs.ev = 0;
    }

    pub fn mc_rollout(&mut self) {
        for _ in 0..79 {
            let ph = self.gs.phase;
            if ph == PH_PLAY {
                let p = self.gs.turn as usize;
                let pos = self.bot_card(p);
                self.do_play(pos);
            } else if ph == PH_TRICKEND {
                self.trick_collect();
            } else {
                break;
            }
        }
    }

    pub fn mc_value(&mut self, p: usize) -> i32 {
        self.calc_round();
        let t = p & 1;
        self.rs_add[t] - self.rs_add[t ^ 1]
    }

    pub fn mc_shuffle(&mut self) {
        let mut i = self.mcm as i32 - 1;
        while i > 0 {
            let j = self.rng_mod(i as u32 + 1) as usize;
            self.mcu.swap(j, i as usize);
            i -= 1;
        }
    }

    pub fn mc_determinize(&mut self, p: usize) {
        let mut unseen = !(self.hand_mask(p) | self.gs.seenmask);
        let mut known: i32 = -1;
        if self.gs.taker >= 0 && self.gs.taker != p as i32 && bit(unseen, self.gs.turned) {
            unseen &= !(1u32 << self.gs.turned);
            known = self.gs.turned;
        }
        let mut m = 0usize;
        for c in 0..32u32 {
            if unseen >> c & 1 != 0 {
                self.mcu[m] = c as u8;
                m += 1;
            }
        }
        self.mcm = m;
        let mut tries = 24;
        loop {
            self.mc_shuffle();
            let mut idx = 0usize;
            let mut ok = true;
            for q in 0..self.np as usize {
                if q == p {
                    continue;
                }
                let need = self.gs.hn[q] as usize;
                let mut slot = 0usize;
                if q as i32 == self.gs.taker && known >= 0 {
                    self.gs.hand[q][0] = known as u8;
                    slot = 1;
                }
                while slot < need {
                    let c = self.mcu[idx];
                    idx += 1;
                    self.gs.hand[q][slot] = c;
                    if bit(self.gs.void[q], (c >> 3) as i32) {
                        ok = false;
                    }
                    slot += 1;
                }
            }
            if self.np == 2 {
                let mut d = self.gs.dpos as usize;
                while idx < self.mcm {
                    self.gs.deck[d] = self.mcu[idx];
                    d += 1;
                    idx += 1;
                }
            }
            if ok {
                break;
            }
            tries -= 1;
            if tries == 0 {
                break;
            }
        }
    }

    pub fn mc_choose(&mut self, p: usize) -> usize {
        let mask = self.legal_mask(p);
        let sv = self.mc_begin();
        self.snap0 = self.gs;
        self.mcsum = [0; 8];
        for _ in 0..self.mcn {
            self.gs = self.snap0;
            self.mc_determinize(p);
            self.snap1 = self.gs;
            for pos in 0..8usize {
                if !bit(mask, pos as i32) {
                    continue;
                }
                self.gs = self.snap1;
                self.do_play(pos);
                self.mc_rollout();
                let v = self.mc_value(p);
                self.mcsum[pos] += v;
            }
        }
        self.gs = self.snap0;
        self.mc_end(sv);
        let mut best = -2000000000;
        let mut bpos = 0usize;
        for pos in 0..8usize {
            if bit(mask, pos as i32) && self.mcsum[pos] > best {
                best = self.mcsum[pos];
                bpos = pos;
            }
        }
        bpos
    }

    pub fn mc_deal_bid(&mut self, p: usize, s: i32) {
        let unseen = !(self.hand_mask(p) | (1u32 << self.gs.turned));
        let mut m = 0usize;
        for c in 0..32u32 {
            if unseen >> c & 1 != 0 {
                self.mcu[m] = c as u8;
                m += 1;
            }
        }
        self.mcm = m;
        self.mc_shuffle();
        let need = if self.np == 2 { 7 } else { 8 };
        let mut idx = 0usize;
        for q in 0..self.np as usize {
            if q == p {
                continue;
            }
            for slot in 0..need {
                self.gs.hand[q][slot] = self.mcu[idx];
                idx += 1;
            }
            self.gs.hn[q] = need as i32;
        }
        let mut n = self.gs.hn[p] as usize;
        self.gs.hand[p][n] = self.gs.turned as u8;
        n += 1;
        if self.np == 4 {
            while n < 8 {
                self.gs.hand[p][n] = self.mcu[idx];
                idx += 1;
                n += 1;
            }
        }
        self.gs.hn[p] = n as i32;
        if self.np == 2 {
            self.gs.dpos = 14;
            let mut d = 14usize;
            while idx < self.mcm {
                self.gs.deck[d] = self.mcu[idx];
                d += 1;
                idx += 1;
            }
        }
        self.gs.trump = s;
        self.gs.taker = p as i32;
        self.gs.phase = PH_PLAY;
        self.gs.tn = 0;
        self.gs.trickno = 0;
        self.gs.seenmask = 0;
        self.gs.belp = -1;
        self.gs.annteam = -1;
        self.gs.turn = (self.dealer + 1) % self.np;
        self.gs.rpts = [0; 2];
        self.gs.rtr = [0; 2];
        self.gs.bel = [0; 2];
        self.gs.ann = [0; 2];
        self.gs.void = [0; 4];
        self.gs.tcard = [0xFF; 8];
    }

    pub fn mc_bid(&mut self, p: usize) -> i32 {
        let sv = self.mc_begin();
        self.snap0 = self.gs;
        let mut best = -2000000000;
        let mut bsuit = -1;
        for s in 0..4i32 {
            let ts = self.gs.turned >> 3;
            if self.gs.bidrnd == 1 {
                if ts != s {
                    continue;
                }
            } else if ts == s {
                continue;
            }
            let mut total = 0i32;
            for _ in 0..self.mcnb {
                self.gs = self.snap0;
                self.mc_deal_bid(p, s);
                self.mc_rollout();
                total += self.mc_value(p);
            }
            let avg = total / self.mcnb;
            if avg > best {
                best = avg;
                bsuit = s;
            }
        }
        self.gs = self.snap0;
        self.mc_end(sv);
        if bsuit < 0 || best < self.mcthr {
            -1
        } else {
            bsuit
        }
    }
}

/// Banc d'essai sans fenêtre : `a` = arguments après `-sim`. Écrit le journal dans sim.log.
pub fn sim_main(a: &[String]) {
    let mut args = [10i32, 1, 4, 1, 1, 0, -1000, 0, 0, 0, -1000, -1000, -1000];
    for (i, s) in a.iter().enumerate() {
        if i < args.len() {
            args[i] = s.parse().unwrap_or(0);
        }
    }
    let mut g = Engine::new();
    g.log.reserve(1 << 22);
    g.rng = ((args[1] as u32 as u64).wrapping_mul(0x9E3779B97F4A7C15)) | 1;
    g.np = args[2];
    g.logon = args[5] != 0;
    if args[6] != -1000 {
        g.thr1[1] = args[6];
        g.thr2[1] = args[7];
        g.thr1[2] = args[8];
        g.thr2[2] = args[9];
    }
    if args[10] != -1000 {
        g.mcn = args[10];
    }
    if args[11] != -1000 {
        g.mcnb = args[11];
    }
    if args[12] != -1000 {
        g.mcthr = args[12];
    }
    if g.np == 4 {
        g.lvl = [args[3], args[4], args[3], args[4]];
    } else {
        g.lvl[0] = args[3];
        g.lvl[1] = args[4];
    }
    let t0 = Instant::now();
    let (mut games, mut rounds, mut v0, mut v1) = (0, 0, 0, 0);
    while games < args[0] {
        g.new_match();
        g.dealer = g.rng_mod(if g.np == 2 { 2 } else { 4 }) as i32;
        g.new_round();
        loop {
            let ph = g.gs.phase;
            if ph == PH_BID1 || ph == PH_BID2 {
                let p = g.gs.turn as usize;
                let c = g.bot_bid(p);
                g.do_bid(c);
            } else if ph == PH_PLAY {
                let p = g.gs.turn as usize;
                let pos = g.bot_card(p);
                if g.logon {
                    g.log.extend_from_slice(b"Q ");
                    g.lg_num(pos as i32);
                    let m = g.legal_mask(p);
                    g.lg_num(m as i32);
                    g.lg_num(g.gs.hn[p]);
                    g.log.push(b'\n');
                }
                g.do_play(pos);
            } else if ph == PH_TRICKEND {
                g.trick_collect();
            } else if ph == PH_ROUNDEND {
                rounds += 1;
                g.next_round();
            } else {
                break;
            }
        }
        rounds += 1;
        if g.rs_winner == 0 {
            v0 += 1;
        } else {
            v1 += 1;
        }
        games += 1;
    }
    let dt = t0.elapsed();
    let line = format!("# {} {} {} {} \n", games, v0, v1, rounds);
    g.log.extend_from_slice(line.as_bytes());
    std::fs::File::create("sim.log").unwrap().write_all(&g.log).unwrap();
    eprintln!("temps {:.3} s", dt.as_secs_f64());
}
