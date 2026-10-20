//! Éléments d'interface communs au style « taverne » : feuilles de parchemin (panneaux, étiquettes des
//! joueurs, bulles), parchemin roulé pour les grands panneaux, boutons de parchemin cerclés d'encre rouge,
//! jeton du donneur en pièce d'or, panneau du jeu en ligne. Deux palettes : les encres pour écrire sur le
//! parchemin, des couleurs claires (lueur des chandelles) pour écrire sur le bois.

use crate::gfx::{alpha, chemin_ellipse, chemin_rrect, rgb, Align, Color, Face, Gfx, Hasard, Xf};
use crate::polices::Polices;
use crate::table::Place;

// ---------- sur le bois ----------
pub const C_GOLD: Color = rgb(242, 196, 92);
pub const C_GOLDDK: Color = rgb(128, 86, 20);
pub const C_WHITE: Color = rgb(246, 234, 206);
pub const C_TXT: Color = rgb(236, 220, 186);
pub const C_TXTDIM: Color = rgb(178, 156, 122);
/// ombre d'une carte en vol au-dessus de la table
pub const C_SHFLY: Color = Color::new(0.06, 0.03, 0.01, 0.42);

// ---------- encres, sur le parchemin ----------
pub const C_ENCRE: Color = rgb(52, 32, 18);
pub const C_ENCRE_PALE: Color = rgb(118, 88, 58);
/// rouge des titres, comme dans les manuscrits
pub const C_RUBRIQUE: Color = rgb(150, 30, 22);
/// notre équipe (outremer)
pub const C_NOUS: Color = rgb(28, 58, 136);
/// les adversaires (cramoisi)
pub const C_EUX: Color = rgb(150, 26, 40);
/// couleurs rouges (♥ ♦) écrites à l'encre
pub const C_ROUGE: Color = rgb(168, 24, 38);

const C_PARCH_HI: Color = rgb(243, 230, 198);
const C_PARCH_LO: Color = rgb(219, 194, 147);
const C_PARCH_BRUN: Color = rgb(122, 74, 30);
const C_PARCH_BORD: Color = rgb(96, 62, 30);
const C_NOIR: Color = rgb(0, 0, 0);

/// Haut d'un texte pour que ses capitales soient centrées verticalement sur cy.
pub fn centre_v(f: &Face, cy: f32) -> f32 {
    cy + (0.34 - f.asc) * f.size
}

/// Ombre douce d'un objet posé (couches de plus en plus larges et légères).
fn ombre_portee(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32, h: f32, r: f32, op: f32) {
    for i in 0..3 {
        let e = i as f32 * 2.0;
        g.rrect(xf, x - e + 2.0, y - e + 4.0, w + 2.0 * e, h + 2.0 * e, r + e, Some(alpha(C_NOIR, 0.13 * op)), None);
    }
}

/// Feuille de parchemin dans le repère xf : dégradé jauni, taches, bords brunis, contour d'encre.
fn feuille(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32, h: f32, r: f32, op: f32) {
    ombre_portee(g, xf, x, y, w, h, r, op);
    if let Some(p) = chemin_rrect(x, y, w, h, r) {
        g.lineaire(xf, &p, (x, y), (x + 0.3 * w, y + h), &[(0.0, alpha(C_PARCH_HI, op)), (1.0, alpha(C_PARCH_LO, op))]);
    }
    let mut hz = Hasard((x as u64) << 32 ^ (y as u64) << 16 ^ (w as u64) << 4 ^ h as u64 | 1);
    for _ in 0..2 {
        let (cx, cy) = (x + w * hz.entre(0.15, 0.85), y + h * hz.entre(0.2, 0.8));
        let rr = w.min(h) * hz.entre(0.25, 0.5);
        g.lueur(xf, (cx, cy), rr, &[(0.0, alpha(C_PARCH_BRUN, 0.08 * op)), (1.0, alpha(C_PARCH_BRUN, 0.0))]);
    }
    // bords brunis : un trait large et pâle, un trait fin plus foncé, le contour
    g.rrect_w(xf, x + 3.0, y + 3.0, w - 6.0, h - 6.0, (r - 3.0).max(0.0), None, Some(alpha(C_PARCH_BRUN, 0.13 * op)), 6.0);
    g.rrect_w(xf, x + 1.2, y + 1.2, w - 2.4, h - 2.4, (r - 1.2).max(0.0), None, Some(alpha(C_PARCH_BRUN, 0.28 * op)), 2.4);
    g.rrect_w(xf, x, y, w, h, r, None, Some(alpha(C_PARCH_BORD, op)), 1.2);
}

