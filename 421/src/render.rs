//! Dessin de la table en projection isométrique : tapis, jetons, boutons 3D, dés (faces éclairées,
//! points, ombres portées), textes. Le jeu est dessiné dans un espace virtuel de 1100 x 800,
//! mis à l'échelle de la fenêtre (proportions gardées, bandes noires si besoin).

use crate::ai;
use crate::game::{mv3, Die, Game, Phase, DIE_H, FACES, FELT};
use bistrot::gfx::{rgb, Align, Color, Face, Gfx, Xf, WH, WW};
use bistrot::iso::{self, Iso, Pt, YSQ};
use bistrot::polices::{Fichiers, Polices};

/// Projection de la table : 64 pixels logiques par unité du monde, centre en (550, 450).
const ISO: Iso = Iso { s: 64.0, cx: 550.0, cy: 450.0 };
const S: f64 = 64.0;

const BTN_A: f64 = 4.9;
const BTN_B: [f64; 2] = [-1.0, 1.0];
const BTN_TZ: f64 = 0.41;
const BOT_Z: f64 = -4.3;
const PL_Z: f64 = 4.8;
const POT_A: f64 = -4.6;

const C_BG: Color = rgb(18, 22, 29);
const C_FLOORSH: Color = rgb(10, 13, 18);
const C_WOOD: Color = rgb(128, 84, 44);
const C_WOODSIDE: Color = rgb(78, 48, 24);
const C_WOODLINE: Color = rgb(52, 32, 16);
const C_EDGE: Color = rgb(150, 142, 120);
const C_PIP: Color = rgb(26, 26, 32);
const C_PIPRED: Color = rgb(196, 30, 30);
const C_ZONE: Color = rgb(40, 160, 104);
const C_GOLD: Color = rgb(255, 205, 70);
const C_POTTOP: Color = rgb(255, 214, 70);
const C_POTSIDE: Color = rgb(176, 132, 20);
const C_PLTOP: Color = rgb(96, 200, 255);
const C_PLSIDE: Color = rgb(30, 104, 160);
const C_BOTTOP: Color = rgb(255, 110, 130);
const C_BOTSIDE: Color = rgb(150, 40, 62);
const C_WHITE: Color = rgb(240, 240, 235);
const C_TXTGND: Color = rgb(190, 235, 205);
const C_HELDDISC: Color = rgb(176, 140, 36);
const C_HINT: Color = rgb(128, 138, 150);
const C_BTNG: (Color, Color) = (rgb(255, 196, 64), rgb(176, 118, 20));
const C_BTNB: (Color, Color) = (rgb(96, 200, 255), rgb(30, 104, 160));
const C_BTNX: (Color, Color) = (rgb(92, 106, 100), rgb(58, 68, 64));
const C_BTNTXT: Color = rgb(28, 24, 16);
const C_BTNTXTX: Color = rgb(150, 162, 156);

const PIPMASK: [u16; 6] = [0x010, 0x101, 0x111, 0x145, 0x155, 0x16D];
const PIPX: [f64; 9] = [-0.5, 0.0, 0.5, -0.5, 0.0, 0.5, -0.5, 0.0, 0.5];
const PIPY: [f64; 9] = [-0.5, -0.5, -0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5];

pub const HINT: &str = "J : 1/2 joueurs  -  L : bot  -  H : aide  -  P : pot  -  N : en ligne  -  F11 : plein écran";

pub struct Fonts {
    fichiers: Fichiers,
    /// polices de l'interface commune (panneau du jeu en ligne)
    pub ui: Polices,
}

impl Fonts {
    pub fn load() -> Fonts {
        let fichiers = Fichiers::charger();
        let ui = Polices::depuis(&fichiers);
        Fonts { fichiers, ui }
    }
}

// ---------- projections ----------

/// Point monde (x, y, z) -> écran (coordonnées du jeu).
fn proj(p: [f64; 3]) -> Pt {
    ISO.proj(p)
}

/// Point du plateau (a vers la droite, b vers le joueur, hauteur z) -> écran.
fn ground(a: f64, b: f64, z: f64) -> Pt {
    ISO.sol(a, b, z)
}

