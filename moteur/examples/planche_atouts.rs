//! Planches de contrôle des atouts illustrés et de l'Excuse, en grand (8 cartes par image).
//! `cargo run --release --example planche_atouts -- <dossier> [échelle]`

use bistrot::cartes::{Carte, Paquet};
use bistrot::gfx::{Gfx, Glyphs, Xf};
use bistrot::polices::Fichiers;
use std::cell::RefCell;
use tiny_skia::Pixmap;

fn main() {
    let dossier = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let k: f32 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(2.4);
    let fichiers = Fichiers::charger();
    let glyphs = RefCell::new(Glyphs::default());
    let mut paquet = Paquet::new(&fichiers);
    paquet.echelle(k);
    let cartes: Vec<Carte> = (1..=21).map(Carte::Atout).chain([Carte::Excuse]).collect();
    // temps de dessin des 22 cartes (une fois chacune, comme au premier affichage dans le jeu)
    {
        let mut pm = Pixmap::new(400, 400).unwrap();
        let g = Gfx::new(&mut pm, &glyphs, 1.0, 0.0, 0.0);
        let t0 = std::time::Instant::now();
        for &c in &cartes {
            paquet.poser(&g, &Xf::place(200.0, 200.0, k as f64, 0.0, 1.0), Some(c), false);
        }
        println!("22 cartes dessinées en {:.1} ms ({:.2} ms par carte, {} Ko en cache)", t0.elapsed().as_secs_f64() * 1000.0, t0.elapsed().as_secs_f64() * 1000.0 / 22.0, paquet.octets() >> 10);
    }
    for (page, lot) in cartes.chunks(8).enumerate() {
        let (cw, ch) = (106.0 * k, 150.0 * k);
        let mut pm = Pixmap::new((cw * 4.0) as u32, (ch * 2.0) as u32).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(70, 46, 26, 255));
        {
            let g = Gfx::new(&mut pm, &glyphs, 1.0, 0.0, 0.0);
            for (i, &c) in lot.iter().enumerate() {
                let (x, y) = ((i % 4) as f32 * cw + cw / 2.0, (i / 4) as f32 * ch + ch / 2.0);
                paquet.poser(&g, &Xf::place(x as f64, y as f64, k as f64, 0.0, 1.0), Some(c), false);
            }
        }
        let nom = format!("{dossier}/atouts_{}.png", page + 1);
        pm.save_png(&nom).unwrap();
        println!("{nom}");
    }
}
