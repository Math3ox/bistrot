//! Les 42 demi-scènes des atouts (en haut la ville, en bas la campagne, comme le Tarot Nouveau) et l'Excuse.
//! Repère d'une demi-scène : x de -43 à 43, y de -63 à -1 ; l'Excuse occupe toute la carte (y de -63 à 63).

use super::outils::*;
use crate::gfx::{alpha, rgb, Color, Gfx, Xf};
use std::f32::consts::PI;
use tiny_skia::PathBuilder;

type Scene = fn(&Gfx, &Xf);

/// Scènes (haut, bas) de l'atout n (1 à 21).
pub fn scenes(n: u8) -> (Scene, Scene) {
    match n {
        1 => (clown, fou_danseuse),
        2 => (enfants_parc, fete),
        3 => (jeunes_gens, jeunes_filles),
        4 => (bureau, meres),
        5 => (grand_pere, grand_mere),
        6 => (petit_dejeuner, fauche),
        7 => (salon, repos_champs),
        8 => (musique, seuil),
        9 => (retour_chasse, veilleur),
        10 => (mine, berger),
        11 => (barque_lac, pique_nique),
        12 => (soiree, danse_village),
        13 => (magasin, epicerie),
        14 => (chasse, peche),
        15 => (photographie, peinture),
        16 => (jardinier, tonte),
        17 => (courses, sechage),
        18 => (marche, battage),
        19 => (patinage, veillee),
        20 => (cartes_cafe, quilles),
        _ => (carnaval, parade),
    }
}

// ---------- petits objets ----------
fn canne(g: &Gfx, xf: &Xf, (x, y): (f32, f32), l: f32) {
    g.line(xf, x, y - 1., x + 1.5, y + l, 0.6, SEPIA);
}

fn ombrelle(g: &Gfx, xf: &Xf, (x, y): (f32, f32), c: Color) {
    g.line(xf, x, y, x + 1., y - 13., 0.5, SEPIA);
    let mut pb = PathBuilder::new();
    pb.move_to(x - 7., y - 11.);
    pb.quad_to(x + 1., y - 21., x + 9., y - 11.);
    pb.close();
    if let Some(p) = pb.finish() {
        g.forme(xf, &p, Some(c), Some(alpha(SEPIA, 0.5)), 0.3);
    }
}

fn fusil(g: &Gfx, xf: &Xf, (x0, y0): (f32, f32), (x1, y1): (f32, f32)) {
    g.line(xf, x0, y0, x1, y1, 0.8, rgb(50, 40, 36));
    g.line(xf, x0, y0, x0 + (x1 - x0) * 0.3, y0 + (y1 - y0) * 0.3, 1.5, BOIS);
}

fn livres(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32) {
    let cols = [ROUGE, BLEU, VERT, OCRE, VIOLET, SEPIA];
    let mut px = x;
    let mut k = 0;
    while px < x + w - 1.5 {
        let h = 5.0 + (k % 3) as f32;
        g.poly(xf, &rect(px, y - h, 1.6, h), Some(cols[k % cols.len()]), None);
        px += 1.8;
        k += 1;
    }
}

fn bibliotheque(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32, h: f32) {
    g.poly(xf, &rect(x, y - h, w, h), Some(assombrir(BOIS, 0.15)), None);
    let mut ry = y - h + 8.;
    while ry < y - 1. {
        livres(g, xf, x + 1., ry, w - 2.);
        g.line(xf, x, ry, x + w, ry, 0.8, BOIS_CLAIR);
        ry += 8.;
    }
}

fn tableau_mural(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32, h: f32) {
    g.poly(xf, &rect(x, y, w, h), Some(rgb(200, 160, 60)), None);
    degrade(g, xf, &rect(x + 1., y + 1., w - 2., h - 2.), y, y + h, rgb(150, 180, 200), rgb(110, 140, 90));
}

fn tasse(g: &Gfx, xf: &Xf, x: f32, y: f32) {
    g.poly(xf, &rect(x - 1.2, y - 2., 2.4, 2.), Some(BLANC), Some(GRIS));
    g.ellipse(xf, x, y, 2., 0.5, Some(BLANC), None);
}

fn cafetiere(g: &Gfx, xf: &Xf, x: f32, y: f32) {
    g.poly(xf, &[(x - 2., y), (x + 2., y), (x + 1.5, y - 5.), (x - 1.5, y - 5.)], Some(rgb(200, 200, 205)), Some(GRIS));
    g.line(xf, x + 2., y - 3.5, x + 3.6, y - 5.2, 0.6, GRIS);
    g.ellipse(xf, x, y - 5.4, 1.0, 0.6, Some(GRIS), None);
}

fn lustre(g: &Gfx, xf: &Xf, x: f32, y: f32) {
    g.line(xf, x, -64., x, y, 0.4, rgb(200, 170, 80));
    g.ellipse(xf, x, y + 1., 7., 1.6, Some(rgb(214, 180, 90)), None);
    for k in 0..5 {
        let bx = x - 6. + k as f32 * 3.;
        g.line(xf, bx, y, bx, y - 2., 0.5, BLANC);
        g.lueur(xf, (bx, y - 2.6), 3.5, &[(0.0, alpha(rgb(255, 230, 150), 0.9)), (1.0, alpha(rgb(255, 230, 150), 0.0))]);
    }
}

fn violon(g: &Gfx, xf: &Xf, x: f32, y: f32, a: f32) {
    let local = xf.puis(&Xf::place(x as f64, y as f64, 1.0, a as f64, 1.0));
    g.ellipse(&local, 0., 1.2, 1.8, 1.4, Some(rgb(150, 70, 30)), None);
    g.ellipse(&local, 0., -1.2, 1.5, 1.2, Some(rgb(150, 70, 30)), None);
    g.line(&local, 0., -2.2, 0., -6.5, 0.6, NOIR);
}

/// Mandoline : caisse en `corps`, manche tendu vers `manche`.
fn mandoline_entre(g: &Gfx, xf: &Xf, corps: (f32, f32), manche: (f32, f32), s: f32) {
    let (dx, dy) = (manche.0 - corps.0, manche.1 - corps.1);
    let a = dx.atan2(-dy);
    mandoline(g, xf, corps.0, corps.1, s, a);
}

fn mandoline(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32, a: f32) {
    let local = xf.puis(&Xf::place(x as f64, y as f64, s as f64, a as f64, 1.0));
    g.poly(&local, &rect(-0.8, -16., 1.6, 12.), Some(rgb(60, 36, 20)), None);
    g.poly(&local, &rect(-1.4, -18.5, 2.8, 3.), Some(rgb(80, 50, 26)), None);
    g.ellipse(&local, 0., 0., 5.2, 6.5, Some(rgb(180, 110, 50)), Some(rgb(100, 56, 24)));
    g.ellipse(&local, 0., -1.5, 1.6, 1.6, Some(rgb(40, 24, 14)), None);
    g.line(&local, 0., 4., 0., -17., 0.15, rgb(230, 220, 200));
}

fn faux(g: &Gfx, xf: &Xf, (x, y): (f32, f32), s: f32) {
    g.line(xf, x - 6. * s, y + 6. * s, x + 8. * s, y - 10. * s, 0.8, BOIS);
    let mut pb = PathBuilder::new();
    pb.move_to(x - 6. * s, y + 6. * s);
    pb.quad_to(x - 14. * s, y + 4. * s, x - 18. * s, y + 9. * s);
    if let Some(p) = pb.finish() {
        g.forme(xf, &p, None, Some(rgb(200, 204, 210)), 0.9);
    }
}

fn fourche(g: &Gfx, xf: &Xf, (x0, y0): (f32, f32), (x1, y1): (f32, f32)) {
    g.line(xf, x0, y0, x1, y1, 0.6, BOIS);
    for d in [-1.0, 0.0, 1.0] {
        g.line(xf, x1 + d, y1, x1 + d * 1.2, y1 - 3., 0.35, GRIS);
    }
}

fn fleurs(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32) {
    let cols = [ROUGE, rgb(250, 220, 90), ROSE, BLANC, VIOLET];
    let mut k = 0;
    let mut px = x;
    while px < x + w {
        let dy = ((k * 7) % 5) as f32 * 0.6;
        g.ellipse(xf, px, y - dy, 0.9, 0.9, Some(cols[k % cols.len()]), None);
        px += 1.7;
        k += 1;
    }
}

fn tonneau(g: &Gfx, xf: &Xf, x: f32, y: f32, s: f32) {
    g.ellipse(xf, x, y - 4. * s, 3. * s, 4. * s, Some(BOIS), Some(assombrir(BOIS, 0.4)));
    for dy in [-6.5, -1.5] {
        g.line(xf, x - 2.6 * s, y + dy * s, x + 2.6 * s, y + dy * s, 0.5, GRIS);
    }
}

fn sac(g: &Gfx, xf: &Xf, x: f32, y: f32, c: Color) {
    g.poly(xf, &[(x - 3., y), (x + 3., y), (x + 2.5, y - 6.), (x + 1., y - 7.), (x - 1., y - 7.), (x - 2.5, y - 6.)], Some(c), Some(assombrir(c, 0.3)));
}

fn cagette(g: &Gfx, xf: &Xf, x: f32, y: f32, c: Color) {
    g.poly(xf, &rect(x - 4., y - 3., 8., 3.), Some(BOIS_CLAIR), Some(BOIS));
    for k in 0..4 {
        rond(g, xf, x - 3. + k as f32 * 2., y - 3.4, 1.0, c);
    }
}

fn store(g: &Gfx, xf: &Xf, x: f32, y: f32, w: f32, c: Color) {
    let n = (w / 3.0) as usize;
    for k in 0..n {
        let col = if k % 2 == 0 { c } else { BLANC };
        g.poly(xf, &[(x + k as f32 * 3., y), (x + k as f32 * 3. + 3., y), (x + k as f32 * 3. + 3.5, y + 4.), (x + k as f32 * 3. + 0.5, y + 4.)], Some(col), None);
    }
}

