//! Boîte à outils des scènes des atouts : ciels, sols, arbres, maisons, intérieurs, personnages en tenue du
//! XIXe siècle, animaux et objets. Repère d'une demi-scène : x de -43 à 43, y de -63 (haut) à -1 (bas) ; les
//! personnages se placent par leurs pieds.

use crate::gfx::{alpha, chemin_ellipse, chemin_poly, mix, rgb, Color, Gfx, Xf};
use tiny_skia::PathBuilder;

pub const NOIR: Color = rgb(30, 24, 22);
pub const BLANC: Color = rgb(250, 248, 240);
pub const PEAU: Color = rgb(240, 204, 172);
pub const PEAU_HALEE: Color = rgb(214, 170, 130);
pub const SEPIA: Color = rgb(70, 46, 28);
pub const BOIS: Color = rgb(122, 80, 44);
pub const BOIS_CLAIR: Color = rgb(176, 128, 78);
pub const HERBE: Color = rgb(108, 144, 74);
pub const HERBE_FONCEE: Color = rgb(70, 108, 52);
pub const FEUILLAGE: Color = rgb(58, 96, 50);
pub const BLE: Color = rgb(226, 190, 96);
pub const ROUGE: Color = rgb(170, 40, 42);
pub const BLEU: Color = rgb(52, 74, 136);
pub const VERT: Color = rgb(58, 110, 76);
pub const OCRE: Color = rgb(196, 146, 64);
pub const GRIS: Color = rgb(120, 118, 124);
pub const ROSE: Color = rgb(222, 150, 160);
pub const VIOLET: Color = rgb(110, 70, 120);
pub const CREME: Color = rgb(236, 222, 190);

pub fn assombrir(c: Color, k: f32) -> Color {
    mix(c, rgb(0, 0, 0), k)
}

pub fn eclaircir(c: Color, k: f32) -> Color {
    mix(c, rgb(255, 255, 255), k)
}

pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
}

/// Polygone rempli d'un dégradé vertical.
pub fn degrade(g: &Gfx, xf: &Xf, pts: &[(f32, f32)], y0: f32, y1: f32, haut: Color, bas: Color) {
    if let Some(p) = chemin_poly(pts) {
        g.lineaire(xf, &p, (0.0, y0), (0.0, y1), &[(0.0, haut), (1.0, bas)]);
    }
}

// ---------- décors ----------
/// Ciel de toute la demi-scène.
pub fn ciel(g: &Gfx, xf: &Xf, haut: Color, bas: Color) {
    degrade(g, xf, &rect(-44., -64., 88., 64.), -63., -10., haut, bas);
}

/// Sol (ou plancher) de y jusqu'en bas.
pub fn sol(g: &Gfx, xf: &Xf, y: f32, haut: Color, bas: Color) {
    degrade(g, xf, &rect(-44., y, 88., 1. - y), y, 0., haut, bas);
}

/// Ligne de collines (points de gauche à droite) remplie jusqu'au bas.
pub fn collines(g: &Gfx, xf: &Xf, pts: &[(f32, f32)], c: Color) {
    let mut v = vec![(-44., 0.)];
    v.extend_from_slice(pts);
    v.push((44., 0.));
    g.poly(xf, &v, Some(c), None);
}

pub fn nuage(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32) {
    let c = alpha(BLANC, 0.85);
    g.ellipse(xf, x, y, 6. * s, 2.2 * s, Some(c), None);
    g.ellipse(xf, x - 3. * s, y - 1.3 * s, 3.2 * s, 2.2 * s, Some(c), None);
    g.ellipse(xf, x + 2.5 * s, y - 1.6 * s, 3.6 * s, 2.6 * s, Some(c), None);
}

pub fn soleil(g: &Gfx, xf: &Xf, x: f32, y: f32, r: f32, c: Color) {
    g.lueur(xf, (x, y), r * 3.0, &[(0.0, alpha(c, 0.7)), (1.0, alpha(c, 0.0))]);
    g.ellipse(xf, x, y, r, r, Some(eclaircir(c, 0.5)), None);
}

