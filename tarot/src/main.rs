//! Tarot en 3D isométrique (règles de la FFT) : 3, 4 ou 5 joueurs contre des bots, ou en ligne avec des amis.
//! Construit sur le moteur commun `bistrot` (fenêtre, dessin processeur, cartes, taverne, réseau).
//! `tarot -sim ...` = banc d'essai sans fenêtre.

#![windows_subsystem = "windows"]

mod bots;
mod cfg;
mod draw;
mod engine;
mod sim;
mod table;
mod vue;

use bistrot::anim;
use bistrot::cartes::Paquet;
use bistrot::gfx::Gfx;
use bistrot::polices::{Fichiers, Polices};
use bistrot::reseau::{Menu, Net};
use bistrot::shell::{self, Key};
use bistrot::table::Decor;
use draw::{BtnId, Overlay};
use engine::*;
use std::hash::{Hash, Hasher};
use table::Table;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "-sim") {
        sim::sim_main(&args[i + 1..]);
        return;
    }
    let mut t = Table::new(bistrot::graine());
    let c = cfg::load(&mut t);
    t.new_match();
    let net = Net::new(c.ip.clone());
    let fullscreen = c.fullscreen;
    let fichiers = Fichiers::charger();
    shell::run(Tarot {
        polices: Polices::depuis(&fichiers),
        paquet: Paquet::new(&fichiers),
        t,
        c,
        net,
        fullscreen,
        quit: false,
        gallery: false,
        memo: false,
        hover: None,
        mouse: (0.0, 0.0),
        sig: 0,
        vue: vue::Vue::default(),
        decor: Decor::default(),
    });
}

struct Tarot {
    polices: Polices,
    /// cartes dessinées à l'échelle de la fenêtre
    paquet: Paquet,
    t: Table,
    c: cfg::Cfg,
    net: Net,
    fullscreen: bool,
    quit: bool,
    gallery: bool,
    /// aide-mémoire (touche F)
    memo: bool,
    hover: Option<usize>,
    mouse: (f32, f32),
    /// empreinte de ce qui est affiché : on ne redessine que si elle change
    sig: u64,
    /// ce qui bouge à l'écran (distribution, chien, main, bulles, scores)
    vue: vue::Vue,
    /// décor de la taverne en cache
    decor: Decor,
}

fn btn_action(t: &mut Table, id: BtnId) {
    match id {
        BtnId::Contrat(c) => t.human_bid(c),
        BtnId::Appel(c) => t.human_appel(c),
        BtnId::Ecart => {
            let e = t.selection;
            t.human_ecart(e);
        }
        BtnId::Poignee => t.poignee = !t.poignee,
        BtnId::Next => {
            if matches!(t.phase(), PH_FINDONNE | PH_FINPARTIE) {
                t.human_next();
            }
        }
    }
}

impl Tarot {
    fn save(&mut self) {
        self.t.save_needed = false;
        cfg::save(&self.t, &mut self.c);
    }

    /// Empreinte de l'état affiché.
    fn signature(&self) -> u64 {
        let t = &self.t;
        let d = &t.m.d;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (d.phase, d.tour, d.mains, d.pli, d.n_pli, d.preneur, d.contrat, d.encheres, d.appel, d.revele).hash(&mut h);
        (t.m.scores, t.m.donne, t.m.donnes, t.m.np, t.hint, t.nextmask, t.wins, t.help, t.botlvl, t.selection, t.poignee).hash(&mut h);
        for p in 0..5 {
            t.bub[p].as_ref().map(|(s, end)| (s.clone(), *end > t.now)).hash(&mut h);
        }
        (t.message(), self.net.panel().map(|(a, l, m)| (a.to_string(), l.map(|x| x.to_string()), m))).hash(&mut h);
        (self.gallery, self.memo, self.hover, draw::buttons(t).iter().position(|b| b.touche(self.mouse.0, self.mouse.1))).hash(&mut h);
        (vue::Vue::chien_au_centre(t), t.my_turn(), anim::libelle()).hash(&mut h);
        h.finish()
    }

    fn hand_index(&self, x: f32, y: f32) -> Option<usize> {
        let main = self.vue.main_visible(&self.t);
        draw::hand_hit(&self.t, &self.vue, x, y).and_then(|c| main.iter().position(|&m| m == c))
    }
}

impl shell::Game for Tarot {
    fn title(&self) -> String {
        "Tarot - jeu du bistro en 3D isométrique".into()
    }

    fn shot_name(&self) -> &str {
        "tarot_shot.png"
    }

    /// Le fond (taverne + panneaux, étiquettes, mains adverses) n'est refait que si cette empreinte change.
    fn fond_signature(&self) -> u64 {
        draw::under_signature(&self.t, &self.vue) ^ self.gallery as u64
    }

