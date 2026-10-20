//! Décor de taverne en perspective : plancher de grosses planches dans la pénombre, table ronde en chêne
//! cerclée de fer, chandelles qui éclairent la table ; et places des joueurs autour, de 2 à 5 joueurs.
//! Le joueur de cet ordinateur est toujours en bas ; les autres sont répartis dans le sens du jeu.
//! Le bois et la lumière demandent ~70 ms de dessin en fenêtre (~130 ms en plein écran) : `Decor` les garde
//! en cache en pleine résolution mais en niveaux de lumière (1 octet par pixel et une palette de 256 couleurs
//! tirée du dessin lui-même, le bois étant d'une seule teinte), et les recopie en ~1 ms ; le fer et les
//! chandelles, d'autres teintes, sont redessinés par-dessus. Textures pseudo-aléatoires à graine fixe : le
//! décor est identique à chaque dessin.

use crate::gfx::{alpha, chemin_ellipse, chemin_poly, mix, rgb, Color, Gfx, Hasard, Xf};
use tiny_skia::{Pixmap, PremultipliedColorU8};
use std::f64::consts::FRAC_PI_2;

const C_NUIT: Color = rgb(14, 9, 5);
const C_PLANCHER_A: Color = rgb(50, 32, 18);
const C_PLANCHER_B: Color = rgb(76, 49, 27);
const C_CHENE: Color = rgb(150, 99, 52);
const C_CHENE_FONCE: Color = rgb(110, 69, 34);
const C_CHANT_HI: Color = rgb(98, 62, 31);
const C_CHANT_LO: Color = rgb(34, 20, 9);
const C_FER: Color = rgb(40, 38, 40);
const C_FER_HI: Color = rgb(128, 122, 116);
const C_LUMIERE: Color = rgb(255, 196, 112);
const C_BLANC: Color = rgb(255, 255, 255);
const C_NOIR: Color = rgb(0, 0, 0);

/// Écrasement vertical du plan de la table (vue isométrique).
pub const SOL: f64 = 0.62;

/// Plateau de la table : centre, rayons, épaisseur vue en perspective.
const TX: f32 = 550.0;
const TY: f32 = 352.0;
const RX: f32 = 500.0;
const RY: f32 = 280.0;
const EP: f32 = 24.0;

/// Pied des chandelles posées sur la table.
pub const CHANDELLES: [(f32, f32); 2] = [(205.0, 196.0), (895.0, 196.0)];

fn eclaircir(c: Color, k: f32) -> Color {
    mix(c, C_BLANC, k)
}

fn assombrir(c: Color, k: f32) -> Color {
    mix(c, C_NOIR, k)
}

/// Décor complet dessiné sans cache : plancher, pénombre, table, fer, chandelles.
pub fn dessiner(g: &Gfx) {
    bois(g);
    details(g);
}

/// Le bois et la lumière (une seule teinte : mis en cache en niveaux de lumière).
fn bois(g: &Gfx) {
    plancher(g);
    penombre(g);
    table(g);
}

/// Ce qui n'est pas de la teinte du bois : cercle de fer, arête éclairée du plateau, chandelles.
fn details(g: &Gfx) {
    fer(g);
    for &(x, y) in &CHANDELLES {
        chandelle(g, x, y);
    }
}

/// Décor en cache : niveau de lumière de chaque pixel et couleur moyenne de chaque niveau.
#[derive(Default)]
pub struct Decor {
    taille: (u32, u32),
    lum: Vec<u8>,
    palette: Vec<PremultipliedColorU8>,
}

impl Decor {
    /// Dessine le décor dans l'image de g (depuis le cache s'il est à la bonne taille).
    pub fn dessiner(&mut self, g: &Gfx) {
        let taille = g.snapshot_size();
        if self.taille == taille && !self.lum.is_empty() {
            g.pixels(|pm| self.restituer(pm));
        } else {
            self.alleger();
            bois(g);
            g.pixels(|pm| self.capturer(pm));
            self.taille = taille;
        }
        details(g);
    }

    /// Oublie le cache (il sera refait au prochain dessin).
    pub fn alleger(&mut self) {
        *self = Decor::default();
    }

    /// Mémoire occupée (octets).
    pub fn octets(&self) -> usize {
        self.lum.capacity() + self.palette.capacity() * 4
    }

