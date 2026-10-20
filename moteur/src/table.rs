//! Table ronde en perspective (bois et tapis vert) et places des joueurs autour, de 2 à 5 joueurs.
//! Le joueur de cet ordinateur est toujours en bas ; les autres sont répartis dans le sens du jeu.

use crate::gfx::{rgb, Color, Gfx, Xf, WW};
use std::f64::consts::FRAC_PI_2;

const C_BG: Color = rgb(18, 22, 29);
const C_WOOD: Color = rgb(128, 84, 44);
const C_WOODDK: Color = rgb(78, 48, 24);
const C_WOODHI: Color = rgb(160, 108, 60);
const C_FELTEDGE: Color = rgb(16, 60, 40);

/// Écrasement vertical du plan de la table (vue isométrique).
pub const SOL: f64 = 0.62;

/// Décor fixe : fond, ombre, bois, tapis en dégradé (à mettre en cache).
pub fn dessiner(g: &Gfx) {
    let id = Xf::ID;
    g.rrect(&id, 0., 0., WW, 800., 0., Some(C_BG), None);
    let ombre = rgb(14, 10, 8);
    g.ellipse(&id, 550., 392., 520., 282., Some(ombre), Some(ombre));
    g.ellipse(&id, 550., 376., 500., 280., Some(C_WOODDK), Some(C_WOODDK));
    g.ellipse(&id, 550., 352., 500., 280., Some(C_WOOD), Some(C_WOODHI));
    for k in 0..12i32 {
        let c = rgb((14 + k * 20 / 11) as u8, (92 + k * 46 / 11) as u8, (62 + k * 32 / 11) as u8);
        let bord = if k == 0 { C_FELTEDGE } else { c };
        g.ellipse(&id, 550., 352., (474 - 26 * k) as f32, (258 - 15 * k) as f32, Some(c), Some(bord));
    }
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

/// Transformation du i-ème dos de la main d'un adversaire (n cartes) à la place `pl`.
pub fn dos_adversaire(pl: &Place, i: usize, n: usize) -> Xf {
    let pas = if n > 1 { pl.pas.min(pl.largeur / (n - 1) as f64) } else { pl.pas };
    // même arrondi que la version d'origine : i * pas - ((n - 1) * pas) / 2 en entier
    let off = i as f64 * pas - (((n.max(1) - 1) as f64 * pas) as i64 / 2) as f64;
    if pl.rotation == 0.0 {
        Xf::place(pl.main.0 + off, pl.main.1, 0.62, 0.0, 1.0)
    } else {
        Xf::place(pl.main.0, pl.main.1 + off, 0.62, pl.rotation, 1.0)
    }
}
