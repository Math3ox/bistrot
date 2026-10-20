//! Mode terminal (`421 -t`) : même moteur que la fenêtre, affichage texte dans la console.
//! Entrée = lancer (ou valider sa main), chiffres des dés à relancer (ex. `13`), `q` = quitter.

use crate::game::{Game, Mode, Phase};
use std::io::{BufRead, Write};

const FACES: [&str; 7] = [" ", "⚀", "⚁", "⚂", "⚃", "⚄", "⚅"];

fn show_dice(g: &Game) {
    for c in 0..2 {
        let held: Vec<String> = (0..3)
            .map(|i| {
                let d = &g.dice[c * 3 + i];
                let mark = if d.held { "*" } else { " " };
                format!("[{}] {} {}{}", i + 1, FACES[d.val as usize], d.val, mark)
            })
            .collect();
        println!("  {:<4} {}", g.label(c), held.join("   "));
    }
}

/// Avance la partie jusqu'à ce que le joueur local doive agir, en affichant les messages.
fn settle(g: &mut Game, last: &mut (String, String)) {
    loop {
        if (g.line1.clone(), g.line2.clone()) != *last {
            *last = (g.line1.clone(), g.line2.clone());
            if !g.rolling {
                println!();
                show_dice(g);
                println!("  {}", g.line1);
                if !g.line2.is_empty() {
                    println!("  {}", g.line2);
                }
            }
        }
        match g.pending_ms() {
            Some(ms) => g.tick(ms + 1.0),
            None => break,
        }
    }
}

pub fn run(mut g: Game, mut save: impl FnMut(&Game)) {
    println!("421 - mode terminal. Entrée = lancer / garder sa main, chiffres = dés à relancer (ex. 13), q = quitter.");
    let mut last = (String::new(), String::new());
    let stdin = std::io::stdin();
    loop {
        settle(&mut g, &mut last);
        if g.save_needed {
            g.save_needed = false;
            save(&g);
        }
        let who = if g.mode == Mode::Duo { format!("{} ", g.who_name(g.cur)) } else { String::new() };
        let ask = match g.phase {
            Phase::PreRoll => "Entrée = lancer",
            Phase::Choose => "dés à relancer (ex. 13), Entrée = garder",
            Phase::Result => "Entrée = manche suivante",
            Phase::Over => "Entrée = nouvelle partie",
            _ => "",
        };
        print!(
            "  [POT {}  {} {}  {} {}] {}{} > ",
            g.pot,
            g.label(0),
            g.tok[0],
            g.label(1),
            g.tok[1],
            who,
            ask
        );
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            return;
        }
        let line = line.trim();
        if line.eq_ignore_ascii_case("q") {
            return;
        }
        if g.phase == Phase::Choose {
            let picks: Vec<usize> =
                line.chars().filter_map(|c| c.to_digit(10)).filter(|&d| (1..=3).contains(&d)).map(|d| d as usize - 1).collect();
            if picks.is_empty() {
                g.ui_second();
            } else {
                for i in 0..3 {
                    if picks.contains(&i) != g.dice[g.cur * 3 + i].held {
                        g.ui_toggle(i);
                    }
                }
                g.ui_main();
            }
        } else {
            g.ui_main();
        }
        last = (String::new(), String::new());
    }
}