// ---------- primitives ----------

struct Pen<'a, 'b> {
    g: &'a Gfx<'b>,
    fonts: &'a Fonts,
}

impl Pen<'_, '_> {
    fn fill(&self, pts: &[Pt], c: Color) {
        iso::remplir(self.g, pts, c);
    }

    fn outline(&self, pts: &[Pt], c: Color, w: f32) {
        iso::contour(self.g, pts, c, w);
    }

    /// Disque posé sur le plateau (a, b, z), rayon en unités monde.
    fn ground_ellipse(&self, a: f64, b: f64, z: f64, r: f64) -> Vec<Pt> {
        ISO.disque(a, b, z, r)
    }

    /// Cylindre debout (jeton, bouton, table) : base, flanc, dessus.
    fn cylinder(&self, a: f64, b: f64, z0: f64, r: f64, h: f64, top: Color, side: Color) {
        ISO.cylindre(self.g, a, b, z0, r, h, top, side);
    }

    fn face(&self, size: f32, bold: bool) -> Face {
        if bold {
            self.fonts.fichiers.gras.face(size)
        } else {
            self.fonts.fichiers.demi.face(size)
        }
    }

    /// Texte à plat. y = haut du texte ; align 0 gauche, 0.5 centré, 1 droite.
    fn text(&self, s: &str, x: f64, y: f64, size: f32, bold: bool, c: Color, align: f32, _squash: f32) {
        let al = if align < 0.25 {
            Align::Left
        } else if align < 0.75 {
            Align::Center
        } else {
            Align::Right
        };
        self.g.text(s, x as f32, y as f32, &self.face(size, bold), c, al);
    }

    /// Texte couché sur le plateau, centré en (a, b) à la hauteur z.
    fn ground_text(&self, s: &str, a: f64, b: f64, z: f64, size: f32, c: Color) {
        ISO.texte_sol(self.g, s, a, b, z, &self.face(size, true), c);
    }
}

fn hull(p: Vec<Pt>) -> Vec<Pt> {
    iso::enveloppe(p)
}

// ---------- dés ----------

/// Repère de la face f dans le monde : (n, a, b).
fn face_frame(d: &Die, f: usize) -> [[f64; 3]; 3] {
    [mv3(&d.w, &FACES[f][0]), mv3(&d.w, &FACES[f][1]), mv3(&d.w, &FACES[f][2])]
}

fn face_pt(d: &Die, fr: &[[f64; 3]; 3], kn: f64, ka: f64, kb: f64) -> [f64; 3] {
    std::array::from_fn(|i| d.p[i] + DIE_H * (kn * fr[0][i] + ka * fr[1][i] + kb * fr[2][i]))
}

fn draw_die_shadow(pen: &Pen<'_, '_>, d: &Die) {
    let fr = face_frame(d, 0);
    let mut pts = Vec::with_capacity(8);
    for k in 0..8 {
        let s = |b: usize| if k >> b & 1 == 1 { 1.0 } else { -1.0 };
        let p = face_pt(d, &fr, s(2), s(0), s(1));
        pts.push(proj([p[0] + p[2] * 0.625, p[1] - p[2] * 0.3125, 0.0]));
    }
    pen.fill(&hull(pts), Color::new(0.0, 0.0, 0.0, 100.0 / 255.0));
}

