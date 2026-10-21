//! Dessin du rami avec le moteur commun : étiquettes, mains adverses, talon et défausse, combinaisons de la
//! table (rangées qui se resserrent quand la table se remplit), main du joueur (cartes choisies levées, conseil
//! cerclé d'or, carte de la défausse à poser cerclée de rouge), cartes qui volent, panneaux des scores et du talon,
//! bulles, boutons, bilan de la manche, classement, aide-mémoire, galerie des cartes.

use crate::engine::*;
use crate::table::{main_touchee, Table, LEVEL_NAMES};
use crate::vue::{Vue, BULLE_FONDU, BULLE_POP, DEFAUSSE, ECHELLE_MAIN, ECHELLE_PILE, MAIN_Y, PETITE, TALON};
use bistrot::anim::{self, rebond, Distribution};
use bistrot::cartes::{Carte, Paquet};
use bistrot::gfx::{alpha, rgb, Align, Color, Face, Gfx, Xf};
use bistrot::polices::Polices;
use bistrot::table::{self as tapis, Place};
use bistrot::ui::{self, *};

/// Action d'un bouton.
#[derive(Clone, Copy, PartialEq)]
pub enum BtnId {
    Poser,
    Defausser,
    Annuler,
    Tri,
    Next,
}

pub type Btn = Bouton<BtnId>;

/// Colonne des boutons, à droite de la main.
const BX: f32 = 950.0;
const BW: f32 = 138.0;

/// Boutons de la phase en cours.
pub fn buttons(t: &Table) -> Vec<Btn> {
    let b = |x: f32, y: f32, w: f32, h: f32, id: BtnId, actif: bool| Btn { x, y, w, h, id, actif };
    match t.phase() {
        PH_FINMANCHE | PH_FINPARTIE => vec![b(425., 566., 250., 52., BtnId::Next, true)],
        _ if t.auto || t.m.niveaux[t.me] != -1 => vec![b(BX, 704., BW, 40., BtnId::Tri, true)],
        _ => {
            let a_moi = t.my_turn();
            let jeu = a_moi && t.phase() == PH_JEU;
            let s = t.selection;
            vec![
                b(BX, 566., BW, 40., BtnId::Poser, jeu && former(s).is_some()),
                b(BX, 612., BW, 40., BtnId::Defausser, jeu && nb(s) == 1),
                b(BX, 658., BW, 40., BtnId::Annuler, a_moi && t.m.annulable()),
                b(BX, 704., BW, 40., BtnId::Tri, true),
            ]
        }
    }
}

/// Texte d'un bouton.
fn label(t: &Table, id: BtnId) -> String {
    match id {
        BtnId::Poser => "POSER".into(),
        BtnId::Defausser => "DÉFAUSSER".into(),
        BtnId::Annuler => "ANNULER".into(),
        BtnId::Tri => "TRIER".into(),
        BtnId::Next if t.phase() == PH_FINPARTIE => "NOUVELLE PARTIE".into(),
        BtnId::Next => "SUITE".into(),
    }
}

/// Carte de la main du bas sous le point, ou None.
pub fn hand_hit(t: &Table, x: f32, y: f32) -> Option<u8> {
    let main = t.ma_main();
    main_touchee(main.len(), x, y).map(|i| main[i])
}

/// Ce qui est sous la souris sur la table.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Cible {
    Talon,
    Defausse,
    /// combinaison, et carte de la combinaison
    Combi(usize, usize),
}

fn dans_pile(c: (f64, f64), x: f32, y: f32) -> bool {
    let (w, h) = (50.0 * ECHELLE_PILE + 4.0, 70.0 * ECHELLE_PILE + 4.0);
    (x as f64 - c.0).abs() <= w && (y as f64 - c.1).abs() <= h
}

