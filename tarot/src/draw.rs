//! Dessin du tarot avec le moteur commun : chien, étiquettes, mains, plis animés, panneaux des scores et du
//! contrat, bulles, bilan de la donne, classement, boutons (contrats, appel, écart, poignée), aide-mémoire,
//! galerie des 78 cartes.

use crate::engine::*;
use crate::table::{main_touchee, main_x, Table, CHIEN_MONTRE, LEVEL_NAMES};
use crate::vue::{place_chien, Vue, BULLE_FONDU, BULLE_POP, VOL_CHIEN};
use bistrot::anim::{self, rebond, sortie, Distribution, Vol};
use bistrot::cartes::{self, Carte, Couleur, Paquet};
use bistrot::gfx::{Align, Color, Face, Gfx, Xf};
use bistrot::polices::Polices;
use bistrot::table::{self as tapis, Place, SOL};
use bistrot::ui::{self, *};

/// Action d'un bouton.
#[derive(Clone, Copy, PartialEq)]
pub enum BtnId {
    Contrat(u8),
    Appel(u8),
    Ecart,
    Poignee,
    Next,
}

pub type Btn = Bouton<BtnId>;

/// Boutons de la phase en cours.
pub fn buttons(t: &Table) -> Vec<Btn> {
    let b = |x: f32, y: f32, w: f32, h: f32, id: BtnId, actif: bool| Btn { x, y, w, h, id, actif };
    let a_moi = t.my_turn();
    match t.phase() {
        PH_ENCHERES if a_moi => {
            // largeurs selon le texte : PASSE, PETITE, GARDE, GARDE SANS, GARDE CONTRE
            const L: [f32; 5] = [112., 122., 118., 168., 188.];
            let mut x = 214.;
            (0..5u8)
                .map(|c| {
                    let bt = b(x, 560., L[c as usize], 46., BtnId::Contrat(c), t.m.enchere_permise(c));
                    x += L[c as usize] + 12.;
                    bt
                })
                .collect()
        }
        PH_APPEL if a_moi => liste(t.m.appels_possibles())
            .into_iter()
            .enumerate()
            .map(|(i, c)| b(204. + 198. * i as f32, 560., 190., 46., BtnId::Appel(c), true))
            .collect(),
        PH_ECART if a_moi => vec![b(440., 520., 220., 46., BtnId::Ecart, t.m.ecart_valide(t.selection))],
        PH_JEU if a_moi && t.m.poignee_possible(t.me) > 0 => vec![b(850., 548., 210., 44., BtnId::Poignee, true)],
        PH_FINDONNE | PH_FINPARTIE => vec![b(425., 566., 250., 52., BtnId::Next, true)],
        _ => vec![],
    }
}

/// Texte d'un bouton.
fn label(t: &Table, id: BtnId) -> String {
    match id {
        BtnId::Contrat(c) => CONTRATS[c as usize].to_uppercase(),
        BtnId::Appel(c) => {
            let r = ["ROI", "DAME", "CAVALIER", "VALET"][(13 - c % 14) as usize];
            format!("{r} DE {}", COULEURS[c as usize / 14].to_uppercase())
        }
        BtnId::Ecart => "VALIDER L'ÉCART".into(),
        BtnId::Poignee if t.poignee => "POIGNÉE : OUI".into(),
        BtnId::Poignee => "MONTRER LA POIGNÉE".into(),
        BtnId::Next if t.phase() == PH_FINPARTIE => "NOUVELLE PARTIE".into(),
        BtnId::Next => "SUITE".into(),
    }
}

