//! Cartes « enluminées » (style taverne) : vélin légèrement dégradé, ombre douce, cadre doré et filet de la
//! couleur, fleurons dorés aux coins, indices en Palatino aux coins (celui du bas retourné, comme sur un vrai
//! jeu), couleurs dessinées en formes vectorielles (♠ ♥ ♦ ♣ nettes à toutes les tailles) aux encres profondes en
//! dégradé, points du bas retournés, figures à double tête (roi couronné et barbu, dame aux cheveux longs, valet
//! au béret à plume, cavalier casqué) sur fond losangé d'or, outremer ou pourpre comme dans les manuscrits ; as
//! dans un anneau d'or perlé ; dos pourpre à losanges dorés et médaillon outremer à la fleur de lys. Toutes les
//! cartes des jeux de bistrot : 2 à 10,
//! valet, cavalier (tarot), dame, roi, as des quatre couleurs, atouts du tarot (1 à 21), excuse, jokers.
//!
//! Chaque carte est dessinée une seule fois dans une petite image à l'échelle de la fenêtre (à la première
//! utilisation), en deux tailles : normale, et réduite pour les cartes vues de loin (plus nette et plus
//! rapide). Le cache est limité en mémoire : les images les moins utilisées sont oubliées au-delà du budget.
//! Repère canonique d'une carte : 100 x 140 centré sur l'origine.

use crate::gfx::{alpha, chemin_ellipse, chemin_rrect, rgb, Align, Color, Face, Gfx, Glyphs, Xf};
use crate::polices::Fichiers;
use std::cell::RefCell;
use std::collections::HashMap;
use tiny_skia::{Path, PathBuilder, Pixmap};

const C_SHADOW: Color = rgb(22, 11, 4);
const C_PAPER_HI: Color = rgb(252, 247, 232);
const C_PAPER_LO: Color = rgb(238, 225, 196);
const C_CRED: Color = rgb(164, 22, 36);
const C_CBLK: Color = rgb(24, 22, 30);
/// dégradés des points : encres rouge et noire profondes
const C_RED_HI: Color = rgb(212, 48, 56);
const C_RED_LO: Color = rgb(122, 10, 26);
const C_BLK_HI: Color = rgb(76, 76, 98);
const C_BLK_LO: Color = rgb(12, 10, 18);
/// filets intérieurs : rouge pâle (♥ ♦), outremer pâle (♠ ♣)
const C_TRED: Color = rgb(212, 128, 118);
const C_TBLK: Color = rgb(110, 122, 172);
const C_PANEL_HI: Color = rgb(250, 243, 224);
const C_PANEL_LO: Color = rgb(232, 215, 178);
const C_GOLDF: Color = rgb(222, 176, 74);
const C_GOLDHI: Color = rgb(252, 226, 146);
const C_GOLDLN: Color = rgb(126, 88, 22);
const C_LAPIS_HI: Color = rgb(54, 78, 164);
const C_LAPIS_LO: Color = rgb(20, 30, 84);
const C_POURPRE_HI: Color = rgb(156, 30, 46);
const C_POURPRE_LO: Color = rgb(80, 10, 24);
const C_BORD_HI: Color = rgb(138, 24, 42);
const C_BORD_LO: Color = rgb(62, 8, 20);
const C_CLN2: Color = rgb(150, 126, 90);
const C_SKIN: Color = rgb(244, 216, 190);
const C_LINE: Color = rgb(70, 50, 40);

/// Budget mémoire des images de cartes (octets).
const BUDGET: usize = 7 << 20;

/// Les quatre couleurs, dans l'ordre pique, cœur, carreau, trèfle.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Couleur {
    Pique,
    Coeur,
    Carreau,
    Trefle,
}

impl Couleur {
    pub const TOUTES: [Couleur; 4] = [Couleur::Pique, Couleur::Coeur, Couleur::Carreau, Couleur::Trefle];

    pub fn depuis(i: usize) -> Couleur {
        Couleur::TOUTES[i & 3]
    }

    pub fn rouge(self) -> bool {
        matches!(self, Couleur::Coeur | Couleur::Carreau)
    }

    pub fn symbole(self) -> &'static str {
        ["♠", "♥", "♦", "♣"][self as usize]
    }

    pub fn nom(self) -> &'static str {
        ["Pique", "Cœur", "Carreau", "Trèfle"][self as usize]
    }
}

/// Hauteur d'une carte de couleur.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Valeur {
    /// 2 à 10
    N(u8),
    Valet,
    /// tarot
    Cavalier,
    Dame,
    Roi,
    As,
}

impl Valeur {
    pub fn indice(self) -> String {
        match self {
            Valeur::N(n) => n.to_string(),
            Valeur::Valet => "V".into(),
            Valeur::Cavalier => "C".into(),
            Valeur::Dame => "D".into(),
            Valeur::Roi => "R".into(),
            Valeur::As => "A".into(),
        }
    }
}

/// Une carte de n'importe quel jeu de bistrot.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Carte {
    Normale(Couleur, Valeur),
    /// atout du tarot, 1 à 21
    Atout(u8),
    Excuse,
    /// joker (rouge ou noir)
    Joker(bool),
}

