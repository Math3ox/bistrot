//! Dessin de la belote avec le moteur commun : talon et retourne, étiquettes, mains, plis animés,
//! panneaux de score et d'atout, bulles, bilan de manche, boutons, messages, force des cartes, galerie.

use crate::engine::*;
use crate::table::{Table, LEVEL_NAMES, SUIT_NAMES};
use crate::vue::{Vue, BULLE_FONDU, BULLE_POP, DUREE_RETOURNE};
use bistrot::ambiance::PLACE_CHOUETTE;
use bistrot::anim::{self, rebond, Distribution, Vol};
use bistrot::cartes::{Carte, Couleur, Paquet};
use bistrot::gfx::{alpha, Align, Color, Face, Gfx, Xf};
use bistrot::polices::Polices;
use bistrot::table::{self as tapis, main_x, Place, SOL};
use bistrot::ui::{self, *};

/// Action d'un bouton.
#[derive(Clone, Copy, PartialEq)]
pub enum BtnId {
    Take,
    Suit(usize),
    Pass,
    Next,
}

pub type Btn = Bouton<BtnId>;

/// Boutons de la phase en cours.
pub fn buttons(t: &Table) -> Vec<Btn> {
    let mine = t.is_human(t.turn());
    let b = |x: f32, y: f32, w: f32, h: f32, id: BtnId, actif: bool| Btn { x, y, w, h, id, actif };
    match t.phase() {
        PH_BID1 if mine => vec![b(400., 560., 140., 48., BtnId::Take, true), b(560., 560., 140., 48., BtnId::Pass, true)],
        PH_BID2 if mine => {
            let turned = (t.e.gs.turned >> 3) as usize;
            let mut v: Vec<Btn> =
                (0..4).map(|k| b(270. + 98. * k as f32, 560., 88., 48., BtnId::Suit(k), k != turned)).collect();
            v.push(b(676., 560., 130., 48., BtnId::Pass, true));
            v
        }
        PH_ROUNDEND | PH_MATCHEND => vec![b(425., 566., 250., 52., BtnId::Next, true)],
        _ => vec![],
    }
}

/// Indice de la carte de la main du bas sous le point, ou None.
pub fn hand_hit(t: &Table, x: f32, y: f32) -> Option<usize> {
    tapis::main_touchee(t.e.gs.hn[t.me] as usize, x, y)
}

/// Ce que l'interface doit afficher en plus de la partie.
pub struct Overlay<'a> {
    /// panneau « force des cartes » (touche F)
    pub strength: bool,
    pub hover: Option<usize>,
    pub mouse: (f32, f32),
    /// panneau du jeu en ligne : (titre, lignes, menu) ; None = rien
    pub net: Option<(&'a str, [&'a str; 3], bool)>,
    pub optline: String,
}

pub struct Scene<'a, 'b> {
    g: &'a Gfx<'b>,
    f: &'a Polices,
    deck: &'a Paquet,
    vue: &'a Vue,
}

fn carte(c: u8) -> Option<Carte> {
    Some(Carte::jeu32(c as usize))
}

impl Scene<'_, '_> {
    fn text(&self, s: &str, x: f32, y: f32, f: &Face, c: Color, al: Align) {
        self.g.text(s, x, y, f, c, al);
    }

    /// Texte écrit sur le bois, avec une ombre pour rester lisible dans la pénombre.
    fn text_ombre(&self, s: &str, x: f32, y: f32, f: &Face, c: Color, al: Align) {
        self.g.text(s, x + 1.5, y + 1.5, f, alpha(Color::new(0.0, 0.0, 0.0, 1.0), 0.75), al);
        self.g.text(s, x, y, f, c, al);
    }

    fn place(&self, t: &Table, p: usize) -> Place {
        t.places()[t.disp_pos(p)]
    }

    fn talon(&self, t: &Table) {
        let gs = &t.e.gs;
        if t.np() == 2 && gs.dpos < 32 {
            for k in 0..3 {
                let xf = Xf::place(300.0 - k as f64, 352.0 - 0.5 * k as f64, 0.95, 0.0, SOL);
                self.deck.poser(self.g, &xf, None, false);
            }
            self.text_ombre(&format!("Talon : {}", 32 - gs.dpos), 300., 398., &self.f.etiquette, C_WHITE, Align::Center);
        }
    }

