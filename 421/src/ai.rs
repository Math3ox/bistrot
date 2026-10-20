//! Règles (rang d'une main) et niveaux du bot.
//!
//! - facile    : stratégie « normale » mais 35 % de décisions au hasard
//! - normal    : garde les as, un 2 et un 4 ; s'arrête dès qu'il bat le 1er joueur
//! - difficile : stratégie optimale (utilité en jetons) par programmation dynamique sur les 56 mains
//!   triées. Premier joueur : tables W/VS calculées au démarrage ; second joueur : calcul exact
//!   contre la combinaison à battre. Même calcul que `gen_tables.py` (référence en Python).

use std::sync::OnceLock;

pub const LEVEL_NAMES: [&str; 3] = ["facile", "normal", "difficile"];

pub const CAT_NAMES: [&str; 7] =
    ["421 !!!", "Trois as (111)", "Deux as", "Brelan", "Suite", "Nénette (221)", "Rien"];

/// Rang, jetons et catégorie (index de `CAT_NAMES`) de trois dés.
pub fn rank3(d0: u32, d1: u32, d2: u32) -> (u32, u32, usize) {
    let mut v = [d0, d1, d2];
    v.sort_unstable_by(|a, b| b.cmp(a));
    let [a, b, c] = v;
    if (a, b, c) == (4, 2, 1) {
        (1000, 8, 0)
    } else if a == 1 {
        (900, 7, 1)
    } else if a == c {
        (800 + 2 * a, a, 3)
    } else if b == 1 && c == 1 {
        (801 + 2 * a, a, 2)
    } else if a == b + 1 && b == c + 1 {
        (700 + a, 2, 4)
    } else if (a, b, c) == (2, 2, 1) {
        (1, 1, 5)
    } else {
        (a * 100 + b * 10 + c, 1, 6)
    }
}

struct Tables {
    hands: Vec<[u32; 3]>,
    sid: [u8; 216],
    rk: Vec<u32>,
    tok: Vec<u32>,
    /// out[i][m] : mains obtenues en relançant les dés du masque m depuis la main i
    out: Vec<[Vec<u8>; 8]>,
    /// w[t] : valeur de s'arrêter après t lancers (premier joueur), t = 1..3
    w: [Vec<f64>; 4],
    /// vs2 : valeur de continuer après 2 lancers... (VS[2] de gen_tables.py)
    vs2: Vec<f64>,
}

fn sid_of(t: &Tables, h: [u32; 3]) -> usize {
    t.sid[((h[0] - 1) * 36 + (h[1] - 1) * 6 + (h[2] - 1)) as usize] as usize
}

fn outcomes(t: &Tables, hand: [u32; 3], mask: u32) -> Vec<u8> {
    let n = (0..3).filter(|i| mask >> i & 1 == 1).count() as u32;
    let mut res = Vec::with_capacity(6usize.pow(n));
    for k in 0..6u32.pow(n) {
        let mut h = hand;
        let mut r = k;
        for (i, v) in h.iter_mut().enumerate() {
            if mask >> i & 1 == 1 {
                *v = r % 6 + 1;
                r /= 6;
            }
        }
        res.push(sid_of(t, h) as u8);
    }
    res
}

fn mean(o: &[u8], v: &[f64]) -> f64 {
    o.iter().map(|&j| v[j as usize]).sum::<f64>() / o.len() as f64
}

/// Valeurs du second joueur contre une cible : vs[k][i] = valeur de la main i avec k relances restantes.
fn follower_v(t: &Tables, target: u32, target_tok: u32, kmax: usize) -> Vec<Vec<f64>> {
    let v0: Vec<f64> = (0..56)
        .map(|i| {
            if t.rk[i] > target {
                t.tok[i] as f64
            } else if t.rk[i] == target {
                0.0
            } else {
                -(target_tok as f64)
            }
        })
        .collect();
    let mut vs = vec![v0.clone()];
    for _ in 1..=kmax {
        let prev = vs.last().unwrap();
        let cur: Vec<f64> = (0..56)
            .map(|i| (1..8).map(|m| mean(&t.out[i][m], prev)).fold(v0[i], f64::max))
            .collect();
        vs.push(cur);
    }
    vs
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut hands = Vec::new();
        for a in 1..=6 {
            for b in a..=6 {
                for c in b..=6 {
                    hands.push([a, b, c]);
                }
            }
        }
        let mut sid = [0u8; 216];
        for a in 1..=6u32 {
            for b in 1..=6u32 {
                for c in 1..=6u32 {
                    let mut s = [a, b, c];
                    s.sort_unstable();
                    sid[((a - 1) * 36 + (b - 1) * 6 + (c - 1)) as usize] =
                        hands.iter().position(|h| *h == s).unwrap() as u8;
                }
            }
        }
        let rk = hands.iter().map(|h| rank3(h[0], h[1], h[2]).0).collect();
        let tok = hands.iter().map(|h| rank3(h[0], h[1], h[2]).1).collect();
        let mut t = Tables { hands, sid, rk, tok, out: Vec::new(), w: Default::default(), vs2: Vec::new() };
        t.out = (0..56)
            .map(|i| std::array::from_fn(|m| if m == 0 { vec![i as u8] } else { outcomes(&t, t.hands[i], m as u32) }))
            .collect();
        let all216: Vec<u8> = t.sid.to_vec();
        for th in 1..=3usize {
            t.w[th] = (0..56)
                .map(|i| {
                    let vs = follower_v(&t, t.rk[i], t.tok[i], th - 1);
                    -mean(&all216, &vs[th - 1])
                })
                .collect();
        }
        t.vs2 = (0..56)
            .map(|i| (1..8).map(|m| mean(&t.out[i][m], &t.w[3])).fold(t.w[2][i], f64::max))
            .collect();
        t
    })
}