impl Carte {
    /// Jeu de 32 cartes (belote, piquet) : 8 rangs 7, 8, 9, 10, V, D, R, A par couleur.
    pub fn jeu32(i: usize) -> Carte {
        const R: [Valeur; 8] = [Valeur::N(7), Valeur::N(8), Valeur::N(9), Valeur::N(10), Valeur::Valet, Valeur::Dame, Valeur::Roi, Valeur::As];
        Carte::Normale(Couleur::depuis(i >> 3), R[i & 7])
    }

    /// Jeu de 52 cartes (rami, bataille...) : 13 rangs 2 à 10, V, D, R, A par couleur.
    pub fn jeu52(i: usize) -> Carte {
        let r = i % 13;
        let v = match r {
            0..=8 => Valeur::N(r as u8 + 2),
            9 => Valeur::Valet,
            10 => Valeur::Dame,
            11 => Valeur::Roi,
            _ => Valeur::As,
        };
        Carte::Normale(Couleur::depuis(i / 13), v)
    }

    /// Jeu de tarot (78 cartes) : 0..55 = 14 rangs (1 à 10, V, C, D, R) par couleur, 56..76 = atouts 1 à 21, 77 = excuse.
    /// Au tarot, la plus petite carte de chaque couleur est le 1 (la plus faible, pas un as).
    pub fn tarot(i: usize) -> Carte {
        match i {
            0..=55 => {
                let r = i % 14;
                let v = match r {
                    0 => Valeur::N(1),
                    1..=9 => Valeur::N(r as u8 + 1),
                    10 => Valeur::Valet,
                    11 => Valeur::Cavalier,
                    12 => Valeur::Dame,
                    _ => Valeur::Roi,
                };
                Carte::Normale(Couleur::depuis(i / 14), v)
            }
            56..=76 => Carte::Atout((i - 55) as u8),
            _ => Carte::Excuse,
        }
    }
}

// ---------- formes vectorielles ----------

/// Contours d'une couleur dans le carré [-1, 1] (plusieurs morceaux pour le trèfle).
fn chemins_couleur(c: Couleur) -> Vec<Path> {
    let mut v = Vec::new();
    let mut pb = PathBuilder::new();
    match c {
        Couleur::Coeur => {
            pb.move_to(0.0, 0.95);
            pb.cubic_to(-0.18, 0.72, -1.0, 0.25, -1.0, -0.32);
            pb.cubic_to(-1.0, -0.82, -0.38, -1.05, 0.0, -0.58);
            pb.cubic_to(0.38, -1.05, 1.0, -0.82, 1.0, -0.32);
            pb.cubic_to(1.0, 0.25, 0.18, 0.72, 0.0, 0.95);
            pb.close();
        }
        Couleur::Carreau => {
            pb.move_to(0.0, -1.0);
            pb.quad_to(0.3, -0.45, 0.78, 0.0);
            pb.quad_to(0.3, 0.45, 0.0, 1.0);
            pb.quad_to(-0.3, 0.45, -0.78, 0.0);
            pb.quad_to(-0.3, -0.45, 0.0, -1.0);
            pb.close();
        }
        Couleur::Pique => {
            pb.move_to(0.0, -1.0);
            pb.cubic_to(0.28, -0.62, 1.0, -0.35, 1.0, 0.12);
            pb.cubic_to(1.0, 0.55, 0.42, 0.68, 0.1, 0.36);
            pb.cubic_to(0.12, 0.62, 0.24, 0.86, 0.46, 1.0);
            pb.line_to(-0.46, 1.0);
            pb.cubic_to(-0.24, 0.86, -0.12, 0.62, -0.1, 0.36);
            pb.cubic_to(-0.42, 0.68, -1.0, 0.55, -1.0, 0.12);
            pb.cubic_to(-1.0, -0.35, -0.28, -0.62, 0.0, -1.0);
            pb.close();
        }
        Couleur::Trefle => {
            for (x, y, r) in [(0.0, -0.5, 0.43), (-0.52, 0.1, 0.43), (0.52, 0.1, 0.43), (0.0, 0.04, 0.26)] {
                let mut c = PathBuilder::new();
                c.push_circle(x, y, r);
                v.extend(c.finish());
            }
            pb.move_to(-0.08, 0.2);
            pb.cubic_to(-0.1, 0.6, -0.24, 0.86, -0.46, 1.0);
            pb.line_to(0.46, 1.0);
            pb.cubic_to(0.24, 0.86, 0.1, 0.6, 0.08, 0.2);
            pb.close();
        }
    }
    v.extend(pb.finish());
    v
}