fn draw_die(pen: &Pen<'_, '_>, d: &Die) {
    for f in 0..6 {
        let fr = face_frame(d, f);
        let n = fr[0];
        if n[0] + n[1] - n[2] >= 0.0 {
            continue; // face cachée
        }
        let l = (n[0] * -0.512 + n[1] * 0.256 + n[2] * 0.82).max(0.0) * 0.55 + 0.45;
        let fill = Color::new((246.0 * l / 255.0) as f32, (240.0 * l / 255.0) as f32, (224.0 * l / 255.0) as f32, 1.0);
        let corners: Vec<(f64, f64)> = [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]
            .iter()
            .map(|&(a, b)| proj(face_pt(d, &fr, 1.0, a, b)))
            .collect();
        pen.fill(&corners, fill);
        pen.outline(&corners, if d.held { C_GOLD } else { C_EDGE }, if d.held { 1.6 } else { 0.8 });
        let pip = if f == 0 { C_PIPRED } else { C_PIP };
        for k in 0..9 {
            if PIPMASK[f] >> k & 1 == 0 {
                continue;
            }
            let ring: Vec<(f64, f64)> = (0..16)
                .map(|j| {
                    let t = j as f64 * std::f64::consts::TAU / 16.0;
                    proj(face_pt(d, &fr, 1.004, PIPX[k] + 0.2 * t.cos(), PIPY[k] + 0.2 * t.sin()))
                })
                .collect();
            pen.fill(&ring, pip);
        }
    }
}

fn draw_dice(pen: &Pen<'_, '_>, g: &Game) {
    for d in g.dice.iter().filter(|d| d.held) {
        let e = pen.ground_ellipse(d.ax, d.by, FELT, 1.4);
        pen.fill(&e, C_HELDDISC);
        pen.outline(&e, C_GOLD, 1.0);
    }
    let mut order: Vec<&Die> = g.dice.iter().collect();
    order.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap());
    for d in &order {
        draw_die_shadow(pen, d);
    }
    for d in &order {
        draw_die(pen, d);
    }
}

// ---------- table, jetons, boutons ----------

fn draw_table(pen: &Pen<'_, '_>) {
    pen.fill(&pen.ground_ellipse(0.0, 0.0, -0.95, 8.2), C_FLOORSH);
    pen.cylinder(0.0, 0.0, -0.95, 7.0, 1.05, C_WOOD, C_WOODSIDE);
    for k in 0..16u32 {
        let c = rgb((10 + k * 18 / 15) as u8, (70 + k * 58 / 15) as u8, (44 + k * 36 / 15) as u8);
        pen.fill(&pen.ground_ellipse(0.0, 0.0, FELT, 6.35 * (1.0 - 0.07 * k as f64)), c);
    }
    pen.outline(&pen.ground_ellipse(0.0, 0.0, FELT, 6.35), C_WOODLINE, 1.0);
    for (a, b) in [(POT_A, 0.0), (0.0, PL_Z), (0.0, BOT_Z)] {
        pen.outline(&pen.ground_ellipse(a, b, FELT, 1.5), C_ZONE, 1.0);
    }
}

/// Piles de 6 jetons (8 au-delà de 12, pot de 21).
fn draw_chips(pen: &Pen<'_, '_>, a0: f64, b: f64, n: u32, top: Color, side: Color) {
    if n == 0 {
        return;
    }
    let hs = if n > 12 { 8 } else { 6 };
    let stacks = n.div_ceil(hs);
    for j in 0..stacks {
        let a = a0 + (j as f64 - (stacks as f64 - 1.0) * 0.5) * 1.2;
        for i in 0..(n - j * hs).min(hs) {
            pen.cylinder(a, b, FELT + i as f64 * 0.17, 0.5, 0.17, top, side);
        }
    }
}

fn draw_button(pen: &Pen<'_, '_>, b: f64, label: &str, active: bool, blue: bool) {
    let ((top, side), txt) = match (active, blue) {
        (false, _) => (C_BTNX, C_BTNTXTX),
        (true, false) => (C_BTNG, C_BTNTXT),
        (true, true) => (C_BTNB, C_BTNTXT),
    };
    pen.cylinder(BTN_A, b, FELT, 1.0, 0.30, top, side);
    pen.ground_text(label, BTN_A, b - 0.18, BTN_TZ, 23.0, txt);
}

fn draw_buttons(pen: &Pen<'_, '_>, g: &Game) {
    let can = g.can_act();
    let (label, active) = match g.phase {
        _ if !can => ("...", false),
        Phase::PreRoll => ("LANCER", true),
        Phase::Choose => ("RELANCER", true),
        Phase::Result => ("SUIVANT", true),
        Phase::Over => ("REJOUER", true),
        _ => ("...", false),
    };
    draw_button(pen, BTN_B[0], label, active, false);
    draw_button(pen, BTN_B[1], "VALIDER", g.phase == Phase::Choose && can, true);
}

