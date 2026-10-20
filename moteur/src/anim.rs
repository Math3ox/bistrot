//! Animations communes : carte envoyée de la main vers le pli (arc avec son ombre au sol, rotation qui se
//! pose, perspective qui s'écrase) puis ramassée vers le gagnant ; carte distribuée (du paquet du donneur
//! vers sa place, retournée en arrivant dans la main du joueur) ; valeurs qui rejoignent leur cible en
//! douceur (cartes de la main qui se réarrangent, carte qui se soulève, score qui défile).

use crate::gfx::Xf;
use crate::table::SOL;

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
    pub fn a(&self, ms: f64, ramassage: Option<((f64, f64), f64)>) -> (Xf, Option<(f32, f32, f32, f32)>) {
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
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Léger dépassement puis retour (apparition « qui rebondit »), t de 0 à 1.
pub fn rebond(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + t * t * (2.7 * t + 1.7)
}

/// Fait rejoindre sa cible à une valeur en douceur (constante de temps `tau` ms) ; renvoie vrai tant
/// qu'elle bouge encore visiblement.
pub fn approcher(v: &mut f64, cible: f64, dt: f64, tau: f64) -> bool {
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
    pub fn a(&self, ms: f64) -> (Xf, Option<(f32, f32, f32, f32)>, bool) {
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
