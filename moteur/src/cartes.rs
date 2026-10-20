//! Cartes au design « bistrot classique » : face ivoire, double filet, indices en serif aux coins, points
//! disposés comme sur un vrai jeu, figures dans un cadre avec couronnes ; dos bordeaux à losanges dorés.
//! Toutes les cartes des jeux de bistrot : 2 à 10, valet, cavalier (tarot), dame, roi, as des quatre
//! couleurs, atouts du tarot (1 à 21), excuse, jokers rouge et noir.
//!
//! Chaque carte est dessinée une seule fois dans une petite image à l'échelle de la fenêtre (à la première
//! utilisation), puis posée avec n'importe quelle rotation ou perspective.
//! Repère canonique d'une carte : 100 x 140 centré sur l'origine.

use crate::gfx::{rgb, Align, Color, Face, Gfx, Glyphs, Xf};
use crate::polices::Fichiers;
use std::cell::RefCell;
use std::collections::HashMap;
use tiny_skia::Pixmap;

const C_IVORY: Color = rgb(250, 247, 238);
const C_SHADOW: Color = rgb(8, 52, 36);
const C_CRED: Color = rgb(194, 38, 46);
const C_CBLK: Color = rgb(27, 27, 34);
const C_TRED: Color = rgb(236, 195, 190);
const C_TBLK: Color = rgb(194, 192, 187);
const C_PANELF: Color = rgb(244, 236, 214);
const C_GOLDF: Color = rgb(217, 178, 106);
const C_GOLDLN: Color = rgb(140, 106, 36);
const C_BORD: Color = rgb(122, 31, 43);
const C_CLN2: Color = rgb(140, 134, 111);

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
    pub fn tarot(i: usize) -> Carte {
        match i {
            0..=55 => {
                let r = i % 14;
                let v = match r {
                    0 => Valeur::As,
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

const ROI: [(f32, f32); 7] = [(-16., 10.), (-16., -6.), (-8., 2.), (0., -10.), (8., 2.), (16., -6.), (16., 10.)];
const DAME: [(f32, f32); 7] = [(-13., 10.), (-13., -3.), (-6., 3.), (0., -7.), (6., 3.), (13., -3.), (13., 10.)];
const VALET: [(f32, f32); 6] = [(-13., 10.), (-13., 0.), (-6., -6.), (6., -6.), (13., 0.), (13., 10.)];
const CAVALIER: [(f32, f32); 7] = [(-14., 10.), (-14., -2.), (-9., -8.), (0., -3.), (9., -8.), (14., -2.), (14., 10.)];

/// Disposition des points (centres) des cartes 2 à 10, comme sur un vrai jeu.
fn points(n: u8) -> &'static [(f32, f32)] {
    match n {
        2 => &[(0., -44.), (0., 44.)],
        3 => &[(0., -44.), (0., 0.), (0., 44.)],
        4 => &[(-17., -44.), (17., -44.), (-17., 44.), (17., 44.)],
        5 => &[(-17., -44.), (17., -44.), (0., 0.), (-17., 44.), (17., 44.)],
        6 => &[(-17., -44.), (17., -44.), (-17., 0.), (17., 0.), (-17., 44.), (17., 44.)],
        7 => &[(-17., -44.), (17., -44.), (-17., 0.), (17., 0.), (-17., 44.), (17., 44.), (0., -22.)],
        8 => &[(-17., -44.), (17., -44.), (-17., 0.), (17., 0.), (-17., 44.), (17., 44.), (0., -22.), (0., 22.)],
        9 => &[(-17., -44.), (17., -44.), (-17., -15.), (17., -15.), (-17., 15.), (17., 15.), (-17., 44.), (17., 44.), (0., 0.)],
        _ => &[
            (-17., -44.),
            (17., -44.),
            (-17., -15.),
            (17., -15.),
            (-17., 15.),
            (17., 15.),
            (-17., 44.),
            (17., 44.),
            (0., -30.),
            (0., 30.),
        ],
    }
}

/// Zone de l'image d'une carte dans son repère (avec l'ombre portée).
pub const X0: f32 = -54.0;
pub const Y0: f32 = -74.0;
pub const X1: f32 = 58.0;
pub const Y1: f32 = 78.0;

pub struct PolicesCartes {
    serif: Face,
    serif_grand: Face,
    serif_petit: Face,
    point: Face,
    grand_symbole: Face,
    symbole: Face,
}

impl PolicesCartes {
    pub fn depuis(f: &Fichiers) -> PolicesCartes {
        PolicesCartes {
            serif: f.serif.face(25.0),
            serif_grand: f.serif.face(50.0),
            serif_petit: f.serif.face(15.0),
            point: f.symboles.face(22.0),
            grand_symbole: f.symboles.face(84.0),
            symbole: f.symboles.face(34.0),
        }
    }
}

fn fond(g: &Gfx, filet: Color) {
    let id = Xf::ID;
    g.rrect(&id, -46., -66., 100., 140., 12., Some(C_SHADOW), Some(C_SHADOW));
    g.rrect(&id, -50., -70., 100., 140., 12., Some(C_IVORY), Some(C_CLN2));
    g.rrect(&id, -47., -67., 94., 134., 8., None, Some(filet));
}

/// Indices aux coins : texte en haut à gauche et en bas à droite, symbole sous chacun.
fn indices(g: &Gfx, f: &PolicesCartes, texte: &str, symbole: &str, encre: Color) {
    g.text(texte, -36., -69., &f.serif, encre, Align::Center);
    g.text(symbole, -36., -43., &f.point, encre, Align::Center);
    g.text(texte, 36., 41., &f.serif, encre, Align::Center);
    g.text(symbole, 36., 14., &f.point, encre, Align::Center);
}

fn dessiner_face(g: &Gfx, f: &PolicesCartes, carte: Carte) {
    let id = Xf::ID;
    match carte {
        Carte::Normale(c, v) => {
            let (encre, filet) = if c.rouge() { (C_CRED, C_TRED) } else { (C_CBLK, C_TBLK) };
            fond(g, filet);
            let sym = c.symbole();
            indices(g, f, &v.indice(), sym, encre);
            match v {
                Valeur::As => g.text(sym, 0., -64., &f.grand_symbole, encre, Align::Center),
                Valeur::N(n) => {
                    for &(x, y) in points(n) {
                        g.text(sym, x, y - 17., &f.point, encre, Align::Center);
                    }
                }
                _ => {
                    g.rrect(&id, -26., -54., 52., 108., 6., Some(C_PANELF), Some(filet));
                    let couronne: &[(f32, f32)] = match v {
                        Valeur::Roi => &ROI,
                        Valeur::Dame => &DAME,
                        Valeur::Cavalier => &CAVALIER,
                        _ => &VALET,
                    };
                    let haut: Vec<(f32, f32)> = couronne.iter().map(|&(x, y)| (x, y - 40.)).collect();
                    let bas: Vec<(f32, f32)> = couronne.iter().map(|&(x, y)| (x, 40. - y)).collect();
                    g.poly_any(&id, &haut, C_GOLDF, Some(C_GOLDLN));
                    g.poly_any(&id, &bas, C_GOLDF, Some(C_GOLDLN));
                    if v == Valeur::Dame {
                        g.ellipse(&id, 0., -50., 3., 3., Some(C_GOLDF), Some(C_GOLDLN));
                        g.ellipse(&id, 0., 50., 3., 3., Some(C_GOLDF), Some(C_GOLDLN));
                    }
                    g.text(&v.indice(), 0., -38., &f.serif_grand, encre, Align::Center);
                    g.text(sym, 0., 2., &f.point, encre, Align::Center);
                }
            }
        }
        Carte::Atout(n) => {
            fond(g, C_GOLDF);
            let n = n.to_string();
            indices(g, f, &n, "★", C_CBLK);
            g.rrect(&id, -30., -50., 60., 100., 8., Some(C_PANELF), Some(C_GOLDF));
            g.ellipse(&id, 0., 0., 24., 24., None, Some(C_GOLDLN));
            g.text(&n, 0., -30., &f.serif_grand, C_CBLK, Align::Center);
            g.text("ATOUT", 0., 30., &f.serif_petit, C_GOLDLN, Align::Center);
        }
        Carte::Excuse => {
            fond(g, C_GOLDF);
            indices(g, f, "★", "", C_CBLK);
            g.rrect(&id, -30., -50., 60., 100., 8., Some(C_PANELF), Some(C_GOLDF));
            g.text("★", 0., -42., &f.grand_symbole, C_CBLK, Align::Center);
            g.text("EXCUSE", 0., 30., &f.serif_petit, C_GOLDLN, Align::Center);
        }
        Carte::Joker(rouge) => {
            let (encre, filet) = if rouge { (C_CRED, C_TRED) } else { (C_CBLK, C_TBLK) };
            fond(g, filet);
            indices(g, f, "J", "★", encre);
            g.rrect(&id, -26., -54., 52., 108., 6., Some(C_PANELF), Some(filet));
            g.poly_any(&id, &ROI.iter().map(|&(x, y)| (x, y - 34.)).collect::<Vec<_>>(), C_GOLDF, Some(C_GOLDLN));
            g.text("★", 0., -18., &f.symbole, encre, Align::Center);
            g.text("JOKER", 0., 24., &f.serif_petit, encre, Align::Center);
        }
    }
}

fn dessiner_dos(g: &Gfx, f: &PolicesCartes) {
    let id = Xf::ID;
    g.rrect(&id, -46., -66., 100., 140., 12., Some(C_SHADOW), Some(C_SHADOW));
    g.rrect(&id, -50., -70., 100., 140., 12., Some(C_IVORY), Some(C_CLN2));
    g.rrect(&id, -44., -64., 88., 128., 7., Some(C_BORD), Some(C_BORD));
    // losanges dorés : droites à 45 degrés recoupées au cadre
    let mut c = -190;
    while c <= 190 {
        let x0 = (-39).max(c - 59) as f32;
        let x1 = 39.min(c + 59) as f32;
        if x0 < x1 {
            let cf = c as f32;
            g.line(&id, x0, x0 - cf, x1, x1 - cf, 0.7, C_GOLDF);
            g.line(&id, -x0, x0 - cf, -x1, x1 - cf, 0.7, C_GOLDF);
        }
        c += 16;
    }
    g.rrect(&id, -44., -64., 88., 128., 7., None, Some(C_IVORY));
    g.rrect(&id, -39., -59., 78., 118., 4., None, Some(C_GOLDF));
    g.ellipse(&id, 0., 0., 22., 31., Some(C_BORD), Some(C_GOLDF));
    g.ellipse(&id, 0., 0., 15., 23., None, Some(C_GOLDF));
    g.text("♥", 0., -17., &f.symbole, C_GOLDF, Align::Center);
}

fn rendre(f: &PolicesCartes, glyphs: &RefCell<Glyphs>, carte: Option<Carte>, ppu: f32) -> Pixmap {
    let (w, h) = (((X1 - X0) * ppu).ceil() as u32, ((Y1 - Y0) * ppu).ceil() as u32);
    let mut pm = Pixmap::new(w.max(1), h.max(1)).unwrap();
    let g = Gfx::new(&mut pm, glyphs, ppu, -X0 * ppu, -Y0 * ppu);
    match carte {
        Some(c) => dessiner_face(&g, f, c),
        None => dessiner_dos(&g, f),
    }
    drop(g);
    pm
}

/// Les cartes déjà dessinées, à l'échelle de la fenêtre (dessinées à la première utilisation).
pub struct Paquet {
    polices: PolicesCartes,
    ppu: f32,
    faces: RefCell<HashMap<Carte, Pixmap>>,
    dos: RefCell<Option<Pixmap>>,
}

impl Paquet {
    pub fn new(fichiers: &Fichiers) -> Paquet {
        Paquet { polices: PolicesCartes::depuis(fichiers), ppu: 0.0, faces: RefCell::default(), dos: RefCell::default() }
    }

    /// Échelle des images : `ppu` pixels par unité de carte (les cartes de la main sont posées environ 1:1).
    /// Changer d'échelle oublie les cartes déjà dessinées.
    pub fn echelle(&mut self, ppu: f32) {
        if ppu != self.ppu {
            self.ppu = ppu;
            self.faces.borrow_mut().clear();
            *self.dos.borrow_mut() = None;
        }
    }

    pub fn ppu(&self) -> f32 {
        self.ppu
    }

    /// Pose une carte (None = dos) avec la transformation xf ; `grise` = carte injouable.
    pub fn poser(&self, g: &Gfx, xf: &Xf, carte: Option<Carte>, grise: bool) {
        let ppu = if self.ppu > 0.0 { self.ppu } else { g.sc };
        match carte {
            Some(c) => {
                let mut faces = self.faces.borrow_mut();
                let img = faces.entry(c).or_insert_with(|| rendre(&self.polices, g.glyphs(), Some(c), ppu));
                g.image(xf, img, X0, Y0, ppu);
            }
            None => {
                let mut dos = self.dos.borrow_mut();
                let img = dos.get_or_insert_with(|| rendre(&self.polices, g.glyphs(), None, ppu));
                g.image(xf, img, X0, Y0, ppu);
            }
        }
        if grise {
            g.rrect(xf, -50., -70., 100., 140., 12., Some(Color::new(0.0, 0.0, 0.0, 0.26)), None);
        }
    }
}
