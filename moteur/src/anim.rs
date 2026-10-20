//! Animations communes : carte envoyée de la main vers le pli (arc avec son ombre au sol, rotation qui se
//! pose, perspective qui s'écrase) puis ramassée vers le gagnant ; carte distribuée (du paquet du donneur
//! vers sa place, retournée en arrivant dans la main du joueur) ; valeurs qui rejoignent leur cible en
//! douceur (cartes de la main qui se réarrangent, carte qui se soulève, score qui défile).
//!
//! **Interrupteur des animations** (processeurs légers) : réglage `auto`, `oui` ou `non`. Coupées, toutes
//! les aides ci-dessous rendent directement l'état final (carte posée, valeur atteinte, pas de rebond) : plus
//! aucune image n'est calculée pour faire bouger quoi que ce soit, seulement quand la partie change. En `auto`,
//! la fenêtre mesure le temps de ses images animées et coupe les animations si le processeur ne suit pas.

use crate::gfx::Xf;
use crate::table::SOL;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Réglage des animations.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Animations {
    /// jouées, sauf si la fenêtre trouve le processeur trop lent
    Auto,
    Oui,
    Non,
}

static REGLAGE: AtomicU8 = AtomicU8::new(0);
static LENT: AtomicBool = AtomicBool::new(false);

impl Animations {
    const TOUS: [Animations; 3] = [Animations::Auto, Animations::Oui, Animations::Non];

    /// Nom dans les fichiers de réglages.
    pub fn nom(self) -> &'static str {
        ["auto", "oui", "non"][self as usize]
    }

    pub fn depuis(s: &str) -> Option<Animations> {
        Self::TOUS.into_iter().find(|a| a.nom() == s)
    }

    /// Réglage suivant (touche du jeu) : auto, oui, non, auto...
    pub fn suivant(self) -> Animations {
        Self::TOUS[(self as usize + 1) % 3]
    }
}

/// Réglage en cours.
pub fn reglage() -> Animations {
    Animations::TOUS[REGLAGE.load(Ordering::Relaxed) as usize % 3]
}

pub fn regler(a: Animations) {
    REGLAGE.store(a as u8, Ordering::Relaxed);
}

/// Les animations sont jouées.
pub fn actives() -> bool {
    match reglage() {
        Animations::Oui => true,
        Animations::Non => false,
        Animations::Auto => !LENT.load(Ordering::Relaxed),
    }
}

/// La fenêtre a trouvé le processeur trop lent (les animations sont coupées en mode auto).
pub fn processeur_lent() -> bool {
    LENT.load(Ordering::Relaxed)
}

pub(crate) fn signaler_lent() {
    LENT.store(true, Ordering::Relaxed);
}

/// Texte pour la ligne des réglages : « auto », « coupées » (auto, processeur trop lent), « oui », « non ».
pub fn libelle() -> String {
    match reglage() {
        Animations::Auto if processeur_lent() => "coupées".into(),
        a => a.nom().into(),
    }
}

/// Durée du vol d'une carte (ms) et du ramassage du pli (ms).
pub const DUREE_VOL: f64 = 420.0;
pub const DUREE_RAMASSAGE: f64 = 300.0;

/// Vol d'une carte.
pub struct Vol {
    /// départ (centre de la carte dans la main) et échelle de départ (1.04 dans la main du bas, 0.62 sinon)
    pub depart: (f64, f64),
    pub echelle: f64,
    /// arrivée dans le pli et angle final
    pub arrivee: (f64, f64),
    pub angle: f64,
}