/// Étoile à cinq branches dans le cercle de rayon 1.
fn chemin_etoile() -> Option<Path> {
    let mut pb = PathBuilder::new();
    for k in 0..10 {
        let a = (-90.0 + k as f32 * 36.0).to_radians();
        let r = if k % 2 == 0 { 1.0 } else { 0.42 };
        let (x, y) = (r * a.cos(), r * a.sin());
        if k == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    pb.close();
    pb.finish()
}

/// Fleur de lys dans le carré [-1, 1] (pétale central, pétales recourbés, bandeau, pied).
fn chemins_lys() -> Vec<Path> {
    let mut v = Vec::new();
    let mut pb = PathBuilder::new();
    pb.move_to(0.0, -1.0);
    pb.cubic_to(0.26, -0.74, 0.36, -0.42, 0.22, -0.12);
    pb.cubic_to(0.16, 0.0, 0.11, 0.08, 0.1, 0.16);
    pb.line_to(-0.1, 0.16);
    pb.cubic_to(-0.11, 0.08, -0.16, 0.0, -0.22, -0.12);
    pb.cubic_to(-0.36, -0.42, -0.26, -0.74, 0.0, -1.0);
    pb.close();
    v.extend(pb.finish());
    for s in [-1.0f32, 1.0] {
        let mut pb = PathBuilder::new();
        pb.move_to(s * 0.12, 0.16);
        pb.cubic_to(s * 0.16, -0.3, s * 0.6, -0.62, s * 0.9, -0.38);
        pb.cubic_to(s * 1.08, -0.2, s * 0.98, 0.1, s * 0.76, 0.14);
        pb.cubic_to(s * 0.84, 0.0, s * 0.8, -0.16, s * 0.66, -0.17);
        pb.cubic_to(s * 0.46, -0.18, s * 0.32, 0.0, s * 0.3, 0.16);
        pb.close();
        v.extend(pb.finish());
        let mut pb = PathBuilder::new();
        pb.move_to(s * 0.06, 0.3);
        pb.cubic_to(s * 0.26, 0.34, s * 0.48, 0.46, s * 0.52, 0.7);
        pb.cubic_to(s * 0.36, 0.58, s * 0.2, 0.52, s * 0.04, 0.52);
        pb.close();
        v.extend(pb.finish());
    }
    let mut pb = PathBuilder::new();
    pb.move_to(-0.1, 0.3);
    pb.cubic_to(-0.12, 0.54, -0.05, 0.72, 0.0, 0.86);
    pb.cubic_to(0.05, 0.72, 0.12, 0.54, 0.1, 0.3);
    pb.close();
    v.extend(pb.finish());
    v.extend(chemin_rrect(-0.46, 0.13, 0.92, 0.19, 0.06));
    v
}

/// Fleur de lys dorée centrée en (cx, cy), de demi-hauteur h.
fn lys(g: &Gfx, xf: &Xf, cx: f32, cy: f32, h: f32) {
    let local = xf.puis(&Xf::centre(cx, cy, h));
    for p in chemins_lys() {
        g.lineaire(&local, &p, (0.0, -1.0), (0.0, 1.0), &[(0.0, C_GOLDHI), (0.5, C_GOLDF), (1.0, rgb(176, 128, 40))]);
        g.forme(&local, &p, None, Some(C_GOLDLN), 0.05);
    }
}

/// Fleuron doré (quatre perles autour d'un cœur de couleur), de taille k.
fn fleuron(g: &Gfx, xf: &Xf, x: f32, y: f32, k: f32, coeur: Color) {
    for i in 0..4 {
        let a = (45.0 + 90.0 * i as f32).to_radians();
        g.ellipse(xf, x + 2.1 * k * a.cos(), y + 2.1 * k * a.sin(), 1.45 * k, 1.45 * k, Some(C_GOLDF), Some(C_GOLDLN));
    }
    g.ellipse(xf, x, y, 1.0 * k, 1.0 * k, Some(coeur), None);
}

/// Treillis de losanges (droites à 45°) recoupé au rectangle [x0, x1] x [y0, y1], perles au centre des losanges.
fn losanges(g: &Gfx, x0: f32, y0: f32, x1: f32, y1: f32, pas: f32, trait_: Color, w: f32, perle: Option<Color>) {
    let id = Xf::ID;
    let mut c = ((x0 - y1) / pas).floor() * pas;
    while c <= x1 - y0 {
        // y = x - c
        let (a, b) = (x0.max(y0 + c), x1.min(y1 + c));
        if a < b {
            g.line(&id, a, a - c, b, b - c, w, trait_);
        }
        c += pas;
    }
    let mut c = ((x0 + y0) / pas).floor() * pas;
    while c <= x1 + y1 {
        // y = c - x
        let (a, b) = (x0.max(c - y1), x1.min(c - y0));
        if a < b {
            g.line(&id, a, c - a, b, c - b, w, trait_);
        }
        c += pas;
    }
    if let Some(col) = perle {
        let h = pas / 2.0;
        for i in (x0 / h).ceil() as i32..=(x1 / h).floor() as i32 {
            for j in (y0 / h).ceil() as i32..=(y1 / h).floor() as i32 {
                let (x, y) = (i as f32 * h, j as f32 * h);
                if (i + j) & 1 == 1 && x > x0 + 1.5 && x < x1 - 1.5 && y > y0 + 1.5 && y < y1 - 1.5 {
                    g.ellipse(&id, x, y, 0.75, 0.75, Some(col), None);
                }
            }
        }
    }
}

/// Dessine le symbole d'une couleur centré en (cx, cy), de demi-hauteur h, dans le repère xf.
pub fn symbole(g: &Gfx, xf: &Xf, c: Couleur, cx: f32, cy: f32, h: f32, col: Color) {
    let local = xf.puis(&Xf::centre(cx, cy, h));
    for p in chemins_couleur(c) {
        g.forme(&local, &p, Some(col), None, 0.0);
    }
}

/// Point d'une carte : symbole à l'encre profonde en dégradé (retourné dans la moitié basse).
fn point(g: &Gfx, xf: &Xf, c: Couleur, cx: f32, cy: f32, h: f32, retourne: bool) {
    let mut local = xf.puis(&Xf::centre(cx, cy, h));
    if retourne {
        local = local.puis(&Xf::DEMI_TOUR);
    }
    let (hi, lo) = if c.rouge() { (C_RED_HI, C_RED_LO) } else { (C_BLK_HI, C_BLK_LO) };
    for p in chemins_couleur(c) {
        g.lineaire(&local, &p, (0.0, -1.0), (0.0, 1.0), &[(0.0, hi), (1.0, lo)]);
    }
}

/// Étoile centrée en (cx, cy), de rayon r.
pub fn etoile(g: &Gfx, xf: &Xf, cx: f32, cy: f32, r: f32, col: Color) {
    if let Some(p) = chemin_etoile() {
        g.forme(&xf.puis(&Xf::centre(cx, cy, r)), &p, Some(col), None, 0.0);
    }
}

// ---------- dessin d'une carte ----------

const ROI: [(f32, f32); 7] = [(-16., 10.), (-16., -6.), (-8., 2.), (0., -10.), (8., 2.), (16., -6.), (16., 10.)];
const DAME: [(f32, f32); 7] = [(-13., 10.), (-13., -3.), (-6., 3.), (0., -7.), (6., 3.), (13., -3.), (13., 10.)];

/// Disposition des points (centres) des cartes 2 à 10, comme sur un vrai jeu.
fn points(n: u8) -> &'static [(f32, f32)] {
    match n {
        2 => &[(0., -42.), (0., 42.)],
        3 => &[(0., -42.), (0., 0.), (0., 42.)],
        4 => &[(-17., -42.), (17., -42.), (-17., 42.), (17., 42.)],
        5 => &[(-17., -42.), (17., -42.), (0., 0.), (-17., 42.), (17., 42.)],
        6 => &[(-17., -42.), (17., -42.), (-17., 0.), (17., 0.), (-17., 42.), (17., 42.)],
        7 => &[(-17., -42.), (17., -42.), (-17., 0.), (17., 0.), (-17., 42.), (17., 42.), (0., -21.)],
        8 => &[(-17., -42.), (17., -42.), (-17., 0.), (17., 0.), (-17., 42.), (17., 42.), (0., -21.), (0., 21.)],
        9 => &[(-17., -42.), (17., -42.), (-17., -14.), (17., -14.), (-17., 14.), (17., 14.), (-17., 42.), (17., 42.), (0., 0.)],
        _ => &[
            (-17., -42.),
            (17., -42.),
            (-17., -14.),
            (17., -14.),
            (-17., 14.),
            (17., 14.),
            (-17., 42.),
            (17., 42.),
            (0., -28.),
            (0., 28.),
        ],
    }
}