pub fn lune(g: &Gfx, xf: &Xf, x: f32, y: f32, r: f32, fond: Color) {
    g.lueur(xf, (x, y), r * 3.0, &[(0.0, alpha(rgb(250, 240, 200), 0.45)), (1.0, alpha(rgb(250, 240, 200), 0.0))]);
    g.ellipse(xf, x, y, r, r, Some(rgb(250, 242, 206)), None);
    g.ellipse(xf, x + r * 0.45, y - r * 0.2, r * 0.85, r * 0.85, Some(fond), None);
}

pub fn etoiles(g: &Gfx, xf: &Xf, pts: &[(f32, f32)]) {
    for &(x, y) in pts {
        g.ellipse(xf, x, y, 0.5, 0.5, Some(rgb(255, 248, 210)), None);
    }
}

/// Arbre feuillu : tronc et masses de feuillage.
pub fn arbre(g: &Gfx, xf: &Xf, x: f32, y: f32, h: f32, c: Color) {
    g.poly(xf, &[(x - h * 0.05, y), (x + h * 0.05, y), (x + h * 0.03, y - h * 0.55), (x - h * 0.03, y - h * 0.55)], Some(SEPIA), None);
    let r = h * 0.28;
    for (dx, dy, k) in [(0.0, -0.72, 1.0), (-0.22, -0.6, 0.8), (0.22, -0.58, 0.8), (0.0, -0.9, 0.75)] {
        g.ellipse(xf, x + dx * h, y + dy * h, r * k, r * k * 0.9, Some(c), None);
    }
    for (dx, dy, k) in [(-0.08, -0.82, 0.45), (0.14, -0.7, 0.4)] {
        g.ellipse(xf, x + dx * h, y + dy * h, r * k, r * k * 0.8, Some(eclaircir(c, 0.18)), None);
    }
}

/// Arbre d'hiver, sans feuilles.
pub fn arbre_nu(g: &Gfx, xf: &Xf, x: f32, y: f32, h: f32) {
    g.line(xf, x, y, x, y - h * 0.6, h * 0.06, SEPIA);
    for (dx, dy, l, a) in [(0., -0.4, 0.3, -0.7f32), (0., -0.5, 0.32, 0.6), (0., -0.6, 0.35, -0.2), (0., -0.6, 0.3, 0.3)] {
        let (x0, y0) = (x + dx * h, y + dy * h);
        g.line(xf, x0, y0, x0 + a.sin() * l * h, y0 - a.cos() * l * h, h * 0.025, SEPIA);
    }
}

pub fn sapin(g: &Gfx, xf: &Xf, x: f32, y: f32, h: f32, c: Color) {
    g.line(xf, x, y, x, y - h * 0.2, h * 0.07, SEPIA);
    for k in 0..3 {
        let b = y - h * 0.15 - k as f32 * h * 0.25;
        let w = h * (0.32 - k as f32 * 0.08);
        g.poly(xf, &[(x - w, b), (x + w, b), (x, b - h * 0.4)], Some(if k % 2 == 0 { c } else { eclaircir(c, 0.1) }), None);
    }
}

/// Maison : murs, toit, porte, fenêtres.
pub fn maison(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32, h: f32, mur: Color, toit: Color) {
    g.poly(xf, &rect(x, y - h, w, h), Some(mur), Some(assombrir(mur, 0.3)));
    g.poly(xf, &[(x - 2., y - h), (x + w + 2., y - h), (x + w / 2., y - h - h * 0.6)], Some(toit), None);
    g.poly(xf, &rect(x + w * 0.42, y - h * 0.55, w * 0.18, h * 0.55), Some(assombrir(BOIS, 0.2)), None);
    for fx in [x + w * 0.12, x + w * 0.7] {
        fenetre(g, xf, fx, y - h * 0.75, w * 0.18, h * 0.3, rgb(170, 200, 214));
    }
}

