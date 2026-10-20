//! Ambiance de la taverne : chope de bière à moitié vide posée sur la table (décor fixe, dessinée avec le
//! fond) et chouette chevêche qui regarde à gauche, à droite, cligne des yeux et penche la tête, en images
//! calculées une fois (comme un GIF) puis rejouées en boucle : entre deux changements d'image, rien n'est
//! redessiné (environ un changement par seconde). Animations coupées : la chouette reste immobile.

use crate::anim;
use crate::gfx::{alpha, chemin_ellipse, chemin_poly, rgb, Color, Gfx, Glyphs, Hasard, Xf};
use std::cell::RefCell;
use tiny_skia::{PathBuilder, Pixmap};

/// Pied de la chope et pattes de la chouette, posées de part et d'autre de la table (près du joueur).
pub const PLACE_CHOPE: (f32, f32) = (905.0, 515.0);
pub const PLACE_CHOUETTE: (f32, f32) = (190.0, 534.0);

const NOIR: Color = rgb(0, 0, 0);
const BLANC: Color = rgb(255, 255, 255);

/// Points du bord d'une ellipse, de l'angle a0 à a1 (n segments).
fn arc(cx: f32, cy: f32, rx: f32, ry: f32, a0: f32, a1: f32, n: usize) -> Vec<(f32, f32)> {
    (0..=n)
        .map(|k| {
            let a = a0 + (a1 - a0) * k as f32 / n as f32;
            (cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}

/// Chope en verre à moitié vide, pied en (x, y) : bière ambrée et ses bulles, belle mousse bombée, traces de
/// mousse sur la paroi, anse, reflets ; ombre et reflet ambré sur la table.
pub fn chope(g: &Gfx, x: f32, y: f32) {
    let id = Xf::ID;
    let pi = std::f32::consts::PI;
    let (r, ry, h) = (23.0f32, 8.5f32, 66.0f32);
    let (ri, ryi) = (r - 2.6, ry - 1.0);
    let fond = y - 7.0; // dessus du fond épais du verre
    let niveau = y - 33.0; // surface de la bière
    let mousse = niveau - 10.0; // dessus de la mousse
    let haut = y - h;
    // ombre et lumière ambrée passée à travers la bière
    g.ellipse(&id, x + 13.0, y + 5.0, 33.0, 11.0, Some(alpha(NOIR, 0.38)), None);
    let xf = Xf { a: 1.0, b: 0.0, c: 0.0, d: 0.38, tx: x + 30.0, ty: y + 4.0 };
    g.lueur(&xf, (0.0, 0.0), 30.0, &[(0.0, alpha(rgb(255, 168, 56), 0.4)), (1.0, alpha(rgb(255, 168, 56), 0.0))]);
    // silhouette du verre (un peu de blanc : le verre se voit)
    let mut verre = vec![(x - r, haut), (x - r, y)];
    verre.extend(arc(x, y, r, ry, pi, 0.0, 24));
    verre.extend([(x + r, y), (x + r, haut)]);
    verre.extend(arc(x, haut, r, ry, 0.0, -pi, 24));
    g.poly(&id, &verre, Some(alpha(BLANC, 0.1)), None);
    // traces de mousse sur la paroi du fond, au-dessus de la bière
    let mut hz = Hasard(0xC40E);
    for k in 0..3 {
        let yy = mousse - 6.0 - 7.5 * k as f32;
        let ph = hz.entre(0.0, 6.0);
        let pts: Vec<(f32, f32)> = arc(x, yy, ri, ryi, -pi, 0.0, 24)
            .into_iter()
            .enumerate()
            .map(|(i, (px, py))| (px, py + 1.1 * (i as f32 * 0.9 + ph).sin()))
            .collect();
        g.polyligne(&id, &pts, 1.0, alpha(BLANC, 0.24 - 0.06 * k as f32));
    }
    // bière : colonne ambrée (plus claire au milieu), bulles qui montent
    let mut biere = vec![(x - ri, niveau), (x - ri, fond)];
    biere.extend(arc(x, fond, ri, ryi, pi, 0.0, 24));
    biere.push((x + ri, niveau));
    biere.extend(arc(x, niveau, ri, ryi, 0.0, pi, 24));
    if let Some(p) = chemin_poly(&biere) {
        let arrets = [(0.0, rgb(116, 48, 4)), (0.3, rgb(212, 124, 22)), (0.55, rgb(248, 180, 56)), (1.0, rgb(104, 42, 4))];
        g.lineaire(&id, &p, (x - ri, 0.0), (x + ri, 0.0), &arrets);
    }
    for _ in 0..16 {
        let (bx, by) = (x + hz.entre(-0.8, 0.8) * ri, hz.entre(niveau + 4.0, fond + 4.0));
        let rb = hz.entre(0.5, 1.2);
        g.ellipse(&id, bx, by, rb, rb, Some(alpha(rgb(255, 238, 176), 0.7)), None);
    }
    // mousse : flanc crème, dessus bombé et ses grosses bulles
    let mut flanc = vec![(x - ri, mousse), (x - ri, niveau)];
    flanc.extend(arc(x, niveau, ri, ryi, pi, 0.0, 24));
    flanc.push((x + ri, mousse));
    if let Some(p) = chemin_poly(&flanc) {
        let arrets = [(0.0, rgb(206, 190, 158)), (0.4, rgb(252, 247, 232)), (1.0, rgb(192, 174, 140))];
        g.lineaire(&id, &p, (x - ri, 0.0), (x + ri, 0.0), &arrets);
    }
    if let Some(p) = chemin_ellipse(x, mousse, ri + 0.6, ryi + 0.6) {
        g.radial(&id, &p, (x - 4.0, mousse - 2.0), ri + 4.0, &[(0.0, rgb(255, 253, 244)), (0.7, rgb(240, 230, 206)), (1.0, rgb(214, 198, 166))]);
    }
    g.ellipse(&id, x - 1.5, mousse - 2.2, ri * 0.75, ryi * 0.7, Some(alpha(BLANC, 0.55)), None);
    for _ in 0..13 {
        let a = hz.entre(0.0, 2.0 * pi);
        let d = hz.f().sqrt() * 0.85;
        let (bx, by) = (x + ri * d * a.cos(), mousse - 1.0 + ryi * d * a.sin());
        let rb = hz.entre(1.2, 2.8);
        g.ellipse(&id, bx, by, rb, rb * 0.62, Some(rgb(250, 244, 228)), Some(alpha(rgb(196, 178, 142), 0.6)));
    }
    // mousse qui déborde un peu sur la paroi de devant
    for (dx, l) in [(-12.0, 5.0), (6.0, 7.5), (14.0, 4.0)] {
        let yb = niveau + ryi * (1.0 - (dx / ri) * (dx / ri)).max(0.0).sqrt();
        g.rrect(&id, x + dx - 1.6, yb - 1.0, 3.2, l, 1.6, Some(alpha(rgb(250, 244, 228), 0.85)), None);
    }
    // verre de devant : fond épais teinté d'ambre, bords, reflets, bord du haut
    let mut socle = vec![(x - r, fond), (x - r, y)];
    socle.extend(arc(x, y, r, ry, pi, 0.0, 24));
    socle.push((x + r, fond));
    socle.extend(arc(x, fond, r, ry, 0.0, pi, 24));
    g.poly(&id, &socle, Some(alpha(rgb(255, 200, 120), 0.22)), None);
    g.polyligne(&id, &arc(x, y, r, ry, 0.0, pi, 24), 1.4, alpha(BLANC, 0.5));
    g.polyligne(&id, &arc(x, fond, ri, ryi, 0.0, pi, 24), 0.8, alpha(BLANC, 0.3));
    g.line(&id, x - r, haut, x - r, y, 1.2, alpha(BLANC, 0.55));
    g.line(&id, x + r, haut, x + r, y, 1.2, alpha(BLANC, 0.45));
    g.rrect(&id, x - r + 3.5, haut + 7.0, 3.6, h - 16.0, 1.8, Some(alpha(BLANC, 0.42)), None);
    g.rrect(&id, x + r - 7.5, haut + 9.0, 1.6, h - 22.0, 0.8, Some(alpha(BLANC, 0.28)), None);
    for k in 1..5 {
        let fx = x - r + 2.0 * r * k as f32 / 5.0;
        g.line(&id, fx, haut + 6.0, fx, y - 3.0, 0.6, alpha(BLANC, 0.1));
    }
    if let Some(p) = chemin_ellipse(x, haut, r, ry) {
        g.forme(&id, &p, None, Some(alpha(BLANC, 0.75)), 1.5);
    }
    if let Some(p) = chemin_ellipse(x, haut + 0.6, ri, ryi) {
        g.forme(&id, &p, None, Some(alpha(BLANC, 0.3)), 0.9);
    }
    // anse en verre épais
    let mut pb = PathBuilder::new();
    pb.move_to(x + r - 2.0, haut + 12.0);
    pb.cubic_to(x + r + 19.0, haut + 9.0, x + r + 21.0, y - 15.0, x + r - 2.0, y - 13.0);
    if let Some(p) = pb.finish() {
        g.forme(&id, &p, None, Some(alpha(BLANC, 0.2)), 7.5);
        g.forme(&id, &p, None, Some(alpha(rgb(255, 214, 150), 0.18)), 3.5);
    }
    let mut pb = PathBuilder::new();
    pb.move_to(x + r + 1.0, haut + 11.0);
    pb.cubic_to(x + r + 19.5, haut + 9.5, x + r + 20.0, y - 20.0, x + r + 6.0, y - 15.0);
    if let Some(p) = pb.finish() {
        g.forme(&id, &p, None, Some(alpha(BLANC, 0.6)), 1.1);
    }
}

// ---------- chouette ----------

/// Poses de la chouette : (tête tournée de -1 à 1, paupières de 0 à 1, tête penchée en radians).
const POSES: [(f32, f32, f32); 8] = [
    (0.0, 0.0, 0.0),
    (-0.5, 0.0, 0.0),
    (-1.0, 0.0, 0.0),
    (0.5, 0.0, 0.0),
    (1.0, 0.0, 0.0),
    (0.0, 0.55, 0.0),
    (0.0, 1.0, 0.0),
    (0.0, 0.0, 0.24),
];

/// La boucle (comme un GIF) : (pose, durée en ms).
const SEQUENCE: [(usize, f64); 19] = [
    (0, 2600.0),
    (1, 90.0),
    (2, 1700.0),
    (1, 90.0),
    (0, 80.0),
    (3, 90.0),
    (4, 1900.0),
    (3, 90.0),
    (0, 1500.0),
    (5, 70.0),
    (6, 120.0),
    (5, 70.0),
    (0, 2400.0),
    (7, 1500.0),
    (0, 1800.0),
    (5, 70.0),
    (6, 120.0),
    (5, 70.0),
    (0, 900.0),
];

/// Taille de la chouette sur la table (unités du dessin -> unités logiques).
const TAILLE: f32 = 1.3;
/// Cadre d'une image de la chouette autour de ses pattes (unités du dessin).
const BX0: f32 = -34.0;
const BY0: f32 = -90.0;
const BX1: f32 = 34.0;
const BY1: f32 = 9.0;

/// Chouette chevêche : temps dans la boucle et images déjà calculées (à l'échelle de la fenêtre).
#[derive(Default)]
pub struct Chouette {
    ms: f64,
    images: RefCell<(f32, Vec<Option<Pixmap>>)>,
}

impl Chouette {
    /// Avance dans la boucle (rien si les animations sont coupées).
    pub fn avancer(&mut self, dt: f64) {
        if anim::actives() {
            let cycle: f64 = SEQUENCE.iter().map(|s| s.1).sum();
            self.ms = (self.ms + dt) % cycle;
        }
    }

    /// Image à montrer en ce moment (0 = de face, immobile si les animations sont coupées).
    pub fn image(&self) -> usize {
        if !anim::actives() {
            return 0;
        }
        let mut t = self.ms;
        for &(i, d) in &SEQUENCE {
            if t < d {
                return i;
            }
            t -= d;
        }
        0
    }

    /// Pose la chouette, pattes en (x, y).
    pub fn dessiner(&self, g: &Gfx, x: f32, y: f32) {
        let ppu = g.sc;
        let i = self.image();
        let mut c = self.images.borrow_mut();
        if c.0 != ppu || c.1.len() != POSES.len() {
            *c = (ppu, vec![None; POSES.len()]);
        }
        if c.1[i].is_none() {
            c.1[i] = rendre(g.glyphs(), POSES[i], ppu);
        }
        if let Some(img) = &c.1[i] {
            g.image(&Xf::centre(x, y, 1.0), img, BX0 * TAILLE, BY0 * TAILLE, ppu);
        }
    }

    /// Oublie les images calculées (elles seront refaites à la demande).
    pub fn alleger(&self) {
        *self.images.borrow_mut() = (0.0, Vec::new());
    }

    /// Mémoire des images (octets).
    pub fn octets(&self) -> usize {
        self.images.borrow().1.iter().flatten().map(|p| p.data().len()).sum()
    }
}

/// Image d'une pose, à `ppu` pixels par unité logique.
fn rendre(glyphs: &RefCell<Glyphs>, pose: (f32, f32, f32), ppu: f32) -> Option<Pixmap> {
    let ppu = ppu * TAILLE;
    let (w, h) = (((BX1 - BX0) * ppu).ceil() as u32, ((BY1 - BY0) * ppu).ceil() as u32);
    let mut pm = Pixmap::new(w.max(1), h.max(1))?;
    {
        let g = Gfx::new(&mut pm, glyphs, ppu, -BX0 * ppu, -BY0 * ppu);
        chouette(&g, pose);
    }
    Some(pm)
}

/// Paupière qui couvre l'œil et son cerne (ellipse) jusqu'à la fraction `p` de sa hauteur ; fermé, un pli.
fn paupiere(g: &Gfx, xf: &Xf, ex: f32, ey: f32, rx: f32, ry: f32, p: f32, c: Color) {
    let pi = std::f32::consts::PI;
    let pli = rgb(60, 40, 24);
    if p >= 0.95 {
        if let Some(e) = chemin_ellipse(ex, ey, rx, ry) {
            g.forme(xf, &e, Some(c), None, 0.0);
        }
        g.polyligne(xf, &arc(ex, ey - 1.2, rx * 0.78, ry * 0.42, 0.25, pi - 0.25, 12), 0.9, pli);
        return;
    }
    let s = (2.0 * p - 1.0).clamp(-1.0, 1.0);
    let (a0, a1) = (-pi - s.asin(), s.asin());
    g.poly(xf, &arc(ex, ey, rx, ry, a0, a1, 20), Some(c), None);
    let y = ey + ry * s;
    let w = rx * (1.0 - s * s).max(0.0).sqrt();
    g.line(xf, ex - w, y, ex + w, y, 0.8, pli);
}

/// Chouette chevêche assise, pattes en (0, 0) : corps tacheté éclairé en haut à gauche, ventre clair rayé,
/// ailes tachetées de blanc, tête ronde (tournée, penchée), sourcils blancs, grands yeux jaunes, bec.
fn chouette(g: &Gfx, (tour, ferme, penche): (f32, f32, f32)) {
    let id = Xf::ID;
    let brun = rgb(118, 84, 52);
    let brun_f = rgb(66, 45, 28);
    let tache = rgb(238, 228, 208);
    // ombre et queue
    g.ellipse(&id, 2.0, 1.5, 25.0, 6.5, Some(alpha(NOIR, 0.42)), None);
    g.poly(&id, &[(-7.0, -10.0), (7.0, -10.0), (4.0, 1.5), (-4.0, 1.5)], Some(brun_f), None);
    // corps
    if let Some(p) = chemin_ellipse(0.0, -30.0, 22.0, 28.0) {
        g.radial(&id, &p, (-8.0, -46.0), 42.0, &[(0.0, rgb(166, 124, 82)), (0.55, brun), (1.0, brun_f)]);
    }
    // ventre clair rayé de brun
    if let Some(p) = chemin_ellipse(0.0, -23.0, 13.5, 19.0) {
        g.radial(&id, &p, (-3.0, -30.0), 22.0, &[(0.0, rgb(240, 228, 204)), (1.0, rgb(196, 172, 134))]);
    }
    for row in 0..4 {
        for col in -1..=1 {
            let (sx, sy) = (col as f32 * 6.5 + if row % 2 == 1 { 3.2 } else { 0.0 } - 1.0, -34.0 + row as f32 * 7.5);
            if (sx / 12.0).powi(2) + ((sy + 23.0) / 17.0).powi(2) < 1.0 {
                g.ellipse(&id, sx, sy, 1.3, 2.6, Some(alpha(rgb(118, 80, 46), 0.75)), None);
            }
        }
    }
    // ailes repliées sur les flancs, tachetées de blanc
    for s in [-1.0f32, 1.0] {
        let mut pb = PathBuilder::new();
        pb.move_to(s * 10.0, -50.0);
        pb.cubic_to(s * 23.0, -50.0, s * 25.5, -26.0, s * 15.0, -4.0);
        pb.cubic_to(s * 11.5, -16.0, s * 9.0, -34.0, s * 10.0, -50.0);
        pb.close();
        if let Some(p) = pb.finish() {
            g.radial(&id, &p, (s * 12.0, -44.0), 44.0, &[(0.0, rgb(132, 94, 60)), (1.0, rgb(78, 53, 32))]);
            g.forme(&id, &p, None, Some(alpha(brun_f, 0.7)), 0.6);
        }
        for (dx, dy) in [(15.0, -42.0), (19.0, -36.0), (15.5, -31.0), (19.5, -25.0), (16.0, -19.0), (18.0, -12.0)] {
            g.ellipse(&id, s * dx, dy, 1.3, 1.0, Some(alpha(tache, 0.9)), None);
        }
    }
    // pattes et serres
    for s in [-1.0f32, 1.0] {
        for k in -1..=1 {
            let kx = s * 7.0 + k as f32 * 2.3;
            g.ellipse(&id, kx, -1.5, 1.2, 2.3, Some(rgb(208, 192, 142)), Some(rgb(116, 100, 66)));
            g.ellipse(&id, kx, 0.9, 0.6, 0.6, Some(rgb(40, 30, 22)), None);
        }
    }
    // tête : tournée de `tour`, penchée de `penche` autour du cou
    let tete = Xf::place(0.0, -50.0, 1.0, penche as f64, 1.0).puis(&Xf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 50.0 });
    let hx = 3.0 * tour;
    if let Some(p) = chemin_ellipse(hx, -64.0, 21.0, 17.0) {
        g.radial(&tete, &p, (hx - 8.0, -75.0), 31.0, &[(0.0, rgb(160, 120, 80)), (0.65, brun), (1.0, brun_f)]);
    }
    for (dx, dy) in [(-11.0, -72.0), (-5.0, -77.0), (2.0, -78.0), (9.0, -74.0), (-14.0, -65.0), (14.0, -67.0), (-6.0, -71.0), (5.0, -72.0)] {
        g.ellipse(&tete, hx + dx + 2.5 * tour, dy, 1.1, 0.9, Some(alpha(tache, 0.85)), None);
    }
    // disque facial, plus clair
    let fx = hx + 6.5 * tour;
    g.ellipse(&tete, fx, -61.0, 15.5 * (1.0 - 0.15 * tour.abs()), 10.5, Some(rgb(184, 156, 118)), None);
    // yeux : celui du côté opposé au regard est vu de biais
    let ecart = 7.2 * (1.0 - 0.22 * tour.abs());
    for (s, loin) in [(-1.0f32, tour > 0.0), (1.0, tour < 0.0)] {
        let k = if loin { 1.0 - 0.32 * tour.abs() } else { 1.0 };
        let (ex, ey, rx, ry) = (fx + s * ecart, -62.0, 5.4 * k, 5.4);
        g.polyligne(&tete, &arc(ex, ey - 0.8, rx + 2.8, ry + 2.6, -2.75, -0.45, 12), 1.7, alpha(tache, 0.95));
        g.ellipse(&tete, ex, ey, rx + 1.2, ry + 1.2, Some(rgb(44, 30, 18)), None);
        if let Some(p) = chemin_ellipse(ex, ey, rx, ry) {
            g.radial(&tete, &p, (ex - 1.0, ey - 1.2), 6.5, &[(0.0, rgb(255, 230, 96)), (1.0, rgb(232, 148, 18))]);
        }
        g.ellipse(&tete, ex + 1.5 * tour * k, ey + 0.3, 2.6 * k, 2.8, Some(rgb(14, 10, 8)), None);
        g.ellipse(&tete, ex - 1.6 * k, ey - 1.9, 0.95, 0.95, Some(alpha(BLANC, 0.9)), None);
        if ferme > 0.0 {
            paupiere(g, &tete, ex, ey, rx + 1.3, ry + 1.3, ferme, rgb(146, 110, 74));
        }
    }
    // bec
    g.poly(&tete, &[(fx - 2.5, -58.5), (fx + 2.5, -58.5), (fx + 0.5 * tour, -52.5)], Some(rgb(206, 192, 130)), Some(rgb(108, 94, 58)));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Planche agrandie des poses de la chouette et de la chope (cargo test --release -p bistrot -- --ignored planche) :
    /// target/planche_ambiance.png.
    #[test]
    #[ignore]
    fn planche() {
        let k = 5.0;
        let glyphs = RefCell::new(Glyphs::default());
        let (w, h) = (((BX1 - BX0) * POSES.len() as f32 + 80.0) * k, (BY1 - BY0) * k);
        let mut pm = Pixmap::new(w as u32, h as u32).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(120, 80, 44, 255));
        {
            let g = Gfx::new(&mut pm, &glyphs, k, 0.0, 0.0);
            for (i, &pose) in POSES.iter().enumerate() {
                let img = rendre(&glyphs, pose, k / TAILLE).unwrap();
                g.image(&Xf::centre(-BX0 + i as f32 * (BX1 - BX0), -BY0, 1.0), &img, BX0, BY0, k);
            }
            chope(&g, (BX1 - BX0) * POSES.len() as f32 + 34.0, -BY0 - 4.0);
        }
        pm.save_png("../target/planche_ambiance.png").unwrap();
    }
}