/// Zone de l'image d'une carte dans son repère (ombre douce comprise).
pub const X0: f32 = -57.0;
pub const Y0: f32 = -75.0;
pub const X1: f32 = 61.0;
pub const Y1: f32 = 84.0;

pub struct PolicesCartes {
    serif: Face,
    serif_petit: Face,
    /// numéros des atouts (un ou deux chiffres)
    index: Face,
    index_petit: Face,
}

impl PolicesCartes {
    pub fn depuis(f: &Fichiers) -> PolicesCartes {
        PolicesCartes { serif: f.livre.face(26.0), serif_petit: f.livre.face(13.0), index: f.livre.face(24.0), index_petit: f.livre.face(19.5) }
    }
}

/// Ombre douce sous la carte (plusieurs couches de plus en plus larges et légères).
fn ombre(g: &Gfx) {
    for i in 0..7 {
        let e = i as f32;
        let c = alpha(C_SHADOW, 0.09);
        g.rrect(&Xf::ID, -50. + 2.5 - e, -70. + 4. - e, 100. + 2. * e, 140. + 2. * e, 12. + e, Some(c), None);
    }
}

/// Vélin, cadre doré, filet de la couleur et fleurons aux deux coins libres.
fn papier(g: &Gfx, filet: Color) {
    let id = Xf::ID;
    ombre(g);
    g.rrect_degrade(&id, -50., -70., 100., 140., 12., C_PAPER_HI, C_PAPER_LO, Some(C_CLN2));
    g.rrect_w(&id, -46.5, -66.5, 93., 133., 8., None, Some(C_GOLDF), 1.1);
    g.rrect(&id, -44.2, -64.2, 88.4, 128.4, 6.5, None, Some(filet));
    fleuron(g, &id, 38.5, -58.0, 1.0, filet);
    fleuron(g, &id, -38.5, 58.0, 1.0, filet);
}

/// Symbole d'un indice de coin.
#[derive(Clone, Copy)]
enum Signe {
    Couleur(Couleur),
    Etoile,
}

