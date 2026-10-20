//! Banc d'essai sans fenêtre : des parties entières jouées par les bots.
//! `tarot -sim <parties> <graine> <joueurs> <niveau du siège 0> <niveau des autres> <journal 0/1> [tirages]`
//! Affiche les statistiques ; avec le journal, écrit `sim.log` (vérifié par `check.py`).

use crate::engine::*;
use std::io::Write;
use std::time::Instant;

/// Joue une partie complète (bots partout) ; renvoie le nombre de donnes jouées.
pub fn partie(m: &mut Moteur, stats: &mut Stats) -> u32 {
    m.nouvelle_partie();
    let mut donnes = 0;
    loop {
        let p = m.d.tour as usize;
        match m.d.phase {
            PH_ENCHERES => {
                if m.d.encheres.iter().all(|&e| e < 0) {
                    let v: Vec<i32> = (0..m.np).map(|q| m.evaluer(m.d.mains[q])).collect();
                    stats.evals.extend_from_slice(&v);
                    stats.max_evals.push(*v.iter().max().unwrap_or(&0));
                }
                let mut c = m.bot_enchere(p);
                if stats.force > 0 && c > 0 && (stats.force as u8) > m.enchere_max() {
                    c = stats.force as u8;
                }
                let avant = m.blanches;
                assert!(m.encherir(c), "enchère refusée");
                if m.blanches > avant {
                    stats.blanches += 1;
                }
                if m.d.preneur >= 0 && stats.v_preneur.is_none() {
                    stats.v_preneur = Some(m.evaluer(m.d.mains[m.d.preneur as usize] & !m.d.chien));
                }
            }
            PH_APPEL => {
                let c = m.bot_appel(p);
                assert!(m.appeler(c), "appel refusé");
            }
            PH_ECART => {
                let e = m.bot_ecart(p);
                assert!(m.ecarter(e), "écart refusé : {:?}", liste(e));
            }
            PH_JEU => {
                let t0 = Instant::now();
                let (c, pg) = m.bot_carte(p);
                stats.decisions += 1;
                stats.temps_max = stats.temps_max.max(t0.elapsed().as_secs_f64() * 1000.0);
                assert!(m.jouer(c, pg), "carte refusée : {c}");
            }
            PH_FINPLI => m.ramasser(),
            PH_FINDONNE | PH_FINPARTIE => {
                donnes += 1;
                if let Some(b) = m.bilan {
                    if let Some(v) = stats.v_preneur.take() {
                        let k = ((v.clamp(20, 99) - 20) / 4) as usize;
                        stats.par_valeur[b.contrat as usize][k].0 += 1;
                        stats.par_valeur[b.contrat as usize][k].1 += b.fait as u32;
                    }
                    stats.contrats[b.contrat as usize] += 1;
                    stats.faits += b.fait as u32;
                    stats.reussis[b.contrat as usize] += b.fait as u32;
                    stats.gain_preneur[b.contrat as usize] += b.gains[b.preneur as usize] as i64;
                    stats.petits_bout += (b.petit_bout != 0) as u32;
                    stats.poignees += (b.poignee != 0) as u32;
                    stats.chelems += (b.chelem != 0) as u32;
                    for q in 0..m.np {
                        stats.gains[q] += b.gains[q] as i64;
                    }
                    assert_eq!(b.gains[..m.np].iter().sum::<i32>(), 0, "gains non équilibrés");
                }
                if m.d.phase == PH_FINPARTIE {
                    break;
                }
                m.donne_suivante();
            }
            _ => unreachable!(),
        }
    }
    donnes
}

#[derive(Default)]
pub struct Stats {
    pub blanches: u32,
    /// contrat imposé à tous les preneurs (réglage des seuils), 0 = aucun
    pub force: i64,
    pub v_preneur: Option<i32>,
    /// (donnes, réussies) par contrat et par tranche de 4 points de valeur de main (à partir de 20)
    pub par_valeur: [[(u32, u32); 20]; 5],
    pub evals: Vec<i32>,
    pub max_evals: Vec<i32>,
    pub contrats: [u32; 5],
    pub reussis: [u32; 5],
    pub gain_preneur: [i64; 5],
    pub faits: u32,
    pub petits_bout: u32,
    pub poignees: u32,
    pub chelems: u32,
    pub gains: [i64; 5],
    pub decisions: u64,
    pub temps_max: f64,
}