    /// La retourne : face cachée pendant la distribution, puis elle se retourne.
    fn retourne(&self, t: &Table) {
        let gs = &t.e.gs;
        if gs.phase != PH_BID1 && gs.phase != PH_BID2 {
            return;
        }
        let k = ((t.now - self.vue.retourne.1) / DUREE_RETOURNE).clamp(0.0, 1.0);
        let base = Xf::place(550.0, 352.0 - 30.0 * (std::f64::consts::PI * k).sin(), 1.0, 0.05, SOL);
        let sx = (std::f64::consts::PI * k).cos().abs().max(0.02) as f32;
        let xf = base.puis(&Xf { a: sx, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 });
        self.deck.poser(self.g, &xf, if k > 0.5 { carte(gs.turned as u8) } else { None }, false);
    }

    /// Cartes en cours de distribution (ou posées, en attendant que toute la main soit arrivée).
    fn distribution(&self, t: &Table) {
        // le paquet du donneur, tant qu'il reste des cartes à distribuer
        let restantes = self.vue.vols.iter().filter(|v| t.now < v.t0).count();
        if let Some(v) = self.vue.vols.iter().find(|v| t.now < v.t0) {
            for k in 0..restantes.min(4) {
                let xf = Xf::place(v.src.0 - k as f64 * 0.8, v.src.1 - k as f64 * 1.6, 0.62, 0.3, SOL);
                self.deck.poser(self.g, &xf, None, false);
            }
        }
        for v in &self.vue.vols {
            let ms = t.now - v.t0;
            if ms < 0.0 {
                continue; // encore dans le paquet
            }
            let pl = self.place(t, v.p);
            let n = t.e.gs.hn[v.p] as usize;
            let d = if v.p == t.me {
                let i = t.e.gs.hand[t.me][..n].iter().position(|&c| c == v.carte).unwrap_or(0);
                Distribution { src: v.src, dst: (main_x(i, n), 686.0), echelle: 1.04, rotation: 0.0, retourner: true }
            } else {
                Distribution { src: v.src, dst: tapis::pos_dos(&pl, v.index, n), echelle: 0.62, rotation: pl.rotation, retourner: false }
            };
            let (xf, ombre, face) = d.a(ms);
            if let Some((x, y, rx, ry)) = ombre {
                self.g.ellipse(&Xf::ID, x, y, rx, ry, Some(C_SHFLY), Some(C_SHFLY));
            }
            self.deck.poser(self.g, &xf, if face { carte(v.carte) } else { None }, false);
        }
    }

    fn labels(&self, t: &Table) {
        let ph = t.phase();
        for p in 0..t.np() {
            let tour = matches!(ph, PH_BID1 | PH_BID2 | PH_PLAY) && p == t.turn();
            ui::etiquette(self.g, self.f, &self.place(t, p), &t.player_name(p), (p ^ t.me) & 1 == 0, tour, p as i32 == t.e.dealer);
        }
    }

    fn opp_hands(&self, t: &Table) {
        for p in 0..t.np() {
            if p == t.me {
                continue;
            }
            let pl = self.place(t, p);
            let n = t.e.gs.hn[p] as usize;
            // seules les cartes déjà arrivées (les autres sont en cours de distribution)
            for i in 0..self.vue.stable[p].min(n) {
                self.deck.poser(self.g, &tapis::dos_adversaire(&pl, i, n), None, false);
            }
        }
    }

    /// Cartes du pli : envoi en arc depuis la main, puis ramassage vers le gagnant.
    fn tricks(&self, t: &Table) {
        let gs = &t.e.gs;
        let np = t.np();
        let ramassage = (t.collect > 0.0).then(|| {
            let w = t.e.trick_best().0 as usize;
            (self.place(t, w).main, t.now - t.collect)
        });
        for k in 0..np {
            let p = (gs.tlead as usize + k) % np;
            let c = gs.tcard[p];
            if c == 0xFF {
                continue;
            }
            let pl = self.place(t, p);
            let vol = Vol {
                depart: (t.asx[p], t.asy[p]),
                echelle: if p == t.me { 1.04 } else { 0.62 },
                arrivee: pl.pli,
                angle: pl.angle,
            };
            let (xf, ombre) = vol.a(t.now - t.ast0[p], ramassage);
            if let Some((x, y, rx, ry)) = ombre {
                self.g.ellipse(&Xf::ID, x, y, rx, ry, Some(C_SHFLY), Some(C_SHFLY));
            }
            self.deck.poser(self.g, &xf, carte(c), false);
        }
    }