/// Talon, défausse ou combinaison sous le point.
pub fn table_hit(t: &Table, vue: &Vue, x: f32, y: f32) -> Option<Cible> {
    if dans_pile(TALON, x, y) {
        return Some(Cible::Talon);
    }
    if dans_pile(DEFAUSSE, x, y) {
        return Some(Cible::Defausse);
    }
    let table = &t.m.d.table;
    for (k, r) in vue.rangees.iter().enumerate() {
        let Some(cb) = table.get(k) else { break };
        let n = cb.cartes.len();
        let (x0, y0, x1, y1) = r.cadre(n);
        if (x as f64) >= x0 && (x as f64) <= x1 && (y as f64) >= y0 && (y as f64) <= y1 {
            return Some(Cible::Combi(k, r.carte_touchee(n, x as f64).unwrap_or(0)));
        }
    }
    None
}

/// Ce que l'interface doit afficher en plus de la partie.
pub struct Overlay<'a> {
    /// aide-mémoire (touche F)
    pub memo: bool,
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

/// i-ème dos de la main d'un adversaire (n cartes), à l'échelle des petites cartes (copie pixel pour pixel).
fn dos(pl: &Place, i: usize, n: usize) -> Xf {
    let (x, y) = tapis::pos_dos(pl, i, n);
    Xf::place(x.round(), y.round(), PETITE, pl.rotation, 1.0)
}

/// Image d'une carte du rami.
pub fn carte(c: u8) -> Option<Carte> {
    Some(if est_joker(c) { Carte::Joker(c % 2 == 0) } else { Carte::jeu52(c as usize % 52) })
}

const C_CIRE: Color = rgb(150, 22, 30);

impl Scene<'_, '_> {
    fn text(&self, s: &str, x: f32, y: f32, f: &Face, c: Color, al: Align) {
        self.g.text(s, x, y, f, c, al);
    }

    /// Texte écrit sur le bois, avec une ombre pour rester lisible dans la pénombre.
    fn text_ombre(&self, s: &str, x: f32, y: f32, f: &Face, c: Color, al: Align) {
        self.g.text(s, x + 1.5, y + 1.5, f, Color::new(0.0, 0.0, 0.0, 0.75), al);
        self.g.text(s, x, y, f, c, al);
    }

    fn place(&self, t: &Table, p: usize) -> Place {
        t.places()[t.disp_pos(p)]
    }

    // ---------- couche fixe ----------
    fn labels(&self, t: &Table) {
        let jeu = matches!(t.phase(), PH_PIOCHE | PH_JEU);
        for p in 0..t.np() {
            let pl = self.place(t, p);
            // sur les côtés, la pièce du donneur passe sur la table, à côté de la main (pas sous les chandelles)
            let cote = pl.rotation != 0.0;
            let donneur = p == t.m.donneur;
            ui::etiquette(self.g, self.f, &pl, &t.player_name(p), p == t.me, jeu && p == t.turn(), donneur && !cote);
            if donneur && cote {
                let x = if pl.main.0 < 550.0 { 160.0 } else { 940.0 };
                ui::piece(self.g, &self.f.etiquette, x, 180.0, 13.0, "D");
            }
        }
    }

    fn opp_hands(&self, t: &Table) {
        for p in 0..t.np() {
            if p == t.me {
                continue;
            }
            let pl = self.place(t, p);
            let n = self.vue.stable[p];
            for i in 0..n {
                self.deck.poser(self.g, &dos(&pl, i, n), None, false);
            }
        }
    }

    /// Le talon : une pile de dos, plus ou moins haute.
    fn talon(&self, t: &Table) {
        let n = t.m.d.talon.len();
        if n == 0 {
            self.g.rrect(&Xf::ID, TALON.0 as f32 - 31., TALON.1 as f32 - 43., 62., 87., 8., None, Some(alpha(C_TXTDIM, 0.6)));
            return;
        }
        let k = n.div_ceil(14).min(6);
        for i in 0..k {
            let xf = Xf::place((TALON.0 - i as f64).round(), (TALON.1 - i as f64 * 2.0).round(), ECHELLE_PILE, 0.0, 1.0);
            self.deck.poser(self.g, &xf, None, false);
        }
    }

