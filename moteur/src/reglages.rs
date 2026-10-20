//! Réglages mémorisés dans un fichier texte « clé = valeur » à côté de l'exécutable.
//! Supprimer le fichier remet les valeurs par défaut ; une valeur illisible est ignorée par le jeu.

use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct Reglages {
    fichier: Option<PathBuf>,
    valeurs: BTreeMap<String, String>,
}

impl Reglages {
    /// Lit `nom` (ex. « belote.cfg ») dans le dossier de l'exécutable.
    pub fn charger(nom: &str) -> Reglages {
        let fichier = std::env::current_exe().ok().map(|p| p.with_file_name(nom));
        let mut valeurs = BTreeMap::new();
        if let Some(text) = fichier.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) {
            for line in text.lines() {
                if line.trim_start().starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    valeurs.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }
        Reglages { fichier, valeurs }
    }

    /// Entier dans l'intervalle [min, max], sinon None.
    pub fn entier(&self, cle: &str, min: i64, max: i64) -> Option<i64> {
        self.valeurs.get(cle)?.parse::<i64>().ok().filter(|v| (min..=max).contains(v))
    }

    pub fn oui(&self, cle: &str) -> Option<bool> {
        self.entier(cle, 0, 1).map(|v| v == 1)
    }

    /// Adresse « ip » ou « ip:port » (chiffres, points, deux-points ; 21 caractères au plus).
    pub fn adresse(&self, cle: &str) -> Option<String> {
        let v = self.valeurs.get(cle)?;
        (v.len() <= 21 && v.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ':')).then(|| v.clone())
    }

    pub fn mettre(&mut self, cle: &str, valeur: impl std::fmt::Display) {
        self.valeurs.insert(cle.to_string(), valeur.to_string());
    }

    /// Écrit le fichier (silencieux si le dossier n'est pas inscriptible).
    pub fn enregistrer(&self, entete: &str) {
        let mut text = format!("# {entete} (supprimer ce fichier remet les valeurs par défaut)\n");
        for (k, v) in &self.valeurs {
            text.push_str(&format!("{k} = {v}\n"));
        }
        if let Some(p) = &self.fichier {
            let _ = std::fs::write(p, text);
        }
    }
}
