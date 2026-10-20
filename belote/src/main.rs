//! Belote en 3D isométrique : contre des bots, belote à deux, ou en ligne de 2 à 4 humains.
//! Construite sur le moteur commun `bistrot` (fenêtre, dessin processeur, cartes, table, réseau).
//! `belote -sim ...` = banc d'essai sans fenêtre.

#![windows_subsystem = "windows"]

mod cfg;
mod draw;
mod engine;
mod table;
mod vue;

use bistrot::cartes::Paquet;
use bistrot::gfx::Gfx;
use bistrot::polices::{Fichiers, Polices};
use bistrot::reseau::{Menu, Net};
use bistrot::shell::{self, Key};
use draw::{BtnId, Overlay};
use engine::*;
use std::hash::{Hash, Hasher};
use table::Table;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "-sim") {
        engine::sim_main(&args[i + 1..]);
        return;
    }
    let mut t = Table::new(bistrot::graine());
    let c = cfg::load(&mut t);
    t.new_match();
    let net = Net::new(c.ip.clone());
    let fullscreen = c.fullscreen;
    let fichiers = Fichiers::charger();
    shell::run(Belote {
        polices: Polices::depuis(&fichiers),
        paquet: Paquet::new(&fichiers),
        t,
        c,
        net,
        fullscreen,
        quit: false,
        gallery: false,
        strength: false,
        hover: None,
        mouse: (0.0, 0.0),
        sig: 0,
        vue: vue::Vue::default(),
    });
}

struct Belote {
    polices: Polices,
    /// cartes dessinées à l'échelle de la fenêtre
    paquet: Paquet,
    t: Table,
    c: cfg::Cfg,
    net: Net,
    fullscreen: bool,
    quit: bool,
    gallery: bool,
    strength: bool,
    hover: Option<usize>,
    mouse: (f32, f32),
    /// empreinte de ce qui est affiché : on ne redessine que si elle change
    sig: u64,
    /// ce qui bouge à l'écran (distribution, main, retourne, bulles, score)
    vue: vue::Vue,
}

fn btn_action(t: &mut Table, id: BtnId) {
    match id {
        BtnId::Next => {
            if matches!(t.phase(), PH_ROUNDEND | PH_MATCHEND) {
                t.human_next();
            }
        }
        BtnId::Pass => t.human_bid(-1),
        BtnId::Take => t.human_bid(t.e.gs.turned >> 3),
        BtnId::Suit(k) => t.human_bid(k as i32),
    }
}

impl Belote {
    fn save(&mut self) {
        self.t.save_needed = false;
        cfg::save(&self.t, &mut self.c);
    }

    /// Empreinte de l'état affiché.
    fn signature(&self) -> u64 {
        let t = &self.t;
        let gs = &t.e.gs;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (gs.phase, gs.turn, gs.tn, gs.trickno, gs.hn, gs.tcard, gs.trump, gs.taker, gs.dpos, gs.passes, gs.bidrnd).hash(&mut h);
        (t.e.score, t.e.dealer, t.e.np, t.e.target, t.e.optann, t.hint, t.nextmask, t.wins, t.help, t.botlvl).hash(&mut h);
        for p in 0..4 {
            t.bub[p].as_ref().map(|(s, end)| (s.clone(), *end > t.now)).hash(&mut h);
        }
        (t.message(), self.net.panel().map(|(a, l, m)| (a.to_string(), l.map(|x| x.to_string()), m))).hash(&mut h);
        (self.gallery, self.strength, self.hover, draw::buttons(t).iter().position(|b| b.touche(self.mouse.0, self.mouse.1))).hash(&mut h);
        h.finish()
    }
}

impl shell::Game for Belote {
    fn title(&self) -> String {
        "Belote - jeu du bistro en 3D isométrique".into()
    }

    fn shot_name(&self) -> &str {
        "belote_shot.png"
    }

    /// Le fond (table + talon, étiquettes, mains adverses) n'est refait que si cette empreinte change.
    fn fond_signature(&self) -> u64 {
        draw::under_signature(&self.t, &self.vue) ^ self.gallery as u64
    }

    fn background(&mut self, g: &Gfx) {
        self.paquet.echelle((g.sc * 1.04).max(0.5));
        bistrot::table::dessiner(g);
        if !self.gallery {
            draw::draw_under(&self.t, g, &self.polices, &self.paquet, &self.vue);
        }
    }

    fn alleger(&mut self) {
        self.paquet.alleger();
    }