/// Fenêtre à petits bois.
pub fn fenetre(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32, h: f32, vitre: Color) {
    g.poly(xf, &rect(x, y, w, h), Some(vitre), Some(CREME));
    g.line(xf, x + w / 2., y, x + w / 2., y + h, 0.5, CREME);
    g.line(xf, x, y + h / 2., x + w, y + h / 2., 0.5, CREME);
}

/// Intérieur bourgeois : papier peint rayé, lambris, parquet ; renvoie la hauteur du plancher.
pub fn interieur(g: &Gfx, xf: &Xf, papier: Color) -> f32 {
    g.poly(xf, &rect(-44., -64., 88., 52.), Some(papier), None);
    let mut x = -44.0;
    while x < 44.0 {
        g.poly(xf, &rect(x, -64., 3.0, 52.), Some(assombrir(papier, 0.08)), None);
        x += 7.0;
    }
    g.poly(xf, &rect(-44., -20., 88., 8.), Some(assombrir(papier, 0.35)), None);
    g.line(xf, -44., -20., 44., -20., 0.6, eclaircir(papier, 0.3));
    sol(g, xf, -12., BOIS_CLAIR, BOIS);
    for k in 0..4 {
        let y = -10. + k as f32 * 2.7;
        g.line(xf, -44., y, 44., y, 0.3, alpha(SEPIA, 0.5));
    }
    -6.0
}

/// Rideaux de part et d'autre d'une fenêtre.
pub fn rideaux(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32, h: f32, c: Color) {
    g.poly(xf, &[(x - 3., y - 1.), (x + 4., y - 1.), (x + 1., y + h * 0.6), (x - 3., y + h + 1.)], Some(c), None);
    g.poly(xf, &[(x + w + 3., y - 1.), (x + w - 4., y - 1.), (x + w - 1., y + h * 0.6), (x + w + 3., y + h + 1.)], Some(c), None);
}

pub fn table(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32, h: f32, nappe: Option<Color>) {
    for px in [x - w / 2. + 2., x + w / 2. - 2.] {
        g.line(xf, px, y - h, px, y, 1.0, SEPIA);
    }
    match nappe {
        Some(c) => g.poly(xf, &[(x - w / 2. - 1., y - h), (x + w / 2. + 1., y - h), (x + w / 2. + 2., y - h + 5.), (x - w / 2. - 2., y - h + 5.)], Some(c), None),
        None => g.poly(xf, &rect(x - w / 2., y - h - 1., w, 2.), Some(BOIS), None),
    }
}

pub fn chaise(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32) {
    g.line(xf, x - 3. * s, y, x - 3. * s, y - 12. * s, 0.8, SEPIA);
    g.line(xf, x + 3. * s, y, x + 3. * s, y - 6. * s, 0.8, SEPIA);
    g.line(xf, x - 3. * s, y - 6. * s, x + 3. * s, y - 6. * s, 1.2, BOIS);
}

pub fn fauteuil(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32, c: Color) {
    g.poly(xf, &rect(x - 7. * s, y - 18. * s, 14. * s, 14. * s), Some(c), None);
    g.poly(xf, &rect(x - 9. * s, y - 9. * s, 18. * s, 6. * s), Some(assombrir(c, 0.15)), None);
    g.line(xf, x - 8. * s, y - 3. * s, x - 8. * s, y, 1.0, SEPIA);
    g.line(xf, x + 8. * s, y - 3. * s, x + 8. * s, y, 1.0, SEPIA);
}

// ---------- personnages ----------
#[derive(Clone, Copy, PartialEq)]
pub enum Chapeau {
    Aucun,
    HautDeForme,
    Melon,
    Paille,
    Casquette,
    /// capeline de dame (couleur)
    Capeline(Color),
    /// coiffe blanche de paysanne
    Coiffe,
    /// bicorne de soldat, képi
    Kepi,
    /// bonnet pointu (clown, fou)
    Pointu(Color),
}