/// Carte de la main du bas sous le point, ou None.
pub fn hand_hit(t: &Table, vue: &Vue, x: f32, y: f32) -> Option<u8> {
    let main = vue.main_visible(t);
    main_touchee(main.len(), x, y).map(|i| main[i])
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

fn carte(c: u8) -> Option<Carte> {
    Some(Carte::tarot(c as usize))
}

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

    /// Nombre de cartes affichées dans la main d'un adversaire (sans le chien encore au centre).
    fn cartes_adverses(&self, t: &Table, p: usize) -> usize {
        let mut n = nb(t.m.d.mains[p]) as usize;
        if t.m.d.preneur == p as i8 && Vue::chien_au_centre(t) {
            n -= t.m.taille_chien();
        }
        n
    }

    // ---------- couche fixe ----------
    fn labels(&self, t: &Table) {
        let ph = t.phase();
        for p in 0..t.np() {
            let tour = matches!(ph, PH_ENCHERES | PH_APPEL | PH_ECART | PH_JEU) && p == t.turn();
            ui::etiquette(self.g, self.f, &self.place(t, p), &t.player_name(p), t.ally_of_me(p), tour, p == t.m.donneur);
        }
    }

    fn opp_hands(&self, t: &Table) {
        for p in 0..t.np() {
            if p == t.me {
                continue;
            }
            let pl = self.place(t, p);
            let n = self.cartes_adverses(t, p);
            for i in 0..self.vue.stable[p].min(n) {
                self.deck.poser(self.g, &tapis::dos_adversaire(&pl, i, n), None, false);
            }
        }
    }

    /// Parchemins des panneaux (couche fixe : seuls les textes changent).
    fn panels_fond(&self, t: &Table) {
        ui::parchemin(self.g, 14., 12., 310., hauteur_scores(t.np()), 8.);
        ui::parchemin(self.g, 824., 12., 262., 96., 8.);
    }

    // ---------- ce qui bouge ----------
    /// Le chien : au centre pendant les enchères, puis montré (petite, garde) et envoyé au preneur.
    fn chien(&self, t: &Table) {
        let d = &t.m.d;
        let k = t.m.taille_chien();
        let chien = liste(d.chien);
        let ph = d.phase;
        if ph == PH_ENCHERES || ph == PH_APPEL || (ph == PH_ECART && t.now < t.chien_vu) {
            // pendant la distribution, le chien est dessiné avec les cartes qui volent
            if self.vue.chien_pose {
                for i in 0..k {
                    let (x, y) = place_chien(i, k);
                    self.deck.poser(self.g, &Xf::place(x, y, 0.62, 0.0, SOL), None, false);
                }
            }
            return;
        }
        if !Vue::chien_au_centre(t) {
            return;
        }
        let ms = t.now - t.chien_vu;
        let preneur = d.preneur as usize;
        for (i, &c) in chien.iter().enumerate() {
            let (x0, y0) = place_chien(i, k);
            let (x1, y1) = (550.0 + (i as f64 - (k as f64 - 1.0) / 2.0) * 92.0, 360.0);
            // retournement, un peu décalé d'une carte à l'autre
            let f = ((ms - 70.0 * i as f64) / 320.0).clamp(0.0, 1.0);
            let e = sortie(f);
            let (mut x, mut y, mut s) = (x0 + (x1 - x0) * e, y0 + (y1 - y0) * e, 0.62 + 0.24 * e);
            let mut sq = SOL + (1.0 - SOL) * e;
            let sx = (std::f64::consts::PI * f).cos().abs().max(0.02);
            let mut face = f > 0.5;
            // puis vol vers le preneur
            let v = ((ms - CHIEN_MONTRE) / VOL_CHIEN).clamp(0.0, 1.0);
            if v > 0.0 {
                let w = sortie(v);
                let (x2, y2, s2) = if preneur == t.me {
                    let main = trier(d.mains[t.me]);
                    let j = main.iter().position(|&m| m == c).unwrap_or(0);
                    (main_x(j, main.len()), 686.0, 1.04)
                } else {
                    let pl = self.place(t, preneur);
                    (pl.main.0, pl.main.1, 0.62)
                };
                x += (x2 - x) * w;
                y += (y2 - y) * w;
                s += (s2 - s) * w;
                sq = 1.0;
                face = preneur == t.me || v < 0.5;
            }
            let xf = Xf::place(x, y, s, 0.0, sq).puis(&Xf { a: if v > 0.0 { 1.0 } else { sx as f32 }, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 });
            self.deck.poser(self.g, &xf, if face { carte(c) } else { None }, false);
        }
    }

    /// Cartes en cours de distribution, et le paquet du donneur tant qu'il en reste.
    fn distribution(&self, t: &Table) {
        let restantes = self.vue.vols.iter().filter(|v| t.now < v.t0).count();
        if let Some(v) = self.vue.vols.iter().find(|v| t.now < v.t0) {
            for k in 0..restantes.min(5) {
                let xf = Xf::place(v.src.0 - k as f64 * 0.8, v.src.1 - k as f64 * 1.6, 0.62, 0.3, SOL);
                self.deck.poser(self.g, &xf, None, false);
            }
        }
        let k = t.m.taille_chien();
        let n_moi = nb(t.m.d.mains[t.me]) as usize;
        for v in &self.vue.vols {
            let ms = t.now - v.t0;
            if ms < 0.0 {
                continue;
            }
            let d = if v.p == 5 {
                Distribution { src: v.src, dst: place_chien(v.index, k), echelle: 0.62, rotation: 0.0, retourner: false }
            } else if v.p == t.me {
                Distribution { src: v.src, dst: (main_x(v.index, n_moi), 686.0), echelle: 1.04, rotation: 0.0, retourner: true }
            } else {
                let pl = self.place(t, v.p);
                let n = nb(t.m.d.mains[v.p]) as usize;
                Distribution { src: v.src, dst: tapis::pos_dos(&pl, v.index, n), echelle: 0.62, rotation: pl.rotation, retourner: false }
            };
            let (xf, ombre, face) = d.a(ms);
            if let Some((x, y, rx, ry)) = ombre {
                self.g.ellipse(&Xf::ID, x, y, rx, ry, Some(C_SHFLY), Some(C_SHFLY));
            }
            self.deck.poser(self.g, &xf, if face { carte(v.carte) } else { None }, false);
        }
    }

    /// Cartes du pli : envoi en arc depuis la main, puis ramassage vers le gagnant.
    fn tricks(&self, t: &Table) {
        let d = &t.m.d;
        let np = t.np();
        let ramassage = (t.collect > 0.0).then(|| (self.place(t, d.gagnant as usize).main, t.now - t.collect));
        for j in 0..d.n_pli as usize {
            let p = (d.entame as usize + j) % np;
            let c = d.pli[j];
            let pl = self.place(t, p);
            let pli = t.places()[t.disp_pos(p)].pli;
            let vol = Vol { depart: (t.asx[p], t.asy[p]), echelle: if p == t.me { 1.04 } else { 0.62 }, arrivee: pli, angle: pl.angle };
            let (xf, ombre) = vol.a(t.now - t.ast0[p], ramassage);
            if let Some((x, y, rx, ry)) = ombre {
                self.g.ellipse(&Xf::ID, x, y, rx, ry, Some(C_SHFLY), Some(C_SHFLY));
            }
            self.deck.poser(self.g, &xf, carte(c), false);
        }
    }

    fn my_hand(&self, t: &Table) {
        let main = self.vue.main_visible(t);
        let n = main.len();
        let legal = t.my_legal();
        let ecart = t.phase() == PH_ECART && t.my_turn();
        let possible = if ecart { t.m.ecart_possible() } else { TOUTES };
        for (i, &c) in main.iter().enumerate() {
            if self.vue.vols.iter().any(|v| v.p == t.me && v.carte == c) {
                continue; // en cours de distribution
            }
            let x = self.vue.x.get(&c).copied().unwrap_or_else(|| main_x(i, n));
            let y = 686.0 - self.vue.levee.get(&c).copied().unwrap_or(0.0);
            let xf = Xf::place(x, y, 1.04, 0.0, 1.0);
            if t.hint == c as i32 || t.selection & bit(c) != 0 {
                self.g.rrect(&xf, -56., -76., 112., 152., 16., Some(C_GOLD), Some(C_GOLD));
            }
            let grise = legal & bit(c) == 0 || (ecart && possible & bit(c) == 0);
            self.deck.poser(self.g, &xf, carte(c), grise);
        }
    }

    fn panels(&self, t: &Table) {
        let f = self.f;
        let np = t.np();
        for k in 0..np {
            let p = (t.me + k) % np;
            let (x, y) = (28.0 + (k % 2) as f32 * 150.0, 18.0 + (k / 2) as f32 * 26.0);
            let encre = if t.ally_of_me(p) { C_NOUS } else { C_EUX };
            self.text(&t.player_name(p), x, y, &f.etiquette, encre, Align::Left);
            let s = (self.vue.score[p].round() as i64).to_string();
            self.text(&s, x + 136.0, y, &f.etiquette, C_ENCRE, Align::Right);
        }
        let m = &t.m;
        let rangs = (np + 1) / 2;
        let ligne = format!("Donne {} / {}", (m.donne + 1).min(m.donnes), m.donnes);
        self.text(&ligne, 28., 22. + 26. * rangs as f32, &f.petit, C_ENCRE_PALE, Align::Left);
        // contrat
        self.text("Contrat", 842., 18., &f.etiquette, C_RUBRIQUE, Align::Left);
        let d = &m.d;
        if d.preneur >= 0 {
            self.text(CONTRATS[d.contrat as usize], 842., 44., &f.moyen, C_ENCRE, Align::Left);
            let mut qui = format!("par {}", t.player_name(d.preneur as usize));
            if d.revele && d.partenaire >= 0 {
                qui = format!("{} et {}", t.player_name(d.preneur as usize), t.player_name(d.partenaire as usize));
            }
            self.text(&qui, 842., 80., &f.petit, C_ENCRE_PALE, Align::Left);
            if d.appel >= 0 {
                let c = d.appel as u8;
                let coul = Couleur::depuis(c as usize / 14);
                let encre = if coul.rouge() { C_ROUGE } else { C_ENCRE };
                cartes::symbole(self.g, &Xf::ID, coul, 1056., 54., 16., encre);
                let r = ["R", "D", "C", "V"][(13 - c % 14) as usize];
                self.text(r, 1056., 74., &f.etiquette, encre, Align::Center);
            }
        } else if d.phase == PH_ENCHERES {
            let max = m.enchere_max();
            let txt = if max == 0 { "enchères en cours".to_string() } else { CONTRATS[max as usize].to_lowercase() + " demandée" };
            self.text(&txt, 842., 52., &f.petit, C_ENCRE_PALE, Align::Left);
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
            let pos = if t.disp_pos(p) == 0 { (550.0, 520.0) } else { self.place(t, p).bulle };
            ui::bulle_animee(self.g, self.f, pos, txt, taille as f32, opacite as f32);
        }
    }

    /// Bilan de la donne, ou classement de fin de partie.
    fn overlay(&self, t: &Table) {
        let ph = t.phase();
        if ph != PH_FINDONNE && ph != PH_FINPARTIE {
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
        } else if b.fait {
            "Contrat réussi"
        } else {
            "Contrat chuté"
        };
        self.text(titre, 550., 128., &f.titre, C_RUBRIQUE, Align::Center);
        let nom = |p: i8| t.player_name(p as usize);
        let qui = if np == 5 {
            if b.partenaire >= 0 {
                format!("{} de {}, avec {}", CONTRATS[b.contrat as usize], nom(b.preneur), nom(b.partenaire))
            } else {
                format!("{} de {}, seul", CONTRATS[b.contrat as usize], nom(b.preneur))
            }
        } else {
            format!("{} de {}", CONTRATS[b.contrat as usize], nom(b.preneur))
        };
        self.text(&qui, 550., 188., &f.etiquette, C_ENCRE, Align::Center);
        let pts = b.demi as f32 / 2.0;
        let pts = if b.demi % 2 == 0 { format!("{}", pts as i32) } else { format!("{pts:.1}").replace('.', ",") };
        let l1 = format!(
            "{} bout{} : il fallait {} points ; faits : {} ({} de {})",
            b.bouts,
            if b.bouts > 1 { "s" } else { "" },
            b.cible,
            pts,
            if b.fait { "réussi" } else { "chuté" },
            b.ecart
        );
        self.text(&l1, 550., 218., &f.petit, C_ENCRE, Align::Center);
        // calcul de la part
        let mult = MULTIPLICATEURS[b.contrat as usize];
        let mut calc = if b.fait { format!("(25 + {}", b.ecart) } else { format!("(-25 - {}", b.ecart) };
        if b.petit_bout != 0 {
            calc += &format!(" {} 10 petit au bout", if b.petit_bout > 0 { "+" } else { "-" });
        }
        calc += &format!(") x {mult}");
        if b.poignee != 0 {
            calc += &format!(" {} {} poignée", if b.poignee > 0 { "+" } else { "-" }, b.poignee.abs());
        }
        if b.chelem != 0 {
            calc += &format!(" {} {} chelem", if b.chelem > 0 { "+" } else { "-" }, b.chelem.abs());
        }
        calc += &format!(" = {} par part", b.unite);
        self.text(&calc, 550., 242., &f.petit, C_ENCRE_PALE, Align::Center);
        // gains et totaux
        self.text("Donne", 690., 276., &f.etiquette, C_RUBRIQUE, Align::Right);
        self.text("Total", 820., 276., &f.etiquette, C_RUBRIQUE, Align::Right);
        let ordre: Vec<usize> = if ph == PH_FINPARTIE { t.m.classement() } else { (0..np).map(|k| (t.me + k) % np).collect() };
        for (r, &p) in ordre.iter().enumerate() {
            let y = 306.0 + 34.0 * r as f32;
            let encre = if t.ally_of_me(p) { C_NOUS } else { C_EUX };
            let mut n = t.player_name(p);
            if p as i8 == b.preneur {
                n += " (preneur)";
            } else if p as i8 == b.partenaire {
                n += " (partenaire)";
            }
            if ph == PH_FINPARTIE {
                n = format!("{}. {n}", r + 1);
            }
            self.text(&n, 280., y, &f.etiquette, encre, Align::Left);
            let g = b.gains[p];
            self.text(&format!("{}{g}", if g > 0 { "+" } else { "" }), 690., y, &f.etiquette, C_ENCRE, Align::Right);
            self.text(&t.m.scores[p].to_string(), 820., y, &f.etiquette, C_ENCRE, Align::Right);
        }
    }

    fn buttons(&self, t: &Table, mouse: (f32, f32)) {
        for b in buttons(t) {
            b.dessiner(self.g, self.f, &label(t, b.id), mouse);
        }
    }

    /// Touche F : valeur des cartes, contrats, points à faire, poignées.
    fn memo(&self, t: &Table) {
        let f = self.f;
        ui::panneau(self.g, 150., 96., 800., 600., 20.);
        self.text("Aide-mémoire", 550., 110., &f.titre, C_RUBRIQUE, Align::Center);
        let y0 = 182.0;
        self.text("Valeur des cartes", 190., y0, &f.moyen, C_ENCRE, Align::Left);
        let valeurs = [
            "Bouts (le Petit, le 21, l'Excuse) : 4,5",
            "Roi : 4,5      Dame : 3,5      Cavalier : 2,5      Valet : 1,5",
            "Autres cartes : 0,5        Total : 91 points",
        ];
        for (i, l) in valeurs.iter().enumerate() {
            self.text(l, 210., y0 + 40. + 26. * i as f32, &f.petit, C_ENCRE, Align::Left);
        }
        let y1 = y0 + 132.0;
        self.text("Points à faire par le preneur", 190., y1, &f.moyen, C_ENCRE, Align::Left);
        self.text("0 bout : 56      1 bout : 51      2 bouts : 41      3 bouts : 36", 210., y1 + 40., &f.petit, C_ENCRE, Align::Left);
        let y2 = y1 + 84.0;
        self.text("Contrats", 190., y2, &f.moyen, C_ENCRE, Align::Left);
        let contrats = [
            "Petite x1 et Garde x2 : le preneur prend le chien et fait son écart (ni roi ni bout)",
            "Garde sans x4 : le chien compte pour le preneur, sans être vu ; Garde contre x6 : pour la défense",
            "Part : (25 + écart) x contrat, + 10 x contrat pour le petit au bout, + primes",
        ];
        for (i, l) in contrats.iter().enumerate() {
            self.text(l, 210., y2 + 40. + 26. * i as f32, &f.petit, C_ENCRE, Align::Left);
        }
        let y3 = y2 + 132.0;
        let s = seuils_poignee(t.np());
        let pg = format!(
            "Poignées à {} joueurs : simple {} atouts (20), double {} (30), triple {} (40). Chelem : 200.",
            t.np(),
            s[0],
            s[1],
            s[2]
        );
        self.text(&pg, 550., y3, &f.petit, C_ENCRE, Align::Center);
        self.text("Fournir la couleur, sinon couper en montant ; l'Excuse se joue à tout moment.", 550., y3 + 26., &f.petit, C_ENCRE, Align::Center);
        self.text("F, Échap ou clic : fermer", 550., y3 + 60., &f.petit, C_ENCRE_PALE, Align::Center);
    }

    /// Touche G : les 78 cartes et le dos.
    fn gallery(&self) {
        for i in 0..79usize {
            let (col, row) = (i % 14, i / 14);
            let xf = Xf::place(70.0 + 74.0 * col as f64, 72.0 + 112.0 * row as f64, 0.6, 0.0, 1.0);
            self.deck.poser(self.g, &xf, if i < 78 { carte(i as u8) } else { None }, false);
        }
    }
}