fn foule(g: &Gfx, xf: &Xf, y: f32, x0: f32, x1: f32) {
    let cols = [rgb(60, 50, 60), rgb(110, 70, 60), rgb(70, 80, 110), rgb(90, 90, 70)];
    let mut x = x0;
    let mut k = 0;
    while x < x1 {
        g.ellipse(xf, x, y - 3., 1.8, 3., Some(cols[k % 4]), None);
        g.ellipse(xf, x, y - 6.8, 1.2, 1.3, Some(PEAU), None);
        x += 3.2;
        k += 1;
    }
}

// ---------- 1 : l'individu ----------
fn clown(g: &Gfx, xf: &Xf) {
    for k in 0..12 {
        let c = if k % 2 == 0 { rgb(176, 40, 46) } else { rgb(236, 218, 186) };
        g.poly(xf, &[(-44. + k as f32 * 8., -64.), (-36. + k as f32 * 8., -64.), (-38. + k as f32 * 8., -20.), (-46. + k as f32 * 8., -20.)], Some(c), None);
    }
    g.lueur(xf, (0., -24.), 30., &[(0.0, alpha(rgb(255, 240, 200), 0.6)), (1.0, alpha(rgb(255, 240, 200), 0.0))]);
    sol(g, xf, -20., rgb(214, 170, 100), rgb(170, 120, 60));
    g.ellipse(xf, 0., -10., 30., 6., None, Some(rgb(150, 40, 40)));
    // tambour
    g.poly(xf, &rect(-6., -14., 12., 7.), Some(ROUGE), Some(rgb(230, 200, 100)));
    g.ellipse(xf, 0., -14., 6., 1.5, Some(CREME), Some(rgb(230, 200, 100)));
    let p = Perso { haut: BLANC, bas: BLANC, peau: rgb(250, 246, 240), cheveux: rgb(200, 80, 40), chapeau: Chapeau::Pointu(rgb(230, 230, 230)), ..Perso::homme(0., -7., 28., BLANC, BLANC, Chapeau::Aucun) }.assis().bras(0.35, 0.35);
    p.dessiner(g, xf);
    for (x, y) in [(0., -22.), (0., -18.5), (0., -27.)] {
        rond(g, xf, x, y, 1.0, ROUGE);
    }
    g.poly(xf, &[(-5., -30.5), (5., -30.5), (3., -29.), (-3., -29.)], Some(rgb(240, 240, 240)), Some(GRIS));
    rond(g, xf, 0., -32.2, 0.7, ROUGE);
    g.line(xf, 1., -33.4, 1.3, -31.6, 0.3, rgb(120, 160, 220));
}

fn fou_danseuse(g: &Gfx, xf: &Xf) {
    g.poly(xf, &rect(-44., -64., 88., 64.), Some(rgb(60, 30, 36)), None);
    for (x0, x1) in [(-44., -30.), (30., 44.)] {
        degrade(g, xf, &[(x0, -64.), (x1, -64.), (x1 * 0.9 + x0 * 0.1, -14.), (x0, -14.)], -64., -14., rgb(170, 30, 40), rgb(110, 16, 24));
    }
    g.poly(xf, &[(-44., -64.), (44., -64.), (44., -57.), (-44., -57.)], Some(rgb(150, 24, 34)), None);
    sol(g, xf, -14., rgb(150, 100, 60), rgb(100, 64, 34));
    for k in 0..6 {
        let x = -35. + k as f32 * 14.;
        g.lueur(xf, (x, -3.), 9., &[(0.0, alpha(rgb(255, 230, 150), 0.7)), (1.0, alpha(rgb(255, 230, 150), 0.0))]);
    }
    // danseuse : tutu rose, bras levés
    let d = Perso::femme(10., -8., 27., rgb(240, 190, 200), rgb(250, 210, 220), Chapeau::Aucun).bras(2.6, 2.4).cheveux(rgb(70, 40, 30));
    d.dessiner(g, xf);
    g.ellipse(xf, 10., -19., 9., 2.2, Some(alpha(rgb(255, 220, 230), 0.95)), Some(rgb(230, 170, 190)));
    // le fou : habit mi-parti, bonnet à grelots
    let f = Perso { haut: rgb(200, 60, 40), bas: rgb(230, 190, 60), ..Perso::homme(-12., -8., 28., rgb(200, 60, 40), rgb(230, 190, 60), Chapeau::Pointu(rgb(60, 110, 70))) }.bras(1.6, 0.6);
    f.dessiner(g, xf);
    let (mx, my) = f.main(-1.);
    g.line(xf, mx, my, mx - 2., my - 7., 0.5, SEPIA);
    rond(g, xf, mx - 2., my - 8., 1.4, rgb(230, 190, 60));
}

// ---------- 2 : l'enfance ----------
fn enfants_parc(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(150, 190, 226), rgb(230, 236, 220));
    nuage(g, xf, 18., -54., 1.0);
    for (x, h) in [(-34., 36.), (-20., 30.), (30., 38.)] {
        arbre(g, xf, x, -24., h, FEUILLAGE);
    }
    sol(g, xf, -24., HERBE, HERBE_FONCEE);
    g.poly(xf, &[(-6., -24.), (6., -24.), (20., -1.), (-14., -1.)], Some(rgb(222, 206, 166)), None);
    // banc et nourrice
    g.poly(xf, &rect(18., -18., 16., 1.5), Some(BOIS), None);
    g.line(xf, 20., -16.5, 20., -12., 0.8, SEPIA);
    g.line(xf, 32., -16.5, 32., -12., 0.8, SEPIA);
    Perso::femme(26., -12., 24., rgb(80, 80, 110), rgb(60, 60, 90), Chapeau::Coiffe).assis().dessiner(g, xf);
    // fillette au cerceau
    let f = Perso::femme(-8., -6., 18., rgb(230, 230, 240), rgb(200, 90, 110), Chapeau::Capeline(rgb(240, 230, 200))).bras(0.4, 0.9);
    f.dessiner(g, xf);
    let (mx, my) = f.main(1.);
    g.line(xf, mx, my, mx + 5., my + 3., 0.4, SEPIA);
    g.ellipse(xf, mx + 7., my + 3., 4., 4., None, Some(rgb(160, 110, 60)));
    // garçon au ballon
    Perso::homme(-26., -4., 17., rgb(40, 60, 110), rgb(230, 230, 230), Chapeau::Casquette).bras(0.3, 1.2).dessiner(g, xf);
    rond(g, xf, -19., -16., 2.0, ROUGE);
}

fn fete(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(140, 186, 228), rgb(244, 230, 200));
    guirlande(g, xf, -44., -58., 44., -56., 12);
    collines(g, xf, &[(-44., -24.), (0., -27.), (44., -23.)], HERBE);
    // manège
    g.poly(xf, &[(-30., -40.), (10., -40.), (-10., -52.)], Some(ROUGE), None);
    for k in 0..4 {
        g.poly(xf, &[(-30. + k as f32 * 10., -40.), (-25. + k as f32 * 10., -40.), (-10., -52.)], Some(rgb(250, 230, 150)), None);
    }
    g.poly(xf, &rect(-31., -41., 42., 2.5), Some(rgb(220, 180, 70)), None);
    for k in 0..5 {
        let x = -28. + k as f32 * 9.;
        g.line(xf, x, -39., x, -20., 0.6, rgb(220, 180, 70));
        cheval(g, xf, x - 1., -24. - (k % 2) as f32 * 3., 0.45, if k % 2 == 0 { BLANC } else { rgb(190, 150, 100) }, false);
    }
    sol(g, xf, -18., rgb(214, 196, 150), rgb(180, 160, 110));
    Perso::homme(20., -5., 18., rgb(150, 40, 40), rgb(70, 70, 90), Chapeau::Casquette).bras(0.3, 2.6).dessiner(g, xf);
    g.line(xf, 25.5, -16., 27., -30., 0.2, SEPIA);
    rond(g, xf, 27., -32., 2.4, rgb(90, 140, 210));
    Perso::homme(32., -4., 17., rgb(60, 100, 70), rgb(200, 190, 170), Chapeau::Paille).bras(0.6, 0.2).dessiner(g, xf);
}

// ---------- 3 : la jeunesse ----------
fn jeunes_gens(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(160, 196, 230), rgb(236, 236, 214));
    nuage(g, xf, -20., -52., 1.1);
    arbre(g, xf, -32., -22., 40., FEUILLAGE);
    arbre(g, xf, 34., -22., 34., rgb(70, 110, 56));
    sol(g, xf, -22., HERBE, HERBE_FONCEE);
    g.poly(xf, &[(-44., -14.), (44., -17.), (44., -8.), (-44., -5.)], Some(rgb(226, 212, 176)), None);
    fleurs(g, xf, -40., -20., 20.);
    fleurs(g, xf, 22., -21., 20.);
    let dame = Perso::femme(-6., -8., 29., rgb(220, 200, 230), rgb(150, 110, 170), Chapeau::Capeline(rgb(240, 236, 220))).bras(0.2, 0.5);
    dame.dessiner(g, xf);
    ombrelle(g, xf, dame.main(1.), rgb(250, 230, 236));
    let m = Perso::homme(8., -9., 31., rgb(50, 50, 64), rgb(140, 130, 120), Chapeau::HautDeForme).bras(0.5, 0.25);
    m.dessiner(g, xf);
    canne(g, xf, m.main(1.), 9.);
}

fn jeunes_filles(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(170, 200, 230), rgb(240, 234, 214));
    maison(g, xf, -44., -18., 24., 20., rgb(226, 206, 170), rgb(150, 70, 50));
    maison(g, xf, -16., -18., 20., 24., rgb(214, 190, 160), rgb(110, 80, 70));
    maison(g, xf, 10., -18., 30., 18., rgb(236, 220, 186), rgb(160, 80, 54));
    sol(g, xf, -18., rgb(190, 180, 160), rgb(150, 140, 120));
    for (x, robe, chap) in [(-18., rgb(80, 120, 170), rgb(250, 240, 220)), (0., rgb(190, 90, 100), rgb(240, 220, 160)), (18., rgb(110, 150, 100), rgb(250, 250, 240))] {
        Perso::femme(x, -5., 27., eclaircir(robe, 0.35), robe, Chapeau::Capeline(chap)).bras(0.5, 0.5).dessiner(g, xf);
    }
}

