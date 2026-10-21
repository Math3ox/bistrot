# Rami 51 en 3D isométrique (Rust, Windows / Linux / macOS)

Rami à 2, 3, 4 ou 5 joueurs contre des bots (ou en ligne avec des amis), selon les règles du **rami 51** : table
ronde en perspective dans une taverne aux chandelles, cartes enluminées, combinaisons étalées sur la table qui se
resserrent quand elle se remplit, cartes qui glissent d'un endroit à l'autre. Construit sur le moteur commun
`bistrot` : dessin **entièrement par le processeur**, sans carte graphique, **jamais plus de 50 Mo de RAM**
(environ 26-28 Mo en fenêtre, 40 Mo au plus en plein écran 1080p).

## Lancer
Double-clic sur `rami.exe` (aucune installation). Sous Linux : `./rami`. Les réglages sont mémorisés dans `rami.cfg`.

## Règles (rami 51)
- **108 cartes** : deux jeux de 52 et 4 jokers. 13 cartes chacun ; une carte retournée commence la défausse, le reste
  fait le talon. Le joueur à gauche du donneur commence.
- **Tour** : piocher (au talon, ou la carte du dessus de la défausse, qui doit alors être posée dans le tour), poser
  si on peut, puis défausser une carte.
- **Combinaisons** : **suite** d'au moins 3 cartes de la même couleur qui se suivent (l'as en bas A-2-3 ou en haut
  D-R-A, pas les deux) ; **brelan** ou **carré** : 3 ou 4 cartes de même valeur, de couleurs différentes. **Un joker au
  plus** par combinaison ; il remplace n'importe quelle carte et garde sa place une fois posé.
