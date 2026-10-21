//! Réglages du rami, mémorisés dans `rami.cfg` (voir `bistrot::reglages`).

use crate::engine::LIMITES;
use crate::table::Table;
use bistrot::anim::{self, Animations};
use bistrot::reglages::Reglages;

pub struct Cfg {
    pub fullscreen: bool,
    pub ip: String,
    r: Reglages,
}

/// Lit les réglages (avant la première partie).
pub fn load(t: &mut Table) -> Cfg {
    let r = Reglages::charger("rami.cfg");
    if let Some(np) = r.entier("joueurs", 2, 5) {
        t.m.np = np as usize;
    }
    if let Some(l) = r.entier("limite", 1, 9999).filter(|l| LIMITES.contains(&(*l as i32))) {
        t.m.limite = l as i32;
    }
    if let Some(l) = r.entier("niveau", 0, 2) {
        t.botlvl = l as i8;
    }
    if let Some(h) = r.oui("aide") {
        t.help = h;
    }
    if let Some(v) = r.oui("tri_valeur") {
        t.tri_valeur = v;
    }
    if let Some(a) = r.mot("animations").and_then(Animations::depuis) {
        anim::regler(a);
    }
    if let Some(w) = r.entier("victoires", 0, 999_999) {
        t.wins[0] = w as u32;
    }
    if let Some(l) = r.entier("defaites", 0, 999_999) {
        t.wins[1] = l as u32;
    }
    Cfg { fullscreen: r.oui("plein_ecran").unwrap_or(false), ip: r.adresse("ip").unwrap_or_default(), r }
}

/// Écrit les réglages. Une partie en ligne impose les réglages de l'hôte : ils ne sont pas mémorisés.
pub fn save(t: &Table, c: &mut Cfg) {
    if t.online {
        return;
    }
    let r = &mut c.r;
    r.mettre("joueurs", t.m.np);
    r.mettre("limite", t.m.limite);
    r.mettre("niveau", t.botlvl);
    r.mettre("aide", t.help as u8);
    r.mettre("tri_valeur", t.tri_valeur as u8);
    r.mettre("animations", anim::reglage().nom());
    r.mettre("plein_ecran", c.fullscreen as u8);
    r.mettre("victoires", t.wins[0]);
    r.mettre("defaites", t.wins[1]);
    r.mettre("ip", &c.ip);
    r.enregistrer("réglages du rami");
}
