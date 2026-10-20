//! Bots du tarot. Ils ne trichent pas : ils ne se servent que de leur main, des cartes jouées, des coupes
//! montrées, du chien quand il a été montré, de leur écart (preneur) et des atouts écartés (montrés).
//! - facile : le jeu « normal » avec une carte au hasard de temps en temps ;
//! - normal : règles d'expert simples (évaluation de la main, écart qui fait des coupes, fournir au plus juste,
//!   couper au plus juste, charger quand le camp est maître, sauver l'excuse, garder le petit pour la fin) ;
//! - difficile : Monte-Carlo (donnes plausibles tirées au sort selon ce qu'il sait, jouées jusqu'au bout).

use crate::engine::*;

/// Plus petite carte (en points puis en force) d'un ensemble non vide.
fn moins_chere(m: Cartes) -> u8 {
    liste(m).into_iter().min_by_key(|&c| (demi_points(c), force(c), c)).unwrap_or(0)
}

/// Carte qui rapporte le plus (points puis force).
fn plus_chere(m: Cartes) -> u8 {
    liste(m).into_iter().max_by_key(|&c| (demi_points(c), force(c), c)).unwrap_or(0)
}

/// Plus faible / plus forte carte d'une même famille (les numéros suivent la force).
fn plus_basse(m: Cartes) -> u8 {
    m.trailing_zeros() as u8
}

fn plus_haute(m: Cartes) -> u8 {
    (127 - m.leading_zeros()) as u8
}

/// Cartes d'une même famille plus fortes que c.
fn au_dessus(c: u8) -> Cartes {
    let f = famille(c);
    let fam = if f == 4 { ATOUTS } else { couleur(f) };
    fam & !((1u128 << (c + 1)) - 1)
}

impl Moteur {
    fn niveau(&self, p: usize) -> i8 {
        if self.niveaux[p] < 0 {
            1
        } else {
            self.niveaux[p]
        }
    }

    /// Cartes dont le joueur p ne connaît pas la place.
    pub fn inconnues(&self, p: usize) -> Cartes {
        let d = &self.d;
        let mut m = TOUTES & !d.mains[p] & !d.joues;
        if d.preneur == p as i8 {
            m &= !d.ecart;
        } else if d.contrat <= 2 {
            m &= !(d.ecart & ATOUTS); // atouts écartés : montrés
        }
        m
    }

    /// Le joueur p sait que q est dans son camp.
    pub fn allie(&self, p: usize, q: usize) -> bool {
        let d = &self.d;
        if p == q {
            return true;
        }
        let pr = d.preneur as usize;
        if p == pr {
            return d.revele && d.partenaire == q as i8;
        }
        if d.partenaire == p as i8 {
            return q == pr; // le partenaire sait qu'il l'est
        }
        q != pr && !(d.revele && d.partenaire == q as i8)
    }

    /// La carte c est la plus forte de sa famille parmi ce que p ne voit pas.
    fn maitresse(&self, c: u8, inc: Cartes) -> bool {
        c != EXCUSE && au_dessus(c) & inc == 0
    }

    /// Gagnant provisoire du pli en cours (joueur, carte).
    fn gagnant_provisoire(&self) -> (usize, u8) {
        let d = &self.d;
        let dem = self.famille_demandee();
        let mut best = (i32::MIN, d.entame as usize, d.pli[0]);
        for j in 0..d.n_pli as usize {
            let c = d.pli[j];
            let cle = match famille(c) {
                5 => -2,
                4 => 100 + force(c) as i32,
                f if Some(f) == dem => force(c) as i32,
                _ => -1,
            };
            if cle > best.0 {
                best = (cle, (d.entame as usize + j) % self.np, c);
            }
        }
        (best.1, best.2)
    }

    // ---------- enchères ----------
    /// Valeur d'une main (barème de joueur de club), ramenée à une main de 18 cartes.
    pub fn evaluer(&self, h: Cartes) -> i32 {
        let atouts = h & ATOUTS;
        let na = nb(atouts) as i32;
        let mut v = 2 * na;
        if h & bit(VINGT_ET_UN) != 0 {
            v += 10;
        }
        if h & bit(EXCUSE) != 0 {
            v += 8;
        }
        if h & bit(PETIT) != 0 {
            v += match na {
                0..=4 => 0,
                5 | 6 => 5,
                _ => 8,
            };
        }
        v += nb(atouts & !((1u128 << 71) - 1) & !bit(VINGT_ET_UN)) as i32; // gros atouts 16 à 20
        for s in 0..4 {
            let m = h & couleur(s);
            let n = nb(m) as i32;
            let roi = m & ROIS != 0;
            let dame = m & bit(s as u8 * 14 + 12) != 0;
            v += if roi { 6 } else { 0 } + if dame { 3 } else { 0 };
            v += if roi && dame { 1 } else { 0 };
            v += nb(m & bit(s as u8 * 14 + 11)) as i32 * 2 + nb(m & bit(s as u8 * 14 + 10)) as i32;
            if n >= 5 {
                v += 2 * (n - 4);
            }
            if na >= 4 {
                v += match n {
                    0 => 6,
                    1 if !roi => 3,
                    _ => 0,
                };
            }
        }
        v * 18 / self.taille_main() as i32
    }

