# Bistrot : les jeux de bistrot en 3D isométrique (Rust)

Un moteur commun et les jeux construits dessus :

| Dossier | Contenu |
|---|---|
| `moteur/` | crate `bistrot` : fenêtre, dessin 100 % processeur, cartes, table, animations, interface, jeu en ligne, réglages |
| `belote/` | belote à 4 contre des bots, belote à deux, en ligne de 2 à 4 joueurs (voir `belote/README.md`) |
| `421/` | 421 contre un bot, à deux sur le même PC, duel en ligne, mode terminal (voir `421/README.md`) |
| `tarot/` | tarot (règles FFT) à 3, 4 ou 5 joueurs contre des bots, en ligne de 2 à 5 joueurs (voir `tarot/README.md`) |
| `outils/` | `shot.py` (pilote un jeu et fait des captures), `screencap.py`, `tools.py` |

Les premières versions (assembleur, puis Rust séparé) restent dans les dépôts `Math3ox/421` et `Math3ox/belote`
avec leurs releases v1.x et v2.0.

## Le moteur (`moteur/`, crate `bistrot`)

- **Dessin sans carte graphique** (`gfx`, `shell`) : tiny-skia pour les formes anticrénelées, ab_glyph pour le texte,
  winit pour la fenêtre ; l'image est affichée directement par GDI sous Windows (softbuffer sous X11 / Wayland).
  Le fond (décor + ce qui ne bouge pas) est gardé en cache et refait seulement quand il change ; les images ne sont
  calculées que si quelque chose bouge (60 images/s au plus pendant les animations, 0 % de processeur au repos).
  Dégradés linéaires et circulaires (lumière des chandelles, bois, parchemin) sur n'importe quelle forme.
- **Jamais plus de 50 Mo de RAM par jeu** : deux images seulement (fond et image en cours), résolution interne
  plafonnée à ~2,2 millions de pixels (au-delà, sur un écran 1440p ou 4K, l'image est calculée plus petite puis
  agrandie par GDI), cache des cartes limité à 7 Mo, pas de police de symboles (♠ ♥ ♦ ♣ dessinés en formes), et un
  garde-fou qui mesure la mémoire du processus toutes les 2 s et vide les caches puis baisse la résolution s'il
  approche de la limite. Le décor de la taverne (~70 ms de dessin en fenêtre, ~130 ms en plein écran) est gardé en
  niveaux de lumière, 1 octet par pixel (0,9 Mo en fenêtre, 2 Mo en plein écran), et recopié en 1 à 2 ms ; les
  polices ne sont lues qu'à leur première utilisation. Mesures (Windows, écran 1080p) : belote 25-28 Mo en fenêtre,
  40 Mo au plus en plein écran pendant une partie ; 421 ~34 Mo en plein écran.
- **Coordonnées logiques 1100 x 800** mises à l'échelle de la fenêtre (proportions gardées, plein écran, clics justes).
- **Cartes** (`cartes`) enluminées : vélin dégradé, ombre douce, cadre doré et fleurons, indices en Palatino dont
  celui du bas est retourné comme sur un vrai jeu, couleurs en formes vectorielles aux encres profondes en dégradé,
  points du bas retournés, figures à double tête (roi couronné et barbu au sceptre, dame à la fleur, valet au
  béret et à la hallebarde, cavalier casqué) sur fond losangé d'or outremer ou pourpre, as dans un anneau d'or
  perlé, dos pourpre à losanges dorés et médaillon à la fleur de lys. Jeux de 32 et 52 cartes, jokers, tarot
  (cavaliers, atouts 1 à 21, excuse) : `Carte::jeu32(i)`, `Carte::jeu52(i)`, `Carte::tarot(i)`. Chaque carte est
  dessinée à la première utilisation, en taille normale et en petite taille (cartes vues de loin).
- **Taverne** (`table`) : plancher de grosses planches dans la pénombre, table ronde en chêne cerclée de fer et
  rivetée, deux chandelles qui éclairent la table (décor en cache : `table::Decor`) ; places de **2 à 5 joueurs**
  (main, pli, bulle, étiquette), main du bas en éventail qui se resserre pour les longues mains, dos des cartes
  adverses.
