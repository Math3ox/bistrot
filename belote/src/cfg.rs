//! Réglages de la belote, mémorisés dans `belote.cfg` (voir `bistrot::reglages`).

use crate::table::Table;
use bistrot::reglages::Reglages;

pub struct Cfg {
    pub fullscreen: bool,
    pub ip: String,
    r: Reglages,
}

/// Lit les réglages (avant la première partie).
pub fn load(t: &mut Table) -> Cfg {
    let r = Reglages::charger("belote.cfg");
    if let Some(np) = r.entier("joueurs", 2, 4).filter(|&n| n != 3) {
        t.e.np = np as i32;
    }
    if let Some(o) = r.entier("objectif", 100, 10000) {
        t.e.target = o as i32;
    }
    if let Some(a) = r.entier("annonces", 0, 1) {
        t.e.optann = a as i32;
    }
    if let Some(l) = r.entier("niveau", 0, 2) {
        t.botlvl = l as i32;
    }
    if let Some(h) = r.oui("aide") {
        t.help = h;
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
    r.mettre("joueurs", t.e.np);
    r.mettre("objectif", t.e.target);
    r.mettre("annonces", t.e.optann);
    r.mettre("niveau", t.botlvl);
    r.mettre("plein_ecran", c.fullscreen as u8);
    r.mettre("aide", t.help as u8);
    r.mettre("victoires", t.wins[0]);
    r.mettre("defaites", t.wins[1]);
    r.mettre("ip", &c.ip);
    r.enregistrer("réglages de la belote");
}