/// Indices de coin : en haut à gauche, et le même retourné en bas à droite (comme sur un vrai jeu).
/// L'indice est dessiné une fois dans une petite image, posée deux fois.
fn indices(g: &Gfx, f: &PolicesCartes, texte: &str, signe: Signe, encre: Color) {
    let (ix0, iy0, ix1, iy1) = (-50.0f32, -69.0f32, -22.0f32, -26.0f32);
    let ppu = g.sc;
    let mut img = match Pixmap::new(((ix1 - ix0) * ppu).ceil() as u32, ((iy1 - iy0) * ppu).ceil() as u32) {
        Some(p) => p,
        None => return,
    };
    {
        let gi = Gfx::new(&mut img, g.glyphs(), ppu, -ix0 * ppu, -iy0 * ppu);
        gi.text(texte, -36., -69., &f.serif, encre, Align::Center);
        match signe {
            Signe::Couleur(c) => symbole(&gi, &Xf::ID, c, -36., -35., 6.5, encre),
            Signe::Etoile => etoile(&gi, &Xf::ID, -36., -35., 6.5, encre),
        }
    }
    g.image(&Xf::ID, &img, ix0, iy0, ppu);
    g.image(&Xf::DEMI_TOUR, &img, ix0, iy0, ppu);
}

/// Moitié haute d'une figure (la moitié basse est la même, retournée) : robe, tête, coiffe.
fn figure(g: &Gfx, xf: &Xf, v: Valeur, c: Couleur) {
    let (robe, robe_ombre) = if c.rouge() { (rgb(166, 26, 40), rgb(100, 12, 24)) } else { (rgb(36, 56, 134), rgb(18, 26, 76)) };
    // attribut tenu, derrière le buste : sceptre (roi), fleur (dame), hallebarde (valet), lance (cavalier)
    match v {
        Valeur::Roi => {
            g.line(xf, -18.0, -1.0, -18.0, -33.0, 1.8, C_GOLDLN);
            g.line(xf, -18.0, -1.0, -18.0, -33.0, 1.0, C_GOLDF);
            g.ellipse(xf, -18.0, -35.0, 2.6, 2.6, Some(C_GOLDF), Some(C_GOLDLN));
        }
        Valeur::Dame => {
            g.line(xf, 17.5, -2.0, 17.5, -25.0, 1.0, rgb(70, 120, 60));
            g.ellipse(xf, 16.0, -15.0, 2.4, 1.2, Some(rgb(90, 150, 80)), None);
            for k in 0..5 {
                let a = (k as f32 * 72.0 - 90.0).to_radians();
                g.ellipse(xf, 17.5 + 2.3 * a.cos(), -27.5 + 2.3 * a.sin(), 1.8, 1.8, Some(robe), None);
            }
            g.ellipse(xf, 17.5, -27.5, 1.3, 1.3, Some(C_GOLDF), None);
        }
        _ => {
            g.line(xf, 18.5, -1.0, 18.5, -40.0, 1.3, rgb(110, 76, 44));
            let lame = [(18.5, -48.0), (21.5, -42.0), (18.5, -39.5), (14.5, -43.0)];
            g.poly_any(xf, &lame, rgb(205, 208, 216), Some(rgb(110, 112, 124)));
        }
    }
    // buste : épaules jusqu'aux bords du cadre
    let mut pb = PathBuilder::new();
    pb.move_to(-26.0, 0.0);
    pb.line_to(-26.0, -7.0);
    pb.cubic_to(-25.0, -13.0, -17.0, -18.0, -6.5, -19.5);
    pb.line_to(6.5, -19.5);
    pb.cubic_to(17.0, -18.0, 25.0, -13.0, 26.0, -7.0);
    pb.line_to(26.0, 0.0);
    pb.close();
    if let Some(p) = pb.finish() {
        g.forme(xf, &p, Some(robe), Some(robe_ombre), 0.6);
    }
    // galon central et motifs de la couleur
    g.rrect(xf, -2.3, -19.0, 4.6, 19.0, 1.0, Some(C_GOLDF), Some(C_GOLDLN));
    symbole(g, xf, c, -13.5, -10.5, 3.2, C_GOLDF);
    symbole(g, xf, c, 13.5, -10.5, 3.2, C_GOLDF);
    // bras croisés devant : manches, revers dorés, mains
    for sx in [-1.0f32, 1.0] {
        let mut pb = PathBuilder::new();
        pb.move_to(sx * 21.0, -14.0);
        pb.quad_to(sx * 17.0, -4.0, sx * 6.0, -3.5);
        pb.line_to(sx * 6.0, -0.5);
        pb.quad_to(sx * 19.0, -0.5, sx * 25.0, -10.0);
        pb.close();
        if let Some(p) = pb.finish() {
            g.forme(xf, &p, Some(robe_ombre), None, 0.0);
        }
        g.rrect(xf, if sx < 0.0 { -7.6 } else { 4.6 }, -4.2, 3.0, 4.2, 0.8, Some(C_GOLDF), None);
    }
    g.ellipse(xf, -3.0, -2.2, 2.4, 1.9, Some(C_SKIN), Some(C_LINE));
    g.ellipse(xf, 3.0, -2.2, 2.4, 1.9, Some(C_SKIN), Some(C_LINE));
    // cheveux longs (dame), derrière la tête
    let cheveux = rgb(92, 58, 34);
    if v == Valeur::Dame {
        g.ellipse(xf, 0.0, -27.0, 9.0, 9.6, Some(cheveux), None);
        g.ellipse(xf, -7.6, -20.5, 3.0, 6.2, Some(cheveux), None);
        g.ellipse(xf, 7.6, -20.5, 3.0, 6.2, Some(cheveux), None);
    }
    g.ellipse(xf, 0.0, -19.5, 7.8, 2.6, Some(C_GOLDF), Some(C_GOLDLN));
    // tête
    g.ellipse(xf, 0.0, -27.5, 6.4, 7.4, Some(C_SKIN), Some(C_LINE));
    g.ellipse(xf, -2.4, -28.6, 0.8, 0.9, Some(C_LINE), None);
    g.ellipse(xf, 2.4, -28.6, 0.8, 0.9, Some(C_LINE), None);
    g.ellipse(xf, -3.6, -25.6, 1.3, 0.8, Some(rgb(240, 180, 170)), None);
    g.ellipse(xf, 3.6, -25.6, 1.3, 0.8, Some(rgb(240, 180, 170)), None);
    if v == Valeur::Roi {
        // barbe
        let mut pb = PathBuilder::new();
        pb.move_to(-6.2, -27.5);
        pb.quad_to(-6.8, -19.0, 0.0, -17.2);
        pb.quad_to(6.8, -19.0, 6.2, -27.5);
        pb.quad_to(4.4, -22.6, 0.0, -22.4);
        pb.quad_to(-4.4, -22.6, -6.2, -27.5);
        pb.close();
        if let Some(p) = pb.finish() {
            g.forme(xf, &p, Some(rgb(128, 96, 64)), Some(rgb(90, 64, 40)), 0.4);
        }
    }
    g.line(xf, -1.6, -24.0, 1.6, -24.0, 0.7, rgb(170, 70, 70));
    // coiffe
    let couronne = |pts: &[(f32, f32)], k: f32, dy: f32| {
        let p: Vec<(f32, f32)> = pts.iter().map(|&(x, y)| (x * k, y * k + dy)).collect();
        g.poly_any(xf, &p, C_GOLDF, Some(C_GOLDLN));
    };
    match v {
        Valeur::Roi => couronne(&ROI, 0.72, -40.5),
        Valeur::Dame => {
            couronne(&DAME, 0.62, -38.5);
            g.ellipse(xf, 0.0, -43.0, 1.6, 1.6, Some(C_GOLDF), Some(C_GOLDLN));
        }
        Valeur::Cavalier => {
            let mut pb = PathBuilder::new();
            pb.move_to(-7.2, -29.0);
            pb.cubic_to(-7.4, -38.5, 7.4, -38.5, 7.2, -29.0);
            pb.close();
            if let Some(p) = pb.finish() {
                g.forme(xf, &p, Some(rgb(196, 198, 208)), Some(rgb(110, 112, 124)), 0.5);
            }
            g.ellipse(xf, 2.0, -40.0, 2.4, 5.0, Some(robe), None);
        }
        _ => {
            // béret à plume
            g.ellipse(xf, 0.0, -34.4, 9.0, 3.0, Some(robe_ombre), None);
            g.ellipse(xf, -0.5, -36.4, 6.8, 4.0, Some(robe), None);
            let mut pb = PathBuilder::new();
            pb.move_to(4.0, -38.0);
            pb.cubic_to(8.0, -43.0, 12.0, -46.0, 17.0, -47.0);
            if let Some(p) = pb.finish() {
                g.forme(xf, &p, None, Some(C_GOLDF), 1.5);
            }
        }
    }
    // petite couleur dorée dans le coin du cadre
    symbole(g, xf, c, -19.0, -46.0, 4.2, C_GOLDHI);
}

