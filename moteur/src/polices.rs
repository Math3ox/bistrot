//! Polices du système, cherchées dans l'ordre (Windows, puis Linux, puis macOS) : aucune police n'est
//! livrée avec le jeu. Tailles en pixels logiques, comme les hauteurs négatives de GDI.
//!
//! Style « taverne » : titres en gothique (Old English, livrée avec Office ; sinon Palatino gras), textes
//! en Palatino Linotype (livrée avec Windows ; sous Linux P052 ou TeX Gyre Pagella, sinon le serif du système).

use crate::gfx::{Face, FontFile};

/// Les fichiers de police utilisés par les jeux (les symboles ♠ ♥ ♦ ♣ et ★ sont dessinés en formes
/// vectorielles : pas de police de symboles à charger). Chaque fichier n'est lu qu'à sa première utilisation.
pub struct Fichiers {
    pub gras: FontFile,
    pub demi: FontFile,
    pub serif: FontFile,
    /// gothique des titres
    pub gothique: FontFile,
    /// Palatino gras (textes, indices des cartes)
    pub livre: FontFile,
    /// Palatino normal (petits textes)
    pub livre_n: FontFile,
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
        let gothique = FontFile::load(&[
            "C:\\Windows\\Fonts\\OLDENGL.TTF",
            "C:\\Windows\\Fonts\\palab.ttf",
            "C:\\Windows\\Fonts\\georgiab.ttf",
            "/usr/share/fonts/urw-base35/P052-Bold.otf",
            "/usr/share/fonts/opentype/urw-base35/P052-Bold.otf",
            "/System/Library/Fonts/Supplemental/Georgia Bold.ttf",
        ], "UnifrakturMaguntia,P052,TeX Gyre Pagella,serif:bold");
        let livre = FontFile::load(&[
            "C:\\Windows\\Fonts\\palab.ttf",
            "C:\\Windows\\Fonts\\georgiab.ttf",
            "/usr/share/fonts/urw-base35/P052-Bold.otf",
            "/usr/share/fonts/opentype/urw-base35/P052-Bold.otf",
            "/usr/share/fonts/opentype/tex-gyre/texgyrepagella-bold.otf",
            "/System/Library/Fonts/Supplemental/Georgia Bold.ttf",
        ], "P052,TeX Gyre Pagella,serif:bold");
        let livre_n = FontFile::load(&[
            "C:\\Windows\\Fonts\\pala.ttf",
            "C:\\Windows\\Fonts\\georgia.ttf",
            "/usr/share/fonts/urw-base35/P052-Roman.otf",
            "/usr/share/fonts/opentype/urw-base35/P052-Roman.otf",
            "/usr/share/fonts/opentype/tex-gyre/texgyrepagella-regular.otf",
            "/System/Library/Fonts/Supplemental/Georgia.ttf",
        ], "P052,TeX Gyre Pagella,serif");
        Fichiers { gras, demi, serif, gothique, livre, livre_n }
    }
}

/// Polices de l'interface des jeux de cartes.
pub struct Polices {
    /// titres (gothique : à écrire en minuscules, les capitales gothiques se lisent mal)
    pub titre: Face,
    pub etiquette: Face,
    pub petit: Face,
    pub bouton: Face,
    pub moyen: Face,
}

impl Polices {
    pub fn depuis(f: &Fichiers) -> Polices {
        Polices {
            titre: f.gothique.face(52.0),
            etiquette: f.livre.face(20.0),
            petit: f.livre_n.face(16.0),
            bouton: f.livre.face(21.0),
            moyen: f.livre.face(28.0),
        }
    }
}