/// Un personnage de face, posé par ses pieds en (x, y), de taille h.
#[derive(Clone, Copy)]
pub struct Perso {
    pub x: f32,
    pub y: f32,
    pub h: f32,
    pub robe: bool,
    pub haut: Color,
    pub bas: Color,
    pub peau: Color,
    pub cheveux: Color,
    pub chapeau: Chapeau,
    /// angle des bras gauche et droit (0 : le long du corps ; positif : vers l'extérieur ; π : levé)
    pub bras: (f32, f32),
    pub assis: bool,
    pub tablier: Option<Color>,
    /// vu de dos (pas de visage)
    pub dos: bool,
}

impl Perso {
    pub fn homme(x: f32, y: f32, h: f32, haut: Color, bas: Color, chapeau: Chapeau) -> Perso {
        Perso { x, y, h, robe: false, haut, bas, peau: PEAU, cheveux: SEPIA, chapeau, bras: (0.15, 0.15), assis: false, tablier: None, dos: false }
    }

    pub fn femme(x: f32, y: f32, h: f32, haut: Color, bas: Color, chapeau: Chapeau) -> Perso {
        Perso { x, y, h, robe: true, haut, bas, peau: PEAU, cheveux: rgb(110, 66, 36), chapeau, bras: (0.12, 0.12), assis: false, tablier: None, dos: false }
    }

    pub fn bras(mut self, g: f32, d: f32) -> Perso {
        self.bras = (g, d);
        self
    }

    pub fn assis(mut self) -> Perso {
        self.assis = true;
        self
    }

    pub fn dos(mut self) -> Perso {
        self.dos = true;
        self
    }

    pub fn tablier(mut self, c: Color) -> Perso {
        self.tablier = Some(c);
        self
    }

    pub fn cheveux(mut self, c: Color) -> Perso {
        self.cheveux = c;
        self
    }

    pub fn peau(mut self, c: Color) -> Perso {
        self.peau = c;
        self
    }

    /// Position de la main gauche (-1) ou droite (1).
    pub fn main(&self, cote: f32) -> (f32, f32) {
        let (h, base) = self.proportions();
        let a = if cote < 0.0 { self.bras.0 } else { self.bras.1 };
        let (sx, sy) = (self.x + cote * h * 0.15, base - h * 0.78);
        let l = h * 0.36;
        (sx + cote * a.sin() * l, sy + a.cos() * l)
    }

    /// Taille et ordonnée de référence (le bas du personnage, plus haut s'il est assis).
    fn proportions(&self) -> (f32, f32) {
        if self.assis {
            (self.h, self.y - self.h * 0.12)
        } else {
            (self.h, self.y)
        }
    }