    /// Parchemins des panneaux (couche fixe : seuls les textes changent).
    fn panels_fond(&self, t: &Table) {
        ui::parchemin(self.g, 14., 12., 322., hauteur_scores(t.np()), 8.);
        ui::parchemin(self.g, 824., 12., 262., 96., 8.);
    }

    // ---------- ce qui bouge ----------
    fn poser_carte(&self, c: u8, (x, y, s): (f64, f64, f64)) {
        self.deck.poser(self.g, &Xf::place(x, y, s, 0.0, 1.0), carte(c), false);
    }

    /// Combinaison sous la souris que la sélection complète (ou dont elle reprend le joker) : cadre doré.
    fn survol_table(&self, t: &Table, mouse: (f32, f32)) {
        if !t.my_turn() || t.phase() != PH_JEU || t.selection == 0 || !t.m.d.ouvert[t.me] {
            return;
        }
        let Some(Cible::Combi(k, i)) = table_hit(t, self.vue, mouse.0, mouse.1) else { return };
        let cb = &t.m.d.table[k];
        let s = t.selection;
        let echange = nb(s) == 1 && cb.cartes.get(i).is_some_and(|&c| est_joker(c)) && remplacer_joker(cb, s.trailing_zeros() as u8).is_some();
        let ajout = completer(cb, s).is_some();
        if !echange && !ajout {
            return;
        }
        let r = self.vue.rangees[k];
        let (x0, y0, x1, y1) = if echange {
            let (x, y) = r.carte(i);
            let (w, h) = (50.0 * r.echelle, 70.0 * r.echelle);
            (x - w, y - h, x + w, y + h)
        } else {
            r.cadre(cb.cartes.len())
        };
        let (x0, y0, w, h) = ((x0 - 4.0) as f32, (y0 - 4.0) as f32, (x1 - x0 + 8.0) as f32, (y1 - y0 + 8.0) as f32);
        self.g.rrect(&Xf::ID, x0, y0, w, h, 10., Some(alpha(C_GOLD, 0.16)), None);
        self.g.rrect_w(&Xf::ID, x0, y0, w, h, 10., None, Some(C_GOLD), 3.0);
    }

    fn ordre_table(&self, t: &Table) -> Vec<u8> {
        ordre_table(t, self.vue)
    }

    /// Cartes de la table et de la défausse arrivées à leur place (couche fixe).
    fn table_fixe(&self, t: &Table) {
        for c in self.ordre_table(t) {
            if !self.vue.en_mouvement(c) {
                if let Some(&p) = self.vue.cible.get(&c) {
                    self.poser_carte(c, p);
                }
            }
        }
    }

    /// Main du joueur, puis les cartes qui bougent (de la main, de la table, de la défausse) par-dessus.
    fn cartes(&self, t: &Table) {
        let d = &t.m.d;
        let main = t.ma_main();
        let distribuees: Vec<u8> = self.vue.vols.iter().filter(|v| v.p == t.me).map(|v| v.carte).collect();
        let mobiles: Vec<u8> = self.ordre_table(t).into_iter().chain(main.iter().copied()).filter(|&c| self.vue.en_mouvement(c)).collect();
        for &c in &main {
            if mobiles.contains(&c) || distribuees.contains(&c) {
                continue;
            }
            let Some(&(x, y, s)) = self.vue.pos.get(&c) else { continue };
            let y = y - self.vue.levee.get(&c).copied().unwrap_or(0.0);
            let xf = Xf::place(x, y, s, 0.0, 1.0);
            if t.selection & bit(c) != 0 || t.hint & bit(c) != 0 {
                self.g.rrect(&xf, -56., -76., 112., 152., 16., Some(C_GOLD), Some(C_GOLD));
            }
            if d.pris == c as i8 && d.phase == PH_JEU && d.tour as usize == t.me {
                self.g.rrect_w(&xf, -55., -75., 110., 150., 15., None, Some(C_RUBRIQUE), 3.0);
            }
            self.deck.poser(self.g, &xf, carte(c), false);
        }
        for &c in &mobiles {
            if let Some(&(x, y, s)) = self.vue.pos.get(&c) {
                let y = y - self.vue.levee.get(&c).copied().unwrap_or(0.0);
                self.g.ellipse(&Xf::ID, (x + 6.0) as f32, (y + 10.0) as f32, (40.0 * s) as f32, (26.0 * s) as f32, Some(C_SHFLY), None);
                self.poser_carte(c, (x, y, s));
            }
        }
    }

