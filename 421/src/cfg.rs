//! Réglages du 421, mémorisés dans `421.cfg` (voir `bistrot::reglages`).

use crate::game::{Game, Mode};
use bistrot::anim::{self, Animations};
use bistrot::reglages::Reglages;

pub struct Cfg {
    pub fullscreen: bool,
    pub ip: String,
    r: Reglages,
}

pub fn load(g: &mut Game) -> Cfg {
    let r = Reglages::charger("421.cfg");
    if let Some(p) = r.entier("pot", 7, 21).filter(|p| [7, 11, 15, 21].contains(p)) {
        g.pot_init = p as u32;
    }
    if let Some(l) = r.entier("niveau", 0, 2) {
        g.level = l as usize;
    }
    if r.oui("deux_joueurs") == Some(true) {
        g.mode = Mode::Duo;
    }
    if let Some(w) = r.entier("victoires", 0, 99_999) {
        g.wins = w as u32;
    }
    if let Some(l) = r.entier("defaites", 0, 99_999) {
        g.losses = l as u32;
    }
    if let Some(a) = r.mot("animations").and_then(Animations::depuis) {
        anim::regler(a);
    }
    g.pot = g.pot_init;
    Cfg { fullscreen: r.oui("plein_ecran").unwrap_or(false), ip: r.adresse("ip").unwrap_or_default(), r }
}

/// Écrit les réglages. Le mode en ligne n'est pas mémorisé : on garde le mode local d'avant.
pub fn save(g: &Game, c: &mut Cfg) {
    let mode = if g.online() { g.mode_local } else { g.mode };
    let r = &mut c.r;
    r.mettre("pot", g.pot_init);
    r.mettre("niveau", g.level);
    r.mettre("deux_joueurs", (mode == Mode::Duo) as u8);
    r.mettre("plein_ecran", c.fullscreen as u8);
    r.mettre("victoires", g.wins);
    r.mettre("defaites", g.losses);
    r.mettre("ip", &c.ip);
    r.mettre("animations", anim::reglage().nom());
    r.enregistrer("réglages du 421");
}
