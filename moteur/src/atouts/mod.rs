//! Atouts illustrés du tarot, dessinés en vectoriel d'après les sujets du Tarot Nouveau : chaque atout a une
//! scène de ville en haut et une scène de campagne en bas (retournée, comme sur une vraie carte) ; l'Excuse est
//! le fou à la mandoline. Les scènes ne sont dessinées qu'une fois par taille, avec le reste de la carte.
//!
//! 1 l'individu (le clown triste ; le fou et la danseuse) ; 2 à 5 les âges (enfance, jeunesse, âge mûr,
//! vieillesse) ; 6 à 9 les heures (matin, après-midi, soir, nuit) ; 10 la terre ; 11 l'eau ; 12 à 15 les
//! loisirs (danse, emplettes, plein air, arts) ; 16 à 19 les saisons ; 20 le jeu ; 21 la fête collective.

mod outils;
mod scenes;

use crate::gfx::{rgb, Color, Gfx, Xf};
use tiny_skia::Pixmap;

const OR: Color = rgb(222, 176, 74);
const OR_FONCE: Color = rgb(126, 88, 22);

/// Dessine une scène dans sa propre image (découpée au cadre), posée en haut ou retournée en bas.
fn poser(g: &Gfx, scene: fn(&Gfx, &Xf), y0: f32, h: f32, retourne: bool) {
    let ppu = g.sc;
    let Some(mut img) = Pixmap::new((86.0 * ppu).ceil() as u32, (h * ppu).ceil() as u32) else { return };
    {
        let gi = Gfx::new(&mut img, g.glyphs(), ppu, 43.0 * ppu, -y0 * ppu);
        scene(&gi, &Xf::ID);
    }
    let xf = if retourne { Xf::DEMI_TOUR } else { Xf::ID };
    g.image(&xf, &img, -43.0, y0, ppu);
}

fn cadre(g: &Gfx, milieu: bool) {
    let id = Xf::ID;
    if milieu {
        g.line(&id, -43.0, 0.0, 43.0, 0.0, 1.5, OR);
        g.line(&id, -43.0, 0.0, 43.0, 0.0, 0.4, OR_FONCE);
    }
    g.rrect_w(&id, -43.5, -63.5, 87.0, 127.0, 2.0, None, Some(OR), 1.6);
    g.rrect(&id, -44.6, -64.6, 89.2, 129.2, 2.6, None, Some(OR_FONCE));
}

/// Illustration de l'atout n (1 à 21) : deux scènes et leur cadre doré, dans le rectangle (-43, -63, 86, 126).
pub fn illustration(g: &Gfx, n: u8) {
    let (haut, bas) = scenes::scenes(n);
    poser(g, haut, -63.0, 62.0, false);
    poser(g, bas, -63.0, 62.0, true);
    cadre(g, true);
}

/// Illustration de l'Excuse : le fou à la mandoline, sur toute la hauteur.
pub fn illustration_excuse(g: &Gfx) {
    poser(g, scenes::excuse, -63.0, 126.0, false);
    cadre(g, false);
}
