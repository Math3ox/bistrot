//! Éléments d'interface communs : palette, boutons, bulles, étiquettes des joueurs, panneaux, panneau du
//! jeu en ligne.

use crate::gfx::{rgb, Align, Color, Gfx, Xf};
use crate::polices::Polices;
use crate::table::Place;

pub const C_IVORY: Color = rgb(250, 247, 238);
pub const C_CARDLN: Color = rgb(120, 115, 100);
pub const C_BLACK: Color = rgb(26, 26, 32);
pub const C_GOLD: Color = rgb(255, 205, 70);
pub const C_GOLDDK: Color = rgb(144, 96, 16);
pub const C_WHITE: Color = rgb(240, 240, 235);
pub const C_SHFLY: Color = rgb(20, 92, 64);
pub const C_BTN: Color = rgb(60, 150, 100);
pub const C_BTNDK: Color = rgb(30, 90, 60);
pub const C_BTNOFF: Color = rgb(70, 74, 84);
pub const C_BTNHOV: Color = rgb(160, 224, 160);
pub const C_PANEL: Color = rgb(30, 36, 46);
pub const C_PANELLN: Color = rgb(80, 90, 106);
pub const C_TXT: Color = rgb(220, 226, 232);
pub const C_TXTDIM: Color = rgb(140, 150, 162);
pub const C_OVER: Color = rgb(38, 46, 60);
pub const C_TURNFILL: Color = rgb(60, 64, 30);
pub const C_PLAYER: Color = rgb(96, 200, 255);
pub const C_ENEMY: Color = rgb(255, 120, 140);
pub const C_TRUMPRED: Color = rgb(255, 120, 100);

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

    /// Dessine le bouton (vert, gris s'il est inactif, clair sous la souris).
    pub fn dessiner(&self, g: &Gfx, p: &Polices, texte: &str, souris: (f32, f32)) {
        let (mut fond, bord) = if self.actif { (C_BTN, C_BTNDK) } else { (C_BTNOFF, C_PANEL) };
        if self.touche(souris.0, souris.1) {
            fond = C_BTNHOV;
        }
        g.rrect(&Xf::ID, self.x, self.y, self.w, self.h, 12., Some(fond), Some(bord));
        let col = if self.actif { C_WHITE } else { C_TXTDIM };
        g.text(texte, self.x + self.w / 2.0, self.y + 11., &p.bouton, col, Align::Center);
    }
}

/// Bulle de parole (« Je passe », « Belote ! »...) centrée en (x, y).
pub fn bulle(g: &Gfx, p: &Polices, (x, y): (f32, f32), texte: &str) {
    let w = texte.chars().count() as f32 * 10.0 + 34.0;
    g.rrect(&Xf::ID, x - w / 2.0, y - 18., w, 36., 14., Some(C_IVORY), Some(C_CARDLN));
    g.text(texte, x, y - 12., &p.etiquette, C_BLACK, Align::Center);
}

/// Étiquette d'un joueur : bord bleu (notre équipe) ou rose (adversaires), doré si c'est son tour,
/// jeton « D » s'il donne.
pub fn etiquette(g: &Gfx, p: &Polices, pl: &Place, nom: &str, partenaire: bool, son_tour: bool, donneur: bool) {
    let (x, y) = pl.etiquette;
    let (fond, bord) = if son_tour {
        (C_TURNFILL, C_GOLD)
    } else {
        (C_PANEL, if partenaire { C_PLAYER } else { C_ENEMY })
    };
    g.rrect(&Xf::ID, x - 80., y - 3., 160., 30., 14., Some(fond), Some(bord));
    g.text(nom, x, y, &p.etiquette, C_WHITE, Align::Center);
    if donneur {
        let cx = if pl.donneur_a_gauche { x - 93. } else { x + 93. };
        g.ellipse(&Xf::ID, cx, y + 12., 13., 13., Some(C_GOLD), Some(C_GOLDDK));
        g.text("D", cx, y + 2., &p.petit, C_BLACK, Align::Center);
    }
}

/// Panneau sombre arrondi.
pub fn panneau(g: &Gfx, x: f32, y: f32, w: f32, h: f32, rayon: f32) {
    g.rrect(&Xf::ID, x, y, w, h, rayon, Some(C_OVER), Some(C_PANELLN));
}

/// Panneau du jeu en ligne : titre et trois lignes (ou deux en mode menu « H / R »).
pub fn panneau_reseau(g: &Gfx, p: &Polices, titre: &str, lignes: [&str; 3], menu: bool) {
    panneau(g, 120., 230., 860., 230., 18.);
    g.text(titre, 550., 248., &p.moyen, C_GOLD, Align::Center);
    if menu {
        g.text(lignes[0], 550., 310., &p.etiquette, C_WHITE, Align::Center);
        g.text(lignes[1], 550., 350., &p.petit, C_TXTDIM, Align::Center);
    } else {
        g.text(lignes[0], 550., 300., &p.etiquette, C_WHITE, Align::Center);
        g.text(lignes[1], 550., 345., &p.petit, C_TXTDIM, Align::Center);
        g.text(lignes[2], 550., 385., &p.petit, C_WHITE, Align::Center);
    }
}