// ---------- 4 : l'âge mûr ----------
fn bureau(g: &Gfx, xf: &Xf) {
    interieur(g, xf, rgb(170, 150, 120));
    bibliotheque(g, xf, -42., -12., 22., 44.);
    fenetre(g, xf, 18., -54., 18., 24., rgb(190, 210, 220));
    rideaux(g, xf, 18., -54., 18., 26., rgb(130, 40, 40));
    Perso::homme(6., -6., 30., rgb(40, 40, 50), rgb(50, 50, 60), Chapeau::Aucun).assis().bras(-0.25, 0.1).cheveux(rgb(90, 80, 70)).dessiner(g, xf);
    g.poly(xf, &rect(-10., -20., 32., 3.), Some(assombrir(BOIS, 0.2)), None);
    g.poly(xf, &rect(-8., -17., 28., 11.), Some(BOIS), None);
    g.poly(xf, &[(0., -21.), (11., -21.), (12., -22.5), (1., -22.5)], Some(BLANC), Some(GRIS));
    g.poly(xf, &rect(-6., -23.5, 3., 3.), Some(NOIR), None);
    g.line(xf, -4.5, -23.5, -2., -28., 0.3, BLANC);
}

fn meres(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(170, 206, 230), rgb(236, 236, 216));
    maison(g, xf, 6., -20., 34., 22., rgb(230, 216, 184), rgb(150, 70, 50));
    arbre(g, xf, -34., -20., 34., FEUILLAGE);
    sol(g, xf, -20., HERBE, HERBE_FONCEE);
    let a = Perso::femme(-14., -6., 28., rgb(110, 70, 90), rgb(70, 60, 90), Chapeau::Coiffe).bras(0.9, 0.9).tablier(BLANC);
    a.dessiner(g, xf);
    g.ellipse(xf, -14., -19., 4., 2.6, Some(BLANC), Some(GRIS));
    rond(g, xf, -16., -20., 1.4, PEAU);
    Perso::femme(8., -5., 28., rgb(90, 120, 160), rgb(60, 80, 110), Chapeau::Coiffe).bras(0.2, 0.7).tablier(rgb(230, 220, 200)).dessiner(g, xf);
    Perso::homme(20., -4., 16., rgb(160, 60, 50), rgb(90, 80, 70), Chapeau::Aucun).bras(-0.6, 0.3).cheveux(rgb(200, 160, 90)).dessiner(g, xf);
    for (x, y) in [(28., -6.), (33., -8.)] {
        g.ellipse(xf, x, y - 2., 2., 1.6, Some(BLANC), None);
        rond(g, xf, x + 1.8, y - 3.5, 0.9, BLANC);
        g.ellipse(xf, x + 2.5, y - 3.5, 0.6, 0.3, Some(OCRE), None);
    }
}

// ---------- 5 : la vieillesse ----------
fn grand_pere(g: &Gfx, xf: &Xf) {
    interieur(g, xf, rgb(176, 140, 110));
    cheminee(g, xf, -24., -10., 1.1);
    tableau_mural(g, xf, 14., -54., 18., 13.);
    fauteuil(g, xf, 12., -6., 1.4, rgb(130, 50, 50));
    let p = Perso::homme(12., -6., 29., rgb(80, 70, 60), rgb(70, 60, 54), Chapeau::Aucun).assis().bras(0.9, 0.5).cheveux(rgb(230, 230, 230));
    p.dessiner(g, xf);
    let (mx, my) = p.main(-1.);
    g.line(xf, mx, my, mx + 4., my - 4., 0.6, BOIS);
    g.poly(xf, &rect(mx + 3.5, my - 6., 2., 2.5), Some(BOIS), None);
    g.lueur(xf, (mx + 4.5, my - 8.), 3., &[(0.0, alpha(BLANC, 0.6)), (1.0, alpha(BLANC, 0.0))]);
    chien(g, xf, 26., -4., 0.9, rgb(160, 110, 60));
}

fn grand_mere(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(190, 210, 226), rgb(240, 232, 210));
    maison(g, xf, -44., -14., 40., 36., rgb(224, 210, 176), rgb(120, 70, 50));
    arbre(g, xf, 32., -16., 38., FEUILLAGE);
    sol(g, xf, -14., rgb(150, 160, 100), rgb(110, 130, 80));
    chaise(g, xf, 8., -5., 1.3);
    let p = Perso::femme(8., -5., 28., rgb(70, 60, 70), rgb(50, 46, 56), Chapeau::Coiffe).assis().bras(-0.15, -0.35).cheveux(rgb(220, 220, 220)).tablier(rgb(210, 200, 190));
    p.dessiner(g, xf);
    let (mx, my) = p.main(-1.);
    g.line(xf, mx, my, mx + 6., my - 4., 0.3, GRIS);
    g.line(xf, mx + 6., my, mx, my - 4., 0.3, GRIS);
    g.poly(xf, &rect(mx + 1., my - 2., 4., 2.), Some(ROUGE), None);
    rond(g, xf, 20., -6., 2.0, ROUGE);
    g.line(xf, 18., -6., mx + 3., my, 0.2, ROUGE);
}

// ---------- 6 : le matin ----------
fn petit_dejeuner(g: &Gfx, xf: &Xf) {
    interieur(g, xf, rgb(220, 200, 160));
    fenetre(g, xf, -8., -56., 16., 26., rgb(250, 236, 180));
    g.lueur(xf, (0., -42.), 22., &[(0.0, alpha(rgb(255, 240, 190), 0.6)), (1.0, alpha(rgb(255, 240, 190), 0.0))]);
    rideaux(g, xf, -8., -56., 16., 28., rgb(220, 230, 240));
    let h = Perso::homme(-20., -6., 30., rgb(60, 60, 80), rgb(60, 60, 70), Chapeau::Aucun).assis().bras(0.45, 0.45);
    h.dessiner(g, xf);
    let (gx, gy) = h.main(-1.);
    let (dx, _) = h.main(1.);
    g.poly(xf, &rect(gx - 0.5, gy - 9., dx - gx + 1., 10.), Some(rgb(236, 232, 220)), Some(GRIS));
    for k in 0..4 {
        g.line(xf, gx + 0.5, gy - 7. + k as f32 * 2., dx - 0.5, gy - 7. + k as f32 * 2., 0.25, GRIS);
    }
    let f = Perso::femme(20., -6., 28., rgb(200, 210, 230), rgb(120, 130, 170), Chapeau::Aucun).assis().bras(0.2, 0.75);
    f.dessiner(g, xf);
    table(g, xf, 0., -6., 34., 12., Some(BLANC));
    cafetiere(g, xf, 8., -18.);
    tasse(g, xf, -4., -18.);
    tasse(g, xf, 2., -18.);
    g.ellipse(xf, -10., -18.6, 3., 1.2, Some(rgb(220, 170, 90)), None);
}

fn fauche(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(170, 160, 200), rgb(250, 200, 160));
    soleil(g, xf, -26., -30., 5., rgb(255, 200, 120));
    collines(g, xf, &[(-44., -30.), (-10., -33.), (20., -29.), (44., -32.)], rgb(130, 140, 110));
    sol(g, xf, -26., BLE, assombrir(BLE, 0.25));
    for k in 0..30 {
        let x = -42. + (k * 37 % 86) as f32;
        let y = -24. + (k * 13 % 22) as f32;
        g.line(xf, x, y, x + 0.5, y - 3., 0.4, assombrir(BLE, 0.3));
    }
    g.poly(xf, &[(-44., -10.), (-10., -12.), (-6., -1.), (-44., -1.)], Some(rgb(200, 170, 90)), None);
    let p = Perso::homme(6., -6., 30., rgb(230, 226, 214), rgb(80, 90, 120), Chapeau::Paille).bras(1.3, 1.0).peau(PEAU_HALEE);
    p.dessiner(g, xf);
    faux(g, xf, p.main(-1.), 1.0);
    gerbe(g, xf, 30., -6., 1.0);
}

// ---------- 7 : l'après-midi ----------
fn salon(g: &Gfx, xf: &Xf) {
    interieur(g, xf, rgb(170, 160, 190));
    tableau_mural(g, xf, -10., -56., 20., 15.);
    fauteuil(g, xf, -22., -6., 1.3, rgb(120, 60, 90));
    fauteuil(g, xf, 22., -6., 1.3, rgb(120, 60, 90));
    Perso::femme(-22., -6., 28., rgb(240, 220, 200), rgb(160, 90, 80), Chapeau::Aucun).assis().bras(-0.2, 0.55).dessiner(g, xf);
    Perso::homme(22., -6., 30., rgb(60, 60, 70), rgb(120, 110, 100), Chapeau::Aucun).assis().bras(0.6, -0.2).dessiner(g, xf);
    table(g, xf, 0., -6., 14., 10., Some(rgb(240, 236, 226)));
    g.ellipse(xf, 0., -18., 3., 2.2, Some(BLANC), Some(GRIS));
    g.line(xf, 3., -18., 5., -20., 0.6, GRIS);
    tasse(g, xf, -4., -16.);
}

fn repos_champs(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(120, 170, 220), rgb(240, 236, 200));
    soleil(g, xf, 24., -54., 4., rgb(255, 230, 150));
    sol(g, xf, -26., BLE, assombrir(BLE, 0.2));
    meule(g, xf, 28., -24., 1.4);
    arbre(g, xf, -16., -12., 50., rgb(54, 92, 48));
    g.ellipse(xf, -14., -8., 22., 4., Some(alpha(rgb(40, 60, 30), 0.35)), None);
    Perso::homme(-24., -6., 26., rgb(230, 226, 210), rgb(80, 90, 120), Chapeau::Paille).assis().bras(0.3, 0.8).peau(PEAU_HALEE).dessiner(g, xf);
    Perso::femme(-4., -6., 25., rgb(200, 80, 70), rgb(90, 80, 110), Chapeau::Coiffe).assis().bras(0.6, 0.3).peau(PEAU_HALEE).tablier(rgb(240, 230, 210)).dessiner(g, xf);
    g.poly(xf, &[(8., -4.), (11., -4.), (11.5, -9.), (7.5, -9.)], Some(rgb(180, 110, 70)), None);
    g.ellipse(xf, 14., -4., 4., 1.5, Some(rgb(220, 190, 120)), None);
}