    fn my_hand(&self, t: &Table, hover: Option<usize>) {
        let me = t.me;
        let n = t.e.gs.hn[me] as usize;
        let legal = t.my_legal();
        for i in 0..n {
            let c = t.e.gs.hand[me][i];
            if self.vue.vols.iter().any(|v| v.p == me && v.carte == c) {
                continue; // en cours de distribution
            }
            // position et levée animées (glissement vers la place, levée au survol)
            let x = self.vue.x.get(&c).copied().unwrap_or_else(|| main_x(i, n));
            let mut y = 686.0 - self.vue.levee.get(&c).copied().unwrap_or(0.0);
            if !self.vue.levee.contains_key(&c) && hover == Some(i) && legal >> i & 1 == 1 {
                y -= 18.0;
            }
            let xf = Xf::place(x, y, 1.04, 0.0, 1.0);
            if t.hint == i as i32 {
                self.g.rrect(&xf, -56., -76., 112., 152., 16., Some(C_GOLD), Some(C_GOLD));
            }
            self.deck.poser(self.g, &xf, carte(c), legal >> i & 1 == 0);
        }
    }

    /// Parchemins des panneaux de score et d'atout (couche fixe : seuls les textes changent).
    fn panels_fond(&self) {
        ui::parchemin(self.g, 14., 12., 310., 96., 8.);
        ui::parchemin(self.g, 824., 12., 262., 96., 8.);
    }

    fn panels(&self, t: &Table) {
        let f = self.f;
        let us = t.me & 1;
        let two = t.np() == 2;
        self.text(if two { "TOI" } else { "NOUS" }, 30., 20., &f.moyen, C_NOUS, Align::Left);
        self.text(if two { "BOT" } else { "EUX" }, 30., 54., &f.moyen, C_EUX, Align::Left);
        // le score défile jusqu'à sa nouvelle valeur
        let sc = |k: usize| (self.vue.score[k].round() as i64).to_string();
        self.text(&sc(us), 308., 20., &f.moyen, C_ENCRE, Align::Right);
        self.text(&sc(us ^ 1), 308., 54., &f.moyen, C_ENCRE, Align::Right);
        let obj = format!("Objectif {}   -   victoires {} / défaites {}", t.e.target, t.wins[0], t.wins[1]);
        self.text(&obj, 30., 86., &f.petit, C_ENCRE_PALE, Align::Left);
        self.text("Atout", 842., 18., &f.etiquette, C_RUBRIQUE, Align::Left);
        let trump = t.e.gs.trump;
        let (suit, col) = if trump < 0 {
            (Couleur::depuis((t.e.gs.turned >> 3) as usize), alpha(C_ENCRE_PALE, 0.6))
        } else {
            let c = Couleur::depuis(trump as usize);
            (c, if c.rouge() { C_ROUGE } else { C_ENCRE })
        };
        bistrot::cartes::symbole(self.g, &Xf::ID, suit, 1048., 60., 20., col);
        if trump >= 0 {
            let s = format!("pris par {}", t.player_name(t.e.gs.taker as usize));
            self.text(&s, 842., 56., &f.petit, C_ENCRE, Align::Left);
        }
    }

    /// Bulles : apparition qui rebondit, fondu à la fin.
    fn bubbles(&self, t: &Table) {
        for p in 0..t.np() {
            let Some((txt, fin, debut)) = &self.vue.bulles[p] else { continue };
            if *fin < t.now {
                continue;
            }
            let taille = 0.6 + 0.4 * rebond((t.now - debut) / BULLE_POP);
            let opacite = if anim::actives() { ((fin - t.now) / BULLE_FONDU).min(1.0) } else { 1.0 };
            ui::bulle_animee(self.g, self.f, self.place(t, p).bulle, txt, taille as f32, opacite as f32);
        }
    }