/// Feuille de parchemin (panneaux de score, d'atout...).
pub fn parchemin(g: &Gfx, x: f32, y: f32, w: f32, h: f32, r: f32) {
    feuille(g, &Xf::ID, x, y, w, h, r, 1.0);
}

/// Bord roulé d'un grand parchemin, avec ses embouts de bois.
fn rouleau(g: &Gfx, x: f32, y: f32, w: f32, h: f32) {
    let id = Xf::ID;
    ombre_portee(g, &id, x, y, w, h, h / 2.0, 1.0);
    for ex in [x - 9.0, x + w - 3.0] {
        g.rrect_degrade(&id, ex, y + 2.0, 12.0, h - 4.0, 4.0, rgb(132, 88, 46), rgb(58, 34, 16), Some(rgb(40, 24, 10)));
    }
    if let Some(p) = chemin_rrect(x, y, w, h, h / 2.0) {
        let arrets = [(0.0, rgb(168, 136, 92)), (0.3, rgb(246, 233, 200)), (0.62, rgb(208, 180, 130)), (1.0, rgb(112, 80, 44))];
        g.lineaire(&id, &p, (0.0, y), (0.0, y + h), &arrets);
    }
    g.rrect_w(&id, x, y, w, h, h / 2.0, None, Some(C_PARCH_BORD), 1.0);
}

/// Grand panneau : parchemin roulé en haut et en bas, double filet rouge (bilan, force des cartes, réseau).
pub fn panneau(g: &Gfx, x: f32, y: f32, w: f32, h: f32, _rayon: f32) {
    let id = Xf::ID;
    parchemin(g, x, y, w, h, 3.0);
    g.rrect_w(&id, x + 14.0, y + 18.0, w - 28.0, h - 36.0, 4.0, None, Some(alpha(C_RUBRIQUE, 0.55)), 1.3);
    g.rrect_w(&id, x + 18.5, y + 22.5, w - 37.0, h - 45.0, 3.0, None, Some(alpha(C_RUBRIQUE, 0.35)), 0.6);
    rouleau(g, x - 12.0, y - 9.0, w + 24.0, 18.0);
    rouleau(g, x - 12.0, y + h - 9.0, w + 24.0, 18.0);
}

/// Bouton rectangulaire arrondi ; `T` identifie l'action.
#[derive(Clone, Copy)]
pub struct Bouton<T> {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub id: T,
    pub actif: bool,
}

impl<T> Bouton<T> {
    pub fn touche(&self, x: f32, y: f32) -> bool {
        self.actif && x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }

    /// Dessine le bouton : parchemin cerclé d'encre rouge, doré et éclairé sous la souris, terne s'il est inactif.
    pub fn dessiner(&self, g: &Gfx, p: &Polices, texte: &str, souris: (f32, f32)) {
        let id = Xf::ID;
        let (x, y, w, h) = (self.x, self.y, self.w, self.h);
        let survol = self.touche(souris.0, souris.1);
        let (hi, lo, bord, encre) = if !self.actif {
            (rgb(168, 154, 126), rgb(138, 122, 94), rgb(104, 90, 70), rgb(92, 78, 58))
        } else if survol {
            (rgb(255, 246, 218), rgb(238, 208, 148), C_GOLDDK, C_RUBRIQUE)
        } else {
            (C_PARCH_HI, C_PARCH_LO, C_RUBRIQUE, C_ENCRE)
        };
        if survol {
            let xf = Xf { a: 1.0, b: 0.0, c: 0.0, d: 0.45, tx: x + w / 2.0, ty: y + h / 2.0 };
            g.lueur(&xf, (0.0, 0.0), w * 0.8, &[(0.0, alpha(C_GOLD, 0.5)), (1.0, alpha(C_GOLD, 0.0))]);
        }
        ombre_portee(g, &id, x, y, w, h, 10.0, 1.0);
        g.rrect_degrade(&id, x, y, w, h, 10.0, hi, lo, None);
        g.rrect_w(&id, x, y, w, h, 10.0, None, Some(bord), 2.0);
        g.rrect_w(&id, x + 4.0, y + 4.0, w - 8.0, h - 8.0, 7.0, None, Some(alpha(bord, 0.5)), 0.8);
        g.text(texte, x + w / 2.0, centre_v(&p.bouton, y + h / 2.0), &p.bouton, encre, Align::Center);
    }
}