pub fn sim_main(a: &[String]) {
    let arg = |i: usize, defaut: i64| a.get(i).and_then(|s| s.parse::<i64>().ok()).unwrap_or(defaut);
    let mut m = Moteur::new();
    m.rng = ((arg(1, 1) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)) | 1;
    m.np = arg(2, 4).clamp(3, 5) as usize;
    m.niveaux = [arg(3, 1) as i8, arg(4, 1) as i8, arg(4, 1) as i8, arg(4, 1) as i8, arg(4, 1) as i8];
    m.journal_on = arg(5, 0) != 0;
    m.mc_tirages = arg(6, m.mc_tirages as i64) as u32;
    for k in 0..4 {
        m.seuils[m.np - 3][k] = arg(7 + k, m.seuils[m.np - 3][k] as i64) as i32;
    }
    m.donnes = 10;
    let parties = arg(0, 10);
    let mut stats = Stats { force: arg(11, 0), ..Stats::default() };
    let t0 = Instant::now();
    let mut donnes = 0;
    for _ in 0..parties {
        donnes += partie(&mut m, &mut stats);
    }
    let dt = t0.elapsed().as_secs_f64();
    let prises: u32 = stats.contrats[1..].iter().sum();
    println!("{parties} parties, {donnes} donnes ({} blanches) à {} joueurs en {dt:.2} s", stats.blanches, m.np);
    println!(
        "contrats : petite {} garde {} sans {} contre {} ; réussis {:.0} % ; petit au bout {} ; poignées {} ; chelems {}",
        stats.contrats[1],
        stats.contrats[2],
        stats.contrats[3],
        stats.contrats[4],
        100.0 * stats.faits as f64 / prises.max(1) as f64,
        stats.petits_bout,
        stats.poignees,
        stats.chelems
    );
    for c in 1..5 {
        let n = stats.contrats[c].max(1) as f64;
        println!(
            "  {:12} {:5} donnes, réussies {:3.0} %, gain moyen du preneur {:+.0}",
            CONTRATS[c],
            stats.contrats[c],
            100.0 * stats.reussis[c] as f64 / n,
            stats.gain_preneur[c] as f64 / n
        );
    }
    let moy: Vec<String> = (0..m.np).map(|q| format!("{:+.1}", stats.gains[q] as f64 / donnes.max(1) as f64)).collect();
    println!("gain moyen par donne et par siège : {}", moy.join("  "));
    println!("temps de décision des bots : {:.1} ms au plus", stats.temps_max);
    for c in 1..5 {
        let lignes: Vec<String> = (0..20)
            .filter(|&k| stats.par_valeur[c][k].0 >= 20)
            .map(|k| format!("{}:{:.0}%", 20 + 4 * k, 100.0 * stats.par_valeur[c][k].1 as f64 / stats.par_valeur[c][k].0 as f64))
            .collect();
        if !lignes.is_empty() {
            println!("  réussite par valeur ({}) : {}", CONTRATS[c], lignes.join(" "));
        }
    }
    for (nom, v) in [("mains", &mut stats.evals), ("meilleure main de la donne", &mut stats.max_evals)] {
        v.sort();
        let q = |f: f64| v.get(((v.len() as f64 - 1.0) * f) as usize).copied().unwrap_or(0);
        println!("valeur des {nom} : 10 % {} | 50 % {} | 90 % {} | 99 % {}", q(0.1), q(0.5), q(0.9), q(0.99));
    }
    if m.journal_on {
        std::fs::File::create("sim.log").and_then(|mut f| f.write_all(&m.journal)).expect("sim.log");
    }
}