// ---------- 8 : le soir ----------
fn musique(g: &Gfx, xf: &Xf) {
    interieur(g, xf, rgb(130, 60, 70));
    lustre(g, xf, 0., -54.);
    // piano droit
    g.poly(xf, &rect(-40., -36., 26., 28.), Some(rgb(36, 26, 24)), None);
    g.poly(xf, &rect(-40., -22., 26., 3.), Some(BLANC), None);
    for k in 0..12 {
        g.line(xf, -39. + k as f32 * 2.2, -22., -39. + k as f32 * 2.2, -20., 0.3, NOIR);
    }
    for x in [-36., -18.] {
        g.line(xf, x, -36., x, -40., 0.4, BLANC);
        g.lueur(xf, (x, -41.), 3., &[(0.0, alpha(rgb(255, 230, 150), 0.9)), (1.0, alpha(rgb(255, 230, 150), 0.0))]);
    }
    let f = Perso::femme(-27., -6., 28., rgb(230, 210, 230), rgb(170, 120, 170), Chapeau::Aucun).assis().bras(0.55, 0.55).dos();
    f.dessiner(g, xf);
    let m = Perso::homme(16., -6., 31., rgb(30, 30, 40), rgb(30, 30, 40), Chapeau::Aucun).bras(1.75, -0.35);
    m.dessiner(g, xf);
    let (gx, gy) = m.main(-1.);
    violon(g, xf, (gx + 16.) / 2. - 1., (gy - 30.) / 2. + 1.5, -1.25);
    let (mx, my) = m.main(1.);
    g.line(xf, mx - 5., my - 7., mx + 5., my + 5., 0.3, CREME);
}

fn seuil(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(90, 80, 140), rgb(250, 170, 110));
    soleil(g, xf, 30., -24., 5., rgb(255, 170, 90));
    collines(g, xf, &[(-44., -24.), (0., -26.), (44., -22.)], rgb(110, 100, 110));
    sol(g, xf, -20., rgb(130, 120, 80), rgb(90, 80, 50));
    // maison et porte éclairée
    g.poly(xf, &rect(-40., -50., 40., 46.), Some(rgb(206, 186, 150)), None);
    g.poly(xf, &[(-43., -50.), (3., -50.), (-20., -62.)], Some(rgb(120, 60, 44)), None);
    g.poly(xf, &rect(-26., -26., 12., 22.), Some(rgb(255, 214, 140)), None);
    g.lueur(xf, (-20., -14.), 18., &[(0.0, alpha(rgb(255, 210, 130), 0.5)), (1.0, alpha(rgb(255, 210, 130), 0.0))]);
    fenetre(g, xf, -38., -40., 8., 9., rgb(255, 214, 140));
    Perso::homme(-28., -4., 29., rgb(90, 80, 70), rgb(60, 60, 70), Chapeau::Casquette).bras(0.2, 0.4).dessiner(g, xf);
    Perso::femme(-12., -4., 27., rgb(150, 60, 60), rgb(60, 60, 80), Chapeau::Coiffe).bras(0.4, 0.2).tablier(BLANC).dessiner(g, xf);
    Perso::homme(0., -3., 16., rgb(70, 100, 150), rgb(110, 100, 90), Chapeau::Aucun).bras(0.3, 1.8).cheveux(rgb(200, 160, 90)).dessiner(g, xf);
    arbre(g, xf, 30., -16., 30., rgb(50, 70, 50));
}

// ---------- 9 : la nuit ----------
fn retour_chasse(g: &Gfx, xf: &Xf) {
    let nuit = rgb(30, 40, 76);
    ciel(g, xf, rgb(20, 26, 56), nuit);
    lune(g, xf, 26., -52., 4.5, rgb(24, 32, 64));
    etoiles(g, xf, &[(-30., -56.), (-12., -60.), (4., -52.), (-38., -46.), (12., -58.), (36., -40.)]);
    for (x, w, h) in [(-44., 18., 22.), (-24., 14., 28.), (24., 20., 24.)] {
        g.poly(xf, &rect(x, -16. - h, w, h), Some(rgb(36, 36, 56)), None);
        fenetre(g, xf, x + w * 0.3, -16. - h * 0.7, w * 0.3, h * 0.25, rgb(255, 210, 120));
    }
    sol(g, xf, -16., rgb(50, 54, 70), rgb(30, 32, 46));
    for (x, c) in [(-8., rgb(80, 70, 50)), (8., rgb(70, 80, 60))] {
        let p = Perso::homme(x, -5., 30., c, rgb(90, 80, 60), Chapeau::Melon).bras(0.15, -0.5);
        p.dessiner(g, xf);
        let (mx, my) = p.main(1.);
        fusil(g, xf, (mx - 1., my + 1.), (mx + 8., my - 15.));
    }
    lanterne(g, xf, -15., -16., 1.0);
    chien(g, xf, 22., -4., 1.0, rgb(150, 120, 90));
}

fn veilleur(g: &Gfx, xf: &Xf) {
    let fond = rgb(26, 34, 66);
    ciel(g, xf, rgb(16, 22, 50), fond);
    lune(g, xf, -30., -54., 4., rgb(18, 24, 54));
    etoiles(g, xf, &[(10., -58.), (24., -54.), (36., -60.), (-8., -60.), (2., -50.)]);
    for (x, w, h) in [(-44., 22., 26.), (-20., 18., 32.), (18., 26., 24.)] {
        g.poly(xf, &rect(x, -14. - h, w, h), Some(rgb(44, 40, 52)), None);
        g.poly(xf, &[(x - 1., -14. - h), (x + w + 1., -14. - h), (x + w / 2., -22. - h)], Some(rgb(34, 30, 40)), None);
    }
    sol(g, xf, -14., rgb(60, 56, 64), rgb(36, 34, 40));
    let p = Perso::homme(6., -5., 31., rgb(70, 60, 50), rgb(60, 54, 50), Chapeau::Melon).bras(1.0, 0.1);
    p.dessiner(g, xf);
    let (mx, my) = p.main(-1.);
    lanterne(g, xf, mx - 1., my + 2., 1.2);
    let (dx, dy) = p.main(1.);
    g.line(xf, dx, dy + 6., dx + 1., dy - 24., 0.6, SEPIA);
    g.poly(xf, &[(dx + 1., dy - 24.), (dx + 3., dy - 21.), (dx - 0.5, dy - 20.)], Some(GRIS), None);
}

// ---------- 10 : la terre ----------
fn mine(g: &Gfx, xf: &Xf) {
    g.poly(xf, &rect(-44., -64., 88., 64.), Some(rgb(54, 40, 30)), None);
    g.ellipse(xf, 0., -30., 34., 30., Some(rgb(30, 22, 18)), None);
    for x in [-28., 28.] {
        g.poly(xf, &rect(x - 2., -50., 4., 46.), Some(BOIS), None);
    }
    g.poly(xf, &rect(-32., -52., 64., 4.), Some(BOIS), None);
    sol(g, xf, -10., rgb(70, 56, 44), rgb(46, 36, 28));
    g.line(xf, -44., -6., 44., -6., 0.8, GRIS);
    g.line(xf, -44., -3., 44., -3., 0.8, GRIS);
    // wagonnet de charbon
    g.poly(xf, &[(4., -16.), (24., -16.), (22., -8.), (6., -8.)], Some(rgb(90, 80, 74)), Some(rgb(50, 44, 40)));
    g.ellipse(xf, 14., -16., 10., 2.6, Some(rgb(24, 22, 24)), None);
    rond(g, xf, 9., -7., 1.6, NOIR);
    rond(g, xf, 19., -7., 1.6, NOIR);
    for (x, a) in [(-14., 2.2), (32., 0.4)] {
        let p = Perso::homme(x, -5., 29., rgb(90, 90, 100), rgb(70, 70, 80), Chapeau::Casquette).bras(a, 0.6).peau(rgb(200, 170, 140));
        p.dessiner(g, xf);
        let (mx, my) = p.main(-1.);
        lanterne(g, xf, mx, my + 2., 0.9);
    }
    g.line(xf, -12., -34., -4., -28., 0.6, BOIS);
    g.line(xf, -6., -31., -2., -26., 1.2, GRIS);
}

fn berger(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(130, 170, 220), rgb(230, 236, 236));
    g.poly(xf, &[(-44., -30.), (-26., -54.), (-12., -36.), (4., -58.), (24., -34.), (36., -46.), (44., -38.), (44., -24.), (-44., -24.)], Some(rgb(120, 130, 150)), None);
    for (x, y) in [(-26., -54.), (4., -58.), (36., -46.)] {
        g.poly(xf, &[(x - 4., y + 5.), (x, y), (x + 4., y + 5.)], Some(BLANC), None);
    }
    collines(g, xf, &[(-44., -24.), (-10., -28.), (44., -22.)], HERBE);
    sol(g, xf, -16., HERBE, HERBE_FONCEE);
    sapin(g, xf, -36., -18., 20., rgb(40, 80, 50));
    sapin(g, xf, -28., -20., 16., rgb(50, 90, 56));
    let p = Perso::homme(-12., -5., 29., rgb(120, 90, 60), rgb(90, 80, 60), Chapeau::Paille).bras(0.2, 0.9).peau(PEAU_HALEE);
    p.dessiner(g, xf);
    let (mx, my) = p.main(1.);
    g.line(xf, mx, my + 8., mx + 1., my - 14., 0.7, BOIS);
    for (x, y) in [(8., -6.), (18., -9.), (26., -4.), (34., -10.)] {
        mouton(g, xf, x, y, 0.9, false);
    }
    chien(g, xf, -26., -3., 0.9, rgb(40, 34, 30));
}

