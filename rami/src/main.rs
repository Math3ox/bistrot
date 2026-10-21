//! Rami 51 en 3D isométrique : 2 à 5 joueurs contre des bots, ou en ligne avec des amis.
//! Construit sur le moteur commun `bistrot` (fenêtre, dessin processeur, cartes, taverne, réseau).
//! `rami -sim ...` = banc d'essai sans fenêtre.

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
use draw::{BtnId, Cible, Overlay};
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
    shell::run(Rami {
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

struct Rami {
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
    /// carte de la main sous la souris
    hover: Option<u8>,
    mouse: (f32, f32),
    /// empreinte de ce qui est affiché : on ne redessine que si elle change
    sig: u64,
    /// ce qui bouge à l'écran (distribution, cartes qui glissent, bulles, scores)
    vue: vue::Vue,
    /// décor de la taverne en cache
    decor: Decor,
}

fn btn_action(t: &mut Table, id: BtnId) {
    match id {
        BtnId::Poser => t.human_poser(),
        BtnId::Defausser => t.human_defausser(),
        BtnId::Annuler => t.human_annuler(),
        BtnId::Tri => t.toggle_tri(),
        BtnId::Next => t.human_next(),
    }
}

impl Rami {
    fn save(&mut self) {
        self.t.save_needed = false;
        cfg::save(&self.t, &mut self.c);
    }

    /// Empreinte de l'état affiché.
    fn signature(&self) -> u64 {
        let t = &self.t;
        let d = &t.m.d;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (d.phase, d.tour, d.mains, d.talon.len(), &d.defausse, &d.table, d.ouvert, d.pose_tour, d.recyclages).hash(&mut h);
        (t.m.scores, t.m.manche, t.m.np, t.m.limite, t.nextmask, t.wins, t.help, t.botlvl, t.selection, t.hint, t.hint_pioche, t.tri_valeur).hash(&mut h);
        for p in 0..5 {
            t.bub[p].as_ref().map(|(s, end)| (s.clone(), *end > t.now)).hash(&mut h);
        }
        (t.message(), self.net.panel().map(|(a, l, m)| (a.to_string(), l.map(|x| x.to_string()), m))).hash(&mut h);
        (self.gallery, self.memo, self.hover, draw::buttons(t).iter().position(|b| b.touche(self.mouse.0, self.mouse.1))).hash(&mut h);
        (draw::table_hit(t, &self.vue, self.mouse.0, self.mouse.1), t.my_turn(), anim::libelle()).hash(&mut h);
        h.finish()
    }
}

impl shell::Game for Rami {
    fn title(&self) -> String {
        "Rami - jeu du bistro en 3D isométrique".into()
    }

    fn shot_name(&self) -> &str {
        "rami_shot.png"
    }

    /// Le fond (taverne + panneaux, étiquettes, mains adverses, talon) n'est refait que si cette empreinte change.
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
        anim::actives() && self.vue.anime()
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
                    } else if t.selection != 0 {
                        t.selection = 0;
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
                    } else if matches!(t.phase(), PH_FINMANCHE | PH_FINPARTIE) {
                        t.human_next();
                    } else if k == Key::Enter && t.my_turn() && t.phase() == PH_JEU {
                        t.human_poser();
                    }
                }
                Key::Back => t.human_annuler(),
                Key::Char(c) if !net.waiting() => match c.to_ascii_uppercase() {
                    'J' => t.cycle_np(),
                    'L' => t.cycle_level(),
                    'H' => t.toggle_help(),
                    'P' => t.cycle_limite(),
                    'T' => t.toggle_tri(),
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
        let carte = draw::hand_hit(&self.t, x, y);
        let cible = draw::table_hit(&self.t, &self.vue, x, y);
        let t = &mut self.t;
        if let Some(b) = draw::buttons(t).into_iter().find(|b| b.touche(x, y)) {
            btn_action(t, b.id);
        } else if matches!(t.phase(), PH_FINMANCHE | PH_FINPARTIE) {
            // le bilan couvre la table
        } else if let Some(c) = carte {
            t.toggle_selection(c);
        } else {
            match cible {
                Some(Cible::Talon) => t.human_pioche(true),
                Some(Cible::Defausse) if t.phase() == PH_PIOCHE => t.human_pioche(false),
                Some(Cible::Defausse) => t.human_defausser(),
                Some(Cible::Combi(k, i)) => {
                    let joker = t.m.d.table.get(k).and_then(|c| c.cartes.get(i)).is_some_and(|&c| est_joker(c));
                    t.human_table(k, joker);
                }
                None => {}
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

    struct Apercu {
        polices: Polices,
        paquet: Paquet,
        glyphs: RefCell<bistrot::gfx::Glyphs>,
        decor: Decor,
    }

    impl Apercu {
        /// Dessine la scène sans fenêtre dans target/apercu_rami_<nom>.png.
        fn dessiner(&mut self, t: &Table, vue: &vue::Vue, mouse: (f32, f32), nom: &str) {
            let mut pm = tiny_skia::Pixmap::new(1100, 800).unwrap();
            let g = Gfx::fit(&mut pm, &self.glyphs);
            self.paquet.echelle(g.sc * 1.04);
            self.decor.dessiner(&g);
            draw::draw_under(t, &g, &self.polices, &self.paquet, vue);
            let ov = Overlay { memo: false, mouse, net: None, optline: draw::optline(t) };
            draw::draw_over(t, &g, &self.polices, &self.paquet, &ov, vue);
            drop(g);
            pm.save_png(format!("../target/apercu_rami_{nom}.png")).unwrap();
        }
    }

    /// Avance la partie (et l'écran) jusqu'à ce que `fin` soit vrai.
    fn avancer(t: &mut Table, vue: &mut vue::Vue, fin: impl Fn(&Table, &vue::Vue) -> bool) -> bool {
        for _ in 0..40_000 {
            if fin(t, vue) {
                return true;
            }
            t.tick(20.0);
            vue.maj(t, None, 20.0);
        }
        false
    }

    /// Joue le tour du joueur local comme le ferait un bot normal (par les actions du joueur).
    fn jouer_comme_bot(t: &mut Table) {
        let talon = t.m.bot_pioche(t.me);
        t.human_pioche(talon);
        for a in t.m.bot_jeu(t.me) {
            match a {
                bots::Action::Poser(m) => {
                    t.selection = m;
                    t.human_poser();
                }
                bots::Action::Ajouter(m, i) => {
                    t.selection = m;
                    t.human_table(i, false);
                }
                bots::Action::Echanger(x, i) => {
                    t.selection = bit(x);
                    t.human_table(i, true);
                }
                bots::Action::Defausser(c) => {
                    t.selection = bit(c);
                    t.human_defausser();
                }
                bots::Action::Pioche(_) => {}
            }
        }
    }

    /// Un clic sur la partie visible de chaque carte de la table et de la main tombe bien sur elle.
    #[test]
    fn clics() {
        for np in [2usize, 5] {
            let mut t = Table::new(9 + np as u64);
            t.m.np = np;
            t.auto = true;
            t.new_match();
            let mut vue = vue::Vue::default();
            avancer(&mut t, &mut vue, |t, _| t.m.d.table.len() >= 9 || t.phase() >= PH_FINMANCHE);
            t.auto = false;
            avancer(&mut t, &mut vue, |_, v| !v.anime());
            assert!(!t.m.d.table.is_empty());
            for (k, cb) in t.m.d.table.iter().enumerate() {
                let r = vue.rangees[k];
                let n = cb.cartes.len();
                for i in 0..n {
                    let (x, y) = r.carte(i);
                    let x = x - 50.0 * r.echelle + if i + 1 == n { 50.0 * r.echelle } else { r.pas / 2.0 };
                    assert_eq!(draw::table_hit(&t, &vue, x as f32, y as f32), Some(Cible::Combi(k, i)), "combinaison {k}, carte {i}");
                }
            }
            assert_eq!(draw::table_hit(&t, &vue, vue::TALON.0 as f32, vue::TALON.1 as f32), Some(Cible::Talon));
            assert_eq!(draw::table_hit(&t, &vue, vue::DEFAUSSE.0 as f32, vue::DEFAUSSE.1 as f32), Some(Cible::Defausse));
            let main = t.ma_main();
            for (i, &c) in main.iter().enumerate() {
                let x = table::main_x(i, main.len()) - 20.0;
                assert_eq!(draw::hand_hit(&t, x as f32, 700.0), Some(c));
            }
        }
    }

    /// Temps de dessin d'une image (cargo test --release -p rami -- --ignored mesure_dessin --nocapture).
    #[test]
    #[ignore]
    fn mesure_dessin() {
        use std::time::Instant;
        let fichiers = Fichiers::charger();
        let polices = Polices::depuis(&fichiers);
        let mut paquet = Paquet::new(&fichiers);
        let glyphs = RefCell::new(bistrot::gfx::Glyphs::default());
        let mut t = Table::new(9);
        t.m.np = 5;
        t.auto = true;
        t.new_match();
        let mut vue = vue::Vue::default();
        avancer(&mut t, &mut vue, |t, _| t.m.d.table.iter().map(|c| c.cartes.len()).sum::<usize>() >= 40 || t.phase() >= PH_FINMANCHE);
        t.auto = false;
        avancer(&mut t, &mut vue, |_, v| !v.anime());
        println!("{} cartes sur la table", t.m.d.table.iter().map(|c| c.cartes.len()).sum::<usize>());
        for (w, h) in [(1100u32, 800u32), (1920, 1080)] {
            let mut bg = tiny_skia::Pixmap::new(w, h).unwrap();
            let mut decor = Decor::default();
            {
                let g = Gfx::fit(&mut bg, &glyphs);
                paquet.echelle(g.sc * 1.04);
                decor.dessiner(&g);
                draw::draw_under(&t, &g, &polices, &paquet, &vue);
            }
            let mut frame = bg.clone();
            let ov = Overlay { memo: false, mouse: (0.0, 0.0), net: None, optline: draw::optline(&t) };
            // première image : les cartes sont dessinées et mises en cache
            draw::draw_over(&t, &Gfx::fit(&mut frame, &glyphs), &polices, &paquet, &ov, &vue);
            let t0 = Instant::now();
            for _ in 0..20 {
                frame.data_mut().copy_from_slice(bg.data());
                draw::draw_over(&t, &Gfx::fit(&mut frame, &glyphs), &polices, &paquet, &ov, &vue);
            }
            println!("image {w}x{h} : {:.2} ms", t0.elapsed().as_secs_f64() * 50.0);
            let t0 = Instant::now();
            for _ in 0..20 {
                let g = Gfx::fit(&mut frame, &glyphs);
                decor.dessiner(&g);
                draw::draw_under(&t, &g, &polices, &paquet, &vue);
            }
            println!("fond {w}x{h} : {:.2} ms", t0.elapsed().as_secs_f64() * 50.0);
        }
        let t0 = Instant::now();
        for _ in 0..100 {
            vue.maj(&t, None, 16.0);
        }
        println!("vue.maj : {:.3} ms", t0.elapsed().as_secs_f64() * 10.0);
    }

    /// Aperçus des écrans du rami (cargo test --release -p rami -- --ignored apercus) : target/apercu_rami_*.png.
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
        // 4 joueurs : pendant la distribution, au début, puis en cours de manche avec une sélection
        let mut t = Table::new(31);
        t.help = true;
        t.new_match();
        let mut vue = vue::Vue::default();
        avancer(&mut t, &mut vue, |t, _| t.now > 10_700.0);
        a.dessiner(&t, &vue, (0.0, 0.0), "distribution");
        assert!(avancer(&mut t, &mut vue, |t, v| t.my_turn() && !v.anime()));
        a.dessiner(&t, &vue, (0.0, 0.0), "debut");
        for _ in 0..8 {
            assert!(avancer(&mut t, &mut vue, |t, v| (t.my_turn() && !v.anime()) || t.phase() >= PH_FINMANCHE));
            if t.phase() >= PH_FINMANCHE || t.m.d.table.len() >= 5 {
                break;
            }
            jouer_comme_bot(&mut t);
        }
        assert!(avancer(&mut t, &mut vue, |t, v| (t.my_turn() && !v.anime()) || t.phase() >= PH_FINMANCHE));
        if t.phase() == PH_PIOCHE {
            t.human_pioche(true);
        }
        let main = t.ma_main();
        t.selection = bit(main[0]) | bit(main[1]);
        t.tick(20.0); // le conseil se calcule au tic suivant
        vue.maj(&t, None, 20.0);
        avancer(&mut t, &mut vue, |_, v| !v.anime());
        let souris = vue.rangees.first().map_or((0.0, 0.0), |r| (r.x0 as f32, r.y as f32));
        a.dessiner(&t, &vue, souris, "jeu");
        // fin de manche et classement
        t.selection = 0;
        t.auto = true;
        assert!(avancer(&mut t, &mut vue, |t, _| t.phase() >= PH_FINMANCHE));
        t.auto = false;
        assert!(avancer(&mut t, &mut vue, |_, v| !v.anime()));
        a.dessiner(&t, &vue, (550.0, 592.0), "bilan");
        t.auto = true;
        assert!(avancer(&mut t, &mut vue, |t, _| t.phase() == PH_FINPARTIE));
        t.auto = false;
        assert!(avancer(&mut t, &mut vue, |_, v| !v.anime()));
        a.dessiner(&t, &vue, (0.0, 0.0), "classement");
        // 5 joueurs : table bien remplie
        let mut t = Table::new(9);
        t.m.np = 5;
        t.auto = true;
        t.new_match();
        let mut vue = vue::Vue::default();
        avancer(&mut t, &mut vue, |t, _| {
            let n: usize = t.m.d.table.iter().map(|c| c.cartes.len()).sum();
            n >= 40 || t.phase() >= PH_FINMANCHE
        });
        t.auto = false;
        avancer(&mut t, &mut vue, |_, v| !v.anime());
        a.dessiner(&t, &vue, (0.0, 0.0), "cinq");
        // 2 joueurs
        let mut t = Table::new(5);
        t.m.np = 2;
        t.new_match();
        let mut vue = vue::Vue::default();
        assert!(avancer(&mut t, &mut vue, |t, v| t.my_turn() && !v.anime()));
        a.dessiner(&t, &vue, (0.0, 0.0), "deux");
        // galerie
        let mut pm = tiny_skia::Pixmap::new(1100, 800).unwrap();
        {
            let g = Gfx::fit(&mut pm, &a.glyphs);
            a.paquet.echelle(g.sc * 1.04);
            a.decor.dessiner(&g);
            draw::gallery(&g, &a.polices, &a.paquet);
        }
        pm.save_png("../target/apercu_rami_galerie.png").unwrap();
    }
}