    pub fn dessiner(&self, g: &Gfx, xf: &Xf) {
        let (h, y) = self.proportions();
        let x = self.x;
        let taille = y - h * 0.47;
        let epaules = y - h * 0.80;
        // jambes ou robe
        if self.robe {
            let bas = if self.assis { y - h * 0.08 } else { y - h * 0.04 };
            g.poly(xf, &[(x - h * 0.11, taille), (x + h * 0.11, taille), (x + h * 0.22, bas), (x - h * 0.22, bas)], Some(self.bas), Some(assombrir(self.bas, 0.25)));
            g.ellipse(xf, x - h * 0.07, bas + h * 0.02, h * 0.05, h * 0.025, Some(NOIR), None);
            g.ellipse(xf, x + h * 0.07, bas + h * 0.02, h * 0.05, h * 0.025, Some(NOIR), None);
        } else if self.assis {
            for s in [-1.0f32, 1.0] {
                g.poly(xf, &rect(x + s * h * 0.1 - h * 0.05, taille, h * 0.1, h * 0.3), Some(self.bas), None);
                g.poly(xf, &rect(x + s * h * 0.1 - h * 0.045, taille + h * 0.3, h * 0.09, h * 0.16), Some(assombrir(self.bas, 0.15)), None);
                g.ellipse(xf, x + s * h * 0.1, y + h * 0.04, h * 0.06, h * 0.03, Some(NOIR), None);
            }
        } else {
            for s in [-1.0f32, 1.0] {
                g.poly(xf, &[(x + s * h * 0.02, taille), (x + s * h * 0.12, taille), (x + s * h * 0.11, y - h * 0.02), (x + s * h * 0.03, y - h * 0.02)], Some(self.bas), None);
                g.ellipse(xf, x + s * h * 0.08, y - h * 0.01, h * 0.06, h * 0.025, Some(NOIR), None);
            }
        }
        // buste
        g.poly(xf, &[(x - h * 0.15, epaules), (x + h * 0.15, epaules), (x + h * 0.11, taille), (x - h * 0.11, taille)], Some(self.haut), Some(assombrir(self.haut, 0.25)));
        if let Some(t) = self.tablier {
            g.poly(xf, &[(x - h * 0.09, taille - h * 0.06), (x + h * 0.09, taille - h * 0.06), (x + h * 0.14, y - h * 0.12), (x - h * 0.14, y - h * 0.12)], Some(t), None);
        }
        // bras et mains
        for (cote, a) in [(-1.0f32, self.bras.0), (1.0, self.bras.1)] {
            let (sx, sy) = (x + cote * h * 0.13, epaules + h * 0.02);
            let (mx, my) = self.main(cote);
            g.line(xf, sx, sy, mx, my, h * 0.075, self.haut);
            let _ = a;
            g.ellipse(xf, mx, my, h * 0.035, h * 0.035, Some(self.peau), None);
        }
        // cou et tête
        let r = h * 0.085;
        let cy = epaules - r * 0.95;
        g.poly(xf, &rect(x - h * 0.025, epaules - r * 0.3, h * 0.05, r * 0.4), Some(self.peau), None);
        if self.robe && !matches!(self.chapeau, Chapeau::Coiffe) {
            g.ellipse(xf, x, cy + r * 0.25, r * 1.15, r * 1.2, Some(self.cheveux), None);
        }
        g.ellipse(xf, x, cy, r, r * 1.1, Some(self.peau), None);
        if self.dos {
            g.ellipse(xf, x, cy - r * 0.1, r * 1.0, r * 1.05, Some(self.cheveux), None);
        } else {
            g.ellipse(xf, x, cy - r * 0.55, r * 0.95, r * 0.55, Some(self.cheveux), None);
            g.ellipse(xf, x - r * 0.35, cy, r * 0.12, r * 0.12, Some(NOIR), None);
            g.ellipse(xf, x + r * 0.35, cy, r * 0.12, r * 0.12, Some(NOIR), None);
            g.ellipse(xf, x, cy + r * 0.5, r * 0.25, r * 0.08, Some(rgb(180, 90, 80)), None);
        }
        chapeau(g, xf, self.chapeau, x, cy - r, r);
    }
}

