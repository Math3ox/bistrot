//! Projection isométrique 3D pour les objets posés sur la table (dés, jetons, boutons) : monde (x, y, z)
//! ou plateau (a vers la droite, b vers le joueur, hauteur z) vers l'écran ; cylindres, disques au sol,
//! texte couché sur le tapis, enveloppe convexe (ombres portées).

use crate::gfx::{Align, Color, Face, Gfx, Xf};

const R0: f64 = std::f64::consts::FRAC_1_SQRT_2;
const U0: f64 = 0.40824829046386302;
const U2: f64 = 0.81649658092772604;
/// Écrasement vertical du plateau (1 / racine de 3).
pub const YSQ: f64 = 0.57735026918962576;

/// Projection : `s` pixels logiques par unité du monde, centre de la table en (cx, cy).
#[derive(Clone, Copy)]
pub struct Iso {
    pub s: f64,
    pub cx: f64,
    pub cy: f64,
}

pub type Pt = (f64, f64);

fn pts32(pts: &[Pt]) -> Vec<(f32, f32)> {
    pts.iter().map(|&(x, y)| (x as f32, y as f32)).collect()
}

impl Iso {
    /// Point du monde (x, y, z) -> écran.
    pub fn proj(&self, p: [f64; 3]) -> Pt {
        ((p[0] - p[1]) * R0 * self.s + self.cx, self.cy - ((p[0] + p[1]) * U0 + p[2] * U2) * self.s)
    }

    /// Point du plateau (a vers la droite, b vers le joueur, hauteur z) -> écran.
    pub fn sol(&self, a: f64, b: f64, z: f64) -> Pt {
        (a * self.s + self.cx, b * self.s * YSQ + self.cy - z * self.s * U2)
    }

    /// Contour d'un disque posé sur le plateau (rayon en unités du monde).
    pub fn disque(&self, a: f64, b: f64, z: f64, r: f64) -> Vec<Pt> {
        let (x, y) = self.sol(a, b, z);
        ellipse(x, y, r * self.s, r * self.s * YSQ)
    }

    /// Cylindre debout (jeton, bouton, table) : base, flanc, dessus avec un liseré.
    pub fn cylindre(&self, g: &Gfx, a: f64, b: f64, z0: f64, r: f64, h: f64, dessus: Color, flanc: Color) {
        remplir(g, &self.disque(a, b, z0, r), flanc);
        let (xl, yt) = self.sol(a - r, b, z0 + h);
        let (xr, yb) = self.sol(a + r, b, z0);
        remplir(g, &[(xl, yt), (xr, yt), (xr, yb), (xl, yb)], flanc);
        let t = self.disque(a, b, z0 + h, r);
        remplir(g, &t, dessus);
        contour(g, &t, flanc, 0.6);
    }

    /// Texte couché sur le plateau, centré en (a, b) à la hauteur z.
    pub fn texte_sol(&self, g: &Gfx, s: &str, a: f64, b: f64, z: f64, f: &Face, c: Color) {
        let (x, y) = self.sol(a, b, z);
        g.text_sq(s, x as f32, y as f32, f, c, Align::Center, YSQ as f32);
    }
}

/// Points d'une ellipse (64 côtés).
pub fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Vec<Pt> {
    (0..64)
        .map(|i| {
            let t = i as f64 * std::f64::consts::TAU / 64.0;
            (cx + rx * t.cos(), cy + ry * t.sin())
        })
        .collect()
}

pub fn remplir(g: &Gfx, pts: &[Pt], c: Color) {
    g.poly(&Xf::ID, &pts32(pts), Some(c), None);
}

pub fn contour(g: &Gfx, pts: &[Pt], c: Color, w: f32) {
    g.poly_w(&Xf::ID, &pts32(pts), None, Some(c), w);
}

/// Enveloppe convexe (chaîne monotone).
pub fn enveloppe(mut p: Vec<Pt>) -> Vec<Pt> {
    p.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let cross = |o: Pt, a: Pt, b: Pt| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut h: Vec<Pt> = Vec::new();
    for pass in 0..2 {
        let start = h.len();
        let it: Box<dyn Iterator<Item = &Pt>> = if pass == 0 { Box::new(p.iter()) } else { Box::new(p.iter().rev()) };
        for &q in it {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
    }
    h
}