    fn capturer(&mut self, pm: &Pixmap) {
        let px = pm.pixels();
        let mut somme = [[0u64; 4]; 256];
        let mut lum = Vec::with_capacity(px.len());
        for p in px {
            let (r, g, b) = (p.red() as u32, p.green() as u32, p.blue() as u32);
            let l = ((54 * r + 183 * g + 19 * b) >> 8).min(255) as usize;
            lum.push(l as u8);
            let s = &mut somme[l];
            s[0] += r as u64;
            s[1] += g as u64;
            s[2] += b as u64;
            s[3] += 1;
        }
        // couleur moyenne de chaque niveau (niveaux absents : celle du niveau présent le plus proche)
        let present: Vec<usize> = (0..256).filter(|&l| somme[l][3] > 0).collect();
        self.palette = (0..256)
            .map(|l| {
                let k = present.iter().copied().min_by_key(|&k| k.abs_diff(l)).unwrap_or(l);
                let s = somme[k];
                let n = s[3].max(1);
                let c = |v: u64| (v / n).min(255) as u8;
                PremultipliedColorU8::from_rgba(c(s[0]), c(s[1]), c(s[2]), 255).unwrap_or(PremultipliedColorU8::TRANSPARENT)
            })
            .collect();
        self.lum = lum;
    }

    fn restituer(&self, pm: &mut Pixmap) {
        for (d, &l) in pm.pixels_mut().iter_mut().zip(&self.lum) {
            *d = self.palette[l as usize];
        }
    }
}

/// Fil du bois : lignes ondulées entre x0 et x1 autour de y, gardées dans [ymin, ymax] et dans la largeur
/// du plateau si `plateau`.
fn fil(g: &Gfx, h: &mut Hasard, x0: f32, x1: f32, y: f32, ymin: f32, ymax: f32, plateau: bool, c: Color) {
    let (amp, freq, phase, pente) = (h.entre(0.4, 2.2), h.entre(0.004, 0.013), h.entre(0.0, 6.3), h.entre(-0.004, 0.004));
    let w = h.entre(0.6, 1.3);
    let mut pts = Vec::with_capacity(96);
    let mut x = x0;
    while x <= x1 {
        let yy = (y + amp * (x * freq + phase).sin() + pente * (x - x0)).clamp(ymin, ymax);
        let dedans = !plateau || (x - TX).abs() < demi(yy) - 3.0;
        if dedans {
            pts.push((x, yy));
        } else if pts.len() > 1 {
            g.polyligne(&Xf::ID, &pts, w, c);
            pts.clear();
        } else {
            pts.clear();
        }
        x += 12.0;
    }
    if pts.len() > 1 {
        g.polyligne(&Xf::ID, &pts, w, c);
    }
}

/// Nœud du bois (ellipse sombre et cernes), écrasé par la perspective.
fn noeud(g: &Gfx, h: &mut Hasard, x: f32, y: f32, c: Color) {
    let (rx, ry) = (h.entre(5.0, 11.0), h.entre(2.4, 4.4));
    g.ellipse(&Xf::ID, x, y, rx, ry, Some(alpha(c, 0.5)), None);
    g.ellipse(&Xf::ID, x, y, rx * 0.45, ry * 0.45, Some(alpha(c, 0.6)), None);
    for k in 1..3 {
        let e = k as f32 * 3.5;
        g.ellipse(&Xf::ID, x, y, rx + e * 1.6, ry + e * 0.6, None, Some(alpha(c, 0.22)));
    }
}

/// Tête de clou forgé.
fn clou(g: &Gfx, x: f32, y: f32) {
    g.ellipse(&Xf::ID, x, y, 2.6, 1.8, Some(rgb(26, 22, 20)), None);
    g.ellipse(&Xf::ID, x - 0.6, y - 0.5, 1.0, 0.6, Some(alpha(C_FER_HI, 0.55)), None);
}