/// Chapeau posé sur une tête de rayon r dont le sommet est en (x, y).
pub fn chapeau(g: &Gfx, xf: &Xf, c: Chapeau, x: f32, y: f32, r: f32) {
    match c {
        Chapeau::Aucun => {}
        Chapeau::HautDeForme => {
            g.ellipse(xf, x, y + r * 0.2, r * 1.5, r * 0.3, Some(NOIR), None);
            g.poly(xf, &rect(x - r * 0.85, y - r * 1.5, r * 1.7, r * 1.7), Some(NOIR), None);
            g.line(xf, x - r * 0.85, y - r * 0.05, x + r * 0.85, y - r * 0.05, r * 0.25, rgb(80, 60, 50));
        }
        Chapeau::Melon => {
            g.ellipse(xf, x, y + r * 0.2, r * 1.4, r * 0.28, Some(NOIR), None);
            g.ellipse(xf, x, y - r * 0.2, r * 0.95, r * 0.75, Some(NOIR), None);
        }
        Chapeau::Paille => {
            g.ellipse(xf, x, y + r * 0.25, r * 2.0, r * 0.4, Some(rgb(226, 200, 130)), Some(rgb(170, 140, 80)));
            g.ellipse(xf, x, y - r * 0.1, r * 0.9, r * 0.55, Some(rgb(226, 200, 130)), None);
            g.line(xf, x - r * 0.9, y + r * 0.1, x + r * 0.9, y + r * 0.1, r * 0.2, ROUGE);
        }
        Chapeau::Casquette => {
            g.ellipse(xf, x, y + r * 0.1, r * 1.0, r * 0.6, Some(rgb(70, 70, 86)), None);
            g.ellipse(xf, x + r * 0.6, y + r * 0.35, r * 0.8, r * 0.2, Some(rgb(50, 50, 60)), None);
        }
        Chapeau::Capeline(c) => {
            g.ellipse(xf, x, y + r * 0.2, r * 1.9, r * 0.45, Some(c), None);
            g.ellipse(xf, x, y - r * 0.15, r * 0.95, r * 0.6, Some(c), None);
            g.ellipse(xf, x + r * 0.6, y - r * 0.3, r * 0.45, r * 0.3, Some(ROSE), None);
        }
        Chapeau::Coiffe => {
            g.ellipse(xf, x, y + r * 0.6, r * 1.25, r * 1.1, Some(BLANC), Some(rgb(200, 196, 186)));
        }
        Chapeau::Kepi => {
            g.poly(xf, &rect(x - r * 0.8, y - r * 0.9, r * 1.6, r * 1.1), Some(ROUGE), None);
            g.poly(xf, &rect(x - r * 0.85, y + r * 0.05, r * 1.7, r * 0.3), Some(BLEU), None);
            g.ellipse(xf, x + r * 0.2, y + r * 0.35, r * 0.9, r * 0.18, Some(NOIR), None);
        }
        Chapeau::Pointu(c) => {
            g.poly(xf, &[(x - r * 1.0, y + r * 0.3), (x + r * 1.0, y + r * 0.3), (x + r * 0.3, y - r * 2.6)], Some(c), None);
            g.ellipse(xf, x + r * 0.3, y - r * 2.6, r * 0.35, r * 0.35, Some(rgb(250, 220, 90)), None);
        }
    }
}

// ---------- animaux ----------
pub fn mouton(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32, tondu: bool) {
    for dx in [-2.6, -1.0, 1.0, 2.6] {
        g.line(xf, x + dx * s, y - 2. * s, x + dx * s, y, 0.5 * s, NOIR);
    }
    let laine = if tondu { rgb(232, 222, 206) } else { rgb(246, 244, 236) };
    g.ellipse(xf, x, y - 3.6 * s, 4.2 * s, 2.6 * s, Some(laine), Some(rgb(170, 166, 156)));
    if !tondu {
        for (dx, dy) in [(-2.5, -5.4), (0.0, -6.0), (2.5, -5.4)] {
            g.ellipse(xf, x + dx * s, y + dy * s, 1.6 * s, 1.2 * s, Some(laine), None);
        }
    }
    g.ellipse(xf, x + 4.4 * s, y - 4.6 * s, 1.4 * s, 1.1 * s, Some(rgb(50, 44, 40)), None);
}