fn draw_hud(pen: &Pen<'_, '_>, g: &Game) {
    pen.ground_text(&format!("POT {}", g.pot), POT_A, 0.95, FELT, 40.0, C_TXTGND);
    pen.ground_text(&format!("{} {}", g.label(0), g.tok[0]), 0.0, 5.4, FELT, 40.0, C_TXTGND);
    pen.ground_text(&format!("{} {}", g.label(1), g.tok[1]), 0.0, -6.0, FELT, 40.0, C_TXTGND);
    pen.text("421", 20.0, 3.0, 42.0, true, C_GOLD, 0.0, 1.0);
    let right = match g.mode {
        crate::game::Mode::Solo => format!(
            "Manche {}   -   Bot {}   -   {} - {}",
            g.round,
            ai::LEVEL_NAMES[g.level],
            g.wins,
            g.losses
        ),
        crate::game::Mode::Duo => format!("Manche {}   -   2 joueurs", g.round),
        crate::game::Mode::Online { .. } => format!("Manche {}   -   En ligne", g.round),
    };
    pen.text(&right, WW as f64 - 20.0, 17.0, 30.0, false, C_WHITE, 1.0, 1.0);
    pen.text(&g.line1, 20.0, 62.0, 30.0, false, C_WHITE, 0.0, 1.0);
    pen.text(&g.line2, 20.0, 98.0, 30.0, false, C_GOLD, 0.0, 1.0);
    pen.text(HINT, 20.0, WH as f64 - 38.0, 23.0, true, C_HINT, 0.0, 1.0);
}

/// Décor fixe (mis en cache) : fond et table.
pub fn background(gfx: &Gfx, fonts: &Fonts) {
    let pen = Pen { g: gfx, fonts };
    gfx.rrect(&Xf::ID, 0.0, 0.0, 1100.0, WH, 0.0, Some(C_BG), None);
    draw_table(&pen);
}

/// Tout ce qui se dessine par-dessus le décor.
pub fn draw(g: &Game, gfx: &Gfx, fonts: &Fonts) {
    let pen = Pen { g: gfx, fonts };
    draw_hud(&pen, g);
    draw_chips(&pen, 0.0, BOT_Z, g.tok[1], C_BOTTOP, C_BOTSIDE);
    draw_chips(&pen, POT_A, 0.0, g.pot, C_POTTOP, C_POTSIDE);
    draw_buttons(&pen, g);
    draw_dice(&pen, g);
    draw_chips(&pen, 0.0, PL_Z, g.tok[0], C_PLTOP, C_PLSIDE);
}

/// Panneau du jeu en ligne (moteur commun), par-dessus tout.
pub fn net_panel(gfx: &Gfx, fonts: &Fonts, panel: Option<(&str, [&str; 3], bool)>) {
    if let Some((titre, lignes, menu)) = panel {
        bistrot::ui::panneau_reseau(gfx, &fonts.ui, titre, lignes, menu);
    }
}

// ---------- clics ----------

pub enum Hit {
    Main,
    Second,
    Die(usize),
    Nothing,
}

/// Ce qui se trouve sous le point (coordonnées du jeu).
pub fn hit(g: &Game, x: f64, y: f64) -> Hit {
    for (k, &b) in BTN_B.iter().enumerate() {
        let (cx, cy) = ground(BTN_A, b, BTN_TZ);
        let (dx, dy) = ((x - cx) / S, (y - cy) / (S * YSQ));
        if dx * dx + dy * dy <= 1.0 {
            return if k == 0 { Hit::Main } else { Hit::Second };
        }
    }
    if g.phase == Phase::Choose {
        for i in 0..3 {
            let (px, py) = proj(g.dice[g.cur * 3 + i].p);
            if (px - x).powi(2) + (py - y).powi(2) <= 3900.0 {
                return Hit::Die(i);
            }
        }
    }
    Hit::Nothing
}