/// Plancher de grosses planches posées en quinconce, clouées aux extrémités (sur toute l'image).
fn plancher(g: &Gfx) {
    let id = Xf::ID;
    let (vx0, vy0, vx1, vy1) = g.visible();
    g.rrect(&id, vx0 - 2., vy0 - 2., vx1 - vx0 + 4., vy1 - vy0 + 4., 0., Some(C_NUIT), None);
    let mut h = Hasard(0x9E37_79B9_7F4A_7C15);
    let large = 52.0;
    let mut y = -16.0 + ((vy0 + 16.0) / large).floor() * large;
    while y < vy1 {
        let mut x = vx0 - h.entre(0.0, 420.0);
        while x < vx1 {
            let long = h.entre(320.0, 640.0);
            let base = mix(C_PLANCHER_A, C_PLANCHER_B, h.f());
            g.rrect_degrade(&id, x + 1.2, y + 1.2, long - 2.4, large - 2.4, 3., eclaircir(base, 0.07), assombrir(base, 0.16), None);
            for _ in 0..7 {
                let yy = y + h.entre(5.0, large - 5.0);
                let c = if h.f() < 0.75 { alpha(assombrir(base, 0.55), 0.35) } else { alpha(eclaircir(base, 0.3), 0.18) };
                fil(g, &mut h, x + 5.0, x + long - 5.0, yy, y + 3.0, y + large - 3.0, false, c);
            }
            if h.f() < 0.4 {
                let (nx, ny) = (x + h.entre(40.0, long - 40.0), y + h.entre(14.0, large - 14.0));
                noeud(g, &mut h, nx, ny, assombrir(base, 0.6));
            }
            for cx in [x + 11.0, x + long - 11.0] {
                clou(g, cx, y + 13.0);
                clou(g, cx, y + large - 13.0);
            }
            x += long;
        }
        y += large;
    }
}

/// Pénombre de la salle : chaude près de la table, noire dans les coins (sur toute l'image).
fn penombre(g: &Gfx) {
    let xf = Xf { a: 1.0, b: 0.0, c: 0.0, d: 0.72, tx: TX, ty: 380.0 };
    let (vx0, vy0, vx1, vy1) = g.visible();
    let (x0, x1) = (vx0 - TX - 2.0, vx1 - TX + 2.0);
    let (y0, y1) = ((vy0 - 380.0) / 0.72 - 2.0, (vy1 - 380.0) / 0.72 + 2.0);
    if let Some(p) = chemin_poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]) {
        let stops = [
            (0.0, alpha(C_LUMIERE, 0.08)),
            (0.42, alpha(C_NUIT, 0.2)),
            (0.72, alpha(C_NUIT, 0.64)),
            (1.0, alpha(C_NUIT, 0.93)),
        ];
        g.radial(&xf, &p, (0.0, 0.0), 770.0, &stops);
    }
}

/// Demi-largeur du plateau à la hauteur y.
fn demi(y: f32) -> f32 {
    let t = ((y - TY) / RY).clamp(-1.0, 1.0);
    RX * (1.0 - t * t).max(0.0).sqrt()
}