    /// Bilan de fin de manche / de partie.
    fn overlay(&self, t: &Table) {
        let ph = t.phase();
        if ph != PH_ROUNDEND && ph != PH_MATCHEND {
            return;
        }
        let f = self.f;
        let e = &t.e;
        let us = t.me & 1;
        ui::panneau(self.g, 250., 122., 600., 426., 20.);
        let title = if ph != PH_MATCHEND {
            "Fin de manche"
        } else if e.rs_winner as usize == us {
            "Victoire !"
        } else {
            "Défaite..."
        };
        self.text(title, 550., 136., &f.titre, C_RUBRIQUE, Align::Center);
        let contract = match e.rs_made {
            1 => "CONTRAT REMPLI",
            0 => "DEDANS !",
            _ => "LITIGE",
        };
        self.text(contract, 550., 196., &f.moyen, C_ENCRE, Align::Center);
        let who = format!(
            "Preneur : {}   -   atout : {}",
            t.player_name(e.gs.taker.max(0) as usize),
            SUIT_NAMES[e.gs.trump.clamp(0, 3) as usize]
        );
        self.text(&who, 550., 228., &f.petit, C_ENCRE_PALE, Align::Center);
        let two = t.np() == 2;
        self.text(if two { "TOI" } else { "NOUS" }, 640., 254., &f.etiquette, C_NOUS, Align::Right);
        self.text(if two { "BOT" } else { "EUX" }, 810., 254., &f.etiquette, C_EUX, Align::Right);
        let rows: [(&str, [i32; 2]); 6] = [
            ("Points des plis", e.gs.rpts),
            ("Belote-rebelote", e.gs.bel),
            ("Annonces", e.gs.ann),
            ("Total", e.rs_tot),
            ("Marqué", e.rs_add),
            ("Score", e.score),
        ];
        for (r, (name, v)) in rows.iter().enumerate() {
            let y = 290.0 + 34.0 * r as f32;
            self.text(name, 290., y, &f.etiquette, if r < 3 { C_ENCRE } else { C_RUBRIQUE }, Align::Left);
            self.text(&v[us].to_string(), 640., y, &f.etiquette, C_ENCRE, Align::Right);
            self.text(&v[us ^ 1].to_string(), 810., y, &f.etiquette, C_ENCRE, Align::Right);
        }
        if e.rs_capot >= 0 {
            self.text("CAPOT ! (+90)", 550., 502., &f.etiquette, C_RUBRIQUE, Align::Center);
        }
    }

    fn buttons(&self, t: &Table, mouse: (f32, f32)) {
        for b in buttons(t) {
            let label = match b.id {
                BtnId::Take => "PRENDRE",
                BtnId::Pass => "PASSER",
                BtnId::Next if t.phase() == PH_MATCHEND => "NOUVELLE PARTIE",
                BtnId::Next => "SUITE",
                BtnId::Suit(k) => SUIT_NAMES[k],
            };
            b.dessiner(self.g, self.f, label, mouse);
        }
    }

    /// Touche F : ordre de force et valeur des cartes, à l'atout et hors atout.
    fn strength(&self, t: &Table) {
        const ORDER_T: [usize; 8] = [4, 2, 7, 3, 6, 5, 1, 0]; // V 9 A 10 R D 8 7
        const ORDER_P: [usize; 8] = [7, 3, 6, 5, 4, 2, 1, 0]; // A 10 R D V 9 8 7
        const PTS_T: [i32; 8] = [0, 0, 14, 10, 20, 3, 4, 11];
        const PTS_P: [i32; 8] = [0, 0, 0, 10, 2, 3, 4, 11];
        let f = self.f;
        let gs = &t.e.gs;
        // couleur d'atout : celle de la donne, sinon celle de la retourne ; une autre couleur pour « hors atout »
        let trump = if gs.trump >= 0 { gs.trump as usize } else { (gs.turned >> 3) as usize };
        let plain = (trump + 1) % 4;
        ui::panneau(self.g, 130., 96., 840., 600., 20.);
        self.text("Force des cartes", 550., 110., &f.titre, C_RUBRIQUE, Align::Center);
        self.text("de la plus forte (à gauche) à la plus faible", 550., 166., &f.petit, C_ENCRE_PALE, Align::Center);
        let rows = [
            (format!("À l'atout ({})", SUIT_NAMES[trump]), trump, ORDER_T, PTS_T, 192.0),
            (format!("Hors atout (ex. {})", SUIT_NAMES[plain]), plain, ORDER_P, PTS_P, 412.0),
        ];
        for (title, suit, order, pts, y) in rows {
            self.text(&title, 170., y, &f.moyen, C_ENCRE, Align::Left);
            for (k, &r) in order.iter().enumerate() {
                let x = 205.0 + 98.0 * k as f32;
                let xf = Xf::place(x as f64, (y + 102.0) as f64, 0.6, 0.0, 1.0);
                self.deck.poser(self.g, &xf, carte((suit * 8 + r) as u8), false);
                let p = pts[r];
                let col = if p > 0 { C_RUBRIQUE } else { C_ENCRE_PALE };
                let label = if p > 1 { format!("{p} pts") } else { format!("{p} pt") };
                self.text(&label, x, y + 152.0, &f.etiquette, col, Align::Center);
            }
        }
        self.text(
            "Total : 152 points + 10 de der = 162 par donne.   Belote-rebelote (R + D d'atout) : 20.",
            550.,
            606.,
            &f.petit,
            C_ENCRE,
            Align::Center,
        );
        self.text(
            "Annonces : tierce 20, cinquante 50, cent 100 ; carré de valets 200, de 9 150, d'as / 10 / rois / dames 100.",
            550.,
            628.,
            &f.petit,
            C_ENCRE,
            Align::Center,
        );
        self.text("F, Échap ou clic : fermer", 550., 652., &f.petit, C_ENCRE_PALE, Align::Center);
    }