    /// Cartes qui volent vers la main d'un adversaire.
    fn envols(&self, t: &Table) {
        for e in &self.vue.envols {
            let (k, h) = Vue::avancement(e, t.now);
            let pl = self.place(t, e.p);
            let (x1, y1) = pl.main;
            let x = e.depart.0 + (x1 - e.depart.0) * k;
            let y = e.depart.1 + (y1 - e.depart.1) * k - 40.0 * h;
            let s = e.depart.2 + (PETITE - e.depart.2) * k;
            let rot = pl.rotation * k;
            self.g.ellipse(&Xf::ID, (x + 6.0) as f32, (y + 10.0 + 40.0 * h) as f32, 26.0, 16.0, Some(C_SHFLY), None);
            self.deck.poser(self.g, &Xf::place(x, y, s, rot, 1.0), e.carte.and_then(carte), false);
        }
    }

    /// Cartes en cours de distribution (depuis le talon).
    fn distribution(&self, t: &Table) {
        let n_moi = nb(t.m.d.mains[t.me]) as usize;
        for v in &self.vue.vols {
            let ms = t.now - v.t0;
            if ms < 0.0 {
                continue;
            }
            let d = if v.p == t.me {
                Distribution { src: TALON, dst: (crate::table::main_x(v.index, n_moi), MAIN_Y), echelle: ECHELLE_MAIN, rotation: 0.0, retourner: true }
            } else {
                let pl = self.place(t, v.p);
                let n = nb(t.m.d.mains[v.p]) as usize;
                Distribution { src: TALON, dst: tapis::pos_dos(&pl, v.index, n), echelle: PETITE, rotation: pl.rotation, retourner: false }
            };
            let (xf, ombre, face) = d.a(ms);
            if let Some((x, y, rx, ry)) = ombre {
                self.g.ellipse(&Xf::ID, x, y, rx, ry, Some(C_SHFLY), Some(C_SHFLY));
            }
            self.deck.poser(self.g, &xf, if face { carte(v.carte) } else { None }, false);
        }
        // cartes adverses déjà arrivées (le fond ne les montre qu'à la fin de la distribution)
        if !self.vue.vols.is_empty() {
            for p in 0..t.np() {
                if p == t.me {
                    continue;
                }
                let pl = self.place(t, p);
                let n = nb(t.m.d.mains[p]) as usize;
                let arrivees = self.vue.vols.iter().filter(|v| v.p == p && t.now >= v.t0 + bistrot::anim::DUREE_DISTRIB).count();
                for i in 0..arrivees.min(n) {
                    self.deck.poser(self.g, &dos(&pl, i, n), None, false);
                }
            }
        }
    }

