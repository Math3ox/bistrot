# Belote en 3D isométrique (Rust, Windows / Linux / macOS)

Belote contre des bots (ou en ligne avec des amis) : table ronde en perspective isométrique, cartes posées sur la
table de chêne d'une taverne aux chandelles, avec rotation et animation. Même esprit que le jeu du 421. Écrit en Rust.
Le dessin est fait **entièrement par le processeur, sans carte graphique ni OpenGL** (tiny-skia pour les formes
anticrénelées, ab_glyph pour le texte, winit + softbuffer pour la fenêtre : affichage par GDI sous Windows).
Comme dans la version assembleur, le décor est calculé une fois puis gardé en cache, seul ce qui bouge est
redessiné, et rien n'est recalculé quand rien ne bouge. Un seul code pour Windows, Linux et macOS. Environ 35 Mo de RAM.

## Lancer
Double-clic sur `belote.exe` (aucune installation, uniquement des DLL de Windows 10/11). Sous Linux : `./belote`.
Les réglages sont mémorisés dans `belote.cfg` (texte, supprimer ce fichier remet les valeurs par défaut).

## Modes de jeu
| Mode | Joueurs | Principe |
|---|---|---|
| **Belote à 4** (par défaut) | toi + 3 bots | toi et le bot d'en face (« partenaire ») contre les deux autres bots |
| **Belote à deux** (touche `J`) | toi + 1 bot | belote « de comptoir » : variante à deux joueurs (voir plus bas) |
| **En ligne** (touche `N`) | 2 à 4 humains | les places libres sont tenues par des bots ; 2 humains seulement en mode « à deux » |