/// Hauteur du panneau des scores (deux colonnes, une ligne pour la donne en cours).
fn hauteur_scores(np: usize) -> f32 {
    28.0 + 26.0 * ((np + 1) / 2) as f32 + 24.0
}

/// Ligne des réglages en bas de l'écran.
pub fn optline(t: &Table) -> String {
    format!(
        "J: {} joueurs  L: niveau {}  P: {} donnes  H: aide {}  M: animations {}  F: aide-mémoire  F11: plein écran  N: en ligne  Echap: quitter",
        t.np(),
        LEVEL_NAMES[t.botlvl as usize],
        t.m.donnes,
        if t.help { "oui" } else { "non" },
        anim::libelle()
    )
}

/// Galerie des cartes (touche G), sur le décor.
pub fn gallery(g: &Gfx, f: &Polices, deck: &Paquet) {
    Scene { g, f, deck, vue: &Vue::default() }.gallery();
}

/// Couche fixe tant que l'état ne change pas (panneaux, étiquettes, mains adverses) : mise en cache.
pub fn draw_under(t: &Table, g: &Gfx, f: &Polices, deck: &Paquet, vue: &Vue) {
    let s = Scene { g, f, deck, vue };
    s.panels_fond(t);
    s.labels(t);
    s.opp_hands(t);
}