    fn panels(&self, t: &Table) {
        let f = self.f;
        let np = t.np();
        let d = &t.m.d;
        for k in 0..np {
            let p = (t.me + k) % np;
            let (x, y) = (26.0 + (k % 2) as f32 * 156.0, 17.0 + (k / 2) as f32 * 24.0);
            let encre = if p == t.me { C_NOUS } else { C_EUX };
            if d.ouvert[p] {
                // sceau de cire : ce joueur a ouvert
                self.g.ellipse(&Xf::ID, x + 3.0, y + 12.0, 5.5, 5.5, Some(C_CIRE), Some(rgb(96, 12, 18)));
            }
            self.text(&t.player_name(p), x + 13.0, y, &f.etiquette, encre, Align::Left);
            let s = (self.vue.score[p].round() as i64).to_string();
            self.text(&s, x + 146.0, y, &f.etiquette, C_ENCRE, Align::Right);
        }
        let rangs = (np + 1) / 2;
        let ligne = format!("Manche {} · limite {} points", t.m.manche + 1 - (d.phase >= PH_FINMANCHE) as u32, t.m.limite);
        self.text(&ligne, 28., 20. + 24. * rangs as f32, &f.petit, C_ENCRE_PALE, Align::Left);
        // talon et ouverture
        self.text(&format!("Talon : {} cartes", d.talon.len()), 842., 18., &f.etiquette, C_RUBRIQUE, Align::Left);
        let me = t.me;
        let l2 = if d.ouvert[me] {
            "Tu as ouvert".to_string()
        } else if d.tour as usize == me && d.pose_tour > 0 {
            format!("Posés ce tour : {} / {OUVERTURE}", d.pose_tour)
        } else {
            format!("Pour ouvrir : {OUVERTURE} points")
        };
        self.text(&l2, 842., 48., &f.petit, C_ENCRE, Align::Left);
        let l3 = if d.recyclages > 0 { format!("Talon refait {} fois sur 2", d.recyclages) } else { format!("{} combinaison{} sur la table", d.table.len(), if d.table.len() > 1 { "s" } else { "" }) };
        self.text(&l3, 842., 74., &f.petit, C_ENCRE_PALE, Align::Left);
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
            let pos = if t.disp_pos(p) == 0 { (545.0, 560.0) } else { self.place(t, p).bulle };
            ui::bulle_animee(self.g, self.f, pos, txt, taille as f32, opacite as f32);
        }
    }

    /// Bilan de la manche, ou classement de fin de partie.
    fn overlay(&self, t: &Table) {
        let ph = t.phase();
        if ph != PH_FINMANCHE && ph != PH_FINPARTIE {
            return;
        }
        let Some(b) = t.m.bilan else { return };
        let f = self.f;
        let np = t.np();
        ui::panneau(self.g, 230., 112., 640., 444., 20.);
        let titre = if ph == PH_FINPARTIE {
            if t.m.classement()[0] == t.me {
                "Victoire !"
            } else {
                "Fin de la partie"
            }
        } else if b.gagnant < 0 {
            "Manche nulle"
        } else if b.rami {
            "Rami !"
        } else {
            "Fin de la manche"
        };
        self.text(titre, 550., 128., &f.titre, C_RUBRIQUE, Align::Center);
        let sous = if b.gagnant < 0 {
            "Le talon est épuisé : personne ne marque.".to_string()
        } else if b.rami {
            format!("{} a tout posé d'un coup : pénalités doublées", t.player_name(b.gagnant as usize))
        } else {
            format!("{} n'a plus de cartes", t.player_name(b.gagnant as usize))
        };
        self.text(&sous, 550., 190., &f.etiquette, C_ENCRE, Align::Center);
        let limite = format!("La partie s'arrête quand un joueur atteint {} points ; le moins pénalisé gagne.", t.m.limite);
        self.text(&limite, 550., 220., &f.petit, C_ENCRE_PALE, Align::Center);
        self.text("Cartes", 560., 262., &f.etiquette, C_RUBRIQUE, Align::Right);
        self.text("Manche", 690., 262., &f.etiquette, C_RUBRIQUE, Align::Right);
        self.text("Total", 820., 262., &f.etiquette, C_RUBRIQUE, Align::Right);
        let ordre: Vec<usize> = if ph == PH_FINPARTIE { t.m.classement() } else { (0..np).map(|k| (t.me + k) % np).collect() };
        for (r, &p) in ordre.iter().enumerate() {
            let y = 294.0 + 40.0 * r as f32;
            let encre = if p == t.me { C_NOUS } else { C_EUX };
            let mut n = t.player_name(p);
            if ph == PH_FINPARTIE {
                n = format!("{}. {n}", r + 1);
            }
            self.text(&n, 270., y, &f.etiquette, encre, Align::Left);
            let reste = b.restes[p];
            let detail = if b.gagnant == p as i8 {
                "fini".to_string()
            } else if b.gagnant < 0 {
                format!("{}", nb(reste))
            } else if !b.ouverts[p] {
                format!("{} (pas ouvert)", nb(reste))
            } else {
                format!("{}", nb(reste))
            };
            self.text(&detail, 560., y, &f.etiquette, C_ENCRE, Align::Right);
            let g = b.penalites[p];
            self.text(&if g > 0 { format!("+{g}") } else { "0".into() }, 690., y, &f.etiquette, C_ENCRE, Align::Right);
            self.text(&t.m.scores[p].to_string(), 820., y, &f.etiquette, C_ENCRE, Align::Right);
        }
    }

    fn buttons(&self, t: &Table, mouse: (f32, f32)) {
        for b in buttons(t) {
            b.dessiner(self.g, self.f, &label(t, b.id), mouse);
        }
    }

    /// Touche F : combinaisons, ouverture, jokers, pénalités, commandes.
    fn memo(&self) {
        let f = self.f;
        ui::panneau(self.g, 110., 70., 880., 650., 20.);
        self.text("Aide-mémoire", 550., 84., &f.titre, C_RUBRIQUE, Align::Center);
        let blocs: [(&str, &[&str]); 4] = [
            (
                "Combinaisons",
                &[
                    "Suite : au moins 3 cartes de la même couleur qui se suivent (as en bas A-2-3 ou en haut D-R-A).",
                    "Brelan ou carré : 3 ou 4 cartes de même valeur, de couleurs différentes. Un joker au plus par combinaison.",
                ],
            ),
            (
                "Ouverture",
                &[
                    "Poser 51 points en un tour : 2 à 10 leur valeur, figures 10, as 11 (1 dans A-2-3), joker = la carte remplacée.",
                    "Ensuite : poser, compléter n'importe quelle combinaison de la table, reprendre un joker avec sa carte.",
                ],
            ),
            (
                "Tour",
                &[
                    "Piocher au talon ou prendre la défausse (cette carte doit être posée dans le tour), poser, défausser.",
                    "Un joker repris doit être reposé dans le tour. ANNULER revient au début du tour.",
                ],
            ),
            (
                "Fin de manche",
                &[
                    "Le premier qui n'a plus de cartes gagne. Pénalités : joker 20, as 11, figures 10, autres leur valeur ;",
                    "au moins 100 sans avoir ouvert ; doublées si le gagnant fait rami (tout posé d'un coup sans avoir ouvert).",
                ],
            ),
        ];
        let mut y = 150.0;
        for (titre, lignes) in blocs {
            self.text(titre, 150., y, &f.moyen, C_ENCRE, Align::Left);
            for (i, l) in lignes.iter().enumerate() {
                self.text(l, 170., y + 36. + 24. * i as f32, &f.petit, C_ENCRE, Align::Left);
            }
            y += 110.0;
        }
        let aide = [
            "Clic sur les cartes de ta main pour les choisir ; POSER (ou Entrée) pour en faire une combinaison ;",
            "clic sur une combinaison de la table pour la compléter ; sur un joker, avec sa carte, pour le reprendre ;",
            "clic sur la défausse avec une carte choisie (ou DÉFAUSSER) pour finir ton tour.",
        ];
        for (i, l) in aide.iter().enumerate() {
            self.text(l, 550., y + 4. + 24. * i as f32, &f.petit, C_ENCRE_PALE, Align::Center);
        }
        self.text("F, Échap ou clic : fermer", 550., y + 86., &f.petit, C_ENCRE_PALE, Align::Center);
    }

    /// Touche G : les 52 cartes, les jokers et le dos.
    fn gallery(&self) {
        for i in 0..55usize {
            let (col, row) = (i % 14, i / 14);
            let xf = Xf::place(70.0 + 74.0 * col as f64, 100.0 + 140.0 * row as f64, 0.62, 0.0, 1.0);
            let c = match i {
                0..=51 => carte(i as u8),
                52 | 53 => carte(PREMIER_JOKER + i as u8 - 52),
                _ => None,
            };
            self.deck.poser(self.g, &xf, c, false);
        }
    }
}

