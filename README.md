# Bistrot : les jeux de bistrot en 3D isométrique (Rust)

Un moteur commun et les jeux construits dessus :

| Dossier | Contenu |
|---|---|
| `moteur/` | crate `bistrot` : fenêtre, dessin 100 % processeur, cartes, table, animations, interface, jeu en ligne, réglages |
| `belote/` | belote à 4 contre des bots, belote à deux, en ligne de 2 à 4 joueurs (voir `belote/README.md`) |
| `421/` | 421 contre un bot, à deux sur le même PC, duel en ligne, mode terminal (voir `421/README.md`) |
| `outils/` | `shot.py` (pilote un jeu et fait des captures), `screencap.py`, `tools.py` |

Les premières versions (assembleur, puis Rust séparé) restent dans les dépôts `Math3ox/421` et `Math3ox/belote`
avec leurs releases v1.x et v2.0.

## Le moteur (`moteur/`, crate `bistrot`)

- **Dessin sans carte graphique** (`gfx`, `shell`) : tiny-skia pour les formes anticrénelées, ab_glyph pour le texte,
  winit + softbuffer pour la fenêtre (GDI sous Windows, X11 / Wayland sous Linux). Le décor est calculé une fois et
  gardé en cache, les images ne sont refaites que si quelque chose change (60 images/s au plus pendant les
  animations, 0 % de processeur au repos). Environ 30-35 Mo de RAM par jeu.
- **Coordonnées logiques 1100 x 800** mises à l'échelle de la fenêtre (proportions gardées, plein écran, clics justes).
- **Cartes** (`cartes`) au design bistrot, dessinées une fois à la première utilisation : jeux de 32 et 52 cartes,
  jokers, tarot (cavaliers, atouts 1 à 21, excuse). `Carte::jeu32(i)`, `Carte::jeu52(i)`, `Carte::tarot(i)`.
- **Table** (`table`) : table ronde en perspective et places de **2 à 5 joueurs** (main, pli, bulle, étiquette),
  main du bas en éventail qui se resserre pour les longues mains, dos des cartes adverses.
- **Animations** (`anim`) : carte envoyée en arc avec son ombre, ramassage du pli vers le gagnant.
- **Iso 3D** (`iso`) : projection isométrique, cylindres, disques et texte couchés sur le tapis (jeux de dés).
- **Interface** (`ui`, `polices`) : boutons, bulles, étiquettes de joueurs avec jeton du donneur, panneaux, polices
  du système (Segoe UI / Georgia sous Windows, DejaVu sous Linux).
- **Jeu en ligne** (`reseau`) : « lockstep » en étoile, TCP port 4210, l'hôte relaie ; de 2 à 5 joueurs, places
  libres tenues par des bots ; seules les actions circulent (12 octets) ; un invité refuse un hôte qui joue à un
  autre jeu ou à une autre version.
- **Réglages** (`reglages`) : fichier texte « clé = valeur » à côté de l'exécutable.

## Ajouter un jeu (tarot, rami...)

1. Créer un dossier `tarot/` avec un `Cargo.toml` qui dépend de `bistrot.workspace = true` et l'ajouter à
   `members` dans le `Cargo.toml` racine.
2. Écrire les **règles sans graphisme**, déterministes à partir d'une graine (donne, enchères, cartes jouables,
   décompte, bots), comme `belote/src/engine.rs`, avec un banc d'essai `-sim` et des tests.
3. Écrire le **déroulement** (tours des bots, délais, bulles, file des actions en ligne) comme `belote/src/table.rs`,
   et implémenter `bistrot::reseau::Partie` (sièges des invités, configuration de départ, actions).
4. Écrire le **dessin** avec le moteur (`table::disposition(n)`, `cartes::Paquet::poser`, `anim::Vol`, `ui::*`)
   comme `belote/src/draw.rs`, et implémenter `bistrot::shell::Game` (dessin, touches, clics, tic) comme
   `belote/src/main.rs`.

## Compiler
- Windows : `build.bat` (Rust via rustup, cible `x86_64-pc-windows-gnu`, éditeur de liens = gcc de MSYS2
  `C:\msys64\ucrt64\bin`). Produit `belote.exe` et `421.exe` à la racine.
- Linux / macOS : `cargo build --release`, puis `target/release/belote` et `target/release/421`.

## Tests
- `cargo test --release` : moteur (trames), belote (parties automatiques, lockstep à 4 machines, vraie partie TCP à 4),
  421 (règles, bots, lockstep, vrai duel TCP).
- Belote : `target/release/belote -sim <parties> <graine> <joueurs> <niveau0> <niveau1> 1` puis `python belote/check.py`
  (réimplémentation indépendante des règles). 421 : `python 421/bottest.py` (bot difficile = référence Python).
- Captures : `python outils/shot.py <belote|421> sortie.png [actions...]` (ne pilote que la fenêtre qu'il lance).
