//! Banc d'essai sans fenêtre : des parties entières jouées par les bots.
//! `rami -sim <parties> <graine> <joueurs> <niveau du siège 0> <niveau des autres> <journal 0/1> [limite] [passifs 0/1]`
//! (passifs : les bots ne posent jamais rien, pour user le talon jusqu'aux manches nulles)
//! Affiche les statistiques ; avec le journal, écrit `sim.log` (vérifié par `check.py`).

use crate::engine::*;
use std::io::Write;
use std::time::Instant;

#[derive(Default)]
pub struct Stats {
    pub manches: u32,
    pub nulles: u32,
    pub ramis: u32,
    pub tours: u64,
    /// manches gagnées par siège, parties gagnées par siège
    pub manches_gagnees: [u32; 5],
    pub parties_gagnees: [u32; 5],
    pub penalites: [i64; 5],
    /// tour d'ouverture (somme, nombre)
    pub ouverture: (u64, u64),
    pub jamais_ouvert: u64,
    pub decisions: u64,
    pub temps_max: f64,
}

/// Joue une partie complète (bots partout).
pub fn partie(m: &mut Moteur, s: &mut Stats, passifs: bool) {
    m.nouvelle_partie();
    let mut ouvert_au = [None::<u32>; 5];
    loop {
        match m.d.phase {
            PH_PIOCHE => {
                let p = m.joueur();
                let t0 = Instant::now();
                let talon = passifs || m.bot_pioche(p);
                assert!(m.piocher(talon), "pioche refusée");
                let plan = if passifs {
                    let c = m.clone().choisir_defausse(p, 1);
                    vec![crate::bots::Action::Defausser(c)]
                } else {
                    m.bot_jeu(p)
                };
                s.decisions += 1;
                s.temps_max = s.temps_max.max(t0.elapsed().as_secs_f64() * 1000.0);
                let tour = m.d.tours;
                for a in plan {
                    assert!(m.appliquer(a), "action refusée : {a:?}");
                }
                assert_ne!(m.d.phase, PH_JEU, "tour non fini");
                for q in 0..m.np {
                    if m.d.ouvert[q] && ouvert_au[q].is_none() {
                        ouvert_au[q] = Some(tour / m.np as u32);
                    }
                }
            }
            PH_FINMANCHE | PH_FINPARTIE => {
                let b = m.bilan.expect("bilan");
                s.manches += 1;
                s.tours += m.d.tours as u64;
                if b.gagnant < 0 {
                    s.nulles += 1;
                } else {
                    s.manches_gagnees[b.gagnant as usize] += 1;
                    s.ramis += b.rami as u32;
                }
                for q in 0..m.np {
                    s.penalites[q] += b.penalites[q] as i64;
                    match ouvert_au[q] {
                        Some(t) => {
                            s.ouverture.0 += t as u64;
                            s.ouverture.1 += 1;
                        }
                        None => s.jamais_ouvert += 1,
                    }
                }
                ouvert_au = [None; 5];
                if m.d.phase == PH_FINPARTIE || (passifs && m.manche >= 3) {
                    s.parties_gagnees[m.classement()[0]] += 1;
                    break;
                }
                m.manche_suivante();
            }
            _ => unreachable!(),
        }
    }
}

pub fn sim_main(a: &[String]) {
    let arg = |i: usize, defaut: i64| a.get(i).and_then(|s| s.parse::<i64>().ok()).unwrap_or(defaut);
    let mut m = Moteur::new();
    m.rng = ((arg(1, 1) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)) | 1;
    m.np = arg(2, 4).clamp(2, 5) as usize;
    m.niveaux = [arg(3, 1) as i8, arg(4, 1) as i8, arg(4, 1) as i8, arg(4, 1) as i8, arg(4, 1) as i8];
    m.journal_on = arg(5, 0) != 0;
    m.limite = arg(6, 200) as i32;
    let parties = arg(0, 10);
    // passifs : les manches sont toutes nulles, la partie ne finirait jamais (arrêt après 3 manches)
    let passifs = arg(7, 0) != 0;
    let mut s = Stats::default();
    let t0 = Instant::now();
    for _ in 0..parties {
        partie(&mut m, &mut s, passifs);
    }
    let dt = t0.elapsed().as_secs_f64();
    let np = m.np;
    println!("{parties} parties, {} manches ({} nulles) à {np} joueurs en {dt:.2} s, limite {}", s.manches, s.nulles, m.limite);
    println!(
        "tours par manche : {:.1} (par joueur : {:.1}) ; ramis : {} ; ouverture au tour {:.1} ; jamais ouvert : {:.0} %",
        s.tours as f64 / s.manches.max(1) as f64,
        s.tours as f64 / s.manches.max(1) as f64 / np as f64,
        s.ramis,
        s.ouverture.0 as f64 / s.ouverture.1.max(1) as f64,
        100.0 * s.jamais_ouvert as f64 / (s.manches as f64 * np as f64).max(1.0)
    );
    let l = |v: &[u32]| v[..np].iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ");
    println!("manches gagnées par siège : {} ; parties : {}", l(&s.manches_gagnees), l(&s.parties_gagnees));
    let pen: Vec<String> = (0..np).map(|q| format!("{:.1}", s.penalites[q] as f64 / s.manches.max(1) as f64)).collect();
    println!("pénalité moyenne par manche et par siège : {}", pen.join("  "));
    println!("temps de décision des bots : {:.1} ms au plus ({} tours)", s.temps_max, s.decisions);
    if m.journal_on {
        std::fs::File::create("sim.log").and_then(|mut f| f.write_all(&m.journal)).expect("sim.log");
    }
}