/// Empreinte de la couche fixe.
pub fn under_signature(t: &Table, vue: &Vue) -> u64 {
    use std::hash::{Hash, Hasher};
    let d = &t.m.d;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mains: Vec<u32> = (0..t.np()).map(|p| nb(d.mains[p])).collect();
    (d.phase, d.tour, mains, vue.stable, t.m.donneur, t.m.np, t.me, t.m.niveaux, d.preneur, d.revele).hash(&mut h);
    (Vue::chien_au_centre(t), t.numero).hash(&mut h);
    h.finish()
}

/// Tout ce qui bouge ou change souvent, par-dessus la couche fixe.
pub fn draw_over(t: &Table, g: &Gfx, f: &Polices, deck: &Paquet, ov: &Overlay, vue: &Vue) {
    let s = Scene { g, f, deck, vue };
    s.chien(t);
    s.tricks(t);
    s.my_hand(t);
    s.distribution(t);
    s.panels(t);
    s.bubbles(t);
    s.overlay(t);
    s.buttons(t, ov.mouse);
    s.text_ombre(&t.message(), 550., 755., &f.etiquette, C_TXT, Align::Center);
    s.text_ombre(&ov.optline, 550., 780., &f.petit, C_TXTDIM, Align::Center);
    if ov.memo {
        s.memo(t);
    }
    if let Some((title, lines, menu)) = ov.net {
        ui::panneau_reseau(g, f, title, lines, menu);
    }
}
