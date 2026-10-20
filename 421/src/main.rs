//! 421 en 3D isométrique : contre un bot, à deux sur le même PC ou en duel en ligne.
//! Construit sur le moteur commun `bistrot` (fenêtre, dessin processeur, projection iso, réseau) ;
//! `421 -t` joue dans le terminal.

#![windows_subsystem = "windows"]

mod ai;
mod cfg;
mod game;
mod render;
mod term;

use bistrot::gfx::Gfx;
use bistrot::reseau::{Menu, Net};
use bistrot::shell::{self, Key};
use game::Game;
use render::{Fonts, Hit};
use std::hash::{Hash, Hasher};

#[cfg(windows)]
mod win {
    #[link(name = "kernel32")]
    extern "system" {
        pub fn AttachConsole(pid: u32) -> i32;
        pub fn AllocConsole() -> i32;
        pub fn SetConsoleOutputCP(cp: u32) -> i32;
    }

    /// Console pour le mode terminal (celle qui a lancé le jeu, sinon une nouvelle).
    pub fn console() {
        unsafe {
            if AttachConsole(u32::MAX) == 0 {
                AllocConsole();
            }
            SetConsoleOutputCP(65001);
        }
    }
}

fn main() {
    let mut g = Game::new(bistrot::graine());
    let mut c = cfg::load(&mut g);
    g.new_game();
    if std::env::args().any(|a| a == "-t") {
        #[cfg(windows)]
        win::console();
        term::run(g, |g| cfg::save(g, &mut c));
        return;
    }
    let net = Net::new(c.ip.clone());
    let fullscreen = c.fullscreen;
    shell::run(App { fonts: Fonts::load(), g, c, net, fullscreen, quit: false, sig: 0 });
}

struct App {
    fonts: Fonts,
    g: Game,
    c: cfg::Cfg,
    net: Net,
    fullscreen: bool,
    quit: bool,
    /// empreinte de ce qui est affiché : on ne redessine que si elle change
    sig: u64,
}

impl App {
    fn save(&mut self) {
        self.g.save_needed = false;
        cfg::save(&self.g, &mut self.c);
    }

    fn signature(&self) -> u64 {
        let g = &self.g;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (&g.line1, &g.line2, g.phase, g.pot, g.tok, g.round, g.level, g.wins, g.losses, g.cur).hash(&mut h);
        (format!("{:?}", g.mode), g.can_act()).hash(&mut h);
        for d in &g.dice {
            (d.val, d.held, d.p.map(|v| (v * 1000.0) as i64)).hash(&mut h);
        }
        self.net.panel().map(|(a, l, m)| (a.to_string(), l.map(|x| x.to_string()), m)).hash(&mut h);
        h.finish()
    }
}

impl shell::Game for App {
    fn title(&self) -> String {
        "421 - jeu du bistro en 3D isométrique".into()
    }

    fn shot_name(&self) -> &str {
        "421_shot.png"
    }

    fn background(&mut self, gfx: &Gfx) {
        render::background(gfx, &self.fonts);
    }

    fn draw(&mut self, gfx: &Gfx) {
        render::draw(&self.g, gfx, &self.fonts);
        render::net_panel(gfx, &self.fonts, self.net.panel());
    }

    fn tick(&mut self, dt: f64) -> bool {
        self.g.tick(dt);
        self.net.poll(&mut self.g);
        if self.g.save_needed {
            self.save();
        }
        let sig = self.signature();
        let changed = sig != self.sig;
        self.sig = sig;
        changed
    }

    fn animating(&self) -> bool {
        self.g.rolling && bistrot::anim::actives()
    }

    fn key(&mut self, k: Key) {
        let g = &mut self.g;
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
                Key::Char(c) => net.menu_key(g, c),
                Key::Esc => net.menu_key(g, '\u{1b}'),
                _ => {}
            },
            Menu::None => match k {
                Key::Esc => {
                    if net.waiting() {
                        net.cancel(g);
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
                Key::Space => g.ui_main(),
                Key::Enter => {
                    if net.hosting() {
                        net.host_start(g);
                    } else {
                        g.ui_second();
                    }
                }
                Key::Char(c) if !net.waiting() => match c.to_ascii_uppercase() {
                    'N' => net.open(g),
                    'J' => g.toggle_duo(),
                    'L' => g.cycle_level(),
                    'H' => g.help(),
                    'P' => g.cycle_pot(),
                    'M' => {
                        bistrot::anim::regler(bistrot::anim::reglage().suivant());
                        g.save_needed = true;
                    }
                    _ => {}
                },
                _ => {}
            },
        }
        if self.g.save_needed {
            self.save();
        }
    }

    fn click(&mut self, x: f32, y: f32) {
        if self.net.menu != Menu::None || self.net.waiting() || !self.g.accepts_input() {
            return;
        }
        match render::hit(&self.g, x as f64, y as f64) {
            Hit::Main => self.g.ui_main(),
            Hit::Second => self.g.ui_second(),
            Hit::Die(i) => self.g.ui_toggle(i),
            Hit::Nothing => {}
        }
    }

    fn hover(&mut self, _x: f32, _y: f32) -> bool {
        false
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
    use game::Phase;
    use std::time::Duration;

    /// Hôte et invité sur 127.0.0.1 : vraie connexion TCP, les deux parties restent identiques.
    #[test]
    fn duel_local_tcp() {
        let mut ga = Game::new(7);
        let mut gb = Game::new(99);
        let mut a = Net::new(String::new());
        let mut b = Net::new("127.0.0.1".into());
        a.open(&mut ga);
        a.menu_key(&mut ga, 'h');
        assert!(a.hosting(), "{:?}", a.lines);
        b.open(&mut gb);
        b.menu_key(&mut gb, 'r');
        b.ip_char('\r');
        let mut step = 0u32;
        for _ in 0..4000 {
            a.poll(&mut ga);
            b.poll(&mut gb);
            ga.tick(120.0);
            gb.tick(45.0);
            for g in [&mut ga, &mut gb] {
                if g.online() && g.idle() && g.inbox.is_empty() && g.can_act() {
                    step += 1;
                    match g.phase {
                        Phase::PreRoll | Phase::Result | Phase::Over => g.ui_main(),
                        Phase::Choose if step % 2 == 0 => g.ui_second(),
                        Phase::Choose => {
                            g.ui_toggle((step % 3) as usize);
                            g.ui_main();
                        }
                        _ => {}
                    }
                }
            }
            if ga.round >= 8 && gb.round == ga.round && ga.idle() && gb.idle() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(ga.online() && gb.online(), "{:?} / {:?}", a.lines, b.lines);
        assert!(ga.round >= 8, "la partie n'a pas avancé : {}", ga.round);
        assert_eq!((ga.pot, ga.tok, ga.rng), (gb.pot, gb.tok, gb.rng));
        assert_eq!((ga.hand(0), ga.hand(1)), (gb.hand(0), gb.hand(1)));
        // l'invité part : l'hôte revient en partie locale
        b.close(&mut gb, "fin");
        for _ in 0..200 {
            a.poll(&mut ga);
            if !ga.online() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!ga.online());
    }
}