// ---------- 11 : l'eau ----------
fn barque_lac(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(130, 170, 220), rgb(236, 230, 214));
    nuage(g, xf, 14., -54., 1.2);
    collines(g, xf, &[(-44., -32.), (0., -35.), (44., -31.)], rgb(110, 140, 110));
    arbre(g, xf, -34., -28., 26., FEUILLAGE);
    arbre(g, xf, 34., -28., 22., rgb(60, 100, 60));
    degrade(g, xf, &rect(-44., -28., 88., 28.), -28., 0., rgb(170, 200, 220), rgb(70, 110, 150));
    reflets(g, xf, 0., -26., -16., BLANC);
    let r = Perso::homme(-6., -10., 26., rgb(240, 240, 236), rgb(50, 60, 90), Chapeau::Paille).assis().bras(1.4, 1.4);
    r.dessiner(g, xf);
    g.line(xf, -22., -14., 8., -20., 0.6, BOIS);
    let d = Perso::femme(10., -10., 25., rgb(240, 210, 220), rgb(200, 120, 140), Chapeau::Capeline(BLANC)).assis().bras(0.2, 0.6);
    d.dessiner(g, xf);
    ombrelle(g, xf, d.main(1.), rgb(250, 240, 200));
    barque(g, xf, 2., -6., 1.4, rgb(140, 90, 50));
    for (x, y) in [(28., -12.), (34., -8.)] {
        g.ellipse(xf, x, y, 2.6, 1.3, Some(BLANC), None);
        g.line(xf, x + 2., y - 1., x + 2.6, y - 4., 0.6, BLANC);
        g.ellipse(xf, x + 3., y - 4.2, 0.8, 0.5, Some(BLANC), None);
    }
}

fn pique_nique(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(140, 186, 226), rgb(236, 236, 214));
    arbre(g, xf, -30., -26., 44., FEUILLAGE);
    arbre(g, xf, 30., -30., 30., rgb(66, 104, 56));
    degrade(g, xf, &[(-44., -30.), (44., -34.), (44., -26.), (-44., -22.)], -34., -22., rgb(160, 196, 216), rgb(100, 140, 170));
    sol(g, xf, -24., HERBE, HERBE_FONCEE);
    // nappe à carreaux
    g.poly(xf, &[(-14., -14.), (18., -14.), (24., -4.), (-20., -4.)], Some(BLANC), None);
    for k in 0..6 {
        let t = k as f32 / 6.;
        g.line(xf, -14. + 32. * t, -14., -20. + 44. * t, -4., 1.2, alpha(ROUGE, 0.6));
    }
    for k in 0..3 {
        let y = -12. + k as f32 * 3.;
        g.line(xf, -15., y, 20., y, 1.0, alpha(ROUGE, 0.6));
    }
    g.poly(xf, &rect(-2., -14., 7., 4.), Some(rgb(180, 140, 80)), Some(BOIS));
    g.poly(xf, &rect(8., -17., 1.6, 5.), Some(rgb(60, 110, 70)), None);
    Perso::homme(-24., -6., 27., rgb(240, 240, 236), rgb(110, 100, 90), Chapeau::Paille).assis().bras(0.3, 0.7).dessiner(g, xf);
    Perso::femme(28., -5., 26., rgb(200, 220, 240), rgb(90, 120, 170), Chapeau::Capeline(rgb(250, 240, 210))).assis().bras(0.7, 0.2).dessiner(g, xf);
    Perso::homme(4., -2., 16., rgb(160, 50, 50), rgb(230, 230, 230), Chapeau::Aucun).assis().bras(0.9, 0.3).cheveux(rgb(200, 160, 90)).dessiner(g, xf);
}

// ---------- 12 : la danse ----------
fn soiree(g: &Gfx, xf: &Xf) {
    interieur(g, xf, rgb(200, 170, 120));
    for x in [-30., 30.] {
        g.poly(xf, &rect(x - 7., -60., 14., 38.), None, Some(rgb(230, 200, 110)));
    }
    lustre(g, xf, 0., -52.);
    foule(g, xf, -12., -42., -26.);
    foule(g, xf, -12., 27., 43.);
    let d = Perso::femme(-4., -5., 29., rgb(240, 230, 236), rgb(230, 170, 190), Chapeau::Aucun).bras(0.3, 1.25);
    d.dessiner(g, xf);
    g.ellipse(xf, -4., -9., 10., 3., Some(alpha(rgb(250, 200, 214), 0.9)), None);
    let m = Perso::homme(7., -5., 31., rgb(24, 24, 30), rgb(24, 24, 30), Chapeau::Aucun).bras(1.25, 0.3);
    m.dessiner(g, xf);
    g.poly(xf, &[(5.5, -27.), (8.5, -27.), (7., -24.)], Some(BLANC), None);
}

fn danse_village(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(150, 190, 230), rgb(244, 236, 210));
    arbre(g, xf, 0., -24., 42., FEUILLAGE);
    guirlande(g, xf, -44., -50., -4., -42., 7);
    guirlande(g, xf, 4., -42., 44., -50., 7);
    sol(g, xf, -24., rgb(200, 186, 140), rgb(160, 146, 100));
    let cols = [(ROUGE, rgb(60, 60, 90)), (rgb(90, 120, 170), rgb(240, 230, 210)), (rgb(110, 150, 90), rgb(80, 70, 60)), (rgb(220, 180, 70), rgb(150, 60, 60))];
    for (k, x) in [-30., -16., 16., 30.].iter().enumerate() {
        let (h, b) = cols[k];
        let p = if k % 2 == 0 {
            Perso::homme(*x, -5. - (k % 2) as f32, 27., h, b, Chapeau::Paille).bras(2.3, 2.3)
        } else {
            Perso::femme(*x, -5., 26., h, b, Chapeau::Coiffe).bras(2.3, 2.3).tablier(BLANC)
        };
        p.peau(PEAU_HALEE).dessiner(g, xf);
    }
    let m = Perso::homme(0., -4., 26., rgb(80, 60, 50), rgb(60, 50, 40), Chapeau::Melon).bras(1.0, 1.1);
    m.dessiner(g, xf);
    g.ellipse(xf, 0., -14., 4., 3., Some(rgb(150, 100, 60)), Some(SEPIA));
    g.line(xf, -2., -16., -6., -22., 0.6, NOIR);
}

// ---------- 13 : les emplettes ----------
fn magasin(g: &Gfx, xf: &Xf) {
    interieur(g, xf, rgb(190, 176, 150));
    g.poly(xf, &rect(-42., -60., 84., 34.), Some(assombrir(BOIS, 0.1)), None);
    let cols = [ROUGE, BLEU, VERT, OCRE, VIOLET, ROSE, BLANC, rgb(90, 140, 160)];
    for r in 0..3 {
        for k in 0..12 {
            let x = -40. + k as f32 * 6.8;
            let y = -58. + r as f32 * 11.;
            g.poly(xf, &rect(x, y, 5.6, 8.), Some(cols[(k + r * 3) % cols.len()]), Some(assombrir(cols[(k + r * 3) % cols.len()], 0.3)));
        }
    }
    Perso::homme(-14., -20., 26., rgb(240, 240, 236), rgb(60, 60, 70), Chapeau::Aucun).bras(0.35, 0.6).tablier(rgb(80, 70, 60)).dessiner(g, xf);
    g.poly(xf, &rect(-36., -20., 46., 14.), Some(BOIS), None);
    g.poly(xf, &rect(-37., -21., 48., 2.), Some(BOIS_CLAIR), None);
    g.poly(xf, &[(-20., -21.), (-6., -21.), (-4., -24.), (-18., -24.)], Some(rgb(180, 60, 80)), None);
    let d = Perso::femme(26., -5., 30., rgb(70, 90, 140), rgb(50, 60, 100), Chapeau::Capeline(rgb(230, 200, 150))).bras(0.85, 0.2);
    d.dessiner(g, xf);
}

fn epicerie(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(160, 196, 228), rgb(240, 236, 214));
    g.poly(xf, &rect(-44., -54., 70., 44.), Some(rgb(226, 206, 168)), None);
    g.poly(xf, &rect(-44., -58., 70., 5.), Some(rgb(150, 70, 50)), None);
    g.poly(xf, &rect(-30., -52., 40., 6.), Some(rgb(60, 100, 70)), None);
    for k in 0..6 {
        g.poly(xf, &rect(-26. + k as f32 * 6., -50.5, 3.5, 3.), Some(CREME), None);
    }
    store(g, xf, -40., -46., 60., ROUGE);
    fenetre(g, xf, -38., -38., 14., 14., rgb(200, 220, 230));
    g.poly(xf, &rect(-18., -40., 10., 30.), Some(assombrir(BOIS, 0.2)), None);
    sol(g, xf, -10., rgb(196, 180, 150), rgb(160, 146, 116));
    tonneau(g, xf, 2., -6., 1.2);
    tonneau(g, xf, 10., -6., 1.1);
    sac(g, xf, 18., -5., rgb(214, 196, 150));
    Perso::homme(-12., -5., 28., rgb(230, 230, 226), rgb(70, 70, 80), Chapeau::Casquette).bras(0.3, 0.6).tablier(rgb(200, 190, 170)).dessiner(g, xf);
    let f = Perso::femme(30., -5., 27., rgb(150, 70, 70), rgb(70, 70, 90), Chapeau::Coiffe).bras(0.2, 0.9).tablier(BLANC);
    f.dessiner(g, xf);
    let (mx, my) = f.main(1.);
    g.ellipse(xf, mx + 2., my + 1., 3.5, 2., Some(rgb(190, 140, 80)), Some(BOIS));
    g.ellipse(xf, mx + 2., my - 0.5, 3., 1.8, None, Some(BOIS));
}