pub fn chien(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32, c: Color) {
    for dx in [-2.6, -1.6, 2.0, 3.0] {
        g.line(xf, x + dx * s, y - 2.5 * s, x + dx * s, y, 0.55 * s, assombrir(c, 0.2));
    }
    g.ellipse(xf, x, y - 3.4 * s, 3.8 * s, 1.6 * s, Some(c), None);
    g.ellipse(xf, x + 3.9 * s, y - 5.0 * s, 1.5 * s, 1.3 * s, Some(c), None);
    g.ellipse(xf, x + 5.3 * s, y - 4.7 * s, 1.0 * s, 0.6 * s, Some(c), None);
    g.ellipse(xf, x + 3.4 * s, y - 4.6 * s, 0.6 * s, 1.2 * s, Some(assombrir(c, 0.35)), None);
    g.line(xf, x - 3.6 * s, y - 3.8 * s, x - 5.2 * s, y - 6.0 * s, 0.5 * s, c);
}

pub fn cheval(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32, c: Color, galop: bool) {
    let jambes: [(f32, f32); 4] = if galop { [(-4.5, -1.5), (-3.0, 1.0), (3.2, 1.5), (4.6, 3.0)] } else { [(-4.0, 0.0), (-2.8, 0.0), (3.0, 0.0), (4.2, 0.0)] };
    for (dx, a) in jambes {
        g.line(xf, x + dx * s, y - 5. * s, x + (dx + a) * s, y, 0.7 * s, assombrir(c, 0.2));
    }
    g.ellipse(xf, x, y - 6.4 * s, 6.0 * s, 2.6 * s, Some(c), None);
    g.poly(xf, &[(x + 4. * s, y - 8. * s), (x + 6.5 * s, y - 12. * s), (x + 8.5 * s, y - 11. * s), (x + 6. * s, y - 6. * s)], Some(c), None);
    g.ellipse(xf, x + 8.6 * s, y - 11.4 * s, 2.0 * s, 1.1 * s, Some(c), None);
    g.line(xf, x + 5. * s, y - 9. * s, x + 7. * s, y - 12.5 * s, 0.8 * s, assombrir(c, 0.5));
    g.line(xf, x - 5.8 * s, y - 7. * s, x - 8. * s, y - 3.5 * s, 0.9 * s, assombrir(c, 0.5));
}

pub fn oiseaux(g: &Gfx, xf: &Xf, pts: &[(f32, f32)]) {
    for &(x, y) in pts {
        g.polyligne(xf, &[(x - 2., y - 1.), (x, y), (x + 2., y - 1.)], 0.45, rgb(50, 40, 40));
    }
}

// ---------- objets ----------
/// Gerbe de blé debout.
pub fn gerbe(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32) {
    g.poly(xf, &[(x - 2. * s, y), (x + 2. * s, y), (x + 3.5 * s, y - 9. * s), (x - 3.5 * s, y - 9. * s)], Some(BLE), Some(assombrir(BLE, 0.3)));
    g.line(xf, x - 2.5 * s, y - 4. * s, x + 2.5 * s, y - 4. * s, 0.6 * s, assombrir(BLE, 0.45));
    g.ellipse(xf, x, y - 9.5 * s, 3.8 * s, 1.6 * s, Some(eclaircir(BLE, 0.15)), None);
}

/// Meule de foin.
pub fn meule(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(x - 7. * s, y);
    pb.cubic_to(x - 7. * s, y - 9. * s, x + 7. * s, y - 9. * s, x + 7. * s, y);
    pb.close();
    if let Some(p) = pb.finish() {
        g.lineaire(xf, &p, (x - 7. * s, 0.), (x + 7. * s, 0.), &[(0.0, eclaircir(BLE, 0.1)), (1.0, assombrir(BLE, 0.25))]);
    }
}

pub fn barque(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32, c: Color) {
    g.poly(xf, &[(x - 12. * s, y - 4. * s), (x + 13. * s, y - 4. * s), (x + 9. * s, y), (x - 8. * s, y)], Some(c), Some(assombrir(c, 0.4)));
    g.line(xf, x - 11. * s, y - 3.3 * s, x + 12. * s, y - 3.3 * s, 0.5 * s, eclaircir(c, 0.3));
}

