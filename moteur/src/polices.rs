//! Polices du système, cherchées dans l'ordre (Windows, puis Linux, puis macOS) : aucune police n'est
//! livrée avec le jeu. Tailles en pixels logiques, comme les hauteurs négatives de GDI.

use crate::gfx::{Face, FontFile};

/// Les trois fichiers de police utilisés par les jeux (les symboles ♠ ♥ ♦ ♣ et ★ sont dessinés en formes
/// vectorielles : pas de police de symboles à charger).
pub struct Fichiers {
    pub gras: FontFile,
    pub demi: FontFile,
    pub serif: FontFile,
}

impl Fichiers {
    pub fn charger() -> Fichiers {
        let gras = FontFile::load(&[
            "C:\\Windows\\Fonts\\segoeuib.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf",
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        ], "sans-serif:bold");
        let demi = FontFile::load(&[
            "C:\\Windows\\Fonts\\seguisb.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ], "sans-serif:semibold");
        let serif = FontFile::load(&[
            "C:\\Windows\\Fonts\\georgiab.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
            "/usr/share/fonts/TTF/DejaVuSerif-Bold.ttf",
            "/usr/share/fonts/dejavu-serif-fonts/DejaVuSerif-Bold.ttf",
            "/System/Library/Fonts/Supplemental/Georgia Bold.ttf",
        ], "serif:bold");
        Fichiers { gras, demi, serif }
    }
}

/// Polices de l'interface des jeux de cartes.
pub struct Polices {
    pub titre: Face,
    pub etiquette: Face,
    pub petit: Face,
    pub bouton: Face,
    pub moyen: Face,
}

impl Polices {
    pub fn depuis(f: &Fichiers) -> Polices {
        Polices {
            titre: f.gras.face(46.0),
            etiquette: f.gras.face(20.0),
            petit: f.demi.face(15.0),
            bouton: f.gras.face(22.0),
            moyen: f.gras.face(28.0),
        }
    }
}