    fn draw(&mut self, g: &Gfx) {
        if self.gallery {
            draw::gallery(g, &self.polices, &self.paquet);
            return;
        }
        let ov = Overlay {
            strength: self.strength,
            hover: self.hover,
            mouse: self.mouse,
            net: self.net.panel(),
            optline: draw::optline(&self.t),
        };
        draw::draw_over(&self.t, g, &self.polices, &self.paquet, &ov, &self.vue);
    }

    fn tick(&mut self, dt: f64) -> bool {
        self.t.tick(dt);
        self.net.poll(&mut self.t);
        self.vue.maj(&self.t, self.hover, dt);
        if self.t.save_needed {
            self.save();
        }
        let sig = self.signature();
        let changed = sig != self.sig || self.animating();
        self.sig = sig;
        changed
    }

    fn animating(&self) -> bool {
        let t = &self.t;
        let last = t.ast0.iter().cloned().fold(f64::MIN, f64::max);
        t.collect > 0.0 || t.now - last < 450.0 || self.vue.anime()
    }

    fn key(&mut self, k: Key) {
        let t = &mut self.t;
        let net = &mut self.net;
        match net.menu {
            Menu::Info => net.menu = Menu::None,
            Menu::Ip => {
                let ch = match k {
                    Key::Esc => '\u{1b}',
                    Key::Enter => '\r',
                    Key::Back => '\u{8}',
                    Key::Char(c) => c,
                    _ => return,
                };
                if net.ip_char(ch) {
                    self.c.ip = net.ip.clone();
                    self.save();
                }
            }
            Menu::Choice => match k {
                Key::Char(c) => net.menu_key(t, c),
                Key::Esc => net.menu_key(t, '\u{1b}'),
                _ => {}
            },
            Menu::None => match k {
                Key::Esc => {
                    if net.waiting() {
                        net.cancel(t);
                    } else if self.strength {
                        self.strength = false;
                    } else if self.gallery {
                        self.gallery = false;
                    } else if self.fullscreen {
                        self.fullscreen = false;
                        self.c.fullscreen = false;
                        self.save();
                    } else {
                        self.quit = true;
                    }
                }
                Key::F11 => {
                    self.fullscreen = !self.fullscreen;
                    self.c.fullscreen = self.fullscreen;
                    self.save();
                }
                Key::Space | Key::Enter => {
                    if net.hosting() {
                        net.host_start(t);
                    } else {
                        btn_action(t, BtnId::Next);
                    }
                }
                Key::Char(c) if !net.waiting() => match c.to_ascii_uppercase() {
                    'J' => t.toggle_np(),
                    'L' => t.cycle_level(),
                    'A' => t.toggle_annonces(),
                    'H' => t.toggle_help(),
                    'P' => t.cycle_target(),
                    'U' => t.auto = !t.auto,
                    'G' => self.gallery = !self.gallery,
                    'F' => self.strength = !self.strength,
                    'N' => net.open(t),
                    _ => {}
                },
                _ => {}
            },
        }
        if self.t.save_needed {
            self.save();
        }
    }

    fn click(&mut self, x: f32, y: f32) {
        self.mouse = (x, y);
        let t = &mut self.t;
        if self.strength {
            self.strength = false; // un clic ferme le panneau (sans jouer)
            return;
        }
        if self.net.menu != Menu::None || self.net.waiting() || self.gallery {
            return;
        }
        if let Some(b) = draw::buttons(t).into_iter().find(|b| b.touche(x, y)) {
            btn_action(t, b.id);
        } else if t.phase() == PH_PLAY && t.is_human(t.turn()) {
            if let Some(i) = draw::hand_hit(t, x, y) {
                if t.e.legal_mask(t.me) >> i & 1 == 1 {
                    t.human_play(i);
                }
            }
        }
        self.hover = draw::hand_hit(&self.t, x, y);
    }

    fn hover(&mut self, x: f32, y: f32) -> bool {
        self.mouse = (x, y);
        self.hover = draw::hand_hit(&self.t, x, y);
        let sig = self.signature();
        let changed = sig != self.sig;
        self.sig = sig;
        changed
    }

    fn fullscreen(&self) -> bool {
        self.fullscreen
    }