fn dessiner_face(g: &Gfx, f: &PolicesCartes, carte: Carte) {
    let id = Xf::ID;
    match carte {
        Carte::Normale(c, v) => {
            let (encre, filet) = if c.rouge() { (C_CRED, C_TRED) } else { (C_CBLK, C_TBLK) };
            papier(g, filet);
            indices(g, f, &v.indice(), Signe::Couleur(c), encre);
            match v {
                Valeur::As => {
                    // anneau d'or perlé d'outremer et de pourpre, fleurons aux diagonales
                    if let Some(p) = chemin_ellipse(0., 0., 33., 33.) {
                        g.forme(&id, &p, None, Some(C_GOLDF), 4.6);
                    }
                    g.ellipse(&id, 0., 0., 35.3, 35.3, None, Some(C_GOLDLN));
                    g.ellipse(&id, 0., 0., 30.7, 30.7, None, Some(C_GOLDLN));
                    for k in 0..16 {
                        let a = (k as f32 * 22.5).to_radians();
                        let perle = if k % 2 == 0 { C_LAPIS_HI } else { C_POURPRE_HI };
                        g.ellipse(&id, 33. * a.cos(), 33. * a.sin(), 1.15, 1.15, Some(perle), None);
                    }
                    for (sx, sy) in [(-1.0f32, -1.0f32), (1., -1.), (-1., 1.), (1., 1.)] {
                        fleuron(g, &id, sx * 31., sy * 45., 1.25, encre);
                    }
                    point(g, &id, c, 0., 0., 23., false);
                }
                Valeur::N(1) => point(g, &id, c, 0., 0., 15., false),
                Valeur::N(n) => {
                    for &(x, y) in points(n) {
                        point(g, &id, c, x, y, 8.5, y > 0.0);
                    }
                }
                _ => {
                    // fond losangé d'or, outremer derrière les figures rouges, pourpre derrière les noires
                    let (hi, lo) = if c.rouge() { (C_LAPIS_HI, C_LAPIS_LO) } else { (C_POURPRE_HI, C_POURPRE_LO) };
                    if let Some(p) = chemin_rrect(-26., -54., 52., 108., 6.) {
                        g.lineaire(&id, &p, (0., -54.), (0., 54.), &[(0.0, lo), (0.5, hi), (1.0, lo)]);
                    }
                    losanges(g, -24.5, -52.5, 24.5, 52.5, 8., alpha(C_GOLDF, 0.42), 0.35, Some(alpha(C_GOLDHI, 0.8)));
                    figure(g, &id, v, c);
                    figure(g, &Xf::DEMI_TOUR, v, c);
                    g.line(&id, -26., 0., 26., 0., 0.8, C_GOLDLN);
                    g.rrect_w(&id, -26., -54., 52., 108., 6., None, Some(C_GOLDF), 1.3);
                    g.rrect(&id, -26., -54., 52., 108., 6., None, Some(C_GOLDLN));
                }
            }
        }
        Carte::Atout(n) => {
            // deux scènes du Tarot Nouveau ; cartouche doré pour les bouts (1 et 21)
            velin(g);
            crate::atouts::illustration(g, n);
            cartouches(g, f, Some(&n.to_string()), n == 1 || n == 21);
        }
        Carte::Excuse => {
            velin(g);
            crate::atouts::illustration_excuse(g);
            cartouches(g, f, None, true);
        }
        Carte::Joker(rouge) => {
            let (encre, filet) = if rouge { (C_CRED, C_TRED) } else { (C_CBLK, C_TBLK) };
            papier(g, filet);
            indices(g, f, "J", Signe::Etoile, encre);
            g.rrect_degrade(&id, -26., -54., 52., 108., 6., C_PANEL_HI, C_PANEL_LO, Some(filet));
            g.poly_any(&id, &ROI.iter().map(|&(x, y)| (x, y - 30.)).collect::<Vec<_>>(), C_GOLDF, Some(C_GOLDLN));
            etoile(g, &id, 0., 2., 14., encre);
            g.text("JOKER", 0., 24., &f.serif_petit, encre, Align::Center);
        }
    }
}

