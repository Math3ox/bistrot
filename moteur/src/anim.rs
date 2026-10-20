//! Carte envoyée de la main vers le pli : arc avec son ombre au sol, rotation qui se pose, perspective
//! qui s'écrase ; puis ramassage du pli vers son gagnant.

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