// ---------- 14 : le plein air ----------
fn chasse(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(170, 190, 200), rgb(236, 220, 190));
    for (x, h) in [(-36., 40.), (-26., 34.), (32., 42.), (40., 30.)] {
        arbre(g, xf, x, -20., h, rgb(70, 90, 50));
    }
    sol(g, xf, -20., rgb(140, 130, 80), rgb(100, 90, 56));
    oiseaux(g, xf, &[(10., -54.), (18., -50.), (14., -58.)]);
    let p = Perso::homme(-6., -5., 30., rgb(100, 90, 60), rgb(80, 70, 50), Chapeau::Melon).bras(2.2, 2.4);
    p.dessiner(g, xf);
    let (mx, my) = p.main(1.);
    fusil(g, xf, (mx - 4., my + 6.), (mx + 10., my - 10.));
    g.lueur(xf, (mx + 11., my - 11.), 3., &[(0.0, alpha(BLANC, 0.8)), (1.0, alpha(BLANC, 0.0))]);
    chien(g, xf, 16., -4., 1.2, rgb(240, 236, 226));
    g.ellipse(xf, 15., -8., 1.6, 1.2, Some(rgb(150, 100, 60)), None);
}

fn peche(g: &Gfx, xf: &Xf) {
    degrade(g, xf, &rect(-44., -64., 88., 41.), -63., -23., rgb(92, 122, 176), rgb(248, 204, 136));
    for (x, y, rx) in [(-20., -52., 9.), (-12., -54., 7.), (24., -48., 8.)] {
        g.ellipse(xf, x, y, rx, 2.2, Some(alpha(rgb(255, 236, 214), 0.55)), None);
    }
    soleil(g, xf, 16., -30., 5.5, rgb(255, 214, 140));
    g.poly(xf, &[(-44., -27.), (-28., -31.), (-14., -28.), (2., -32.), (20., -27.), (44., -30.), (44., -23.), (-44., -23.)], Some(rgb(104, 116, 136)), None);
    degrade(g, xf, &rect(-44., -24., 88., 24.), -24., -1., rgb(232, 190, 142), rgb(54, 84, 116));
    reflets(g, xf, 16., -22.5, -8., rgb(255, 232, 170));
    g.poly(xf, &[(-44., -16.), (-30., -17.), (-18., -13.), (-11., -8.), (-14., 0.), (-44., 0.)], Some(rgb(48, 64, 36)), None);
    for (x, y, r) in [(-38., -30., 7.), (-31., -36., 8.), (-24., -27., 6.), (-36., -22., 6.)] {
        g.line(xf, x, y + r * 0.6, x, -15., 1.2, SEPIA);
        g.ellipse(xf, x, y, r * 0.8, r, Some(rgb(38, 62, 38)), None);
        g.ellipse(xf, x - 1.5, y - 2., r * 0.5, r * 0.6, Some(rgb(64, 94, 52)), None);
    }
    barque(g, xf, 2., -7., 1.0, rgb(96, 58, 30));
    let p = Perso::homme(0., -11., 20., rgb(52, 64, 120), rgb(52, 64, 120), Chapeau::Melon).assis().bras(0.8, 1.3);
    p.dessiner(g, xf);
    let (mx, my) = p.main(1.);
    g.line(xf, mx, my, 22., -36., 0.6, rgb(90, 60, 30));
    g.line(xf, 22., -36., 24., -10., 0.25, alpha(rgb(220, 220, 220), 0.8));
    for (x, h) in [(30., 12.), (33., 15.), (36., 10.), (39., 13.)] {
        g.line(xf, x, 0., x - 1., -h, 0.5, rgb(46, 70, 34));
        g.ellipse(xf, x - 1., -h + 2., 0.7, 1.8, Some(rgb(96, 60, 30)), None);
    }
    oiseaux(g, xf, &[(-6., -50.), (2., -55.), (-12., -45.)]);
}

// ---------- 15 : les arts ----------
fn photographie(g: &Gfx, xf: &Xf) {
    interieur(g, xf, rgb(150, 150, 160));
    degrade(g, xf, &rect(4., -60., 38., 50.), -60., -10., rgb(110, 120, 140), rgb(70, 76, 96));
    g.poly(xf, &[(36., -14.), (42., -14.), (40., -4.), (37., -4.)], Some(rgb(180, 120, 80)), None);
    for (dx, dy) in [(-2., -22.), (2., -20.), (0., -26.), (-3., -18.), (3., -24.)] {
        g.ellipse(xf, 39. + dx, dy, 2.2, 1.2, Some(VERT), None);
    }
    chaise(g, xf, 20., -5., 1.3);
    Perso::femme(20., -5., 28., rgb(230, 220, 236), rgb(110, 80, 120), Chapeau::Capeline(rgb(240, 236, 226))).assis().bras(-0.2, -0.2).dessiner(g, xf);
    // appareil sur trépied, photographe sous le voile
    for dx in [-6., 0., 6.] {
        g.line(xf, -18., -24., -18. + dx, -5., 0.6, BOIS);
    }
    g.poly(xf, &rect(-24., -32., 12., 9.), Some(rgb(110, 70, 40)), Some(rgb(60, 36, 20)));
    g.poly(xf, &rect(-14., -30., 5., 5.), Some(rgb(40, 40, 44)), None);
    rond(g, xf, -8.5, -27.5, 1.6, rgb(120, 130, 150));
    let p = Perso::homme(-30., -5., 29., rgb(60, 50, 50), rgb(70, 66, 60), Chapeau::Aucun).bras(1.4, 1.0);
    p.dessiner(g, xf);
    g.poly(xf, &[(-34., -36.), (-24., -33.), (-24., -24.), (-35., -26.)], Some(NOIR), None);
}

fn peinture(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(150, 190, 226), rgb(240, 236, 214));
    nuage(g, xf, -16., -54., 1.0);
    collines(g, xf, &[(-44., -30.), (-10., -34.), (20., -30.), (44., -33.)], rgb(130, 160, 110));
    degrade(g, xf, &[(-44., -26.), (44., -28.), (44., -20.), (-44., -18.)], -28., -18., rgb(170, 200, 216), rgb(110, 150, 180));
    sol(g, xf, -20., HERBE, HERBE_FONCEE);
    arbre(g, xf, 34., -18., 36., FEUILLAGE);
    // chevalet et toile
    for (x0, x1) in [(-8., -12.), (-8., -4.), (-8., -8.)] {
        g.line(xf, x0, -38., x1, -4., 0.6, BOIS);
    }
    g.poly(xf, &rect(-16., -36., 16., 12.), Some(BLANC), Some(BOIS));
    degrade(g, xf, &rect(-15., -35., 14., 6.), -35., -29., rgb(150, 190, 226), rgb(240, 236, 214));
    g.poly(xf, &rect(-15., -29., 14., 4.), Some(HERBE), None);
    let p = Perso::homme(12., -5., 30., rgb(120, 90, 130), rgb(80, 80, 90), Chapeau::Paille).bras(0.6, 1.5);
    p.dessiner(g, xf);
    let (mx, my) = p.main(-1.);
    g.ellipse(xf, mx - 1., my + 1., 3.5, 2.2, Some(rgb(220, 190, 140)), Some(BOIS));
    for (k, c) in [ROUGE, BLEU, rgb(250, 220, 90), VERT].iter().enumerate() {
        rond(g, xf, mx - 2.6 + k as f32 * 1.4, my + 0.4, 0.55, *c);
    }
    let (px, py) = p.main(1.);
    g.line(xf, px, py, px - 6., py - 2., 0.4, BOIS);
}

// ---------- 16 : le printemps ----------
fn jardinier(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(160, 200, 236), rgb(240, 240, 220));
    for (x, h) in [(-36., 30.), (36., 34.)] {
        arbre(g, xf, x, -26., h, rgb(90, 150, 80));
        for k in 0..5 {
            rond(g, xf, x - 6. + k as f32 * 3., -26. - h * 0.75 + (k % 2) as f32 * 3., 0.9, ROSE);
        }
    }
    sol(g, xf, -26., HERBE, HERBE_FONCEE);
    g.poly(xf, &[(-44., -18.), (44., -20.), (44., -12.), (-44., -10.)], Some(rgb(226, 212, 176)), None);
    for y in [-23., -6.] {
        g.ellipse(xf, -18., y, 16., 3., Some(rgb(110, 80, 50)), None);
        fleurs(g, xf, -32., y, 28.);
    }
    let p = Perso::homme(18., -8., 29., rgb(80, 110, 80), rgb(110, 100, 80), Chapeau::Paille).bras(0.3, 1.3).tablier(rgb(70, 90, 120));
    p.dessiner(g, xf);
    let (mx, my) = p.main(1.);
    g.poly(xf, &rect(mx, my - 1., 6., 5.), Some(rgb(110, 140, 120)), Some(rgb(60, 80, 70)));
    g.line(xf, mx + 6., my + 1., mx + 10., my + 3., 0.7, rgb(110, 140, 120));
    for k in 0..4 {
        g.line(xf, mx + 10.5, my + 3.5 + k as f32 * 0.3, mx + 12. + k as f32 * 0.6, my + 7., 0.2, rgb(150, 190, 230));
    }
}

fn tonte(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(170, 206, 236), rgb(240, 240, 224));
    g.poly(xf, &rect(-44., -50., 36., 34.), Some(rgb(150, 100, 70)), None);
    g.poly(xf, &[(-46., -50.), (-6., -50.), (-26., -62.)], Some(rgb(110, 70, 50)), None);
    g.poly(xf, &rect(-32., -36., 12., 20.), Some(rgb(60, 40, 30)), None);
    sol(g, xf, -16., HERBE, HERBE_FONCEE);
    for (x, y) in [(22., -14.), (32., -18.), (38., -10.)] {
        mouton(g, xf, x, y, 1.0, false);
    }
    mouton(g, xf, -4., -5., 1.4, true);
    let p = Perso::homme(10., -4., 26., rgb(230, 226, 214), rgb(90, 80, 70), Chapeau::Casquette).assis().bras(1.6, 1.2).peau(PEAU_HALEE);
    p.dessiner(g, xf);
    let (mx, my) = p.main(-1.);
    g.line(xf, mx, my, mx - 3., my + 2., 0.6, GRIS);
    g.ellipse(xf, -30., -6., 7., 3., Some(rgb(246, 244, 236)), Some(rgb(200, 196, 186)));
}