/// Stratégie optimale. `hand` = les 3 dés, `throws` = lancers déjà faits, `lim` = lancers permis,
/// `target` = rang à battre (0 = on joue en premier). Renvoie le masque des dés à relancer (0 = s'arrêter).
pub fn hard_choose(hand: [u32; 3], throws: u32, lim: u32, target: u32, target_tok: u32) -> u32 {
    if throws >= lim {
        return 0;
    }
    let t = tables();
    let i = sid_of(t, hand);
    let (mut best, nxt): (f64, Vec<f64>) = if target == 0 {
        let nxt = if throws == 1 { t.vs2.clone() } else { t.w[3].clone() };
        (t.w[throws.clamp(1, 3) as usize][i], nxt)
    } else {
        let l = (lim - throws) as usize;
        let vs = follower_v(t, target, target_tok, l);
        (vs[0][i], vs[l - 1].clone())
    };
    let mut bm = 0;
    for m in 1..8 {
        let e = mean(&outcomes(t, hand, m), &nxt);
        if e > best + 1e-6 {
            best = e;
            bm = m;
        }
    }
    bm
}

/// Bot normal : garde les as, un 2 et un 4. Renvoie le masque (0 = s'arrêter).
pub fn normal_choose(hand: [u32; 3], target: u32) -> u32 {
    let r = rank3(hand[0], hand[1], hand[2]).0;
    if r >= 700 || (target != 0 && r > target) {
        return 0;
    }
    let (mut mask, mut have2, mut have4) = (0, false, false);
    for (i, &v) in hand.iter().enumerate() {
        let keep = match v {
            1 => true,
            2 if !have2 => {
                have2 = true;
                true
            }
            4 if !have4 => {
                have4 = true;
                true
            }
            _ => false,
        };
        if !keep {
            mask |= 1 << i;
        }
    }
    mask
}

/// Probabilités pour une sélection : (nombre de 421, nombre de mains meilleures que l'actuelle, total).
pub fn stats(hand: [u32; 3], mask: u32) -> (u32, u32, u32) {
    let cur = rank3(hand[0], hand[1], hand[2]).0;
    let n = (0..3).filter(|i| mask >> i & 1 == 1).count() as u32;
    let total = 6u32.pow(n);
    let (mut n421, mut better) = (0, 0);
    for k in 0..total {
        let mut h = hand;
        let mut r = k;
        for (i, v) in h.iter_mut().enumerate() {
            if mask >> i & 1 == 1 {
                *v = r % 6 + 1;
                r /= 6;
            }
        }
        let rk = rank3(h[0], h[1], h[2]).0;
        n421 += (rk == 1000) as u32;
        better += (rk > cur) as u32;
    }
    (n421, better, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rangs() {
        assert_eq!(rank3(1, 2, 4), (1000, 8, 0));
        assert_eq!(rank3(1, 1, 1), (900, 7, 1));
        assert_eq!(rank3(6, 6, 6), (812, 6, 3));
        assert_eq!(rank3(1, 6, 1), (813, 6, 2));
        assert_eq!(rank3(3, 4, 5), (705, 2, 4));
        assert_eq!(rank3(2, 1, 2), (1, 1, 5));
        assert_eq!(rank3(6, 5, 1), (651, 1, 6));
    }

    #[test]
    fn bot_normal() {
        assert_eq!(normal_choose([1, 2, 6], 0), 4);
        assert_eq!(normal_choose([2, 2, 4], 0), 2);
        assert_eq!(normal_choose([4, 2, 1], 0), 0);
        assert_eq!(normal_choose([6, 5, 3], 643), 0);
    }

    /// Décisions du bot difficile pour 26 situations x 216 mains -> 421_bot.bin (comparées par bottest.py).
    #[test]
    #[ignore]
    fn bot_dump() {
        let mut scen = vec![(0, 0, 1, 3), (0, 0, 2, 3)];
        for (tr, tk) in [(1000, 8), (900, 7), (813, 6), (806, 3), (706, 2), (665, 1), (541, 1), (1, 1)] {
            for (thr, lim) in [(1, 2), (1, 3), (2, 3)] {
                scen.push((tr, tk, thr, lim));
            }
        }
        let mut out = Vec::new();
        for (tr, tk, thr, lim) in scen {
            for n in 0..216u32 {
                let hand = [n / 36 + 1, n / 6 % 6 + 1, n % 6 + 1];
                out.push(hard_choose(hand, thr, lim, tr, tk) as u8);
            }
        }
        std::fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/421_bot.bin"), out).unwrap();
    }

    #[test]
    fn difficile_garde_421() {
        assert_eq!(hard_choose([4, 2, 1], 1, 3, 0, 0), 0);
        assert_eq!(hard_choose([4, 2, 1], 1, 3, 541, 1), 0);
    }
}