    /// Contrat voulu par le bot p (0 = passe), plus fort que l'enchère en cours.
    pub fn bot_enchere(&mut self, p: usize) -> u8 {
        let mut v = self.evaluer(self.d.mains[p]);
        if self.niveau(p) == 0 {
            v += self.rng_mod(11) as i32 - 5;
        }
        let voulu = self.seuils[self.np - 3].iter().filter(|&&s| v >= s).count() as u8;
        if voulu > self.enchere_max() {
            voulu
        } else {
            0
        }
    }

    // ---------- appel ----------
    /// Carte appelée : dans la couleur où le preneur est le plus long (le partenaire y fera des levées).
    pub fn bot_appel(&mut self, p: usize) -> u8 {
        let possibles = self.appels_possibles();
        let h = self.d.mains[p];
        liste(possibles)
            .into_iter()
            .max_by_key(|&c| {
                let s = famille(c);
                (h & bit(c) == 0, nb(h & couleur(s)), demi_points_de(h & couleur(s)))
            })
            .unwrap_or(13)
    }

    // ---------- écart ----------
    /// Écart : vider les couleurs les plus courtes (coupes), puis mettre à l'abri les honneurs isolés.
    pub fn bot_ecart(&mut self, _p: usize) -> Cartes {
        let k = self.taille_chien();
        let possibles = self.ecart_possible();
        let normales = possibles & !ATOUTS;
        let mut ecart: Cartes = 0;
        // couleurs de la plus courte à la plus longue (en cartes écartables)
        let mut couleurs: Vec<usize> = (0..4).filter(|&s| normales & couleur(s) != 0).collect();
        couleurs.sort_by_key(|&s| (nb(normales & couleur(s)), -(demi_points_de(normales & couleur(s)))));
        for &s in &couleurs {
            let m = normales & couleur(s);
            if (nb(ecart) + nb(m)) as usize <= k {
                ecart |= m;
            }
        }
        // compléter avec les cartes qui rapportent le plus, dans les couleurs courtes
        while (nb(ecart) as usize) < k {
            let reste = normales & !ecart;
            if reste == 0 {
                break;
            }
            let c = liste(reste)
                .into_iter()
                .max_by_key(|&c| (demi_points(c), -(nb(normales & couleur(famille(c))) as i32), force(c)))
                .unwrap_or(0);
            ecart |= bit(c);
        }
        // sinon des atouts (les plus petits), comme la règle l'impose
        while (nb(ecart) as usize) < k {
            let reste = possibles & ATOUTS & !ecart;
            if reste == 0 {
                break;
            }
            ecart |= bit(plus_basse(reste));
        }
        ecart
    }

    // ---------- jeu ----------
    /// Poignée montrée par un bot : toujours celle qu'il peut montrer.
    pub fn bot_poignee(&self, p: usize) -> u8 {
        self.poignee_possible(p)
    }

    /// Carte jouée par le bot p et poignée montrée.
    pub fn bot_carte(&mut self, p: usize) -> (u8, u8) {
        let leg = self.legales(p);
        let poignee = self.bot_poignee(p);
        if nb(leg) == 1 {
            return (plus_basse(leg), poignee);
        }
        let c = match self.niveau(p) {
            2 if !self.simule => self.mc_carte(p, leg),
            0 if self.rng_mod(4) == 0 => {
                let v = liste(leg);
                v[self.rng_mod(v.len() as u32) as usize]
            }
            _ => self.heuristique(p, leg),
        };
        (c, poignee)
    }

