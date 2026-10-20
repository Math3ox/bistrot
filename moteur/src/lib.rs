//! Moteur commun des jeux de bistrot (belote, 421, tarot, et les suivants : rami...).
//!
//! - [`shell`] : fenêtre et boucle (winit + softbuffer), décor en cache, images seulement si quelque chose change.
//! - [`gfx`] : dessin 100 % processeur (tiny-skia + ab_glyph) en coordonnées logiques 1100 x 800.
//! - [`polices`] : polices du système (gothique et Palatino sous Windows, équivalents sous Linux), lues à la demande.
//! - [`cartes`] : cartes enluminées (jeux de 32, 52, jokers, tarot), dessinées une fois puis posées.
//! - [`table`] : décor de taverne (plancher, table de chêne, chandelles) en cache, places de 2 à 5 joueurs autour.
//! - [`anim`] : interrupteur des animations (auto, oui, non) ; carte envoyée en arc avec son ombre, ramassage du pli.
//! - [`iso`] : projection isométrique 3D (jeux de dés : cylindres, ombres, texte couché).
//! - [`ui`] : parchemins (panneaux, étiquettes de joueurs, bulles), boutons, pièce du donneur.
//! - [`reseau`] : jeu en ligne « lockstep » en étoile (l'hôte relaie), 2 à 5 joueurs.
//! - [`reglages`] : fichier de réglages texte « clé = valeur » à côté de l'exécutable.
//!
//! Un jeu fournit ses règles (sans graphisme, déterministes à partir d'une graine) et implémente
//! [`shell::Game`] pour la fenêtre et [`reseau::Partie`] pour le jeu en ligne.

pub mod anim;
pub mod cartes;
pub mod gfx;
pub mod iso;
pub mod polices;
pub mod reglages;
pub mod reseau;
pub mod shell;
pub mod table;
pub mod ui;

/// Graine aléatoire tirée de l'horloge et du numéro de processus.
pub fn graine() -> u64 {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    (t.as_nanos() as u64) ^ 0x2545F4914F6CDD1D ^ (std::process::id() as u64) << 32
}