## Règles (belote classique à 4)
- 32 cartes. On donne 3 puis 2 cartes à chacun, on retourne une carte : c'est la proposition d'atout.
  Au **1er tour**, chacun à son tour peut **prendre** (la couleur de la retourne devient l'atout) ou passer.
  Si tout le monde passe, **2e tour** : on peut prendre dans une autre couleur. Si tout le monde repasse, on redonne.
  Le preneur ramasse la retourne, puis on complète : tout le monde a 8 cartes.
- Valeur des cartes : à l'**atout** V 20, 9 14, A 11, 10 10, R 4, D 3 ; hors atout A 11, 10 10, R 4, D 3, V 2.
  Total 152 + 10 de der = **162** points par donne.
- Il faut **fournir** la couleur demandée ; sinon **couper** (jouer atout) sauf si le partenaire est maître du pli ;
  il faut **monter** à l'atout quand on le peut.
- **Belote-rebelote** : roi et dame d'atout dans la même main, +20 (annoncés automatiquement quand tu les joues).
- **Annonces** (touche `A` pour les activer/couper) : tierce 20, cinquante 50, cent 100 (suite de 5 ou plus),
  carrés (valets 200, 9 150, as/10/rois/dames 100). Elles sont comptées automatiquement après le 1er pli :
  seule l'équipe qui a la meilleure annonce marque les siennes.
- **Contrat** : l'équipe qui a pris doit faire plus de points que l'autre (annonces et belote comprises).
  Sinon elle est **dedans** : les adversaires marquent les 162 points (la belote reste à celui qui l'a).
  À égalité : **litige**, les points du preneur sont mis de côté pour la prochaine donne.
- **Capot** (un camp fait tous les plis) : 252 points.
- La partie se joue en 1000 points par défaut (touche `P` : 501 / 1000 / 1500).

## Belote à deux (variante implémentée)
6 cartes chacun, un talon de 20 cartes avec la retourne dessus. Le joueur qui n'a pas donné parle en premier (prendre ou
passer, 2 tours comme à 4). Le preneur ramasse la retourne et l'adversaire pioche une carte (7 chacun). **Tant que le talon
n'est pas vide, aucune obligation** de fournir, couper ou monter ; après chaque pli le gagnant pioche en premier. Quand le talon est
vide, les obligations de la belote classique s'appliquent. Même décompte (162 points par donne, contrat à battre, belote 20).

## Jouer
- **Enchères** : boutons PRENDRE / PASSER (1er tour), couleur / PASSER (2e tour).
- **Jeu** : clique sur une carte (les cartes grisées sont injouables). Espace ou Entrée : SUITE à la fin d'une manche.
- Raccourcis : `J` 4 joueurs / à deux · `L` niveau des bots · `A` annonces · `H` aide · `P` objectif · `F` force des cartes ·
  `M` animations · `F11` plein écran · `F12` capture `belote_shot.png` · `N` en ligne · `Échap` quitter.
- `M` : **animations** auto → oui → non. `non` pour un processeur léger : cartes posées d'un coup,
  plus aucune image calculée pour l'animation. En `auto` (par défaut), le jeu coupe lui-même les animations s'il
  trouve le processeur trop lent (« animations coupées » dans la ligne du bas).
- `H` (aide) : conseil pour prendre ou passer, et la carte conseillée est surlignée en or pendant le jeu.
- `F` : **force des cartes** : ordre des cartes de la plus forte à la plus faible et leurs points, à l'atout et hors atout
  (avec la vraie couleur d'atout de la donne), rappel des annonces et de la belote. `F`, Échap ou un clic le ferme.
- `G` : galerie des 32 cartes et du dos (contrôle du dessin).
- `U` : l'ordinateur joue à ta place (démonstration / tests).
- La fenêtre est redimensionnable (proportions gardées) ; sur un petit écran elle est réduite pour tenir.

## Niveaux des bots (touche `L`)
- **facile** : heuristiques normales mais 35 % de coups au hasard et enchères bruitées.
- **normal** : évaluation de la main pour prendre ; comptage des cartes sorties, cartes maîtresses, couleurs manquantes,
  on charge le partenaire quand il est maître, on gagne le pli avec la plus petite carte possible.
- **difficile** : simulations de Monte-Carlo : le bot tire au hasard les cartes qu'il ne voit pas (en respectant les couleurs
  que chaque joueur a montré ne plus avoir), joue la fin de la donne pour chaque carte possible et choisit celle qui rapporte
  le plus en moyenne ; même méthode pour décider de prendre. En simulation il bat « normal » dans environ 80 % des parties.

## En ligne (touche `N`)
Un joueur choisit **H** (héberger) : le jeu affiche son adresse IP (réseau local, ou VPN type Tailscale / ZeroTier, ou
redirection du port TCP **4210** sur la box). Les autres choisissent **R**, tapent l'IP de l'hôte (`ip` ou `ip:port`) puis Entrée.
- À **4 joueurs** : jusqu'à 3 invités (le 1er prend la place de ton partenaire). La partie démarre quand ils sont tous là, ou
  quand l'hôte appuie sur **Entrée** : les places libres sont tenues par des bots.
- En mode **à deux** : un seul invité, la partie démarre dès qu'il est connecté.
- Principe « lockstep » : toutes les machines exécutent la même partie avec la même graine aléatoire (envoyée par l'hôte) ;
  seules les actions (enchère, carte jouée, « suite ») circulent, 12 octets chacune ; l'hôte relaie celles des invités.
  Les joueurs doivent avoir la même version (Windows et Linux peuvent jouer ensemble).
  Si quelqu'un se déconnecte, tout le monde revient à une partie locale.

## Compiler
Depuis la racine du dépôt : `build.bat` (Windows) ou `cargo build --release -p belote` (voir le README racine).

## Tests
- `cargo test --release -p belote` : parties complètes en mode automatique (4 joueurs et à deux), lockstep entre
  4 machines de vitesses différentes, et une **vraie partie TCP à 4 sur 127.0.0.1** (un hôte qui relaie, trois invités)
  qui vérifie que les quatre machines restent identiques.
- Banc d'essai : `belote -sim <parties> <graine> <joueurs 2|4> <niveau équipe 0> <niveau équipe 1> <journal 0|1>`
  écrit `sim.log`, puis `python check.py` le rejoue avec une réimplémentation indépendante des règles (légalité de
  chaque coup, mains, scores, annonces, litiges). `tune.py` / `tune2.py` : réglage des bots.
- Captures : `python outils/shot.py belote sortie.png [actions...]` (depuis la racine).

## Fichiers
- `src/engine.rs` : règles (enchères, annonces, plis, décompte), bots facile / normal, Monte-Carlo, banc d'essai `-sim`.
- `src/table.rs` : déroulement (tours des bots, délais, animations, bulles, aide, actions en ligne) ; jeu en ligne
  (`bistrot::reseau::Partie`).
- `src/draw.rs` : dessin de la scène avec le moteur (cartes, places, animations, panneaux, force des cartes), au
  style taverne du moteur (table de chêne aux chandelles, parchemins, cartes enluminées, titres en gothique).
- `src/vue.rs` : ce qui bouge à l'écran sans toucher à la partie : distribution des cartes, main qui se réarrange,
  levée au survol, retourne qui se retourne, bulles, score qui défile.
- `src/main.rs` : le jeu dans sa fenêtre (`bistrot::shell::Game` : touches, clics, couche fixe en cache), `-sim`.
- `src/cfg.rs` : réglages mémorisés dans `belote.cfg`.
- Fenêtre, dessin, cartes, table, réseau : moteur commun `../moteur` (crate `bistrot`).