/// Reflets sur l'eau.
pub fn reflets(g: &Gfx, xf: &Xf, x: f32, y0: f32, y1: f32, c: Color) {
    let mut y = y0;
    let mut k = 0;
    while y < y1 {
        let w = 7.0 - k as f32 * 0.7;
        g.rrect(xf, x - w / 2., y, w.max(1.0), 0.8, 0.4, Some(alpha(c, (0.75 - 0.08 * k as f32).max(0.1))), None);
        y += 2.3;
        k += 1;
    }
}

/// Fanion ou drapeau sur une hampe.
pub fn drapeau(g: &Gfx, xf: &Xf, x: f32, y: f32, h: f32, couleurs: &[Color]) {
    g.line(xf, x, y, x, y - h, 0.6, SEPIA);
    let w = 3.0;
    for (i, &c) in couleurs.iter().enumerate() {
        g.poly(xf, &rect(x + i as f32 * w, y - h, w, h * 0.35), Some(c), None);
    }
}

/// Guirlande de fanions entre deux points.
pub fn guirlande(g: &Gfx, xf: &Xf, x0: f32, y0: f32, x1: f32, y1: f32, n: usize) {
    let cols = [ROUGE, BLANC, BLEU, OCRE, VERT];
    let pts: Vec<(f32, f32)> = (0..=n)
        .map(|k| {
            let t = k as f32 / n as f32;
            (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t + 3.0 * (std::f32::consts::PI * t).sin())
        })
        .collect();
    g.polyligne(xf, &pts, 0.3, SEPIA);
    for k in 0..n {
        let (a, b) = (pts[k], pts[k + 1]);
        let m = ((a.0 + b.0) / 2., (a.1 + b.1) / 2. + 2.6);
        g.poly(xf, &[a, b, m], Some(cols[k % cols.len()]), None);
    }
}

pub fn lanterne(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32) {
    g.lueur(xf, (x, y), 9. * s, &[(0.0, alpha(rgb(255, 210, 120), 0.6)), (1.0, alpha(rgb(255, 210, 120), 0.0))]);
    g.poly(xf, &rect(x - 1.5 * s, y - 2. * s, 3. * s, 4. * s), Some(rgb(255, 222, 140)), Some(SEPIA));
    g.line(xf, x, y - 2. * s, x, y - 3.5 * s, 0.4, SEPIA);
}

/// Cheminée avec un feu.
pub fn cheminee(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32) {
    g.poly(xf, &rect(x - 9. * s, y - 18. * s, 18. * s, 18. * s), Some(rgb(150, 136, 120)), None);
    g.poly(xf, &rect(x - 11. * s, y - 20. * s, 22. * s, 2.5 * s), Some(rgb(120, 104, 90)), None);
    g.poly(xf, &rect(x - 6. * s, y - 12. * s, 12. * s, 12. * s), Some(rgb(40, 26, 20)), None);
    g.lueur(xf, (x, y - 3. * s), 12. * s, &[(0.0, alpha(rgb(255, 170, 60), 0.7)), (1.0, alpha(rgb(255, 120, 40), 0.0))]);
    for (dx, h) in [(-2.5, 6.), (0., 8.), (2.5, 5.5)] {
        let mut pb = PathBuilder::new();
        pb.move_to(x + (dx - 1.6) * s, y - 1. * s);
        pb.quad_to(x + dx * s, y - (h + 2.) * s, x + (dx + 1.6) * s, y - 1. * s);
        pb.close();
        if let Some(p) = pb.finish() {
            g.forme(xf, &p, Some(rgb(255, 180, 60)), None, 0.0);
        }
    }
}

/// Ellipse pleine (raccourci).
pub fn rond(g: &Gfx, xf: &Xf, x: f32, y: f32, r: f32, c: Color) {
    if let Some(p) = chemin_ellipse(x, y, r, r) {
        g.forme(xf, &p, Some(c), None, 0.0);
    }
}