// ---------- 17 : l'été ----------
fn courses(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(130, 180, 230), rgb(240, 236, 214));
    // tribune
    g.poly(xf, &rect(-44., -50., 50., 18.), Some(rgb(230, 220, 200)), None);
    g.poly(xf, &[(-46., -50.), (8., -50.), (4., -56.), (-42., -56.)], Some(rgb(70, 110, 80)), None);
    foule(g, xf, -36., -42., 4.);
    for x in [-42., -24., -6.] {
        drapeau(g, xf, x, -56., 6., &[ROUGE]);
    }
    sol(g, xf, -32., HERBE, HERBE_FONCEE);
    g.line(xf, -44., -26., 44., -27., 0.7, BLANC);
    for x in [-40., -28., -16., -4., 8., 20., 32.] {
        g.line(xf, x, -26., x, -23., 0.5, BLANC);
    }
    degrade(g, xf, &rect(-44., -22., 88., 22.), -22., 0., rgb(150, 170, 90), rgb(110, 130, 70));
    for (x, y, c, soie) in [(-12., -6., rgb(110, 60, 30), ROUGE), (14., -10., rgb(40, 30, 26), BLEU)] {
        cheval(g, xf, x, y, 1.4, c, true);
        let j = Perso::homme(x + 1., y - 9., 12., soie, BLANC, Chapeau::Casquette).assis().bras(1.4, 1.4);
        j.dessiner(g, xf);
    }
}

fn sechage(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(120, 170, 226), rgb(244, 230, 190));
    soleil(g, xf, -28., -52., 5., rgb(255, 230, 140));
    collines(g, xf, &[(-44., -30.), (44., -32.)], rgb(160, 170, 110));
    sol(g, xf, -28., BLE, assombrir(BLE, 0.22));
    for (x, y, s) in [(-34., -22., 0.7), (-22., -24., 0.6), (-10., -22., 0.7), (30., -24., 0.6), (38., -20., 0.8)] {
        gerbe(g, xf, x, y, s);
    }
    meule(g, xf, -26., -6., 1.6);
    let p = Perso::homme(10., -5., 29., rgb(230, 226, 214), rgb(80, 90, 120), Chapeau::Paille).bras(1.2, 2.4).peau(PEAU_HALEE);
    p.dessiner(g, xf);
    let (mx, my) = p.main(1.);
    fourche(g, xf, (mx - 6., my + 10.), (mx + 3., my - 6.));
    gerbe(g, xf, 28., -4., 1.0);
}

// ---------- 18 : l'automne ----------
fn marche(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(170, 186, 200), rgb(236, 226, 206));
    maison(g, xf, -44., -24., 30., 22., rgb(206, 190, 160), rgb(130, 70, 50));
    maison(g, xf, 16., -24., 28., 20., rgb(220, 204, 170), rgb(110, 80, 60));
    sol(g, xf, -24., rgb(180, 170, 150), rgb(140, 130, 110));
    for (x, c) in [(-26., rgb(170, 40, 42)), (12., rgb(60, 110, 76))] {
        store(g, xf, x - 14., -34., 28., c);
        g.poly(xf, &rect(x - 13., -30., 26., 2.), Some(BOIS), None);
        g.line(xf, x - 12., -30., x - 12., -16., 0.6, BOIS);
        g.line(xf, x + 12., -30., x + 12., -16., 0.6, BOIS);
        g.poly(xf, &rect(x - 13., -18., 26., 2.), Some(BOIS_CLAIR), None);
        cagette(g, xf, x - 6., -18., rgb(220, 120, 40));
        cagette(g, xf, x + 5., -18., if x < 0.0 { rgb(200, 50, 40) } else { rgb(120, 170, 60) });
    }
    Perso::femme(-26., -4., 27., rgb(120, 70, 60), rgb(70, 60, 60), Chapeau::Coiffe).bras(0.5, 0.3).tablier(BLANC).dessiner(g, xf);
    let f = Perso::femme(2., -4., 28., rgb(80, 90, 130), rgb(60, 60, 90), Chapeau::Capeline(rgb(200, 170, 120))).bras(0.2, 0.9);
    f.dessiner(g, xf);
    let (mx, my) = f.main(1.);
    g.ellipse(xf, mx + 2., my + 1., 3.5, 2., Some(rgb(190, 140, 80)), Some(BOIS));
    Perso::homme(26., -4., 29., rgb(110, 90, 70), rgb(80, 70, 60), Chapeau::Melon).bras(0.3, 0.3).dessiner(g, xf);
}

fn battage(g: &Gfx, xf: &Xf) {
    g.poly(xf, &rect(-44., -64., 88., 64.), Some(rgb(110, 76, 50)), None);
    g.poly(xf, &rect(-30., -58., 60., 44.), Some(rgb(230, 210, 160)), None);
    g.poly(xf, &rect(-30., -58., 60., 44.), None, Some(rgb(80, 50, 30)));
    collines(g, xf, &[(-30., -26.), (30., -28.)], rgb(170, 160, 110));
    for x in [-30., 30.] {
        g.poly(xf, &rect(x - 2.5, -60., 5., 60.), Some(rgb(90, 60, 36)), None);
    }
    sol(g, xf, -14., rgb(190, 160, 100), rgb(150, 120, 70));
    for k in 0..24 {
        let x = -38. + (k * 29 % 76) as f32;
        let y = -12. + (k * 7 % 10) as f32;
        g.line(xf, x, y, x + 3., y - 0.6, 0.4, BLE);
    }
    for (x, a) in [(-14., 2.6), (14., 2.4)] {
        let p = Perso::homme(x, -5., 29., rgb(230, 226, 214), rgb(90, 90, 110), Chapeau::Aucun).bras(a, a - 0.3).peau(PEAU_HALEE);
        p.dessiner(g, xf);
        let (mx, my) = p.main(1.);
        g.line(xf, mx, my, mx + 2., my - 10., 0.6, BOIS);
        g.line(xf, mx + 2., my - 10., mx + 8., my - 12., 0.9, BOIS_CLAIR);
    }
    meule(g, xf, 34., -2., 1.0);
}

// ---------- 19 : l'hiver ----------
fn patinage(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(170, 180, 196), rgb(226, 228, 232));
    for (x, h) in [(-36., 36.), (-24., 28.), (32., 40.)] {
        arbre_nu(g, xf, x, -26., h);
    }
    sol(g, xf, -28., rgb(246, 246, 250), rgb(220, 224, 234));
    degrade(g, xf, &[(-36., -22.), (36., -24.), (40., -4.), (-40., -2.)], -24., -2., rgb(200, 220, 236), rgb(150, 180, 210));
    for k in 0..5 {
        g.line(xf, -30. + k as f32 * 12., -8., -24. + k as f32 * 12., -10., 0.3, alpha(BLANC, 0.8));
    }
    let d = Perso::femme(-10., -6., 28., rgb(110, 40, 50), rgb(90, 30, 40), Chapeau::Capeline(rgb(60, 50, 60))).bras(-0.35, -0.35);
    d.dessiner(g, xf);
    let (mx, my) = d.main(1.);
    g.ellipse(xf, mx - 1., my - 0.5, 3.4, 2.2, Some(rgb(210, 200, 190)), Some(rgb(150, 140, 130)));
    let m = Perso::homme(12., -8., 30., rgb(40, 50, 70), rgb(50, 50, 60), Chapeau::HautDeForme).bras(0.7, 0.45);
    m.dessiner(g, xf);
    for (x, y) in [(-12., -5.), (-8., -5.), (10., -7.), (14., -7.)] {
        g.line(xf, x - 1.5, y, x + 1.5, y, 0.4, GRIS);
    }
    for k in 0..18 {
        rond(g, xf, -42. + (k * 31 % 84) as f32, -60. + (k * 17 % 40) as f32, 0.5, BLANC);
    }
}

fn veillee(g: &Gfx, xf: &Xf) {
    g.poly(xf, &rect(-44., -64., 88., 64.), Some(rgb(70, 52, 40)), None);
    g.poly(xf, &rect(-44., -64., 88., 6.), Some(rgb(50, 36, 26)), None);
    cheminee(g, xf, -24., -10., 1.2);
    g.lueur(xf, (-24., -14.), 40., &[(0.0, alpha(rgb(255, 170, 80), 0.35)), (1.0, alpha(rgb(255, 170, 80), 0.0))]);
    sol(g, xf, -10., rgb(110, 80, 56), rgb(80, 56, 40));
    // rouet
    g.ellipse(xf, 22., -18., 6., 6., None, Some(BOIS));
    for k in 0..4 {
        let a = k as f32 * PI / 4.;
        g.line(xf, 22. - 6. * a.cos(), -18. - 6. * a.sin(), 22. + 6. * a.cos(), -18. + 6. * a.sin(), 0.3, BOIS);
    }
    g.line(xf, 18., -12., 26., -5., 0.6, BOIS);
    Perso::femme(32., -5., 27., rgb(60, 50, 60), rgb(50, 40, 50), Chapeau::Coiffe).assis().bras(1.3, 0.6).cheveux(rgb(220, 220, 220)).dessiner(g, xf);
    Perso::homme(0., -5., 29., rgb(110, 80, 60), rgb(70, 60, 50), Chapeau::Aucun).assis().bras(1.0, 1.6).cheveux(rgb(120, 110, 100)).dessiner(g, xf);
    Perso::homme(-8., -2., 15., rgb(150, 60, 50), rgb(80, 70, 60), Chapeau::Aucun).assis().bras(0.4, 0.4).cheveux(rgb(200, 160, 90)).dessiner(g, xf);
    Perso::femme(10., -2., 14., rgb(90, 110, 150), rgb(70, 80, 110), Chapeau::Aucun).assis().bras(0.4, 0.4).cheveux(rgb(150, 90, 50)).dessiner(g, xf);
}