/// Vélin seul (atouts illustrés).
fn velin(g: &Gfx) {
    ombre(g);
    g.rrect_degrade(&Xf::ID, -50., -70., 100., 140., 12., C_PAPER_HI, C_PAPER_LO, Some(C_CLN2));
}

/// Cartouches des atouts : numéro (ou étoile pour l'Excuse) sur un petit cadre de vélin, doré pour les bouts,
/// en haut à gauche et, retourné, en bas à droite : lisible quand les cartes se chevauchent dans la main.
fn cartouches(g: &Gfx, f: &PolicesCartes, texte: Option<&str>, bout: bool) {
    let (x0, y0, w, h) = (-49.5f32, -69.5f32, 23.0f32, 26.0f32);
    let ppu = g.sc;
    let Some(mut img) = Pixmap::new(((w + 1.) * ppu).ceil() as u32, ((h + 1.) * ppu).ceil() as u32) else { return };
    {
        let gi = Gfx::new(&mut img, g.glyphs(), ppu, -(x0 - 0.5) * ppu, -(y0 - 0.5) * ppu);
        let fond = if bout { C_GOLDHI } else { C_PAPER_HI };
        gi.rrect_w(&Xf::ID, x0, y0, w, h, 4., Some(fond), Some(C_GOLDLN), 0.7);
        let (cx, cy) = (x0 + w / 2., y0 + h / 2.);
        match texte {
            Some(t) => {
                let face = if t.len() > 1 { &f.index_petit } else { &f.index };
                gi.text(t, cx, crate::ui::centre_v(face, cy), face, C_CBLK, Align::Center);
            }
            None => etoile(&gi, &Xf::ID, cx, cy, 7., C_CBLK),
        }
    }
    g.image(&Xf::ID, &img, x0 - 0.5, y0 - 0.5, ppu);
    g.image(&Xf::DEMI_TOUR, &img, x0 - 0.5, y0 - 0.5, ppu);
}