- **Animations** (`anim`) : carte jouée envoyée en arc avec son ombre et ramassée vers le gagnant ; carte distribuée
  depuis le paquet du donneur, qui se retourne en arrivant dans la main du joueur ; valeurs qui rejoignent leur
  cible en douceur (`approcher` : main qui se réarrange, carte qui se soulève, score qui défile) ; apparition qui
  rebondit (`rebond` : bulles).
- **Interrupteur des animations** (`anim::reglage`, touche `M` des jeux, mémorisé dans leurs réglages) : `auto`,
  `oui` ou `non`. Coupées, toutes les aides d'animation rendent directement l'état final (carte posée, valeur
  atteinte) : seules les images où la partie change sont calculées (une partie de belote en
  automatique passe de ~17 % à ~3 % d'un cœur, 0 % au repos). En `auto`, la fenêtre chronomètre les images animées
  et coupe tout si elles prennent plus de 25 ms en moyenne (processeur léger) ; la ligne des réglages affiche alors
  « animations coupées ».
- **Iso 3D** (`iso`) : projection isométrique, cylindres, disques et texte couchés sur le tapis (jeux de dés).
- **Interface** (`ui`, `polices`) : parchemins (panneaux, étiquettes des joueurs, bulles animées), grands panneaux
  en parchemin roulé à double filet rouge, boutons de parchemin cerclés d'encre rouge et éclairés au survol, pièce
  d'or du donneur ; encres sur le parchemin, texte clair sur le bois. Polices du système : titres en gothique (Old
  English, livrée avec Office ; sinon Palatino gras), textes en Palatino Linotype (livrée avec Windows) ; sous
  Linux P052 ou TeX Gyre Pagella, ou ce que `fc-match` propose.
- **Jeu en ligne** (`reseau`) : « lockstep » en étoile, TCP port 4210, l'hôte relaie ; de 2 à 5 joueurs, places
  libres tenues par des bots ; seules les actions circulent (12 octets) ; un invité refuse un hôte qui joue à un
  autre jeu ou à une autre version.
- **Réglages** (`reglages`) : fichier texte « clé = valeur » à côté de l'exécutable.

## Ajouter un jeu (rami...)
Le tarot (`tarot/`) est l'exemple le plus complet : règles en ensembles de cartes sur 128 bits, bots qui ne
trichent pas, Monte-Carlo, vérificateur indépendant, aperçus sans fenêtre.

1. Créer un dossier `rami/` avec un `Cargo.toml` qui dépend de `bistrot.workspace = true` et l'ajouter à
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
  `C:\msys64\ucrt64\bin`). Produit `belote.exe`, `421.exe` et `tarot.exe` à la racine.
- Linux / macOS : `cargo build --release`, puis `target/release/belote`, `target/release/421` et `target/release/tarot`.

## Tests
- `cargo test --release` : moteur (trames), belote (parties automatiques, lockstep à 4 machines, vraie partie TCP à 4),
  421 (règles, bots, lockstep, vrai duel TCP), tarot (parties automatiques et lockstep à 3, 4 et 5 machines).
- Belote : `target/release/belote -sim <parties> <graine> <joueurs> <niveau0> <niveau1> 1` puis `python belote/check.py`
  (réimplémentation indépendante des règles). 421 : `python 421/bottest.py` (bot difficile = référence Python).
  Tarot : `target/release/tarot -sim <parties> <graine> <joueurs> <niveau0> <niveau1> 1` puis
  `python tarot/check.py sim.log` (réimplémentation indépendante des règles et du décompte).
- Captures : `python outils/shot.py <belote|421|tarot> sortie.png [actions...]` (ne pilote que la fenêtre qu'il lance).
  Sans fenêtre : `cargo test --release -p belote -- --ignored apercus` dessine les enchères et le bilan de manche
  dans `target/apercu_*.png` ; `-- --ignored mesure_dessin --nocapture` donne les temps de dessin ; tarot :
  `cargo test --release -p tarot -- --ignored apercus` (`target/apercu_tarot_*.png`).