impl Vol {
    /// Position au temps `ms` après le départ ; `ramassage` = (main du gagnant, ms depuis le début du ramassage).
    /// Renvoie la transformation de la carte et, pendant le vol, l'ombre au sol (centre, rayons).
    /// Animations coupées : la carte est tout de suite posée dans le pli, et y reste jusqu'au ramassage.
    pub fn a(&self, ms: f64, ramassage: Option<((f64, f64), f64)>) -> (Xf, Option<(f32, f32, f32, f32)>) {
        let (ms, ramassage) = if actives() { (ms, ramassage) } else { (DUREE_VOL, None) };
        let t = (ms / DUREE_VOL).clamp(0.0, 1.0);
        let e = 1.0 - (1.0 - t).powi(3); // arrivée en décélérant
        let h = (std::f64::consts::PI * t).sin(); // hauteur du vol
        let (mut ax, mut ay, mut cs) = (self.arrivee.0, self.arrivee.1, 1.0);
        if let Some(((gx, gy), r)) = ramassage {
            let u = (r / DUREE_RAMASSAGE).clamp(0.0, 1.0);
            let u = u * u; // accélère
            ax += (gx - ax) * u;
            ay += (gy - ay) * u;
            cs = 1.0 - 0.3 * u;
        }
        let x = self.depart.0 + (ax - self.depart.0) * e;
        let yg = self.depart.1 + (ay - self.depart.1) * e;
        let ombre = (h > 0.02).then(|| {
            let rx = (30.0 + 34.0 * h) as i32;
            let ry = (rx >> 1) + (rx >> 3);
            ((x + 8.0) as f32, (yg + 8.0) as f32, rx as f32, ry as f32)
        });
        let s0 = self.echelle;
        let scale = (s0 + (1.0 - s0) * e + 0.14 * h) * cs;
        let sq = 1.0 + (SOL - 1.0) * e;
        let ang = self.angle * (5.0 - 4.0 * e);
        (Xf::place(x, yg - 64.0 * h, scale, ang, sq), ombre)
    }
}

/// Départ rapide, arrivée en douceur (t de 0 à 1).
pub fn sortie(t: f64) -> f64 {
    if !actives() {
        return 1.0;
    }
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Léger dépassement puis retour (apparition « qui rebondit »), t de 0 à 1.
pub fn rebond(t: f64) -> f64 {
    if !actives() {
        return 1.0;
    }
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + t * t * (2.7 * t + 1.7)
}

/// Fait rejoindre sa cible à une valeur en douceur (constante de temps `tau` ms) ; renvoie vrai tant
/// qu'elle bouge encore visiblement. Animations coupées : la valeur saute à sa cible.
pub fn approcher(v: &mut f64, cible: f64, dt: f64, tau: f64) -> bool {
    if !actives() {
        *v = cible;
        return false;
    }
    let k = 1.0 - (-dt / tau).exp();
    *v += (cible - *v) * k;
    if (cible - *v).abs() < 0.25 {
        *v = cible;
        false
    } else {
        true
    }
}

/// Durée du vol d'une carte distribuée (ms).
pub const DUREE_DISTRIB: f64 = 340.0;

/// Carte distribuée : du paquet `src` à sa place `dst` (échelle et rotation finales), en arc.
pub struct Distribution {
    pub src: (f64, f64),
    pub dst: (f64, f64),
    pub echelle: f64,
    pub rotation: f64,
    /// la carte se retourne en arrivant (main du joueur local)
    pub retourner: bool,
}

impl Distribution {
    /// Au temps `ms` après le départ : transformation, ombre au sol (centre, rayons), face visible.
    /// Animations coupées : la carte est tout de suite à sa place.
    pub fn a(&self, ms: f64) -> (Xf, Option<(f32, f32, f32, f32)>, bool) {
        let ms = if actives() { ms } else { DUREE_DISTRIB };
        let t = (ms / DUREE_DISTRIB).clamp(0.0, 1.0);
        let e = sortie(t);
        let h = (std::f64::consts::PI * t).sin();
        let x = self.src.0 + (self.dst.0 - self.src.0) * e;
        let yg = self.src.1 + (self.dst.1 - self.src.1) * e;
        let scale = 0.5 + (self.echelle - 0.5) * e + 0.08 * h;
        // la carte tourne un peu en volant et se pose droite
        let rot = self.rotation * e + (1.0 - e) * 0.9;
        let base = Xf::place(x, yg - 40.0 * h, scale, rot, 1.0);
        let (xf, face) = if self.retourner {
            // retournement sur la deuxième moitié du vol : dos, carte vue de profil, face
            let f = ((t - 0.4) / 0.6).clamp(0.0, 1.0);
            let sx = (std::f64::consts::PI * f).cos();
            (base.puis(&Xf { a: sx.abs().max(0.02) as f32, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 }), f > 0.5)
        } else {
            (base, false)
        };
        let ombre = (h > 0.03).then(|| {
            let rx = (24.0 + 22.0 * h) as f32;
            ((x + 6.0) as f32, (yg + 8.0) as f32, rx, rx * 0.62)
        });
        (xf, ombre, face)
    }

    /// Le vol est terminé.
    pub fn fini(ms: f64) -> bool {
        ms >= DUREE_DISTRIB
    }
}