/// Bulle de parole (« Je passe », « Belote ! »...) centrée en (x, y).
pub fn bulle(g: &Gfx, p: &Polices, pos: (f32, f32), texte: &str) {
    bulle_animee(g, p, pos, texte, 1.0, 1.0);
}

/// Bulle qui apparaît en rebondissant (`taille` autour de 1) et s'efface (`opacite` de 1 à 0) : un bout
/// de parchemin écrit à l'encre.
pub fn bulle_animee(g: &Gfx, p: &Polices, (x, y): (f32, f32), texte: &str, taille: f32, opacite: f32) {
    let xf = Xf::centre(x, y, taille);
    let w = g.text_width(texte, &p.etiquette) + 40.0;
    feuille(g, &xf, -w / 2.0, -19.0, w, 38.0, 10.0, opacite);
    let mut face = p.etiquette.clone();
    face.size *= taille;
    g.text(texte, x, centre_v(&face, y), &face, alpha(C_ENCRE, opacite), Align::Center);
}

/// Pièce d'or (jeton du donneur) de rayon r, frappée d'une lettre.
pub fn piece(g: &Gfx, f: &Face, cx: f32, cy: f32, r: f32, lettre: &str) {
    let id = Xf::ID;
    g.ellipse(&id, cx + 1.5, cy + 2.5, r, r, Some(alpha(C_NOIR, 0.35)), None);
    if let Some(p) = chemin_ellipse(cx, cy, r, r) {
        g.radial(&id, &p, (cx - 0.35 * r, cy - 0.35 * r), 1.5 * r, &[(0.0, rgb(255, 238, 160)), (0.45, rgb(228, 174, 60)), (1.0, rgb(140, 92, 20))]);
    }
    g.ellipse(&id, cx, cy, r, r, None, Some(C_GOLDDK));
    g.ellipse(&id, cx, cy, r * 0.78, r * 0.78, None, Some(alpha(C_GOLDDK, 0.6)));
    g.text(lettre, cx, centre_v(f, cy), f, rgb(96, 60, 12), Align::Center);
}

/// Étiquette d'un joueur : plaque de parchemin écrite à l'encre de son équipe (bleu : nous, rouge : eux),
/// dorée et éclairée si c'est son tour, pièce d'or « D » s'il donne.
pub fn etiquette(g: &Gfx, p: &Polices, pl: &Place, nom: &str, partenaire: bool, son_tour: bool, donneur: bool) {
    let id = Xf::ID;
    let (x, y) = pl.etiquette;
    let encre = if partenaire { C_NOUS } else { C_EUX };
    if son_tour {
        let xf = Xf { a: 1.0, b: 0.0, c: 0.0, d: 0.32, tx: x, ty: y + 12.0 };
        g.lueur(&xf, (0.0, 0.0), 140.0, &[(0.0, alpha(C_GOLD, 0.7)), (0.6, alpha(C_GOLD, 0.25)), (1.0, alpha(C_GOLD, 0.0))]);
    }
    parchemin(g, x - 80.0, y - 3.0, 160.0, 30.0, 6.0);
    let (bord, lw) = if son_tour { (C_GOLDDK, 2.6) } else { (encre, 1.6) };
    g.rrect_w(&id, x - 80.0, y - 3.0, 160.0, 30.0, 6.0, None, Some(bord), lw);
    g.text(nom, x, centre_v(&p.etiquette, y + 12.0), &p.etiquette, encre, Align::Center);
    if donneur {
        let cx = if pl.donneur_a_gauche { x - 95.0 } else { x + 95.0 };
        piece(g, &p.etiquette, cx, y + 12.0, 13.0, "D");
    }
}

/// Panneau du jeu en ligne : titre et trois lignes (ou deux en mode menu « H / R »).
pub fn panneau_reseau(g: &Gfx, p: &Polices, titre: &str, lignes: [&str; 3], menu: bool) {
    panneau(g, 120., 230., 860., 230., 18.);
    g.text(titre, 550., 252., &p.moyen, C_RUBRIQUE, Align::Center);
    if menu {
        g.text(lignes[0], 550., 312., &p.etiquette, C_ENCRE, Align::Center);
        g.text(lignes[1], 550., 352., &p.petit, C_ENCRE_PALE, Align::Center);
    } else {
        g.text(lignes[0], 550., 302., &p.etiquette, C_ENCRE, Align::Center);
        g.text(lignes[1], 550., 346., &p.petit, C_ENCRE_PALE, Align::Center);
        g.text(lignes[2], 550., 386., &p.petit, C_ENCRE, Align::Center);
    }
}
