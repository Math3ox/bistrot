# 421 en 3D isométrique (Rust, Windows / Linux / macOS)

Jeu du 421 contre un bot, à deux sur le même PC ou en duel en ligne : table ronde verte, jetons, dés et boutons dessinés
en projection isométrique (rotation 3D réelle des dés pendant le lancer). Écrit en Rust.
Le dessin est fait **entièrement par le processeur, sans carte graphique ni OpenGL** (tiny-skia pour les formes
anticrénelées, ab_glyph pour le texte, winit + softbuffer pour la fenêtre : affichage par GDI sous Windows).
Comme dans la version assembleur, le décor est calculé une fois puis gardé en cache, seul ce qui bouge est
redessiné, et rien n'est recalculé quand rien ne bouge. Un seul code pour Windows, Linux et macOS. Environ 27 Mo de RAM.

## Lancer
Double-clic sur `421.exe` (environ 1 Mo, aucune installation, uniquement des DLL de Windows 10/11).
Sous Linux : `./421` (X11 ou Wayland, aucun pilote graphique particulier).

## Jouer
- **LANCER** : lance les trois dés (3 lancers maximum ; le 2e joueur ne peut pas lancer plus de fois que le 1er).
- Clique sur les dés à **relancer** (ils se soulèvent, disque doré), puis **RELANCER** ; ou **VALIDER** pour garder ta main telle quelle.
- Le perdant de la manche prend dans le pot les jetons de la combinaison gagnante ; pot vide, le gagnant repasse les siens.
  Le premier sans jeton gagne. Valeurs : 421 (8), 111 (7), 11X et XXX (X), suites (2), le reste (1).
- Raccourcis : Espace = bouton du haut, Entrée = VALIDER, F12 = capture `421_shot.png`, Échap = quitter.

## Taille de la fenêtre
La fenêtre fait 1100x800 si l'écran le permet ; sur un écran plus petit (ou très zoomé, ex. portable à 150 %), elle est
réduite pour tenir dans la zone de travail. Elle est redimensionnable : le jeu est mis à l'échelle (proportions gardées,
bandes noires si besoin) et les clics restent justes.

## Options (rappelées en bas de la fenêtre)
| Touche | Effet |
|---|---|
| `J` | 1 joueur contre le bot ↔ **2 joueurs sur le même PC** (chacun son tour, mêmes boutons ; nouvelle partie) |
| `L` | niveau du bot : facile → normal → difficile |
| `H` | **aide** pendant le choix des dés : probabilité de 421 et de « mieux » pour les dés sélectionnés + conseil optimal |
| `P` | pot de départ 7 / 11 / 15 / 21 jetons (à la prochaine partie) |
| `M` | **animations** auto → oui → non : `non` (processeur léger) pose les dés d'un coup ; `auto` les coupe tout seul si le processeur est trop lent |
| `F11` | **plein écran** (proportions gardées). Échap en sort |

Niveaux du bot : *facile* = stratégie normale avec 35 % de décisions au hasard ; *normal* = garde les as, un 2 et un 4 ;
*difficile* = stratégie **optimale** (programmation dynamique sur les 56 mains possibles, utilité en jetons). En simulation :
difficile bat normal dans ~58 % des parties, normal bat facile dans ~63 %.

Les réglages (pot, niveau, 1/2 joueurs, animations, plein écran, victoires/défaites contre le bot, dernière IP) sont mémorisés dans
`421.cfg` (texte) à côté de l'exécutable ; supprimer ce fichier remet les valeurs par défaut.

## Multijoueur en ligne (duel à 2, IP directe)
Touche **N** : un joueur choisit **H** (héberger), l'autre **R** (rejoindre) et tape l'IP de l'hôte (port 4210 par défaut,
ou `ip:port`) puis Entrée ; Échap annule, `N` coupe la connexion. L'hôte est le joueur 1, l'invité le joueur 2.
- L'hôte voit son adresse IP à l'écran. Réseau local : la sienne (192.168.x.x). Par Internet : **VPN type Tailscale / ZeroTier**
  (donner l'IP du VPN) ou **redirection du port TCP 4210** sur la box. Le pare-feu demandera d'autoriser le jeu.
- Principe « lockstep » : les deux PC exécutent la même partie avec le même générateur aléatoire (graine envoyée par l'hôte) ;
  seules les **actions** (lancer, dés cochés, valider, suivant, rejouer) sont échangées, 12 octets chacune. Les deux joueurs
  doivent avoir la même version du jeu (Windows et Linux peuvent jouer ensemble). Si la connexion tombe, on revient à la partie locale.

## Mode terminal
`421 -t` joue dans la console (mêmes règles, même bot) : Entrée pour lancer ou garder sa main, chiffres des dés à
relancer (ex. `13`), `q` pour quitter.

## Compiler
Depuis la racine du dépôt : `build.bat` (Windows) ou `cargo build --release -p jeu421` (voir le README racine).

## Tests
- `cargo test --release -p jeu421` : rangs des mains, bot normal, partie complète contre chaque niveau de bot,
  conservation des jetons, lockstep entre deux machines de vitesses différentes, et un **vrai duel TCP sur 127.0.0.1**
  (hôte + invité) qui vérifie que les deux parties restent identiques.
- `python bottest.py` : les décisions du bot difficile (26 situations x 216 mains) sont comparées à la stratégie de
  référence en Python (`gen_tables.py`).
- Captures : `python outils/shot.py 421 sortie.png [actions...]` (depuis la racine).

## Fichiers
- `src/game.rs` : règles et déroulement (1 joueur, 2 joueurs, en ligne), dés en 3D (orientation, trajectoire du lancer),
  file des actions en ligne, jeu en ligne (`bistrot::reseau::Partie`). Sans graphisme : sert à la fenêtre, au terminal
  et aux tests.
- `src/ai.rs` : rang des mains, niveaux du bot, stratégie optimale, probabilités de l'aide.
- `src/render.rs` : dessin isométrique (table, jetons, boutons, dés éclairés, ombres, textes) et zones cliquables,
  avec la projection `bistrot::iso`.
- `src/main.rs` : le jeu dans sa fenêtre (`bistrot::shell::Game`), `-t`. `src/term.rs` : mode terminal.
- `src/cfg.rs` : réglages mémorisés dans `421.cfg`. `gen_tables.py` : stratégie optimale de référence en Python.
- Fenêtre, dessin, projection iso, réseau : moteur commun `../moteur` (crate `bistrot`).