    fn quit(&self) -> bool {
        self.quit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::time::{Duration, Instant};

    /// Un hôte et trois invités sur 127.0.0.1 (vraies connexions TCP, l'hôte relaie) :
    /// les quatre machines jouent en mode automatique et restent identiques.
    #[test]
    fn quatre_joueurs_tcp() {
        let n = 4;
        let mut ts: Vec<Table> = (0..n).map(|k| Table::new(100 + k as u64)).collect();
        let mut nets: Vec<Net> = (0..n).map(|_| Net::new("127.0.0.1".into())).collect();
        nets[0].open(&mut ts[0]);
        nets[0].menu_key(&mut ts[0], 'h');
        assert!(nets[0].hosting(), "{:?}", nets[0].lines);
        for k in 1..n {
            nets[k].open(&mut ts[k]);
            nets[k].menu_key(&mut ts[k], 'r');
            nets[k].ip_char('\r');
            // attendre que l'hôte ait accepté cet invité avant le suivant (sièges dans l'ordre)
            for _ in 0..500 {
                nets[k].poll(&mut ts[k]);
                nets[0].poll(&mut ts[0]);
                if nets[0].invites() == k {
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        for t in ts.iter_mut() {
            t.auto = true;
        }
        let speeds = [50.0, 37.0, 71.0, 44.0];
        // scores à chaque fin de manche, pour chaque machine
        let mut hist: Vec<Vec<[i32; 2]>> = vec![Vec::new(); n];
        let mut was_end = [false; 4];
        for _ in 0..40_000 {
            for k in 0..n {
                nets[k].poll(&mut ts[k]);
                ts[k].tick(speeds[k]);
                let end = ts[k].phase() == PH_ROUNDEND || ts[k].phase() == PH_MATCHEND;
                if end && !was_end[k] {
                    hist[k].push(ts[k].e.score);
                }
                was_end[k] = end;
            }
            if hist.iter().all(|h| h.len() >= 3) {
                break;
            }
            std::thread::sleep(Duration::from_micros(300));
        }
        assert!(ts.iter().all(|t| t.online), "pas tous en ligne : {:?}", nets.iter().map(|n| n.lines[0].clone()).collect::<Vec<_>>());
        assert_eq!(ts[0].humans, 0b1111);
        assert!(hist.iter().all(|h| h.len() >= 3), "manches jouées : {:?}", hist.iter().map(|h| h.len()).collect::<Vec<_>>());
        for h in &hist {
            assert_eq!(h[..3], hist[0][..3], "machines désynchronisées");
        }
        // un invité part : tout le monde revient en partie locale
        nets[2].close(&mut ts[2], "fin");
        for _ in 0..500 {
            for k in 0..n {
                nets[k].poll(&mut ts[k]);
            }
            if !ts[0].online {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(!ts[0].online);
    }

    /// Temps de dessin d'une image en pleine partie (cargo test --release -p belote -- --ignored mesure_dessin --nocapture).
    #[test]
    #[ignore]
    fn mesure_dessin() {
        let fichiers = Fichiers::charger();
        let polices = Polices::depuis(&fichiers);
        let mut paquet = Paquet::new(&fichiers);
        let glyphs = RefCell::new(bistrot::gfx::Glyphs::default());
        let mut t = Table::new(5);
        t.auto = true;
        for _ in 0..400 {
            t.tick(50.0);
            if t.e.gs.tn == 3 {
                break;
            }
        }
        let mut bg = tiny_skia::Pixmap::new(1100, 800).unwrap();
        {
            let g = Gfx::fit(&mut bg, &glyphs);
            paquet.echelle(g.sc * 1.04);
            bistrot::table::dessiner(&g);
        }
        let mut frame = bg.clone();
        let ov = Overlay { strength: false, hover: None, mouse: (0.0, 0.0), net: None, optline: draw::optline(&t) };
        let mut vue = vue::Vue::default();
        vue.maj(&t, None, 16.0);
        let n = 100;
        let mut tm = [0f64; 3];
        for _ in 0..n {
            let t0 = Instant::now();
            frame.data_mut().copy_from_slice(bg.data());
            let t1 = Instant::now();
            let g = Gfx::fit(&mut frame, &glyphs);
            draw::draw_under(&t, &g, &polices, &paquet, &vue);
            let t2 = Instant::now();
            draw::draw_over(&t, &g, &polices, &paquet, &ov, &vue);
            let t3 = Instant::now();
            for (k, d) in [t1 - t0, t2 - t1, t3 - t2].iter().enumerate() {
                tm[k] += d.as_secs_f64() * 1000.0;
            }
        }
        println!("copie decor {:.2} ms | couche fixe {:.2} ms | reste {:.2} ms (par image)", tm[0] / n as f64, tm[1] / n as f64, tm[2] / n as f64);
    }
}