/// Dos : pourpre à losanges dorés perlés, double cadre doré, médaillon outremer à la fleur de lys.
fn dessiner_dos(g: &Gfx) {
    let id = Xf::ID;
    ombre(g);
    g.rrect_degrade(&id, -50., -70., 100., 140., 12., C_PAPER_HI, C_PAPER_LO, Some(C_CLN2));
    if let Some(p) = chemin_rrect(-44., -64., 88., 128., 7.) {
        g.lineaire(&id, &p, (0., -64.), (0., 64.), &[(0.0, C_BORD_LO), (0.5, C_BORD_HI), (1.0, C_BORD_LO)]);
    }
    losanges(g, -39., -59., 39., 59., 16., alpha(C_GOLDF, 0.8), 0.6, Some(C_GOLDHI));
    g.rrect(&id, -44., -64., 88., 128., 7., None, Some(C_PAPER_HI));
    g.rrect_w(&id, -39., -59., 78., 118., 4., None, Some(C_GOLDF), 1.3);
    g.rrect(&id, -36.5, -56.5, 73., 113., 3., None, Some(alpha(C_GOLDF, 0.7)));
    for (sx, sy) in [(-1.0f32, -1.0f32), (1., -1.), (-1., 1.), (1., 1.)] {
        fleuron(g, &id, sx * 31.5, sy * 51.5, 1.2, C_LAPIS_HI);
    }
    if let Some(p) = chemin_ellipse(0., 0., 22., 31.) {
        g.radial(&id, &p, (0., -8.), 36., &[(0.0, C_LAPIS_HI), (1.0, C_LAPIS_LO)]);
        g.forme(&id, &p, None, Some(C_GOLDF), 1.5);
    }
    g.ellipse(&id, 0., 0., 18.5, 27., None, Some(alpha(C_GOLDF, 0.8)));
    lys(g, &id, 0., 1., 14.);
}

/// Dessine une carte dans une image (ppu pixels par unité).
fn rendre(f: &PolicesCartes, glyphs: &RefCell<Glyphs>, carte: Option<Carte>, ppu: f32) -> Option<Pixmap> {
    let (w, h) = (((X1 - X0) * ppu).ceil() as u32, ((Y1 - Y0) * ppu).ceil() as u32);
    let mut pm = Pixmap::new(w.max(1), h.max(1))?;
    {
        let g = Gfx::new(&mut pm, glyphs, ppu, -X0 * ppu, -Y0 * ppu);
        match carte {
            Some(c) => dessiner_face(&g, f, c),
            None => dessiner_dos(&g),
        }
    }
    Some(pm)
}

/// Cartes déjà dessinées : (carte ou dos, taille) -> (image, dernier usage).
#[derive(Default)]
struct Cache {
    images: HashMap<(Option<Carte>, u8), (Pixmap, u64)>,
    octets: usize,
    horloge: u64,
}

/// Les cartes, dessinées à l'échelle de la fenêtre à la première utilisation (cache limité en mémoire).
pub struct Paquet {
    polices: PolicesCartes,
    ppu: f32,
    cache: RefCell<Cache>,
}

/// Réduction de la petite taille (cartes vues de loin : mains adverses, panneaux).
const PETIT: f32 = 0.62;

impl Paquet {
    pub fn new(fichiers: &Fichiers) -> Paquet {
        Paquet { polices: PolicesCartes::depuis(fichiers), ppu: 0.0, cache: RefCell::default() }
    }

    /// Échelle des images : `ppu` pixels par unité de carte (les cartes de la main sont posées environ 1:1).
    /// Changer d'échelle oublie les cartes déjà dessinées.
    pub fn echelle(&mut self, ppu: f32) {
        if ppu != self.ppu {
            self.ppu = ppu;
            self.alleger();
        }
    }

    pub fn ppu(&self) -> f32 {
        self.ppu
    }

    /// Oublie toutes les cartes dessinées (elles seront refaites à la demande).
    pub fn alleger(&self) {
        *self.cache.borrow_mut() = Cache::default();
    }

    /// Mémoire occupée par les images de cartes (octets).
    pub fn octets(&self) -> usize {
        self.cache.borrow().octets
    }

    /// Pose une carte (None = dos) avec la transformation xf ; `grise` = carte injouable.
    pub fn poser(&self, g: &Gfx, xf: &Xf, carte: Option<Carte>, grise: bool) {
        let base = if self.ppu > 0.0 { self.ppu } else { g.sc };
        let taille = u8::from(xf.scale() < 0.8);
        let ppu = if taille == 1 { base * PETIT } else { base };
        let mut cache = self.cache.borrow_mut();
        cache.horloge += 1;
        let maintenant = cache.horloge;
        let cle = (carte, taille);
        if !cache.images.contains_key(&cle) {
            let Some(img) = rendre(&self.polices, g.glyphs(), carte, ppu) else { return };
            cache.octets += img.data().len();
            cache.images.insert(cle, (img, maintenant));
            // au-delà du budget : on oublie les images les moins récemment utilisées
            while cache.octets > BUDGET && cache.images.len() > 1 {
                let Some(vieille) = cache.images.iter().filter(|(k, _)| **k != cle).min_by_key(|(_, v)| v.1).map(|(k, _)| *k) else { break };
                if let Some((img, _)) = cache.images.remove(&vieille) {
                    cache.octets -= img.data().len();
                }
            }
        }
        if let Some(entree) = cache.images.get_mut(&cle) {
            entree.1 = maintenant;
            g.image(xf, &entree.0, X0, Y0, ppu);
        }
        drop(cache);
        if grise {
            g.rrect(xf, -50., -70., 100., 140., 12., Some(Color::new(0.0, 0.0, 0.0, 0.26)), None);
        }
    }
}