/// Table ronde en chêne : ombre portée, chant cerclé de fer et riveté, plateau en planches éclairé au centre.
fn table(g: &Gfx) {
    let id = Xf::ID;
    let ell = |cy: f32| Xf { a: 1.0, b: 0.0, c: 0.0, d: RY / RX, tx: TX, ty: cy };
    // ombre portée sur le plancher
    g.lueur(&ell(TY + EP + 30.0), (0.0, 0.0), RX + 80.0, &[(0.0, alpha(C_NUIT, 0.9)), (0.84, alpha(C_NUIT, 0.8)), (1.0, alpha(C_NUIT, 0.0))]);
    // chant du plateau (arrondi : clair au milieu, sombre sur les côtés)
    if let Some(p) = chemin_ellipse(TX, TY + EP, RX, RY) {
        g.lineaire(&id, &p, (TX - RX, 0.0), (TX + RX, 0.0), &[(0.0, C_CHANT_LO), (0.5, C_CHANT_HI), (1.0, C_CHANT_LO)]);
    }
    // plateau : planches de chêne d'un bord à l'autre
    let mut h = Hasard(0x2545_F491_4F6C_DD1D);
    g.ellipse(&id, TX, TY, RX, RY, Some(C_CHENE), None);
    let n = 12;
    let pas = 2.0 * RY / n as f32;
    for k in 0..n {
        let (y0, y1) = (TY - RY + k as f32 * pas, TY - RY + (k + 1) as f32 * pas);
        let mut pts = Vec::with_capacity(34);
        for i in 0..=16 {
            let y = y0 + (y1 - y0) * i as f32 / 16.0;
            pts.push((TX + demi(y), y));
        }
        for i in (0..=16).rev() {
            let y = y0 + (y1 - y0) * i as f32 / 16.0;
            pts.push((TX - demi(y), y));
        }
        let base = mix(C_CHENE_FONCE, eclaircir(C_CHENE, 0.06), h.f());
        if let Some(p) = chemin_poly(&pts) {
            g.lineaire(&id, &p, (0.0, y0), (0.0, y1), &[(0.0, eclaircir(base, 0.08)), (0.55, base), (1.0, assombrir(base, 0.14))]);
        }
        for _ in 0..8 {
            let yy = h.entre(y0 + 3.0, y1 - 3.0);
            let c = if h.f() < 0.7 { alpha(assombrir(base, 0.5), 0.42) } else { alpha(eclaircir(base, 0.35), 0.2) };
            let d = demi(yy);
            fil(g, &mut h, TX - d, TX + d, yy, y0 + 2.5, y1 - 2.5, true, c);
        }
        if k > 0 {
            // joint entre deux planches
            let d = demi(y0) - 3.0;
            g.line(&id, TX - d, y0, TX + d, y0, 1.6, alpha(rgb(30, 17, 8), 0.85));
            g.line(&id, TX - d, y0 + 1.4, TX + d, y0 + 1.4, 0.7, alpha(eclaircir(base, 0.4), 0.25));
        }
    }
    for _ in 0..6 {
        let y = h.entre(TY - RY * 0.8, TY + RY * 0.8);
        let x = TX + h.entre(-0.8, 0.8) * demi(y);
        noeud(g, &mut h, x, y, rgb(60, 36, 16));
    }
    // lumière des chandelles : flaques chaudes, centre éclairé, bords dans l'ombre
    g.lueur(&ell(TY - 10.0), (0.0, 0.0), 470.0, &[(0.0, alpha(C_LUMIERE, 0.40)), (0.55, alpha(C_LUMIERE, 0.14)), (1.0, alpha(C_LUMIERE, 0.0))]);
    for &(x, y) in &CHANDELLES {
        let xf = Xf { a: 1.0, b: 0.0, c: 0.0, d: 0.6, tx: x, ty: y };
        g.lueur(&xf, (0.0, 0.0), 210.0, &[(0.0, alpha(C_LUMIERE, 0.48)), (0.4, alpha(C_LUMIERE, 0.16)), (1.0, alpha(C_LUMIERE, 0.0))]);
    }
    if let Some(p) = chemin_ellipse(0.0, 0.0, RX, RX) {
        g.radial(&ell(TY), &p, (0.0, 0.0), RX, &[(0.0, alpha(C_NUIT, 0.0)), (0.62, alpha(C_NUIT, 0.0)), (1.0, alpha(C_NUIT, 0.62))]);
    }
}

/// Cercle de fer riveté autour du chant (moitié visible, devant le plateau) et arête du plateau éclairée.
fn fer(g: &Gfx) {
    let id = Xf::ID;
    let yb = TY + EP * 0.55;
    let arc = |dy: f32| -> Vec<(f32, f32)> {
        (0..=96)
            .map(|k| {
                let t = std::f32::consts::PI * k as f32 / 96.0;
                (TX + RX * t.cos(), yb + dy + RY * t.sin())
            })
            .collect()
    };
    g.polyligne(&id, &arc(0.0), 5.0, C_FER);
    g.polyligne(&id, &arc(-1.6), 1.0, alpha(C_FER_HI, 0.45));
    for k in 0..30 {
        let t = std::f32::consts::PI * (k as f32 + 0.5) / 30.0;
        clou(g, TX + RX * t.cos(), yb + RY * t.sin());
    }
    if let Some(p) = chemin_ellipse(TX, TY, RX, RY) {
        g.forme(&id, &p, None, Some(alpha(rgb(196, 140, 82), 0.75)), 1.4);
    }
}