/// Défausse (les trois cartes du dessus) et combinaisons de la table, dans l'ordre où elles se recouvrent.
fn ordre_table(t: &Table, vue: &Vue) -> Vec<u8> {
    let d = &t.m.d;
    let mut ordre: Vec<u8> = Vec::with_capacity(vue.cible.len());
    if vue.vols.is_empty() {
        // pendant la distribution, la défausse attend sur le talon
        let nd = d.defausse.len();
        ordre.extend(d.defausse[nd.saturating_sub(3)..].iter().copied());
    }
    for cb in &d.table {
        ordre.extend(cb.cartes.iter().copied());
    }
    ordre
}

/// Hauteur du panneau des scores (deux colonnes, une ligne pour la manche en cours).
fn hauteur_scores(np: usize) -> f32 {
    26.0 + 24.0 * ((np + 1) / 2) as f32 + 22.0
}

/// Ligne des réglages en bas de l'écran.
pub fn optline(t: &Table) -> String {
    format!(
        "J: {} joueurs  L: niveau {}  P: limite {}  H: aide {}  M: animations {}  F: aide-mémoire  F11: plein écran  N: en ligne  Echap: quitter",
        t.np(),
        LEVEL_NAMES[t.botlvl as usize],
        t.m.limite,
        if t.help { "oui" } else { "non" },
        anim::libelle()
    )
}