    /// Jeu « normal » : la carte jouée par p parmi ses cartes légales.
    pub fn heuristique(&self, p: usize, leg: Cartes) -> u8 {
        let h = self.d.mains[p];
        // l'excuse ne doit pas attendre le dernier pli (elle y serait perdue)
        if leg & bit(EXCUSE) != 0 && nb(h) == 2 {
            return EXCUSE;
        }
        let inc = self.inconnues(p);
        if self.d.n_pli == 0 {
            self.entame(p, leg, inc)
        } else {
            self.suivre(p, leg, inc)
        }
    }

    /// Un adversaire de p (vu de p) pourrait encore couper la famille s.
    fn coupe_possible(&self, p: usize, s: usize, inc: Cartes) -> bool {
        inc & ATOUTS != 0 && (0..self.np).any(|q| q != p && !self.allie(p, q) && self.d.vides[q] >> s & 1 == 1 && self.d.vides[q] >> 4 & 1 == 0)
    }

    fn entame(&self, p: usize, leg: Cartes, inc: Cartes) -> u8 {
        let d = &self.d;
        let np = self.np;
        let pr = d.preneur as usize;
        let sans_exc = leg & !bit(EXCUSE);
        if sans_exc == 0 {
            return EXCUSE;
        }
        let atouts = sans_exc & ATOUTS;
        let dehors = nb(inc & ATOUTS);
        // petit au bout : plus aucun atout dehors, on joue nos autres atouts maîtres et on garde le petit
        if atouts & bit(PETIT) != 0 && dehors == 0 {
            let autres = atouts & !bit(PETIT);
            if autres != 0 {
                return plus_haute(autres);
            }
            let reste = sans_exc & !bit(PETIT);
            if reste != 0 {
                return self.petite_entame(p, reste, inc);
            }
            return PETIT;
        }
        let attaquant = p == pr || d.partenaire == p as i8;
        if attaquant {
            // 5 joueurs : le preneur cherche son partenaire dans la couleur appelée
            if p == pr && np == 5 && !d.revele && d.appel >= 0 && d.plis > 0 {
                let m = sans_exc & couleur(famille(d.appel as u8)) & !bit(d.appel as u8);
                if m != 0 {
                    return plus_basse(m);
                }
            }
            // faire tomber les atouts quand on en a au moins autant que tous les autres réunis
            let sans_petit = atouts & !bit(PETIT);
            if p == pr && sans_petit != 0 && nb(atouts) >= dehors.max(1) {
                let haute = plus_haute(sans_petit);
                return if self.maitresse(haute, inc) { haute } else { plus_basse(sans_petit) };
            }
        }
        // rois maîtres d'une couleur que personne n'a encore coupée
        for s in 0..4 {
            let roi = s as u8 * 14 + 13;
            if sans_exc & bit(roi) != 0 && !self.coupe_possible(p, s, inc) && nb(d.joues & couleur(s)) < 4 {
                if np == 5 && !attaquant && !d.revele && d.appel == roi as i8 {
                    continue;
                }
                return roi;
            }
        }
        // défense : faire couper le preneur (lui faire user ses atouts)
        if !attaquant && d.vides[pr] & 0x10 == 0 {
            for s in 0..4 {
                let m = sans_exc & couleur(s) & !ROIS;
                if d.vides[pr] >> s & 1 == 1 && m != 0 {
                    return plus_basse(m);
                }
            }
        }
        self.petite_entame(p, sans_exc, inc)
    }

    /// Entame sans risque : petite carte de la couleur la plus longue, sinon le plus petit atout (pas le petit).
    fn petite_entame(&self, p: usize, m: Cartes, _inc: Cartes) -> u8 {
        let d = &self.d;
        let attaquant = p as i8 == d.preneur || d.partenaire == p as i8;
        let evite = if self.np == 5 && !attaquant && !d.revele && d.appel >= 0 { couleur(famille(d.appel as u8)) } else { 0 };
        let mut couleurs: Vec<usize> = (0..4).filter(|&s| m & couleur(s) & !ROIS != 0).collect();
        couleurs.sort_by_key(|&s| (m & couleur(s) & evite != 0, -(nb(m & couleur(s)) as i32)));
        if let Some(&s) = couleurs.first() {
            return plus_basse(m & couleur(s) & !ROIS);
        }
        let atouts = m & ATOUTS & !bit(PETIT);
        if atouts != 0 {
            return plus_basse(atouts);
        }
        moins_chere(m)
    }