/// Chandelle de cire dans un bougeoir de laiton, flamme et halo.
fn chandelle(g: &Gfx, x: f32, y: f32) {
    let id = Xf::ID;
    let laiton = rgb(196, 146, 60);
    // ombre et bougeoir
    g.ellipse(&id, x + 4.0, y + 4.0, 26.0, 9.0, Some(alpha(C_NUIT, 0.45)), None);
    g.ellipse(&id, x, y + 3.0, 22.0, 8.0, Some(rgb(96, 64, 22)), None);
    if let Some(p) = chemin_ellipse(x, y, 22.0, 8.0) {
        g.lineaire(&id, &p, (x - 22.0, y), (x + 22.0, y), &[(0.0, rgb(120, 82, 28)), (0.35, rgb(236, 196, 108)), (1.0, rgb(110, 74, 24))]);
    }
    g.ellipse(&id, x, y - 0.5, 15.0, 5.2, None, Some(alpha(rgb(90, 58, 18), 0.8)));
    g.ellipse(&id, x, y - 1.0, 9.5, 3.4, Some(laiton), Some(rgb(110, 74, 24)));
    // cire : cylindre arrondi par un dégradé, coulures, mèche
    let (hc, r) = (48.0, 7.0);
    if let Some(p) = chemin_poly(&[(x - r, y - 2.0), (x - r, y - hc), (x + r, y - hc), (x + r, y - 2.0)]) {
        g.lineaire(&id, &p, (x - r, 0.0), (x + r, 0.0), &[(0.0, rgb(196, 176, 138)), (0.38, rgb(250, 240, 214)), (1.0, rgb(176, 152, 112))]);
    }
    g.ellipse(&id, x, y - 2.0, r, 2.6, Some(rgb(186, 164, 124)), None);
    g.ellipse(&id, x, y - hc, r, 2.6, Some(rgb(244, 234, 206)), Some(rgb(206, 190, 154)));
    g.ellipse(&id, x + 0.5, y - hc + 0.3, r * 0.55, 1.3, Some(rgb(255, 236, 190)), None);
    for (dx, l) in [(-4.6, 13.0), (3.4, 21.0), (5.6, 8.0)] {
        g.rrect(&id, x + dx - 1.4, y - hc, 2.8, l, 1.4, Some(rgb(246, 236, 210)), None);
        g.ellipse(&id, x + dx, y - hc + l - 0.8, 1.7, 1.7, Some(rgb(240, 228, 198)), None);
    }
    g.line(&id, x, y - hc - 0.5, x + 0.8, y - hc - 5.0, 1.1, rgb(36, 26, 18));
    // flamme : halo, corps orangé, cœur clair, pied bleu
    let fy = y - hc - 14.0;
    g.lueur(&id, (x, fy), 70.0, &[(0.0, alpha(rgb(255, 220, 150), 0.6)), (0.22, alpha(rgb(255, 190, 100), 0.25)), (1.0, alpha(rgb(255, 170, 80), 0.0))]);
    let goutte = |k: f32| {
        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(x + 0.8, fy - 13.0 * k);
        pb.cubic_to(x + 5.6 * k, fy - 4.0 * k, x + 6.4 * k, fy + 5.0 * k, x, fy + 8.0 * k);
        pb.cubic_to(x - 6.4 * k, fy + 5.0 * k, x - 5.4 * k, fy - 4.0 * k, x + 0.8, fy - 13.0 * k);
        pb.close();
        pb.finish()
    };
    if let Some(p) = goutte(1.0) {
        g.radial(&id, &p, (x, fy + 3.0), 15.0, &[(0.0, rgb(255, 236, 170)), (0.55, rgb(255, 176, 62)), (1.0, alpha(rgb(230, 100, 30), 0.85))]);
    }
    if let Some(p) = goutte(0.55) {
        g.forme(&id, &p, Some(alpha(rgb(255, 252, 236), 0.95)), None, 0.0);
    }
    g.ellipse(&id, x, fy + 6.5, 2.2, 1.8, Some(alpha(rgb(90, 120, 230), 0.55)), None);
}

/// Une place autour de la table (coordonnées logiques 1100 x 800).
#[derive(Clone, Copy, Debug)]
pub struct Place {
    /// centre de la main
    pub main: (f64, f64),
    /// rotation des cartes de la main (0 en bas et en haut, ±π/2 sur les côtés)
    pub rotation: f64,
    /// écart entre les dos de cartes d'un adversaire
    pub pas: f64,
    /// longueur maximale de la main d'un adversaire
    pub largeur: f64,
    /// arrivée de la carte jouée dans le pli
    pub pli: (f64, f64),
    /// angle de la carte posée dans le pli
    pub angle: f64,
    /// bulle (« Je passe »...)
    pub bulle: (f32, f32),
    /// étiquette du nom
    pub etiquette: (f32, f32),
    /// le jeton du donneur se place à gauche de l'étiquette (côté droit de la table)
    pub donneur_a_gauche: bool,
}

