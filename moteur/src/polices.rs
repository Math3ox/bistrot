//! Polices du système, cherchées dans l'ordre (Windows, puis Linux, puis macOS) : aucune police n'est
//! livrée avec le jeu. Tailles en pixels logiques, comme les hauteurs négatives de GDI.

use crate::gfx::{Face, FontFile};

/// Les quatre fichiers de police utilisés par les jeux.
pub struct Fichiers {
    pub gras: FontFile,
    pub demi: FontFile,
    pub symboles: FontFile,
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
        ]);
        let demi = FontFile::load(&[
            "C:\\Windows\\Fonts\\seguisb.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ]);
        let symboles = FontFile::load(&[
            "C:\\Windows\\Fonts\\seguisym.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
            "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
            "/System/Library/Fonts/Apple Symbols.ttf",
        ]);
        let serif = FontFile::load(&[
            "C:\\Windows\\Fonts\\georgiab.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
            "/usr/share/fonts/TTF/DejaVuSerif-Bold.ttf",
            "/usr/share/fonts/dejavu-serif-fonts/DejaVuSerif-Bold.ttf",
            "/System/Library/Fonts/Supplemental/Georgia Bold.ttf",
        ]);
        Fichiers { gras, demi, symboles, serif }
    }
}

/// Polices de l'interface des jeux de cartes.
pub struct Polices {
    pub titre: Face,
    pub etiquette: Face,
    pub petit: Face,
    pub bouton: Face,
    pub moyen: Face,
    pub symbole: Face,
}

impl Polices {
    pub fn depuis(f: &Fichiers) -> Polices {
        Polices {
            titre: f.gras.face(46.0),
            etiquette: f.gras.face(20.0),
            petit: f.demi.face(15.0),
            bouton: f.gras.face(22.0),
            moyen: f.gras.face(28.0),
            symbole: f.symboles.face(50.0),
        }
    }
}