    fn suivre(&self, p: usize, leg: Cartes, inc: Cartes) -> u8 {
        let d = &self.d;
        let np = self.np;
        let dem = self.famille_demandee();
        let (w, wc) = self.gagnant_provisoire();
        let allie_gagne = self.allie(p, w);
        let apres: Vec<usize> = (d.n_pli as usize + 1..np).map(|j| (d.entame as usize + j) % np).collect();
        let adv_apres: Vec<usize> = apres.iter().copied().filter(|&q| !self.allie(p, q)).collect();
        let sans_exc = leg & !bit(EXCUSE);
        if sans_exc == 0 {
            return EXCUSE;
        }
        let pli_pts: i32 = d.pli[..d.n_pli as usize].iter().map(|&c| demi_points(c)).sum();
        // le pli est-il assuré à notre camp ?
        let sur = allie_gagne
            && (adv_apres.is_empty()
                || if famille(wc) == 4 {
                    self.maitresse(wc, inc) || adv_apres.iter().all(|&q| d.vides[q] >> 4 & 1 == 1)
                } else {
                    self.atout_max() == 0
                        && self.maitresse(wc, inc)
                        && !dem.is_some_and(|s| self.coupe_possible(p, s, inc))
                        && dem.is_some_and(|s| nb(inc & couleur(s)) as usize >= 2 * adv_apres.len())
                });
        // l'excuse pour ne pas donner une grosse carte à l'adversaire
        if leg & bit(EXCUSE) != 0 && !allie_gagne && demi_points(moins_chere(sans_exc)) >= 5 {
            return EXCUSE;
        }
        // fournir la couleur demandée
        if let Some(s) = dem.filter(|&s| s < 4) {
            let m = sans_exc & couleur(s);
            if m != 0 {
                if sur {
                    return plus_chere(m);
                }
                if self.atout_max() == 0 && famille(wc) == s {
                    let gagnantes = m & au_dessus(wc);
                    if gagnantes != 0 {
                        if adv_apres.is_empty() {
                            return plus_basse(gagnantes);
                        }
                        let haute = plus_haute(gagnantes);
                        if self.maitresse(haute, inc) && !self.coupe_possible(p, s, inc) {
                            return haute;
                        }
                    }
                }
                return plus_basse(m);
            }
        }
        // couper, ou monter à l'atout
        let at = sans_exc & ATOUTS;
        if at != 0 {
            let adv_atouts = adv_apres.iter().any(|&q| d.vides[q] >> 4 & 1 == 0) && inc & ATOUTS != 0;
            if at & bit(PETIT) != 0 && self.atout_max() == 0 && !adv_atouts && !allie_gagne {
                return PETIT; // le petit passe : personne ne peut le prendre
            }
            let choix = if at & !bit(PETIT) != 0 { at & !bit(PETIT) } else { at };
            if sur || (allie_gagne && pli_pts < 6) {
                return plus_basse(choix);
            }
            let dessus = choix & !((1u128 << (56 + self.atout_max())) - 1);
            if dessus != 0 {
                // un pli qui vaut cher et d'autres adversaires derrière : monter franchement
                if pli_pts >= 9 && adv_atouts {
                    let haute = plus_haute(dessus);
                    if self.maitresse(haute, inc) || haute == VINGT_ET_UN {
                        return haute;
                    }
                }
                return plus_basse(dessus);
            }
            return plus_basse(choix);
        }
        // défausse : charger si le pli est à nous, sinon la plus petite carte de la couleur la plus courte
        if sur {
            return plus_chere(sans_exc);
        }
        let petites = sans_exc & !ROIS;
        let m = if petites != 0 { petites } else { sans_exc };
        liste(m)
            .into_iter()
            .min_by_key(|&c| (demi_points(c), if c < 56 { nb(d.mains[p] & couleur(famille(c))) } else { 99 }, force(c)))
            .unwrap_or_else(|| moins_chere(sans_exc))
    }