pub const BAS: Place = Place {
    main: (550.0, 686.0),
    rotation: 0.0,
    pas: 84.0,
    largeur: 860.0,
    pli: (550.0, 402.0),
    angle: -0.06,
    bulle: (550.0, 598.0),
    etiquette: (130.0, 742.0),
    donneur_a_gauche: false,
};
pub const GAUCHE: Place = Place {
    main: (92.0, 345.0),
    rotation: FRAC_PI_2,
    pas: 24.0,
    largeur: 300.0,
    pli: (464.0, 352.0),
    angle: 0.09,
    bulle: (215.0, 345.0),
    etiquette: (92.0, 214.0),
    donneur_a_gauche: false,
};
pub const HAUT: Place = Place {
    main: (550.0, 96.0),
    rotation: 0.0,
    pas: 30.0,
    largeur: 420.0,
    pli: (550.0, 302.0),
    angle: 0.04,
    bulle: (550.0, 172.0),
    etiquette: (550.0, 24.0),
    donneur_a_gauche: false,
};
pub const DROITE: Place = Place {
    main: (1008.0, 345.0),
    rotation: -FRAC_PI_2,
    pas: 24.0,
    largeur: 300.0,
    pli: (636.0, 352.0),
    angle: -0.10,
    bulle: (885.0, 345.0),
    etiquette: (1008.0, 214.0),
    donneur_a_gauche: true,
};
pub const HAUT_GAUCHE: Place = Place {
    main: (410.0, 96.0),
    rotation: 0.0,
    pas: 26.0,
    largeur: 230.0,
    pli: (500.0, 310.0),
    angle: 0.05,
    bulle: (410.0, 172.0),
    etiquette: (410.0, 24.0),
    donneur_a_gauche: false,
};
pub const HAUT_DROITE: Place = Place {
    main: (690.0, 96.0),
    rotation: 0.0,
    pas: 26.0,
    largeur: 230.0,
    pli: (600.0, 310.0),
    angle: -0.05,
    bulle: (690.0, 172.0),
    etiquette: (690.0, 24.0),
    donneur_a_gauche: false,
};

/// Places pour n joueurs (2 à 5), en partant du joueur local (en bas) dans le sens du jeu.
pub fn disposition(n: usize) -> &'static [Place] {
    match n {
        2 => &[BAS, HAUT],
        3 => &[BAS, GAUCHE, DROITE],
        5 => &[BAS, GAUCHE, HAUT_GAUCHE, HAUT_DROITE, DROITE],
        _ => &[BAS, GAUCHE, HAUT, DROITE],
    }
}

/// Position relative du joueur p vu depuis le joueur `moi` (0 = en bas).
pub fn relative(p: usize, moi: usize, n: usize) -> usize {
    (p + n - moi) % n
}

/// Abscisse de la carte i (sur n) de la main du bas, en éventail serré si la main est longue.
pub fn main_x(i: usize, n: usize) -> f64 {
    let pas = if n > 1 { BAS.pas.min(BAS.largeur / (n - 1) as f64) } else { BAS.pas };
    BAS.main.0 + (i as f64 - (n as f64 - 1.0) / 2.0) * pas
}

/// Carte de la main du bas sous le point (x, y), ou None.
pub fn main_touchee(n: usize, x: f32, y: f32) -> Option<usize> {
    if !(610.0..=764.0).contains(&y) {
        return None;
    }
    (0..n).rev().find(|&i| (x as f64 - main_x(i, n)).abs() <= 52.0)
}

/// Centre du i-ème dos de la main d'un adversaire (n cartes) à la place `pl`.
pub fn pos_dos(pl: &Place, i: usize, n: usize) -> (f64, f64) {
    let pas = if n > 1 { pl.pas.min(pl.largeur / (n - 1) as f64) } else { pl.pas };
    // même arrondi que la version d'origine : i * pas - ((n - 1) * pas) / 2 en entier
    let off = i as f64 * pas - (((n.max(1) - 1) as f64 * pas) as i64 / 2) as f64;
    if pl.rotation == 0.0 {
        (pl.main.0 + off, pl.main.1)
    } else {
        (pl.main.0, pl.main.1 + off)
    }
}

/// Transformation du i-ème dos de la main d'un adversaire (n cartes) à la place `pl`.
pub fn dos_adversaire(pl: &Place, i: usize, n: usize) -> Xf {
    let (x, y) = pos_dos(pl, i, n);
    Xf::place(x, y, 0.62, pl.rotation, 1.0)
}

/// Paquet du donneur : entre sa main et le centre de la table (d'où partent les cartes distribuées).
pub fn paquet_donneur(pl: &Place) -> (f64, f64) {
    (pl.main.0 + (550.0 - pl.main.0) * 0.35, pl.main.1 + (352.0 - pl.main.1) * 0.35)
}
