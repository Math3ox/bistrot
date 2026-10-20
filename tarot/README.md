# Tarot en 3D isométrique (Rust, Windows / Linux / macOS)

Tarot à 3, 4 ou 5 joueurs contre des bots (ou en ligne avec des amis), selon les règles de la Fédération française de
tarot : table ronde en perspective dans une taverne aux chandelles, cartes enluminées (78 cartes : atouts sur fond
outremer, bouts sur fond pourpre), animations. Construit sur le moteur commun `bistrot` : dessin **entièrement par le
processeur**, sans carte graphique, **jamais plus de 50 Mo de RAM** (environ 26-30 Mo en fenêtre, 42 Mo au plus en
plein écran 1080p).

## Lancer
Double-clic sur `tarot.exe` (aucune installation). Sous Linux : `./tarot`. Les réglages sont mémorisés dans `tarot.cfg`.

## Règles (FFT)
- **78 cartes** : 4 couleurs de 14 cartes (1 à 10, Valet, Cavalier, Dame, Roi), 21 atouts, l'Excuse.
  Les **bouts** : le Petit (atout 1), le 21 et l'Excuse.
- **Donne** : à 4, 18 cartes chacun et un chien de 6 ; à 3, 24 cartes et un chien de 6 ; à 5, 15 cartes et un chien de 3.
  Un joueur qui a le Petit sec (seul atout, sans l'Excuse) fait redonner.
- **Enchères** (un tour, chacun parle une fois, plus haut que le précédent) : **Petite** x1, **Garde** x2,
  **Garde sans le chien** x4, **Garde contre le chien** x6. Si tout le monde passe, on redonne.
- **À 5 joueurs**, le preneur **appelle un roi** (une dame s'il a les quatre rois) : son détenteur est son partenaire,
  en secret jusqu'à ce que la carte tombe. On n'entame pas le premier pli dans la couleur appelée, sauf avec le roi.
- **Chien** : à la petite et à la garde, il est montré à tous, le preneur le prend et fait son **écart** (ni roi ni bout ;
  des atouts seulement s'il ne peut pas faire autrement, et ils sont montrés). Garde sans : le chien compte pour le
  preneur sans être vu ; garde contre : il compte pour la défense.
- **Jeu** : fournir la couleur demandée ; sinon couper ; à l'atout, monter sur le plus fort atout du pli si on le peut.
  L'**Excuse** se joue à tout moment, ne gagne pas le pli et reste à son camp (qui donne une carte d'un demi-point en
  échange) ; jouée au dernier pli elle est perdue, sauf si son camp a fait tous les autres plis (chelem).
- **Valeur des cartes** : bouts et rois 4,5 ; dames 3,5 ; cavaliers 2,5 ; valets 1,5 ; autres 0,5 (91 points).
  Le preneur doit faire **56** points sans bout, **51** avec 1, **41** avec 2, **36** avec 3.
- **Décompte** : une part vaut (25 + écart avec le contrat) x multiplicateur, + 10 x multiplicateur pour le **petit au
  bout** (le Petit au dernier pli, au camp qui le remporte), + **poignée** (simple 20, double 30, triple 40, au camp qui
  gagne le contrat ; à 4 : 10, 13 ou 15 atouts, l'Excuse compte comme un atout), + **chelem** 200. À 4, le preneur
  gagne ou perd 3 parts, chaque défenseur une ; à 3, 2 parts ; à 5, le preneur 2 parts et son partenaire 1 (4 parts s'il
  joue seul). Le bilan de chaque donne affiche tout le calcul.
- La partie se joue en 10 donnes (touche `P` : 5 / 10 / 15 / 20) ; le meilleur total gagne.

## Jouer
- **Enchères** : boutons PASSE, PETITE, GARDE, GARDE SANS, GARDE CONTRE (ceux qui ne sont plus permis sont grisés).
- **Appel** (à 5) : un bouton par roi (ou dame) possible.
- **Écart** : le chien est montré, puis rejoint ta main : clique sur les cartes à écarter (elles se lèvent, cerclées
  d'or ; les cartes interdites sont grisées), puis VALIDER L'ÉCART (ou Entrée).
- **Jeu** : clique sur une carte (les cartes grisées sont injouables). Avant ta première carte, si tu as assez d'atouts,
  le bouton MONTRER LA POIGNÉE la déclare avec cette carte.
- Raccourcis : `J` 3 / 4 / 5 joueurs · `L` niveau des bots · `P` nombre de donnes · `H` aide · `F` aide-mémoire ·
  `M` animations · `F11` plein écran · `F12` capture `tarot_shot.png` · `N` en ligne · `Échap` quitter.
- `H` (aide) : contrat conseillé pendant les enchères, carte conseillée surlignée en or pendant le jeu.
- `F` : aide-mémoire (valeur des cartes, points à faire, contrats, poignées).
- `M` : animations auto → oui → non (`non` pour un processeur léger ; `auto` les coupe tout seul si le processeur ne
  suit pas). `G` : galerie des 78 cartes. `U` : l'ordinateur joue à ta place.

## Niveaux des bots (touche `L`)
Les bots ne trichent pas : ils ne voient que leur main, les cartes jouées, les coupes montrées, le chien quand il a été
montré, leur écart et les atouts écartés.
- **facile** : le jeu normal avec une carte au hasard de temps en temps, enchères bruitées.
- **normal** : barème de club pour évaluer la main (bouts, atouts, honneurs, coupes, longues) ; seuils réglés au banc
  d'essai pour 3, 4 et 5 joueurs ; écart qui fait des coupes et met les points à l'abri ; fournir au plus juste, couper
  et monter au plus juste, charger quand le camp est maître, sauver l'Excuse, garder le Petit pour le dernier pli quand
  il n'y a plus d'atouts dehors, faire couper le preneur en défense, chercher le partenaire dans la couleur appelée.
- **difficile** : Monte-Carlo : 24 donnes plausibles tirées au sort pour chaque décision (selon ce que le bot sait),
  jouées jusqu'au bout pour chaque carte possible ; il garde celle qui rapporte le plus. Au banc d'essai, il gagne
  environ 19 points de plus par donne qu'un bot normal. Moins de 20 ms de réflexion par carte.

## En ligne (touche `N`)
Comme la belote : un joueur héberge (H), les autres rejoignent (R) avec son adresse IP (port TCP 4210). De 2 à 5
humains selon le nombre de joueurs choisi par l'hôte ; l'hôte démarre avec Entrée, les places libres sont tenues par
des bots. Seules les actions circulent (enchère, roi appelé, écart, carte et poignée, suite), 12 octets chacune.

## Tests
- `cargo test --release -p tarot` : parties complètes en mode automatique à 3, 4 et 5 joueurs, et lockstep entre 3, 4
  ou 5 machines de vitesses différentes.
- Banc d'essai : `tarot -sim <parties> <graine> <joueurs> <niveau du siège 0> <niveau des autres> <journal 0|1>
  [tirages] [seuils petite garde sans contre] [contrat imposé]` : statistiques (contrats, réussite par contrat et par
  valeur de main, gains par siège, temps de réflexion) ; avec le journal, écrit `sim.log`, puis
  `python tarot/check.py sim.log` le rejoue avec une **réimplémentation indépendante** des règles (enchères, appel,
  écart, cartes jouables, poignées, gagnant des plis, excuse, petit au bout, chelem, décompte de chaque donne).
- `cargo test --release -p tarot -- --ignored apercus` : dessine sans fenêtre les écrans (enchères, chien, écart, jeu,
  poignée, appel à 5, bilan, classement, galerie) dans `target/apercu_tarot_*.png`.

## Fichiers
- `src/engine.rs` : règles (donne, enchères, appel, chien et écart, cartes jouables, plis, excuse, décompte), journal.
- `src/bots.rs` : bots facile / normal, Monte-Carlo (difficile).
- `src/sim.rs` : banc d'essai `-sim`. `check.py` : vérificateur indépendant.
- `src/table.rs` : déroulement (délais, bulles, chien montré, écart à la souris, poignée, actions en ligne,
  `bistrot::reseau::Partie`), places autour de la table.
- `src/vue.rs` : ce qui bouge (distribution, chien retourné puis envoyé au preneur, main, bulles, scores).
- `src/draw.rs` : dessin (chien, mains, plis, panneaux, bilan, boutons, aide-mémoire, galerie).
- `src/main.rs` : le jeu dans sa fenêtre (`bistrot::shell::Game`), `-sim`. `src/cfg.rs` : réglages (`tarot.cfg`).