    /// Touche G : les 32 cartes et le dos.
    fn gallery(&self) {
        for i in 0..33 {
            let xf = Xf::place((118 + (i & 7) * 126) as f64, (100 + (i >> 3) * 150) as f64, 0.9, 0.0, 1.0);
            self.deck.poser(self.g, &xf, if i < 32 { carte(i as u8) } else { None }, false);
        }
    }
}

/// Ligne des réglages en bas de l'écran.
pub fn optline(t: &Table) -> String {
    let yn = |b: bool| if b { "oui" } else { "non" };
    format!(
        "J: {} joueurs  L: niveau {}  A: annonces {}  H: aide {}  M: animations {}  F: force des cartes  F11: plein écran  N: en ligne  Echap: quitter",
        t.np(),
        LEVEL_NAMES[t.botlvl as usize],
        yn(t.e.optann != 0),
        yn(t.help),
        anim::libelle()
    )
}

/// Galerie des cartes (touche G), sur le décor.
pub fn gallery(g: &Gfx, f: &Polices, deck: &Paquet) {
    Scene { g, f, deck, vue: &Vue::default() }.gallery();
}

/// Couche fixe tant que l'état ne change pas (talon, étiquettes, mains adverses) : mise en cache.
pub fn draw_under(t: &Table, g: &Gfx, f: &Polices, deck: &Paquet, vue: &Vue) {
    let s = Scene { g, f, deck, vue };
    s.panels_fond();
    s.talon(t);
    s.labels(t);
    s.opp_hands(t);
}

/// Empreinte de la couche fixe.
pub fn under_signature(t: &Table, vue: &Vue) -> u64 {
    use std::hash::{Hash, Hasher};
    let gs = &t.e.gs;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (gs.phase, gs.turn, gs.hn, vue.stable, gs.dpos, t.e.dealer, t.e.np, t.me, t.e.lvl).hash(&mut h);
    h.finish()
}

/// Tout ce qui bouge ou change souvent, par-dessus la couche fixe.
pub fn draw_over(t: &Table, g: &Gfx, f: &Polices, deck: &Paquet, ov: &Overlay, vue: &Vue) {
    let s = Scene { g, f, deck, vue };
    // la chouette est posée sur la table : les cartes passent au-dessus
    vue.chouette.dessiner(g, PLACE_CHOUETTE.0, PLACE_CHOUETTE.1);
    s.retourne(t);
    s.tricks(t);
    s.my_hand(t, ov.hover);
    s.distribution(t);
    s.panels(t);
    s.bubbles(t);
    s.overlay(t);
    s.buttons(t, ov.mouse);
    s.text_ombre(&t.message(), 550., 755., &f.etiquette, C_TXT, Align::Center);
    s.text_ombre(&ov.optline, 550., 780., &f.petit, C_TXTDIM, Align::Center);
    if ov.strength {
        s.strength(t);
    }
    if let Some((title, lines, menu)) = ov.net {
        ui::panneau_reseau(g, f, title, lines, menu);
    }
}