/// Galerie des cartes (touche G), sur le décor.
pub fn gallery(g: &Gfx, f: &Polices, deck: &Paquet) {
    Scene { g, f, deck, vue: &Vue::default() }.gallery();
}

/// Couche fixe tant que l'état ne change pas (panneaux, étiquettes, mains adverses, talon) : mise en cache.
pub fn draw_under(t: &Table, g: &Gfx, f: &Polices, deck: &Paquet, vue: &Vue) {
    let s = Scene { g, f, deck, vue };
    s.panels_fond(t);
    s.labels(t);
    s.opp_hands(t);
    s.talon(t);
    s.table_fixe(t);
}

/// Empreinte de la couche fixe.
pub fn under_signature(t: &Table, vue: &Vue) -> u64 {
    use std::hash::{Hash, Hasher};
    let d = &t.m.d;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (d.phase >= PH_FINMANCHE, d.tour, vue.stable, t.m.donneur, t.m.np, t.me, t.m.niveaux, d.talon.len().div_ceil(14).min(6), d.talon.is_empty()).hash(&mut h);
    t.numero.hash(&mut h);
    // cartes de la table arrivées à leur place
    for c in ordre_table(t, vue) {
        if !vue.en_mouvement(c) {
            if let Some(&(x, y, _)) = vue.cible.get(&c) {
                (c, x.round() as i32, y.round() as i32).hash(&mut h);
            }
        }
    }
    h.finish()
}

/// Tout ce qui bouge ou change souvent, par-dessus la couche fixe.
pub fn draw_over(t: &Table, g: &Gfx, f: &Polices, deck: &Paquet, ov: &Overlay, vue: &Vue) {
    let s = Scene { g, f, deck, vue };
    s.cartes(t);
    s.survol_table(t, ov.mouse);
    s.envols(t);
    s.distribution(t);
    s.panels(t);
    s.bubbles(t);
    s.overlay(t);
    s.buttons(t, ov.mouse);
    let alerte = t.alerte.is_some();
    s.text_ombre(&t.message(), 545., 755., &f.etiquette, if alerte { rgb(255, 178, 150) } else { C_TXT }, Align::Center);
    s.text_ombre(&ov.optline, 550., 780., &f.petit, C_TXTDIM, Align::Center);
    if ov.memo {
        s.memo();
    }
    if let Some((title, lines, menu)) = ov.net {
        ui::panneau_reseau(g, f, title, lines, menu);
    }
}