// ---------- 20 : le jeu ----------
fn cartes_cafe(g: &Gfx, xf: &Xf) {
    interieur(g, xf, rgb(160, 120, 80));
    g.poly(xf, &rect(-16., -58., 32., 20.), Some(rgb(200, 210, 210)), Some(rgb(200, 160, 60)));
    for (x, c) in [(-36., rgb(60, 110, 70)), (-31., rgb(150, 40, 40)), (34., rgb(200, 180, 120))] {
        g.poly(xf, &rect(x, -40., 2.5, 8.), Some(c), None);
    }
    g.poly(xf, &rect(-40., -32., 82., 1.5), Some(BOIS), None);
    for (x, c, chap) in [(-24., rgb(60, 50, 50), Chapeau::Melon), (0., rgb(100, 80, 60), Chapeau::Aucun), (24., rgb(50, 60, 80), Chapeau::HautDeForme)] {
        let p = Perso::homme(x, -6., 29., c, rgb(60, 60, 64), chap).assis().bras(-0.2, -0.45);
        p.dessiner(g, xf);
        let (mx, my) = p.main(1.);
        for k in 0..3 {
            g.poly(xf, &rect(mx - 1. + k as f32 * 1.2, my - 3.5, 2., 3.), Some(BLANC), Some(GRIS));
        }
    }
    g.poly(xf, &[(-30., -16.), (30., -16.), (34., -10.), (-34., -10.)], Some(rgb(40, 100, 60)), None);
    g.line(xf, -28., -10., -28., -3., 1.0, SEPIA);
    g.line(xf, 28., -10., 28., -3., 1.0, SEPIA);
    for (x, k) in [(-6., 0.0), (-2., 1.0), (2., 2.0)] {
        g.poly(xf, &rect(x, -15.5 + k * 0.2, 2.6, 1.6), Some(BLANC), Some(GRIS));
    }
    g.poly(xf, &rect(12., -21., 2., 5.), Some(rgb(70, 110, 70)), None);
    g.poly(xf, &rect(16., -18., 1.8, 2.), Some(alpha(rgb(200, 160, 60), 0.8)), None);
}

fn quilles(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(150, 190, 230), rgb(240, 236, 214));
    g.poly(xf, &rect(-44., -48., 24., 30.), Some(rgb(220, 200, 160)), None);
    g.poly(xf, &rect(-44., -52., 26., 5.), Some(rgb(140, 70, 50)), None);
    arbre(g, xf, 30., -22., 40., FEUILLAGE);
    sol(g, xf, -22., HERBE, HERBE_FONCEE);
    g.poly(xf, &[(-6., -22.), (14., -22.), (34., 0.), (-26., 0.)], Some(rgb(214, 196, 150)), None);
    for (k, (x, y)) in [(0., -22.), (3., -22.), (6., -22.), (1.5, -20.), (4.5, -20.), (3., -18.)].iter().enumerate() {
        let x = x + 0.0;
        g.ellipse(xf, x, y - 2., 0.9, 2.2, Some(BLANC), Some(GRIS));
        rond(g, xf, x, y - 4.6, 0.7, if k % 2 == 0 { ROUGE } else { BLANC });
    }
    let p = Perso::homme(-6., -4., 29., rgb(230, 226, 214), rgb(70, 80, 110), Chapeau::Paille).bras(0.4, 0.9);
    p.dessiner(g, xf);
    rond(g, xf, 2., -6., 2.2, BOIS);
    Perso::homme(-30., -6., 27., rgb(110, 80, 60), rgb(70, 60, 50), Chapeau::Casquette).bras(0.2, 0.3).dessiner(g, xf);
    Perso::homme(24., -5., 28., rgb(60, 70, 90), rgb(90, 80, 70), Chapeau::Melon).bras(0.9, 0.3).dessiner(g, xf);
}

// ---------- 21 : la fête collective ----------
fn carnaval(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(120, 160, 220), rgb(240, 220, 200));
    maison(g, xf, -44., -18., 26., 26., rgb(226, 206, 170), rgb(150, 70, 50));
    maison(g, xf, 18., -18., 26., 28., rgb(214, 196, 166), rgb(110, 80, 70));
    guirlande(g, xf, -44., -54., 44., -54., 14);
    sol(g, xf, -18., rgb(200, 190, 170), rgb(160, 150, 130));
    let cols = [ROUGE, rgb(250, 220, 90), BLEU, VERT, ROSE, BLANC];
    for k in 0..40 {
        rond(g, xf, -42. + (k * 23 % 84) as f32, -60. + (k * 37 % 56) as f32, 0.55, cols[k % cols.len()]);
    }
    // arlequin
    let a = Perso::homme(-18., -5., 29., rgb(200, 60, 60), rgb(60, 140, 80), Chapeau::Pointu(rgb(250, 220, 90))).bras(2.5, 0.8);
    a.dessiner(g, xf);
    for (dx, dy) in [(-1.5, -20.), (1.5, -17.), (-1.5, -14.)] {
        g.poly(xf, &[(-18. + dx, dy - 1.5), (-16.5 + dx, dy), (-18. + dx, dy + 1.5), (-19.5 + dx, dy)], Some(rgb(250, 220, 90)), None);
    }
    // pierrot
    Perso { peau: rgb(250, 248, 244), ..Perso::homme(2., -5., 30., BLANC, BLANC, Chapeau::Aucun) }.bras(1.8, 1.8).cheveux(NOIR).dessiner(g, xf);
    for y in [-26., -22., -18.] {
        rond(g, xf, 2., y, 0.9, NOIR);
    }
    // dame masquée
    let d = Perso::femme(22., -5., 28., rgb(110, 60, 140), rgb(80, 40, 110), Chapeau::Capeline(NOIR)).bras(0.4, 2.0);
    d.dessiner(g, xf);
    g.poly(xf, &rect(19.5, -31.5, 5., 1.6), Some(NOIR), None);
}

fn parade(g: &Gfx, xf: &Xf) {
    ciel(g, xf, rgb(150, 186, 226), rgb(240, 236, 220));
    maison(g, xf, -44., -24., 28., 22., rgb(226, 210, 176), rgb(140, 70, 50));
    maison(g, xf, -14., -24., 24., 26., rgb(214, 196, 166), rgb(110, 80, 70));
    maison(g, xf, 14., -24., 30., 22., rgb(230, 216, 184), rgb(150, 76, 54));
    foule(g, xf, -22., -42., 42.);
    sol(g, xf, -20., rgb(196, 186, 166), rgb(160, 150, 130));
    drapeau(g, xf, -30., -6., 32., &[BLEU, BLANC, ROUGE]);
    for k in 0..4 {
        let x = -22. + k as f32 * 15.;
        let p = Perso::homme(x, -4., 28., BLEU, ROUGE, Chapeau::Kepi).bras(0.1, 0.5);
        p.dessiner(g, xf);
        let (mx, my) = p.main(1.);
        g.line(xf, mx - 1., my + 3., mx + 1.5, my - 14., 0.7, rgb(60, 50, 44));
        g.line(xf, mx + 1.5, my - 14., mx + 1.8, my - 17., 0.35, GRIS);
    }
    let t = Perso::homme(36., -4., 26., BLEU, ROUGE, Chapeau::Kepi).bras(1.2, 1.2);
    t.dessiner(g, xf);
    g.poly(xf, &rect(32., -16., 8., 5.), Some(rgb(230, 220, 200)), Some(ROUGE));
    g.ellipse(xf, 36., -16., 4., 1., Some(BLANC), Some(GRIS));
}

// ---------- l'Excuse ----------
/// Le fou à la mandoline, sur toute la carte (y de -63 à 63).
pub fn excuse(g: &Gfx, xf: &Xf) {
    degrade(g, xf, &rect(-44., -64., 88., 128.), -64., 64., rgb(60, 50, 90), rgb(150, 70, 60));
    for k in 0..14 {
        let x = -40. + (k * 29 % 80) as f32;
        let y = -60. + (k * 41 % 70) as f32;
        g.ellipse(xf, x, y, 0.6, 0.6, Some(rgb(255, 240, 190)), None);
    }
    g.lueur(xf, (0., 10.), 50., &[(0.0, alpha(rgb(255, 220, 150), 0.45)), (1.0, alpha(rgb(255, 220, 150), 0.0))]);
    degrade(g, xf, &rect(-44., 44., 88., 20.), 44., 64., rgb(120, 80, 50), rgb(70, 46, 28));
    let f = Perso { haut: rgb(200, 50, 40), bas: rgb(240, 196, 60), ..Perso::homme(0., 54., 96., rgb(200, 50, 40), rgb(240, 196, 60), Chapeau::Pointu(rgb(60, 120, 80))) }.bras(0.8, -0.3);
    f.dessiner(g, xf);
    for (x, y) in [(-6., -14.), (6., -14.), (0., -6.), (-6., 2.), (6., 2.)] {
        g.poly(xf, &[(x, y - 3.5), (x + 2.5, y), (x, y + 3.5), (x - 2.5, y)], Some(rgb(240, 196, 60)), None);
    }
    // collerette et grelots
    for k in 0..7 {
        let a = PI * (0.15 + 0.7 * k as f32 / 6.);
        rond(g, xf, -9. * a.cos(), -24. + 3. * a.sin(), 2.2, if k % 2 == 0 { BLANC } else { rgb(250, 220, 90) });
    }
    for (x, y) in [(-10., 2.), (10., 2.), (0., 12.)] {
        rond(g, xf, x, y, 1.6, rgb(250, 220, 90));
    }
    let (gx, gy) = f.main(-1.);
    let (dx, dy) = f.main(1.);
    mandoline_entre(g, xf, (dx + 3., dy + 1.), (gx, gy), 1.6);
    rond(g, xf, gx, gy, 3.4, PEAU);
    rond(g, xf, dx, dy, 3.4, PEAU);
}