    fn background(&mut self, g: &Gfx) {
        self.paquet.echelle((g.sc * 1.04).max(0.5));
        self.decor.dessiner(g);
        if !self.gallery {
            draw::draw_under(&self.t, g, &self.polices, &self.paquet, &self.vue);
        }
    }

    fn alleger(&mut self) {
        self.paquet.alleger();
        self.decor.alleger();
    }

    fn draw(&mut self, g: &Gfx) {
        if self.gallery {
            draw::gallery(g, &self.polices, &self.paquet);
            return;
        }
        let ov = Overlay { memo: self.memo, mouse: self.mouse, net: self.net.panel(), optline: draw::optline(&self.t) };
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
        if !anim::actives() {
            return false;
        }
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
                    } else if self.memo {
                        self.memo = false;
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
                    } else if t.phase() == PH_ECART && t.my_turn() {
                        let e = t.selection;
                        t.human_ecart(e);
                    } else {
                        btn_action(t, BtnId::Next);
                    }
                }
                Key::Char(c) if !net.waiting() => match c.to_ascii_uppercase() {
                    'J' => t.cycle_np(),
                    'L' => t.cycle_level(),
                    'H' => t.toggle_help(),
                    'P' => t.cycle_donnes(),
                    'U' => t.auto = !t.auto,
                    'G' => self.gallery = !self.gallery,
                    'F' => self.memo = !self.memo,
                    'M' => {
                        anim::regler(anim::reglage().suivant());
                        t.save_needed = true;
                    }
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
        if self.memo {
            self.memo = false; // un clic ferme l'aide-mémoire (sans jouer)
            return;
        }
        if self.net.menu != Menu::None || self.net.waiting() || self.gallery {
            return;
        }
        let carte = draw::hand_hit(&self.t, &self.vue, x, y);
        let t = &mut self.t;
        if let Some(b) = draw::buttons(t).into_iter().find(|b| b.touche(x, y)) {
            btn_action(t, b.id);
        } else if let Some(c) = carte.filter(|_| t.my_turn()) {
            match t.phase() {
                PH_ECART => t.toggle_selection(c),
                PH_JEU => t.human_play(c),
                _ => {}
            }
        }
        self.hover = self.hand_index(x, y);
    }

    fn hover(&mut self, x: f32, y: f32) -> bool {
        self.mouse = (x, y);
        self.hover = self.hand_index(x, y);
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

    struct Apercu {
        polices: Polices,
        paquet: Paquet,
        glyphs: RefCell<bistrot::gfx::Glyphs>,
        decor: Decor,
    }

    impl Apercu {
        /// Dessine la scène sans fenêtre dans target/apercu_tarot_<nom>.png.
        fn dessiner(&mut self, t: &Table, vue: &vue::Vue, mouse: (f32, f32), nom: &str) {
            let mut pm = tiny_skia::Pixmap::new(1100, 800).unwrap();
            let g = Gfx::fit(&mut pm, &self.glyphs);
            self.paquet.echelle(g.sc * 1.04);
            self.decor.dessiner(&g);
            draw::draw_under(t, &g, &self.polices, &self.paquet, vue);
            let ov = Overlay { memo: false, mouse, net: None, optline: draw::optline(t) };
            draw::draw_over(t, &g, &self.polices, &self.paquet, &ov, vue);
            drop(g);
            pm.save_png(format!("../target/apercu_tarot_{nom}.png")).unwrap();
        }
    }

    /// Avance la partie (et l'écran) jusqu'à ce que `fin` soit vrai.
    fn avancer(t: &mut Table, vue: &mut vue::Vue, fin: impl Fn(&Table, &vue::Vue) -> bool) -> bool {
        for _ in 0..20_000 {
            if fin(t, vue) {
                return true;
            }
            t.tick(20.0);
            vue.maj(t, None, 20.0);
        }
        false
    }

    /// Aperçus des écrans du tarot (cargo test --release -p tarot -- --ignored apercus) : target/apercu_tarot_*.png.
    #[test]
    #[ignore]
    fn apercus() {
        let fichiers = Fichiers::charger();
        let mut a = Apercu {
            polices: Polices::depuis(&fichiers),
            paquet: Paquet::new(&fichiers),
            glyphs: RefCell::new(bistrot::gfx::Glyphs::default()),
            decor: Decor::default(),
        };
        // 4 joueurs : enchères, chien, écart, jeu, bilan, classement
        let mut t = Table::new(77);
        t.m.donnes = 1;
        t.new_match();
        let mut vue = vue::Vue::default();
        assert!(avancer(&mut t, &mut vue, |t, v| t.phase() == PH_ENCHERES && t.my_turn() && !v.anime()));
        a.dessiner(&t, &vue, (0.0, 0.0), "encheres");
        let c = t.m.enchere_max() + 1;
        assert!(c <= 2, "graine à changer : il faut pouvoir prendre une petite ou une garde");
        t.human_bid(2);
        assert!(avancer(&mut t, &mut vue, |t, _| t.phase() == PH_ECART && t.now > t.chien_vu + 900.0));
        a.dessiner(&t, &vue, (0.0, 0.0), "chien");
        assert!(avancer(&mut t, &mut vue, |t, v| t.my_turn() && !v.anime()));
        t.selection = t.m.bot_ecart(t.me);
        avancer(&mut t, &mut vue, |_, v| !v.anime());
        a.dessiner(&t, &vue, (550.0, 543.0), "ecart");
        let e = t.selection;
        t.human_ecart(e);
        // quelques plis joués par l'ordinateur, puis une main où il faut suivre
        t.auto = true;
        assert!(avancer(&mut t, &mut vue, |t, _| t.m.d.plis >= 3));
        t.auto = false;
        for _ in 0..20 {
            assert!(avancer(&mut t, &mut vue, |t, v| t.my_turn() && !v.anime()));
            if t.m.d.n_pli >= 2 {
                break;
            }
            let leg = t.m.legales(t.me);
            let c = t.m.heuristique(t.me, leg);
            t.human_play(c);
        }
        a.dessiner(&t, &vue, (0.0, 0.0), "jeu");
        t.auto = true;
        assert!(avancer(&mut t, &mut vue, |t, _| t.phase() == PH_FINPARTIE));
        t.auto = false;
        assert!(avancer(&mut t, &mut vue, |_, v| !v.anime()));
        a.dessiner(&t, &vue, (550.0, 592.0), "classement");
        // 5 joueurs : appel du roi
        let mut t = Table::new(5);
        t.m.np = 5;
        t.new_match();
        let mut vue = vue::Vue::default();
        assert!(avancer(&mut t, &mut vue, |t, v| t.phase() == PH_ENCHERES && t.my_turn() && !v.anime()));
        t.human_bid(4);
        assert!(avancer(&mut t, &mut vue, |t, v| t.phase() == PH_APPEL && t.my_turn() && !v.anime()));
        a.dessiner(&t, &vue, (0.0, 0.0), "appel");
        // poignée : on donne exprès 12 atouts au joueur local, puis on va jusqu'à sa première carte
        let mut t = Table::new(41);
        let mut vue = vue::Vue::default();
        {
            let d = &mut t.m.d;
            let mut atouts = liste(ATOUTS & !d.mains[0] & !bit(PETIT));
            let mut autres = liste(d.mains[0] & !ATOUTS);
            while nb(d.mains[0] & ATOUTS) < 12 {
                let (a, c) = (atouts.pop().unwrap(), autres.pop().unwrap());
                let q = (1..t.m.np).find(|&q| d.mains[q] & bit(a) != 0);
                match q {
                    Some(q) => {
                        d.mains[q] = d.mains[q] & !bit(a) | bit(c);
                    }
                    None => {
                        d.chien = d.chien & !bit(a) | bit(c);
                    }
                }
                d.mains[0] = d.mains[0] & !bit(c) | bit(a);
            }
        }
        t.auto = true;
        assert!(avancer(&mut t, &mut vue, |t, _| t.phase() == PH_JEU && t.turn() == t.me));
        t.auto = false;
        assert!(avancer(&mut t, &mut vue, |t, v| t.my_turn() && !v.anime()));
        t.poignee = true;
        a.dessiner(&t, &vue, (970.0, 543.0), "poignee");
        // galerie des 78 cartes
        let mut pm = tiny_skia::Pixmap::new(1100, 800).unwrap();
        {
            let g = Gfx::fit(&mut pm, &a.glyphs);
            a.paquet.echelle(g.sc * 1.04);
            a.decor.dessiner(&g);
            draw::gallery(&g, &a.polices, &a.paquet);
        }
        pm.save_png("../target/apercu_tarot_galerie.png").unwrap();
        // 3 joueurs : bilan d'une donne
        let mut t = Table::new(3);
        t.m.np = 3;
        t.auto = true;
        t.new_match();
        let mut vue = vue::Vue::default();
        assert!(avancer(&mut t, &mut vue, |t, _| t.phase() == PH_FINDONNE));
        t.auto = false;
        assert!(avancer(&mut t, &mut vue, |_, v| !v.anime()));
        a.dessiner(&t, &vue, (0.0, 0.0), "bilan");
    }
}