    // ---------- Monte-Carlo ----------
    /// Donne plausible vue de p : les cartes qu'il ne voit pas sont réparties au hasard entre les autres mains (et
    /// l'écart ou le chien cachés), en respectant les coupes montrées, les cartes du chien montré (chez le
    /// preneur) et les règles de l'écart.
    pub fn distribuer_inconnues(&mut self, p: usize) {
        let np = self.np;
        let k = self.taille_chien();
        let d = self.d;
        let pr = d.preneur;
        let inc = self.inconnues(p);
        let chien_montre = if d.contrat <= 2 && pr >= 0 && pr as usize != p { d.chien & inc } else { 0 };
        // places : mains des autres joueurs, écart caché (vu des autres), chien caché (gardes sans et contre)
        let mut places: Vec<(i8, usize, Cartes)> = Vec::with_capacity(7);
        for q in 0..np {
            if q == p {
                continue;
            }
            let mut permis = TOUTES;
            for s in 0..4 {
                if d.vides[q] >> s & 1 == 1 {
                    permis &= !couleur(s);
                }
            }
            if d.vides[q] >> 4 & 1 == 1 {
                permis &= !ATOUTS;
            }
            if pr != q as i8 {
                permis &= !chien_montre;
            }
            places.push((q as i8, nb(d.mains[q]) as usize, permis));
        }
        if d.contrat <= 2 && pr >= 0 && pr as usize != p && d.phase >= PH_JEU {
            places.push((-1, k - nb(d.ecart & ATOUTS) as usize, TOUTES & !ROIS & !BOUTS & !ATOUTS));
        }
        if d.contrat >= 3 {
            places.push((-2, k, TOUTES & !chien_montre));
        }
        let mut cartes = liste(inc);
        for i in (1..cartes.len()).rev() {
            let j = self.rng_mod(i as u32 + 1) as usize;
            cartes.swap(i, j);
        }
        // les cartes les plus contraintes d'abord
        cartes.sort_by_key(|&c| places.iter().filter(|pl| pl.2 & bit(c) != 0).count());
        let mut libre: Vec<usize> = places.iter().map(|pl| pl.1).collect();
        let mut recu: Vec<Cartes> = vec![0; places.len()];
        for &c in &cartes {
            let total: usize = (0..places.len()).filter(|&i| libre[i] > 0 && places[i].2 & bit(c) != 0).map(|i| libre[i]).sum();
            let i = if total > 0 {
                let mut r = self.rng_mod(total as u32) as usize;
                let mut choix = 0;
                for i in 0..places.len() {
                    if libre[i] > 0 && places[i].2 & bit(c) != 0 {
                        if r < libre[i] {
                            choix = i;
                            break;
                        }
                        r -= libre[i];
                    }
                }
                choix
            } else {
                // impasse (rare) : n'importe quelle place libre
                match (0..places.len()).find(|&i| libre[i] > 0) {
                    Some(i) => i,
                    None => break,
                }
            };
            libre[i] -= 1;
            recu[i] |= bit(c);
        }
        for (i, pl) in places.iter().enumerate() {
            match pl.0 {
                -1 => self.d.ecart = recu[i] | (d.ecart & ATOUTS),
                -2 => self.d.chien = recu[i],
                q => self.d.mains[q as usize] = recu[i],
            }
        }
        // partenaire de cette donne tirée au sort
        if np == 5 && d.appel >= 0 && !d.revele && d.partenaire != p as i8 && pr != p as i8 {
            let c = d.appel as u8;
            self.d.partenaire = (0..np).find(|&q| q as i8 != pr && self.d.mains[q] & bit(c) != 0).map_or(-1, |q| q as i8);
        } else if np == 5 && d.appel >= 0 && !d.revele && pr == p as i8 {
            let c = d.appel as u8;
            self.d.partenaire = (0..np).find(|&q| q != p && self.d.mains[q] & bit(c) != 0).map_or(-1, |q| q as i8);
        }
    }

    /// Joue la donne jusqu'au bout avec le jeu « normal » pour tout le monde.
    pub fn derouler(&mut self) {
        loop {
            match self.d.phase {
                PH_JEU => {
                    let q = self.d.tour as usize;
                    let leg = self.legales(q);
                    let c = if nb(leg) == 1 { plus_basse(leg) } else { self.heuristique(q, leg) };
                    self.jouer(c, 0);
                }
                PH_FINPLI => self.ramasser(),
                _ => break,
            }
        }
    }

    /// Bot difficile : chaque carte légale est essayée sur des donnes plausibles jouées jusqu'au bout ; on garde
    /// celle qui rapporte le plus en moyenne.
    pub fn mc_carte(&mut self, p: usize, leg: Cartes) -> u8 {
        let cands = liste(leg);
        let d0 = self.d;
        let sim0 = self.simule;
        self.simule = true;
        let mut sommes = vec![0i64; cands.len()];
        for _ in 0..self.mc_tirages {
            self.d = d0;
            self.distribuer_inconnues(p);
            let d1 = self.d;
            for (k, &c) in cands.iter().enumerate() {
                self.d = d1;
                if self.jouer(c, 0) {
                    self.derouler();
                    sommes[k] += self.calculer_bilan().gains[p] as i64;
                }
            }
        }
        self.d = d0;
        self.simule = sim0;
        let best = (0..cands.len()).max_by_key(|&k| (sommes[k], -(k as i64))).unwrap_or(0);
        cands[best]
    }
}