- **Ouverture** : la première fois, il faut poser **51 points** en un seul tour (2 à 10 leur valeur, figures 10, as 11
  ou 1 dans A-2-3, le joker vaut la carte qu'il remplace).
- Une fois ouvert, on peut à chaque tour poser de nouvelles combinaisons, **compléter n'importe quelle combinaison de
  la table**, et **reprendre un joker** en donnant la carte qu'il remplace (le joker doit être reposé dans le tour).
- **Fin de la manche** : le premier qui n'a plus de cartes (toutes posées, ou la dernière défaussée). Les autres
  marquent la **pénalité** de leurs cartes : joker 20, as 11, figures 10, les autres leur valeur ; **au moins 100**
  pour qui n'a pas ouvert. **Rami** : le gagnant a tout posé d'un coup sans avoir ouvert avant, les pénalités sont
  doublées.
- Talon vide : il est refait avec la défausse (sauf sa carte du dessus), deux fois au plus ; ensuite la manche est
  nulle (personne ne marque).
- **Partie** : elle s'arrête quand un joueur atteint la limite (touche `P` : 100 / 200 / 300 / 500 points) ; le moins
  pénalisé gagne.

## Jouer
- **Piocher** : clic sur le talon, ou sur la défausse pour prendre sa carte (elle est alors cerclée de rouge dans la
  main jusqu'à ce qu'elle soit posée).
- **Choisir des cartes** : clic sur les cartes de la main (elles se lèvent, cerclées d'or) ; Échap pour tout lâcher.
- **POSER** (ou Entrée) : les cartes choisies deviennent une combinaison sur la table.
- **Compléter** : choisir les cartes, puis cliquer sur la combinaison de la table (elle s'entoure d'or si c'est
  possible). **Reprendre un joker** : choisir la carte qu'il remplace, puis cliquer sur le joker.
- **Défausser** : une carte choisie, puis clic sur la défausse (ou DÉFAUSSER).
- **ANNULER** (ou Retour arrière) : revient au début du tour (juste après la pioche au talon), pour recommencer ses
  poses ; une action refusée s'explique en bas de l'écran (combinaison invalide, pas encore ouvert, 51 points pas
  atteints, carte de la défausse pas posée...).
- **TRIER** (ou `T`) : main triée par couleur ou par valeur.
- Le panneau du haut à gauche donne les pénalités cumulées (un sceau rouge devant ceux qui ont ouvert) ; celui de
  droite, le talon et où tu en es de ton ouverture.
- Raccourcis : `J` 2 à 5 joueurs · `L` niveau des bots · `P` limite de points · `H` aide · `F` aide-mémoire ·
  `M` animations · `F11` plein écran · `F12` capture `rami_shot.png` · `N` en ligne · `Échap` quitter.
- `H` (aide) : la pioche conseillée, puis les cartes à poser, à ajouter ou à défausser cerclées d'or, avec le conseil
  écrit en bas. `G` : galerie des cartes. `U` : l'ordinateur joue à ta place.

## Niveaux des bots (touche `L`)
Les bots ne trichent pas : ils ne voient que leur main, la table, la défausse et ce que chacun y a pris ou jeté.
Tous cherchent la meilleure façon de découper leur main en combinaisons (recherche exacte, jokers compris), ouvrent
dès qu'ils ont 51 points, ne prennent la défausse que s'ils peuvent la poser dans le tour (essayé sur une copie).
- **facile** : ne prend la défausse que pour ouvrir ou finir, ne reprend jamais de joker, défausse parfois au hasard.
- **normal** : pose tout ce qu'il peut une fois ouvert, complète la table, reprend les jokers qu'il peut reposer
  aussitôt, garde ses paires et ses cartes qui se suivent, jette les grosses cartes quand un adversaire approche de
  la fin, ne donne pas au joueur suivant une carte qui va sur la table.
- **difficile** : pour chaque défausse possible, regarde toutes les cartes qu'il peut encore piocher (selon celles qui
  sont sorties) et ce qu'il pourrait poser au tour suivant ; se méfie de ce que ramasse le joueur suivant. Au banc
  d'essai (4 joueurs, 400 parties), il prend environ 4 points de pénalité de moins par manche qu'un bot normal ;
  le facile en prend environ 8 de plus. Moins de 40 ms de réflexion par tour (le plus souvent quelques ms).

## En ligne (touche `N`)
Comme la belote et le tarot : un joueur héberge (H), les autres rejoignent (R) avec son adresse IP (port TCP 4210).
De 2 à 5 humains selon le nombre de joueurs choisi par l'hôte ; l'hôte démarre avec Entrée, les places libres sont
tenues par des bots. Seules les actions circulent (pioche, pose, ajout, échange de joker, défausse, annulation,
suite), 12 octets chacune : une pose est envoyée comme la liste des positions des cartes dans la main.

## Tests
- `cargo test --release -p rami` : combinaisons (suites, as bas et haut, brelans, joker qui garde sa place, échange),
  annulation, recherche des combinaisons, parties entières entre bots de tous niveaux, parties complètes en mode
  automatique de 2 à 5 joueurs, lockstep entre 2 à 5 machines de vitesses différentes, joueur en ligne qui pose,
  complète et annule, et clics (chaque carte de la table et de la main se touche à sa place).
- Banc d'essai : `rami -sim <parties> <graine> <joueurs> <niveau du siège 0> <niveau des autres> <journal 0|1>
  [limite] [passifs 0|1]` : statistiques (manches, tours, ramis, tour d'ouverture, manches et parties gagnées et
  pénalité moyenne par siège, temps de réflexion) ; avec le journal, écrit `sim.log`, puis
  `python rami/check.py sim.log` le rejoue avec une **réimplémentation indépendante** des règles (distribution,
  pioche, talon refait, suites et brelans, joker qui garde sa place, ouverture à 51, ajouts et échanges après
  l'ouverture, carte de la défausse et joker repris à poser dans le tour, fin de manche, pénalités, rami, limite).
  `passifs = 1` : les bots ne posent rien, pour vérifier le talon refait et les manches nulles.
- `cargo test --release -p rami -- --ignored apercus` : dessine sans fenêtre les écrans (distribution, début, jeu avec
  une sélection, bilan, classement, 5 joueurs, 2 joueurs, galerie) dans `target/apercu_rami_*.png` ;
  `-- --ignored mesure_dessin --nocapture` : temps de dessin d'une image et du fond.

## Fichiers
- `src/engine.rs` : règles (combinaisons, donne, pioche, pose, ajout, échange de joker, défausse, annulation, fin de
  manche, pénalités), journal.
- `src/bots.rs` : recherche des combinaisons, tour des bots, choix de la défausse (facile, normal, difficile).
- `src/sim.rs` : banc d'essai `-sim`. `check.py` : vérificateur indépendant.
- `src/table.rs` : déroulement (délais, bulles, conseils, actions du joueur et leurs refus expliqués, actions en
  ligne, `bistrot::reseau::Partie`), places autour de la table.
- `src/vue.rs` : ce qui bouge (distribution, cartes qui glissent, cartes ramassées par les adversaires, main, bulles,
  scores) et place des combinaisons sur la table.
- `src/draw.rs` : dessin (piles, table, mains, panneaux, bilan, boutons, aide-mémoire, galerie).
- `src/main.rs` : le jeu dans sa fenêtre (`bistrot::shell::Game`), `-sim`. `src/cfg.rs` : réglages (`rami.cfg`).
