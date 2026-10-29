# Slimy : mémoire du projet

## Décisions

- Le slime est un compagnon de bureau en priorité. Le web n'est pas une cible du MVP.
- Corps de gel réaliste, jamais sticker ni humanoïde. Références : Rimuru, Slime Rancher, planches kawaii de l'auteur.
- Évolution continue par génome (pas de tempéraments figés), nourrie par la journée d'interaction et digérée la nuit.
- Le slime n'a que les pouvoirs accordés par l'utilisateur. Observation toujours visible.
- Stack choisie par le builder (Rust, WebGL2, Tauri, React, daisyUI, SQLite). Go reporté. L'auteur a délégué ce choix.
- Le slime se déplace sur tout l'écran, pas dans une bande basse (réponse de l'auteur). Fenêtre mobile de petite taille plutôt que calque plein écran.
- Design écrit à partir du brainstorming du 2026-10-24.

## Pièges connus

- Wayland : positionnement de fenêtre et clics traversants limités. Spike T0 obligatoire avant tout le reste.
- Le gel ne peut pas réfracter le vrai bureau derrière la fenêtre.

## Résultats du spike T0 (Fedora 44, session Wayland, 2026-10-24)

- Wayland natif (`GDK_BACKEND=wayland`) : `set_position` n'a aucun effet, la fenêtre reste en (0,0). Inutilisable pour une fenêtre mobile.
- XWayland (`GDK_BACKEND=x11`) : la fenêtre suit la trajectoire demandée (écart de lecture jusqu'à 316 px pendant les déplacements rapides, retard asynchrone). Fluidité et transparence réelles à confirmer à l'oeil par l'auteur.
- Piège trouvé : juste après le démarrage, X11 renvoie une taille de fenêtre fausse (hauteur énorme). Une trajectoire calculée avec cette taille envoie la fenêtre en haut puis en bas de l'écran en alternance. Il faut attendre une taille plausible avant de calculer des positions. `Monitor::work_area()` renvoie aussi des valeurs absurdes ici : ne pas s'en servir, la zone utile réelle est dans `_NET_WORKAREA` (0, 32, 1920, 1168 sur cette machine, barre GNOME de 32 px).
- Vérifié après correction : écart médian de 6 px entre la position demandée et la position réelle (trace horodatée de l'application comparée à `xdotool`).
- Décision : l'application force `GDK_BACKEND=x11` sous Linux si la variable n'est pas définie (`apps/desktop/src-tauri/src/main.rs`).
- Dépendances installées : webkit2gtk4.1-devel, libsoup3-devel, librsvg2-devel, openssl-devel, libappindicator-gtk3-devel, libxdo-devel, rust-std-static-wasm32-unknown-unknown ; `cargo install wasm-bindgen-cli`.
- Lancer le spike : `cargo build -p slimy-desktop --features custom-protocol` puis `SLIMY_SPIKE_SECONDS=12 target/debug/slimy-desktop`.

## Rulings du builder (2026-10-24)

- Rendu : shader WebGL2 (dôme éclairé à partir du contour) plutôt que Canvas 2D, après retour de l'auteur ("pas assez slime réaliste, ça glitche"). Coût si faux : `gel.js` seul, API `draw(frame)` stable.
- Amortissement : traînée globale faible (0.35) et amortissement interne fort (5.5). Avec un seul amortissement le slime tombait à 170 px/s (flottant). Test `falls_freely_instead_of_floating`.
- Respiration : le corps ne s'immobilise jamais complètement (vitesse résiduelle inférieure à 8 px/s), c'est voulu.
- Fenêtre : couvre la zone utile, sprite CSS qui suit le centroïde. La version fenêtre mobile 380 par 320 tremblait (déplacement asynchrone). Pas fixe 120 Hz pour la simulation (le dt variable des images rendait le mouvement irrégulier).
- Rest mode avec hystérésis (entrée sous 25 px/s pendant 0.8 s, sortie au dessus de 60 px/s ou au toucher) : sans hystérésis la respiration (jusqu'à 17 px/s) empêchait le repos et le CPU restait à 33%. Mesuré 10.3% au repos avec sprite et repos à 20 images par seconde.
- Mesures CPU (anciennes) (processus Tauri plus WebKitWebProcess, % d'un coeur, slime immobile) : avant optimisation 44.6%. Avec mode repos (20 images par seconde après 0.8 s de calme, pas d'IPC si la fenêtre ne bouge pas) : 9.2% à 25.6% selon les essais, la variation vient surtout de la souris qui survole ou non le slime. Version release non meilleure que debug dans les essais (38 à 45%), à réexaminer : le coût dominant est le rendu Canvas 2D et la composition de la fenêtre transparente, pas le Rust. Script de mesure à refaire si besoin (lecture de /proc/PID/stat sur 10 s).
- Surfaces : le slime se colle au bord qu'il touche lentement (vitesse d'impact sous 420 px/s et vitesse moyenne vers le bord pas trop négative), la gravité tire vers ce bord, le corps pivote. Il se détache après 0.45 s sans contact. Le plafond colle aussi (futur gène d'adhérence).
- Déchirure : un tirage violent faisait voler des points (outline en pointes). Garde-fous : traction de saisie plafonnée à 1.1 rayon, portée max du contour 2.1 rayon, test `a_violent_drag_stretches_the_gel_without_tearing_it`.
- Quitter : clic droit sur le slime, Quitter Slimy (ou Ctrl+C dans le terminal).
- Ancien : Ctrl+C dans le terminal qui l'a lancé (pas encore de menu, tâche à venir).

- Lumière : le film de gel à la base n'existe que si le corps touche sa surface (`surface_gap` du corps, fondu sur 22 px), sinon il restait collé sous le slime quand on le soulevait. Le halo extérieur s'éteint avant le bord du quad (sinon rectangle visible). Visage : saisi = yeux ronds et grands plus bouche en o (surpris), écrasé = yeux en arche (impact), plus de ">" "<".
- Rendu du gel à 0.65 de la résolution (étiré par la page) pour limiter le coût du contour par pixel. Le CPU au repos reste bruité (20 à 34%) et reste à traiter (T1.4) : piste principale, texture de distance calculée une fois au lieu de 112 segments par pixel.

- Souris : sous Wayland la position globale n'est pas lisible (GDK et xdotool restent figés hors des fenêtres X11, constaté en déplaçant le curseur). Les yeux suivent donc la souris au dessus de la zone du slime ; ailleurs ils flânent. La commande `pointer_position` reste pour les sessions X11. Une vraie solution globale demanderait le portail ScreenCast (curseur dans les métadonnées) ou une extension GNOME Shell.
- Visage : ressorts (centre, deux yeux, bouche) avec traînée selon la vitesse du corps et inertie selon son accélération, contenu dans le corps. Tests : `cd apps/desktop/web && node --test tests/face.test.mjs`.
- Modèle de données : `models/base-slime.json`, normalisé par `model.js`. Le corps Rust reçoit forme et respiration par `set_shape` et `set_breathing`.

- Rotation libre : le corps estime sa rotation réelle (moindres carrés contre la forme de repos) et la suit, avec un rappel vers l'aplomb de la surface plafonné à 0.1 rad par pas (fort au contact 0.35, faible en l'air 0.06, très faible quand il est tenu 0.04). Résultat : tenu par le côté, il pend et s'incline comme une goutte (test `a_body_lifted_by_its_side_hangs_and_tilts`), tenu par le haut il reste droit, relâché il se redresse. Les points saisis gardent leur position dans le repère du corps, donc la zone tenue peut pivoter autour de la souris.
- Les forces internes (ressorts, lissage, pression) sont rendues de somme nulle : elles ne poussent plus le corps entier (le slime glissait le long du mur en pivotant). Adhérence de surface (7 par seconde) et 85% de la vitesse tangentielle absorbée à l'atterrissage sur une nouvelle surface. Un choc rapide (au dessus de 420 px/s) interdit de se coller à cette surface pendant 0.3 s.
- Ombrage : dégradé et lueur suivent la gravité du monde, le visage et le film de base suivent l'inclinaison du corps.

- Souris globale sous GNOME Wayland : seule solution fiable, une extension Shell minuscule (`apps/desktop/gnome-extension`, méthode D-Bus `GetPosition` en lecture seule). Le backend l'utilise si elle est là (client D-Bus vérifié contre un service factice, valeur lue 123, 456), sinon GDK. Elle s'installe avec `apps/desktop/gnome-extension/install.sh` et demande de se reconnecter une fois (Wayland ne découvre une extension qu'au login). Non installée automatiquement : elle s'exécute dans le Shell et change la configuration de l'utilisateur. La même extension servira pour les fenêtres (s'accrocher aux bords).
- Expressions : machine à états `expressions.js` (tests `tests/expressions.test.mjs`), dessin dans `face.js`. 10 tests web : `cd apps/desktop/web && node --test tests/face.test.mjs tests/expressions.test.mjs`.

- Faux positifs d'amour : un curseur immobile sur la zone du slime (ou le slime qui atterrit sous un curseur fixe, WebKit renvoie alors des mouvements synthétiques) déclenchait les coeurs. Maintenant seul un pointeur en mouvement (plus de 40 px/s) compte comme caresse, et le contenu de la pose attend 0.4 s avant un changement d'humeur doux (sauf surpris, joie, aïe, étourdi). Sortie de zone retardée de 160 ms, et le regard garde la dernière position vue 6 s.
- Gel rendu à pleine résolution : à 0.65 le contour grésillait quand le sprite bougeait.

- Naissance : `birth.js` (histoire, testée), `ritual.js` (oeuf, éclosion, bulle), `dialogue.js` (bulle de verre, texte inséré en texte seulement), `i18n.js` (fr, en), `egg.js` (fissures, étincelles). L'âme est écrite par la commande `soul_create` (validation Rust, fichier `soul.json` en 0600 dans `~/.local/share/dev.slimy.desktop/`, sauvegarde `.bak` de la version précédente, écriture atomique). Une âme illisible n'est jamais écrasée : la naissance est alors sautée et l'erreur journalisée.
- Refaire la naissance sans toucher à ses vraies données : `XDG_DATA_HOME=/tmp/slimy-test target/debug/slimy-desktop`.
- Pièges : `display:flex` d'une classe écrase l'attribut `hidden` (formulaire resté visible, règle `[hidden]` ajoutée). `node --test tests/` ne marche pas, utiliser `node --test tests/*.test.mjs`.
- Pages de contrôle sans Tauri : `web/dev/preview.html` (corps, expressions) et `web/dev/ritual-preview.html?stage=cracked|hatch|ask|welcome` (naissance, avec un faux `invoke`), capturées avec Chrome sans écran.
- Repos : images à 12 par seconde (30 si le visage bouge, 6 en somnolence), contour à 56 points, marge du quad 20 px, sondage souris espacé au repos. Mesure : 10 à 11% au repos (trois essais), un essai isolé à 7%.

- Fluidité (mesures dans l'application pendant la naissance) : images à 17 ms au médian et 18 à 20 ms au 95e centile, pire cas 29 ms ; retard des minuteurs de la page 2 à 4 ms (la page n'est pas bloquée) ; aller retour vers le fil principal GTK 3 ms au médian mais 14 ms au 95e centile et 20 au pire (le fil principal a des à-coups, c'est ce qui touche la saisie). Réponses : région d'entrée limitée à 10 changements par seconde (marge proportionnelle à la vitesse), bulle sur sa propre couche et qui glisse (lissage, taille mise en cache, plus de lecture de mise en page par image), mouvement interpolé entre deux pas de physique (plus de saccade quand une image dure plus longtemps), plafond à 75 images par seconde, champ de saisie sélectionné tout de suite, question posée 0.4 s plus tôt.
- Si la saisie reste lente : voir la méthode d'entrée du système (IBus ou Fcitx) pour les fenêtres XWayland, et mesurer de bout en bout (touche enfoncée jusqu'au caractère affiché).

- Vie quotidienne : `slimy-core` porte `Relation`, `Sulk` et `Initiative` (22 tests), exposés au navigateur par la classe `Companion` de `slimy-wasm` ; `life.js` fait le lien avec la bulle et le fichier d'âme (commandes `soul_learn`, `soul_feelings`). Les sentiments sont sauvegardés toutes les 60 s s'ils ont bougé. Un fichier d'âme ancien sans sentiments reçoit des valeurs neutres.
- Bug trouvé par le test de démarrage : `window.set_focus` était refusé (permission `core:window:allow-set-focus` absente de `capabilities/default.json`). La fenêtre ne prenait donc jamais le focus clavier pour la saisie, ce qui expliquait au moins une partie de la gêne à la frappe. Corrigé. Leçon : lire le journal de `log_error` après chaque ajout d'appel Tauri.
- Pages de contrôle : `web/dev/life-preview.html?stage=question|sulk`.

- Initiative motrice : `slimy-core::Autonomy` (décisions pures, graine, 10 tests) pilote `SoftBody::hop` et `crouch` (formule de saut : puissance = racine de 1170 fois la distance, mesurée sur le vrai corps). Test d'intégration `crates/slimy-wasm/tests/roam.rs` avec le vrai corps : explore, reste à l'écran, boude vers un coin du bas, dort dans un coin, grimpe à un mur. Au repos le dessin tourne à faible cadence, donc l'accroupissement est mis à l'échelle du temps écoulé.
- Cerveau : fournisseurs dans `crates/slimy-brain/src/provider.rs`. Groq par défaut avec `openai/gpt-oss-120b` depuis le 2026-10-27 (le 20b se trompait parfois de pouvoir et inventait ; le 120b a demandé le bon pouvoir 6 fois sur 6 en essai réel ; seuls modèles de chat accessibles avec la clé de l'auteur le jour du test : openai/gpt-oss-120b, openai/gpt-oss-20b, qwen/qwen3.8-27b, allam-2-7b ; `llama-3.3-70b-versatile` n'existe plus pour elle). Un modèle de raisonnement reçoit `reasoning_effort: low` et au moins 700 jetons de sortie, sinon la réponse est vide. Bedrock : clé d'API Bedrock (Bearer) et région, pas encore la signature AWS (SigV4). Adresses personnalisées : https ou machine locale seulement.
- Clés d'API : trousseau du système (Secret Service via la crate oo7), jamais dans un fichier ni dans le dépôt, jamais renvoyées à la page. Repli en lecture : variables `SLIMY_KEY_<FOURNISSEUR>` ou les noms habituels (GROQ_API_KEY, OPENAI_API_KEY, ANTHROPIC_API_KEY, DEEPSEEK_API_KEY, OPENROUTER_API_KEY, XAI_API_KEY, AWS_BEARER_TOKEN_BEDROCK). Outils : `slimy-desktop set-key <fournisseur>` (clé lue sur l'entrée standard) et `slimy-desktop brain-test`.
- La clé Groq de l'auteur a été collée dans la conversation : à régénérer sur la console Groq, puis `set-key groq` ou panneau Cerveau.

## Commandes utiles

- Tests : `cargo test -p slimy-body`
- Build web : `./scripts/build-web.sh` puis `cargo build -p slimy-desktop --features custom-protocol`
- Lancer : `target/debug/slimy-desktop` (GDK_BACKEND=x11 forcé) Prototype jetable de la sensation de toucher : `.superpowers/brainstorm/*/content/gel-touchable.html`, à ne pas livrer. Ajouter `.superpowers/` au `.gitignore`.

## Reprise du 2026-10-25

- État initial conservé dans le commit `8053c12` après 86 tests Rust et 34 tests web réussis.
- Coordination : `face.js` ne transmettait pas `intent` et `crouching` à la machine d’expressions. Transmission rétablie ; préparation et élan sans délai de maintien, pas de douleur due à l’accroupissement ni de vertige dû au mouvement autonome. Suppression de la langue tirée automatiquement toutes les quatre promenades.
- Déplacements : annulation du saut préparé au repos, au changement vers un coin de sommeil ou de bouderie, à la perte de stabilité et au plafond. Distance des petits sauts limitée à la destination, sans minimum de 55 px.
- Vérification : `cargo test -p slimy-core -p slimy-wasm --quiet` (41 tests), `node --test tests/*.test.mjs` depuis le dossier web (37 tests), `cargo clippy -p slimy-core -p slimy-wasm --lib -- -D warnings`, `./scripts/build-web.sh`, `cargo build -p slimy-desktop --features custom-protocol`, `git diff --check`. Six nouveaux tests ont échoué avant correction puis réussi. Ressenti visuel sur le bureau non vérifié.

## Coins et coordination du 2026-10-25

- Reproduction physique : dans le coin bas gauche, 1 200 changements sol/mur en 10 s au repos. La surface courante reste prioritaire tant qu’elle est touchée, sauf départ net (vitesse vers l’extérieur supérieure à 60 px/s), ce qui conserve l’escalade. Test symétrique des deux coins.
- Séparer appui (`supported`) et immobilité (`settled`) : l’accroupissement accélère les points sans perdre l’appui. Annuler sur la vitesse empêchait les sauts à 6 images/s. Le budget de rattrapage de la page passe à 0.25 s pour couvrir les images espacées de 1/6 s sans ralentir la physique.
- Une balade qui arrive sur un mur redescend ; le suivi du curseur expire après 8 à 16 s. La désactivation de la balade informe le moteur et annule la préparation. Le sommeil attend l’arrivée stable au coin. Le regard est converti dans le repère tourné du corps.
- La douleur utilise le maximum des chocs physiques de l’image, transmis du corps Rust au visage via WASM. Une compression statique et une balade interrompue ne déclenchent plus de réaction injustifiée.
- Vérification : `cargo test --workspace --quiet` (94 tests), `node --test tests/*.test.mjs` dans le dossier web (42 tests), Clippy des bibliothèques corps/cœur/WASM sans avertissement. Les deux coins restent au repos 30 s après un trajet de 120 s à 6, 12 et 60 images/s ; escalade conservée. Captures Chrome des deux coins après 90 s avec les vrais modules WASM/rendu : corps droit, état `nap`, gravité vers le sol. Interaction native GNOME non vérifiée.

## Locomotion et suivi du curseur

- `roam.update` est appelé dans la boucle physique de 120 Hz, avant chaque pas du corps. Les trajectoires et nombres de sauts sont identiques pour des rendus à 6, 12, 30, 60 et 120 images/s. Les bonds ordinaires passent de 190 à 110 px maximum. Le rendu reste actif pendant une balade, une préparation et les mouvements récents du pointeur.
- Yeux : séparation préservée sous accélération forte (le test reproduisait deux yeux espacés de moins de 4 px), taille moins dépendante de l’étirement, clignement adouci. Préparation et élan gardent les yeux ovales pour éviter de les fermer à chaque pas. Le regard est séparé du ressort d’inertie : le test de réponse à 100 ms passe, auparavant seulement 21% du déplacement était parcouru.
- Suivi : les positions globales étaient toujours marquées inactives ; leur activité est maintenant transmise au moteur. Sondage à 33 ms en activité, 100 ms au repos, sans requêtes concurrentes. La commande native de lecture utilise `spawn_blocking` pour ne pas bloquer le fil UI pendant D-Bus/GDK. Le pont GNOME répond sur cette machine.
- Pendant le suivi, les yeux regardent la position réelle du curseur plutôt qu’une direction latérale fixe. Un changement de côté annule puis prépare le saut dans la bonne direction.
- Vérifications : `cargo test --workspace --quiet` (96 tests), `node --test apps/desktop/web/tests/*.test.mjs` (44 tests), `node --check apps/desktop/web/main.js`, Clippy des bibliothèques corps/cœur/WASM, `./scripts/build-web.sh`, `cargo build -p slimy-desktop --features custom-protocol`, `git diff --check`. Aperçu `web/dev/preview.html?settle&roam` ajouté pour observer la locomotion avec les vrais modules. Captures du rendu contrôlées ; latence complète souris-écran dans GNOME non mesurée.

## Lumière au contact

- Le film lumineux tournait avec le corps et s’ajoutait au halo pulsant ; le bas recevait aussi trop de lumière interne et secondaire. Remplacés dans `gel.js` par une ombre douce et une faible teinte réfléchie. Assombrissement local de la base, lumière secondaire et caustiques atténuées.
- `render.js::contactPatch` calcule le plan et la largeur d’appui depuis les points du corps, la normale de la surface et la distance au contact. L’ombre suit le sol, le mur ou le plafond, indépendamment de l’inclinaison du corps.
- Vérification : 44 tests web, `node --check apps/desktop/web/gel.js`, assertions de géométrie pour sol/mur/plafond avec distance, `git diff --check`, build desktop. Comparaison visuelle Chrome avant/après au sol et contrôle soulevé : bande lumineuse et halo supprimés. Rendu sur le bureau natif non contrôlé.

## Calibrage des bonds du 2026-10-25

- Mesure sur le vrai corps (rayon 64, accroupissement de 0.17 s puis bond, 10 minutes de vie à 120 Hz, graines 1 à 8) : la formule `puissance = racine(1170 x distance)` ne correspondait plus à la physique. Un bond demandé à 110 px en parcourait 35, et l'approche d'une destination se faisait en bonds décroissants de 14 à 5 px (504 bonds sur 1646 sous 12 px). En descendant d'un mur, le bond vers le bas pressé contre le sol produisait un choc de 420 à 447 px/s hors accroupissement, donc l'expression de douleur sur un mouvement autonome (45 images sur 8 vies).
- `slimy-core::behavior` : la puissance d'un bond au sol se lit sur une courbe mesurée (`HOP_CURVE`, interpolation linéaire, `hop_reach` en est l'inverse publique). Bond ordinaire de 36 px visés, avec une variation de 0.7 à 1.15 pour casser le rythme métronomique. Au dessus de 42 px la courbe devient raide et dispersée (régime différent à partir de 390 px/s) : ne pas la prolonger sans remesurer.
- Descente de mur (`descend_hop`) : bonds fermes (560) loin du sol, 380 à moins de 60 px, puis pas diagonal de côté (330, direction 0.8 / 0.6) à moins de 25 px du sol. Variantes mesurées et écartées : pas vers le haut ou vers l'extérieur du mur (le slime reste collé, 2000 à 3900 s sur les murs au lieu de 1100), bond vers le bas adouci (1 à 8 chocs).
- Résultats avant puis après, 8 graines x 600 s : micro-bonds 504 puis 15 ; distance moyenne d'un bond au sol 23.6 puis 32.3 px ; images de choc autonome 45 puis 0 ; temps passé hors du sol 1309 s puis 1096 s. Ressenti visuel sur le bureau non vérifié.
- Test sur le vrai corps `wandering_never_hurts_the_slime_and_never_creeps` (graines 3, 4, 5 sur 300 s) : échoue avec l'ancienne logique, réussit avec la nouvelle. Test du cœur `it_stays_calm_it_never_races_around` : plafond passé de 140 à 300 bonds en 10 minutes, l'ancien chiffre supposait un pantin qui parcourait 110 px par bond.
- Page : `main.js` ne capture plus les points avant chaque pas de physique mais seulement avant le dernier, et lit la position du pointeur une seule fois par image (moins d'allocations à 120 Hz). `roam.js` : le commentaire et le plafond de l'accroupissement dataient de l'époque où `update` était appelé par image.
- Vérification : `cargo test --workspace --quiet`, `node --test apps/desktop/web/tests/*.test.mjs` (44 tests), `cargo clippy -p slimy-core -p slimy-body -p slimy-wasm --lib -- -D warnings`, `./scripts/build-web.sh`, `cargo build -p slimy-desktop --features custom-protocol`. Piège : l'éditeur applique rustfmt à l'enregistrement, donc `behavior.rs` et ses tests ont été entièrement reformatés (diff plus gros que le changement réel).

## Bascules de surface, essai abandonné (2026-10-25)

- Mesure : 160 changements de surface sur 8 vies de 10 minutes, dont 72 à moins de 0.5 s du précédent (sol vers mur dans les coins bas après une descente). Un délai minimal de 0.15 s entre deux changements dans `slimy-body` les réduisait à 34, mais faisait réapparaître des chocs autonomes de plus de 420 px/s en fin de descente (4 à 12 selon la direction du pas de côté), alors que le code sans délai en donne 0. Abandonné et retiré. À reprendre seulement avec une mesure qui garde ces chocs à zéro.
- Cadence laissée telle quelle : la pause entre deux bonds est déjà irrégulière (0.17 à 0.63 s), le délai aléatoire de 0.5 à 1.1 s étant surtout absorbé par les 0.64 s de stabilisation du corps. L'allonger ralentirait le rythme général : décision à prendre avec l'auteur.

## Interaction mesurée le 2026-10-25 (aucun changement de code)

- Corps réel, rayon 64, 120 Hz : un survol à 60, 150 et 400 px/s déplace le corps de 0.7 à 1.5 px et il se calme en moins de 0.3 s ; un appui de 0.5 s ne dépasse pas 4 px/s de vitesse de point ; un lancer moyen (1254 px/s au lâcher) atterrit une fois avec un choc de 890 px/s, un geste vif (1882 px/s) une fois à 1499 px/s ; une chute de 700 px atterrit une fois (choc 969 px/s) et se calme en 3.0 s. Rien d'anormal (pas de rebonds en chaîne, pas d'oscillation longue). Le ressenti (détendu ou non) ne se lit pas dans ces chiffres : il demande un retour de l'auteur sur le bureau.
- Session Claude `df906ff8` (24 et 25 octobre) relue : sa dernière demande ouverte était la coordination mouvement et expressions, traitée depuis (T1.8 à T1.12) ; elle s'est arrêtée sur la limite hebdomadaire. Restes qu'elle listait : T4 (génome continu, digestion nocturne), CPU au repos vers 3% (T1.4), pseudopodes et contenu avalé, cerveau qui commente la vie, extension GNOME du pointeur (à installer par l'auteur, reconnexion requise), test des fournisseurs autres que Groq, Bedrock SigV4, export et import `.slime`, outils MCP, voix, relais Go.

## Génome et nuits dans `slimy-core` (2026-10-25)

- Modules : `genome.rs` (25 gènes dans [0,1], teinte circulaire, étapes de vie bébé à ancien avec plasticité 1.0 à 0.3 et taille 0.7 à 1.0, historique plafonné à 400 nuits), `harmony.rs` (palette OKLCH vers sRGB avec réduction de chroma pour rester dans le gamut sans changer la teinte ; règles : écart de teinte peau/noyau au plus 40 degrés, contraste du visage au moins 4.5), `night.rs` (`Genome::digest`, `Genome::replayed`), `rng.rs` (générateur à graine partagé avec `behavior.rs`, mêmes suites qu'avant).
- Le contraste du visage est calculé contre le milieu de la peau (la partie la plus sombre qu'il recouvre) avec un visage sombre fixe (OKLCH 0.2) : seul le gène de luminosité est monté pour réparer. Les valeurs de départ du bébé (luminosité 0.6, saturation 0.7, teinte 0.69) donnent un bleu proche de `base-slime.json`.
- Une nuit : pousse = signal x 0.02 x plasticité, plafonnée à 0.03 par gène ; mutation à 3% (une matière ou croissance d'accessoire absente passe à 0.2, jamais retirée) ; puis harmonie. Les cinq signaux (affection, énergie, curiosité, usage nocturne, ténacité) sont bornés à [-1,1] à l'entrée, NaN compris. Rien ne pousse le décalage du noyau ni la luminosité : l'harmonie ne répare donc que les génomes de naissance ou importés.
- Le génome stocké est assaini au chargement : valeurs bornées, nombre de gènes complété, indices inconnus retirés de l'historique, historique plafonné, harmonie appliquée. Ce que `.slime` devra faire à l'import.
- Vérification : 22 tests (`tests/genome.rs`) dont deux tests de propriété ; trois mutants volontaires (plafond retiré, harmonie neutralisée, bornage du gène retiré) sont tous détectés. `cargo test --workspace` : 122 tests ; `cargo clippy -p slimy-core --lib --test genome -- -D warnings` sans avertissement.
- À cette étape (branchement terminé ci-dessous) : rien de tout cela n'est encore visible, la page ne lit pas le génome (T4.5). La couleur et les gènes de forme n'ont pas été regardés à l'oeil.

## Contrôle natif de la lumière

- Build combiné courant recompilé avec `./scripts/build-web.sh` puis `cargo build -p slimy-desktop --features custom-protocol`. Contrôle Tauri/WebKit sur Xvfb 1024x768, âme de test et données isolées dans `/tmp/slimy-native-check-p_kumcb9`, mouvements du pointeur limités à cet écran virtuel.
- Captures natives `rest.png` et `moving.png` dans ce dossier : bande lumineuse absente, déplacement visible. Application toujours active à la fin, journal sans erreur. Le serveur Xvfb et l’application de test ont été arrêtés. Aucun défaut supplémentaire constaté ; pas de modification du shader pendant ce contrôle.
- `node --test apps/desktop/web/tests/*.test.mjs` : 59 tests réussis ; `git diff --check` réussi. Limite : Xvfb sans compositeur, donc la transparence sur un vrai fond GNOME reste à contrôler.

## Branchement du génome et clôture du jour (2026-10-25)

- T4.3 et T4.5 terminées : âme persistée avec génome et journée, modèle visuel dérivé au lancement, signaux internes comptés, digestion au changement de date locale, phrase du matin en français ou anglais.
- Corrections du cycle de vie : lots datés au moment des événements, conservation de leur date après échec, attente des enregistrements déjà en cours et vidage des nouveaux événements, clôture à minuit même sans activité. Le menu Quitter attend la sauvegarde ; un échec conserve la fenêtre et affiche une erreur. Le comptage est suspendu pendant cette fermeture.
- Le journal attend la fin du chat ou des questions, bloque les nouvelles questions pendant sa lecture et ne ferme pas un dialogue qui l’a remplacé.
- Vérification : `node --test apps/desktop/web/tests/*.test.mjs` (65 réussis), `cargo test --workspace --quiet` (148 réussis, 1 ignoré pour génération de fixture), `./scripts/build-web.sh`, `cargo build -p slimy-desktop --features custom-protocol`, `node --check apps/desktop/web/main.js` et `git diff --check`.
- Contrôle natif Tauri/WebKit sous Xvfb avec données isolées : une journée de la veille fait passer le compteur de nuits de 0 à 1, modifie 9 gènes et affiche la phrase du matin (`/tmp/slimy-native-check-p_kumcb9/rest_000.png`). Application active, journal sans erreur ; processus de test arrêtés.
- Limites : les nouvelles proportions sont appliquées au lancement suivant, les matières mélangées restent T4.2. Un arrêt forcé peut perdre le dernier lot de moins de 60 secondes. Le clic natif Quitter a ensuite été vérifié ci-dessous.

## Fermeture native et redémarrage le même jour (2026-10-25)

- Vérification dans Tauri/WebKit sous Xvfb, avec le fichier d’âme isolé du contrôle précédent. Clic droit sur le slime, sélection de Quitter au clavier : sortie normale (code 0), 4.407 secondes d’activité enregistrées avant le délai périodique de 60 secondes, journal sans erreur.
- Le redémarrage le même jour laisse le compteur de nuits à 1 : aucune seconde digestion. Application et serveur Xvfb de test arrêtés ; aucune donnée personnelle modifiée.
- Aucun correctif nécessaire. Harnais temporaire : `/tmp/slimy-native-check-p_kumcb9/check-quit.py`. Le refus de fermeture en cas d’erreur disque n’a pas été testé dans l’interface native.

## Investigation des bandes pendant la saisie (2026-10-26)

- Point de reprise : commit `15af190`. Les captures de l’auteur montrent des bandes horizontales coupant le slime près du champ de réponse ; la disparition du pointeur lui-même n’est pas confirmée.
- Contrôle supplémentaire : Weston 15 avec backend X11 sur Xvfb :117, XWayland interne :2, données synthétiques isolées, comparaison WebKit par défaut et `WEBKIT_USE_SKIA_FOR_COMPOSITION=0`. Harnais temporaire `/tmp/slimy-compositor-check/run.py`, captures `caret-default_000.png` et `caret-0.png` dans ce dossier.
- Test non concluant : Weston place la fenêtre plein écran vers (500, 300), l’injection du pointeur Xwayland échoue (`EI setup failed`) et le champ n’a pas été ouvert. Le découpage observé dans ce montage ne suffit pas à reproduire le défaut GNOME. Aucun contournement du compositeur appliqué au produit.
- Les applications de test et les serveurs virtuels ont été arrêtés ; aucun fichier d’âme personnel touché. Prochaine vérification nécessaire : saisie dans le champ sur GNOME avec comparaison des deux compositeurs, position de fenêtre et échelle identiques.

## Matières du génome visibles (2026-10-26)

- T4.2 terminée : `Look.layers` transmet rayures, miel, cristal, frange, lave et étoiles à `material.layers`. Le modèle borne chaque valeur et ignore les clés inconnues ; les anciens modèles sans couches gardent le gel pur.
- Shader : coordonnées centrées sur le corps, orientation physique distincte de la gravité de contact, motifs liés à la palette puis éclairés avec le gel. Somme des poids normalisée, aucune émission extérieure ni changement de collision. Les couches inactives évitent leurs calculs.
- Aperçu déterministe ajouté au mode `genome-preview.html?materials`, sans modifier les fixtures de génome. Comparaison Chrome/SwiftShader avec l’ancien shader : gel pur pixel-identique, différence visible pour chacune des six couches et leur mélange. Capture `/tmp/slimy-material-final.png`.
- Vérification : 67 tests web ; `cargo test -p slimy-core --quiet` (89 réussis, 1 ignoré) ; `cargo clippy -p slimy-core --lib --test genome -- -D warnings` ; build desktop avec `custom-protocol` ; syntaxe JS et `git diff --check`. Contrôle natif Tauri/WebKit sur Xvfb avec âme synthétique mélangée : rendu visible, application active, journal vide (`/tmp/slimy-material-native-4i37unwd/mixed.png`). Processus de test arrêtés.
- Limites : coût CPU des couches non mesuré sur le bureau ; frange rendue dans le gel, sans poils externes. T1.13 reste ouvert : ce travail ne résout pas les bandes observées sous GNOME.

## Mesure CPU et essais retirés (2026-10-26)

- Matières committées : `9c5f427`. Baseline native avec âme synthétique, balade désactivée par le menu, pointeur éloigné, Xvfb 1024x768 et `LIBGL_ALWAYS_SOFTWARE=1`. Deux fenêtres de 10 secondes après stabilisation, somme des ticks `/proc` du processus et de ses descendants : 11.08% puis 11.18% d’un cœur, trois processus. Harnais temporaire `/tmp/slimy-cpu-check.py`.
- Essai 1 : remplacer les rappels RAF sautés au repos par une attente interruptible au pointeur. 11.28% et 11.28%, pas de gain. Module et tests expérimentaux retirés.
- Essai 2 : court-circuiter la recherche du contour pour les pixels hors de sa boîte, en conservant l’ombre. Huit aperçus de matière pixel-identiques à la baseline, mais 11.97% et 11.47% CPU. Modification retirée.
- Le code produit reste celui du commit ; build desktop restauré. Vérifications des essais : 67 tests web sur l’essai shader, syntaxe JS, compilation native, captures et journal natif vide. Aucune amélioration de performance revendiquée. T1.4 reste ouvert ; prochaine action : profilage du processus WebKit pour distinguer simulation, dessin Canvas et composition, puis mesure sur GPU réel. Les chiffres logiciels isolés ne valident pas le budget GNOME.

## Attribution CPU par thread (2026-10-26)

- `perf` et `sysprof-cli` absents ; profilage limité aux ticks des threads dans `/proc/PID/task/TID/stat`, sans échantillonnage de piles. Même âme synthétique, balade désactivée, deux fenêtres de 10 s après stabilisation. Harnais `/tmp/slimy-cpu-threads.py` et `/tmp/slimy-cpu-gpu.py`.
- Skia par défaut, logiciel demandé : totaux 12.77% et 11.07% ; compositeur WebKit 4.69% et 4.19%, thread principal WebKit 3.09% et 2.89%, thread principal Tauri 1.70%. Le reste se répartit notamment entre files IPC et HeapHelper. Les noms de threads identifient des zones, pas les fonctions responsables.
- `WEBKIT_USE_SKIA_FOR_COMPOSITION=0`, mêmes conditions : 10.77% et 9.78% ; compositeur 2.89% et 2.79%, thread principal WebKit 3.09% et 2.89%. Gain local au compositeur, objectif 3% toujours non atteint.
- Essai sans `LIBGL_ALWAYS_SOFTWARE` : accès NVIDIA et renderD129 observés, mais avertissement Mesa indiquant DRI3 indisponible sous Xvfb. Totaux Skia 11.57% et 11.48%, alternative 10.98% et 10.08%. Ne pas présenter cette mesure comme un benchmark GPU réel. La variable LIBGL seule ne prouve pas non plus le chemin de rendu choisi par chaque pilote.
- Aucun changement au code ou à l’environnement de lancement du produit. Tous les processus de test arrêtés, données personnelles intactes. Prochaine mesure utile : même scénario sur le bureau GNOME avec accélération confirmée ; un profil par piles est nécessaire pour séparer le travail JS/WASM du reste du thread principal. Ne plus optimiser le shader sur la seule base du CPU global Xvfb.

## Coin et expressions (2026-10-26)

- Bug : le slime se cachait à 4 px du mur (marge 1.2 rayon, demi-largeur du corps environ 1.1). Marge portée à `EDGE_MARGIN` 1.5 dans `behavior.rs` ; test `hiding_in_either_corner_never_climbs_the_wall_or_jitters` (3 départs, 12 graines).
- `SoftBody::last_impact()` donne la vitesse de contact la plus forte du dernier pas ; `hop` ajoute une rotation autour du centroïde (HOP_LEAN 3.0, quantité de mouvement inchangée).
- Les expressions dépendent de `intent` et `crouching` venus de `roam.js` ; plus de minuteur aléatoire. Piège : un test de geste dans `behavior.rs` doit ignorer les ticks `Peek`, le regard s'y détourne volontairement du mur.

## Remarques spontanées (2026-10-26)

- `remarks.js` : événement discret envoyé comme tour utilisateur à `brain_chat` ; budget local (PER_HOUR 3, MIN_GAP 900 s, BACKOFF 1800 s sur erreur, sans message d'erreur). Couvert par `tests/remarks.test.mjs`. Vérifié : `brain-test` répond via Groq ; le déclenchement dans l'overlay réel n'est pas observé.

## Contrôle réel GNOME et marche trop lente (2026-10-26)

- Extension du pointeur active (`GetPosition` répond sur la session Wayland). Mesure sur le bureau réel avec une âme de test dans `XDG_DATA_HOME` temporaire (jamais dans le dossier de l'utilisateur) : 7 à 20% d'un cœur en balade, environ 7 à 10% posé, moyenne proche de 10%. L'objectif 3% n'est pas atteint, aucune amélioration revendiquée.
- Sans âme enregistrée le rituel de naissance tourne à 60 images/s en continu : mesure non représentative. Le dossier de données de l'utilisateur n'a pas de `soul.json` (naissance jamais terminée).
- Bug de marche : `HOP_CURVE` s'arrêtait à 42 px par saut, donc 500 px prenaient plus de 40 s. Courbe étendue avec les mesures du corps (400 donne 77 px, 500 donne 195 px, 600 donne 286 px), `HOP_MAX` 80 px ; `WALL_SPEED` baissé à 400 (à 450 le saut contre le mur frappait à 425, au dessus du seuil d'accroche 420). Test `a_walk_reaches_its_target_instead_of_pacing_around_it` : aucune marche de plus de 30 s sur 10 graines. `sweep_hop_reach` (ignoré) refait la mesure.
- Un autre processus édite les mêmes fichiers en parallèle : relire avant d'éditer.

## Pseudopodes (2026-10-26)

- Corps : bosse dirigée ajoutée à la forme cible (`Arm` dans `slimy-body`, ressort ARM_STIFFNESS 90 / ARM_DAMPING 9, léger dépassement). Les points du lobe se déplacent tous dans la même direction monde (doigt, pas cône) avec un profil en puissance 4 pour un bout arrondi ; le reste du corps s'amincit de 11% (ARM_THINNING) pour garder le volume ; aire à 25% près, pas de dérive du centre, corps stable bras en l'air (tests dans `body.rs`).
- Décision : `Autonomy::reach` (`Intent::Reach`, sortie `reach`), pas pendant la bouderie, la somnolence ou l'affection sous 0.55 ; `roam.js` lit les sorties 8 à 10, sinon `retract()`. Le visage est mesuré sans les points du lobe (`ARM_SKIP` dans `render.js`) pour ne pas s'élargir. Expression `fond` pendant le geste ; pas de repos d'images pendant qu'il est sorti.
- Pas encore : le bras ne saisit rien, il reste un lobe large plutôt qu'un tentacule fin, et rien n'a été observé dans l'overlay réel. Aperçu : `dev/preview.html?settle&warm&arm` (ajouter `&armup` pour une diagonale).

## Membres en vrai gel (2026-10-26)

- Retour de l'utilisateur : les pinces et pattes dessinées en 2D à plat "ne font pas slime". Le shader (`gel.js`) fusionne maintenant plusieurs polygones (`uPolys`, `uEnds`, union lisse `smin` de 7 px, MAX_OUTLINE 192, 8 polygones) : le corps est le premier, les membres (`limbPolygons` dans `forms.js` : pinces, pattes, queue, nageoire, bulles du nuage) sont des polygones en plus, donc translucides, éclairés et brillants comme le corps. Les normales ne tiennent compte que des arêtes proches du contour de l'union (sinon des anneaux apparaissent à l'intérieur). Le slime simple est inchangé. Restent en 2D : le ventre de la baleine et le `>_` du nuage.
- Les membres ne sont pas dans la zone cliquable (elle suit les points du corps avec une marge de 14 px).

## Membres et bords d'écran (2026-10-26)

- Les membres d'une forme dépassent du corps (queue, pinces, pattes) mais la physique ne les voit pas : ils étaient coupés par le bord de l'écran dans le coin de repos. `Autonomy::set_inset` écarte le corps des bords de cette distance (`formInset` : crabe 30, baleine 48, nuage 8 px) et un slime à membres ne grimpe plus aux murs. La physique n'est pas modifiée : si l'utilisateur plaque le slime au bord, les membres sont encore coupés.
- Les pattes du crabe reposent maintenant sur le sol au lieu de le dépasser de 12 px. Queue de baleine : tige épaisse et deux lobes arrondis (Chaikin).

## Leçon sur les membres fins (2026-10-26)

- Le gel est translucide : une partie de moins d'environ 10 px d'épaisseur devient presque invisible, même si son polygone est large (la queue de baleine faite d'une seule pièce fine ne montrait qu'une corne). Un membre doit être fait de masses épaisses : la queue est maintenant une tige et deux lobes potelés (`blob`) qui fusionnent par l'union lisse. Le ventre clair de la baleine (ellipse plate) est retiré.

## Épaisseur de gel par membre (2026-10-26)

- Retour de l'utilisateur : la queue de baleine "ne fait pas partie du corps, on dirait un plan arrière". Cause : le shader éclairait tous les polygones avec l'épaisseur du corps entier, donc un membre mince avait un dôme aplati. `uPolyThick[k]` donne à chaque polygone sa propre épaisseur (rayon local fois `material.thickness`) et le shader la pondère par la proximité au contour de l'union (`thickHere`). Les polygones de `forms.js` portent `.thick`. Le slime simple est inchangé.
- Queue de baleine plus grosse (échelle 0.34 de la largeur), ancrée plus profond dans le dos, marge d'écran 56 px.

## Membres touchables et vivants (2026-10-26)

- Retour de l'utilisateur : les formes "ne sont pas touchables, comme si elles ne faisaient pas partie du corps". Maintenant : (1) la zone cliquable inclut la boîte des membres dessinés à l'image précédente (`renderer.limbBox`, fusionnée avec la boîte des bulles) ; (2) un appui sur un membre saisit le corps au point le plus proche du contour, tiré de 12 px vers l'intérieur (`bodyPointNear`), et le décalage est gardé tant que le bouton est tenu (`grabShift` dans `main.js`) ; (3) les membres se balancent contre l'accélération du corps avec deux ressorts (`swing` dans `render.js` : queue, pinces qui traînent, bulles du nuage).
- Vérifié avec le vrai wasm sous Node (sonde jetable) : appui sur la queue, saisie enclenchée, corps soulevé de 99 px en tirant. Les membres restent rigides par rapport au corps : ils ne se déforment pas sous le doigt, ils ne font que balancer.

## Gel souple des membres (2026-10-26)

- `limbjelly.js` : chaque sommet d'un membre est un petit ressort dans le repère du corps (raideur 85, amortissement 8.5), plus libre vers la pointe (0.2 contre le corps, 1 au delà de 0.14 de largeur), freiné par l'accélération du corps (INERTIA) et repoussé par le curseur qui survole (rayon 0.2 de la largeur, pas pendant une saisie). Décalage borné à 13% de la largeur. Le balancement rigide (`swing`) reste en plus. Tests dans `limbjelly.test.mjs`. Aperçu : `&poke=dx,dy` place un curseur près du slime (l'aperçu simule une caresse continue, d'où les cœurs).
- Pas encore : les membres ne sont toujours pas dans la physique du corps (pas de collision, pas de masse) ; c'est une couche visuelle.

## Saisie d'un membre : câble élastique (2026-10-26)

- Retour de l'utilisateur : en saisissant la queue "c'est le corps qui est saisi". La saisie rigide (corps accroché au curseur avec un décalage fixe) ne faisait jamais s'étirer le membre. Maintenant : le corps est tiré par la racine du membre sur un ressort mou (`tether` dans `main.js`, raideur 130, amortissement 19) et le gel autour du sommet saisi suit le curseur (`hold` dans `limbjelly.js`, prise 1100, normalisée par la liberté du sommet saisi pour que seule la racine reste ancrée, étirement borné à 75% de la largeur). Le sommet est choisi au premier appui (`held` dans `render.js`).
- Sonde jetable sous Node avec le vrai wasm (refait la logique de `main.js` et du rendu, balayage en arc de 240 images) : sommet tenu à 5 px du curseur en moyenne (9 au pire), queue étirée de 61 à 99 px en moyenne, corps 15 px en retard. Avant la normalisation : 37 px d'écart. La première version (décalage écran fixe) donnait 21 px d'écart moyen avec le corps qui tourne de 45 degrés.

## Revue de Codex sur la saisie d'un membre (2026-10-26)

- Revue en lecture seule demandée à Codex (fenêtre `codex` de la session tmux `WORK`, 2 min 19 s) : 4 défauts confirmés puis corrigés. (1) Le ressort de rappel gardait le sommet tenu à 4 px du curseur : il est maintenant contraint exactement au curseur (voisins par ressorts, étirement 1.4 de la largeur). (2) Le sommet était choisi sur la géométrie avant déformation : `pickVertex` travaille sur les polygones affichés. (3) `held`, `grabShift` et `tether` n'étaient pas invalidés au changement de forme (et `let` déclarés trop bas pour un appel précoce de `syncForm`) : remis à zéro dans `syncForm`, déclarés avant lui, référence périmée ignorée. (4) Le plafond de `dt` jetait du temps (à 12 images/s le gel n'avançait que de 0.4 s par seconde) : sous-pas de 1/90 s qui consomment tout le temps (gel, balancement, ressort de traîne).
- Méthode : envoi du prompt par `tmux load-buffer` + `paste-buffer` puis deux Entrées (la première ne soumettait pas) ; lecture du rapport par `capture-pane -p -J -S -`.

- Seconde revue de Codex (1 min, commit 084d532) : 3 défauts, tous confirmés et corrigés. Saut de 43 px au relâchement (la limite d'étirement retombait d'un coup de 1.4 à 0.13 de la largeur) : le décalage se résorbe maintenant (`RELAX`), la limite reste stricte seulement pendant la saisie. Prises Rust conservées lors d'une métamorphose : `pressSuppressed` ignore l'appui jusqu'au relâchement. Plafond de temps de 0.1 s alors que la page accepte 0.25 s (`MAX_CATCH_UP`, 6 images/s en somnolence) : tous les sous-pas utilisent 0.25 s.

- Troisième revue de Codex (27 s, commit 029fcbc) : un défaut confirmé et corrigé. `pressSuppressed` n'était remis à zéro qu'au rendu : un relâchement suivi d'un nouvel appui entre deux images ignorait ce nouvel appui. Il est maintenant remis à zéro par les événements `pointerdown`, `pointerup` et `pointercancel`, et n'est activé par `syncForm` que si un appui est en cours. Non vérifié par Codex : le rendu graphique et le coût CPU réel dans WebKit (23 sous-pas par image à 6 images/s sur une centaine de sommets).

## Le slime attrape le curseur avec son corps (2026-10-26)

- Demande : "de fois ça attrape le curseur automatiquement, pas avec sa main mais avec son corps, c'est réaliste". Le pseudopode fin devient un lobe de corps large et long qui s'écoule jusqu'au curseur et le recouvre (`set_arm_style(longueur, largeur)` dans `slimy-body`, lissé par ARM_SHAPE_RATE). On ne déplace PAS le vrai curseur : sous Wayland ce serait un pouvoir système qui demande une permission explicite (idée pour plus tard, dans `slimy-perms`).
- Tous les calculs ont été délégués à Codex (fenêtre tmux `WORK:codex`, 11 min 21 s, scripts et tableaux dans `/tmp/codex-calc/`, rien dans le dépôt), mesurés sur le vrai corps compilé : longueur du lobe `1.95 * (distance_en_rayons - 1)` valable de 1.5 à 2.0 rayons (la pointe finit à 6.5 px au plus du curseur ; la longueur n'est pas une longueur littérale : la pointe tombe à `1 + longueur / 1.95` rayons), largeur `0.65 + 0.125 * longueur`, ressort `ARM_STIFFNESS 60` / `ARM_DAMPING 16` (90 / 9 dépassait la pointe de 43 à 62%), amincissement `1 + x - sqrt(1 + x^2)` avec `x = 0.3 * longueur * largeur`, `REACH_FAR 2.0`. Contact au sol toujours 0, dérive horizontale au plus 10.7 px, impact au plus 72 px/s, aucun saut compensatoire nécessaire.
- Limites données par Codex : l'aire à plus ou moins 10% n'est pas garantie sur tout le cycle pour les grands lobes (0.76 à 1.24 pour longueur 2.8), curseur mobile et rendu non validés. Piège : un contact hover repousse la pointe du lobe (erreur de +0.5 à -10.9 px) : `main.js` envoie un contact Away au corps pendant l'intention reach tant que ce n'est pas un vrai appui.
- Tests : `the_fitted_lobe_rule_puts_the_tip_on_a_cursor_one_and_a_half_to_two_radii_away` (corps, 5 distances, 2 côtés) et les tests de décision (formule, portée, couverture). Expression pendant la capture : `joy`. Aperçu : `dev/preview.html?settle&warm&arm&flow` (ajouter `&armup` pour la diagonale).

## Formes des autres fournisseurs (2026-10-26)

- `FORM_OF_PROVIDER` : anthropic crabe, openai nuage, deepseek baleine, xai chapeau soucoupe (`ufo`), ollama lama, openrouter chauve-souris, bedrock rocher à cristaux ; groq (cerveau par défaut) et custom gardent le slime simple. Chaque forme a sa silhouette, sa base, sa couleur, ses membres en masses épaisses (le gel rend les parties fines presque invisibles) et sa marge d'écran (`formInset` : chauve-souris 80 px pour ses ailes).
- Le lama ressemble un peu à un lapin (deux grandes oreilles) : un vrai lama demanderait un cou et un museau.

## Test de toutes les formes dans l'app réelle (2026-10-26)

- Méthode de capture sous Wayland sans boîte de dialogue : `gnome-screenshot` et `grim` ne conviennent pas (le premier attend une autorisation, le second n'existe pas pour GNOME). L'overlay est une fenêtre XWayland : `xdotool search --pid` donne son identifiant, `import -window ID fichier.png` la capture seule, sur fond noir (elle est transparente), puis recadrage sur la zone non noire. Scripts jetables dans le dossier temporaire de session. Ne jamais utiliser `pkill -f` avec un motif présent dans sa propre ligne de commande : il tue le shell.
- Résultat : les 8 fournisseurs (groq, anthropic, openai, deepseek, xai, ollama, openrouter, bedrock) lancent leur forme, journaux vides, 2 captures chacun. Défaut trouvé : les yeux et la bouche sombres sont illisibles sur le gel noir de xAI : `formInk` (`#eef2ff` pour `ufo`) et `face.setInk`, avec un test qui impose une encre claire sur gel sombre et une encre sombre sinon.
- CPU (build de débogage, deux fenêtres de 10 s) : chauve-souris 12 à 28%, slime simple 23 à 25% : les formes n'ajoutent pas de coût mesurable. Le build release reste la référence (environ 10%).

## Tests d'interaction sur la vraie page (2026-10-26)

- `xdotool` ne convient pas pour l'overlay : sous XWayland ses événements souris injectés n'atteignent pas la fenêtre (même un appui au centre du corps ne saisit rien). Méthode qui marche : `apps/desktop/web/dev/app-harness.html` charge la vraie page (`main.js`, wasm, rendu) avec une fausse API Tauri (`window.__TAURI__` : `world`, `soul_load`, `genome_state`, `brain_config` selon `?provider=`, `window.__calls` et `window.__errors` pour inspection), et `dev/interact-test.mjs <fournisseur> <top|left|right|center> [port]` la pilote dans Chrome headless par le protocole DevTools (`WebSocket` natif de Node 22) : arrêt de la balade par l'entrée de menu `roam`, saisie à un point extrême de la silhouette, glissement vers le haut, lâcher, mesures par captures. Nécessite `python3 -m http.server 8765` dans `apps/desktop/web`. Ne pas utiliser `pkill -f` : arrêter le serveur avec `fuser -k 8765/tcp`.
- Résultat : les 8 formes réussissent (soulevé de 132 à 174 px pendant la saisie, retour au sol, aucune erreur de page) : deepseek par la queue, anthropic par une pince, openai par une bulle, openrouter par une aile, xai par le chapeau, ollama par une oreille, bedrock par un cristal, groq par le corps. Le crabe reste étiré avec une patte au sol (corps large tenu par une pince).

## Test de la capture du curseur et défaut d'hystérésis (2026-10-26)

- `dev/cdp.mjs` (pilotage de Chrome factorisé : `launch`, `shot`, `mouse`, `hover`, `stopRoaming`) sert maintenant à `dev/interact-test.mjs` (saisie) et `dev/catch-test.mjs <fournisseur> <left|right> <distance en rayons>` (capture du curseur : curseur qui bouge à 1.6 à 1.9 rayon, mesure de la pointe du lobe, retrait ensuite).
- Défaut trouvé par ce test : une capture commencée à 1.6 rayon s'arrêtait après 0.7 s, parce que le lobe rapproche le centre du corps du curseur et que la distance repassait sous le seuil d'entrée (1.5), ce qui annulait la capture et déclenchait 10 s de repos. Correction : hystérésis dans `Autonomy::reach` (une capture en cours reste tant que le curseur est entre 0.5 rayon et 1.35 fois la portée), tests `a_catch_that_has_begun_survives_the_body_coming_closer_to_the_cursor` et `a_new_catch_still_needs_the_cursor_to_be_in_the_entry_range`.
- Résultat après correction : le lobe dure environ 3 s et la pointe finit à 5 à 9 px du curseur (groq 5, bedrock 7, xai 9, openai 6, deepseek 9, groq à gauche 6, ollama 2). Largeur retrouvée au repos.
- Limite du capteur : la boîte englobante de l'image inclut les membres (pinces du crabe, ailes de la chauve-souris), donc un curseur placé "à 1.7 rayon du centre de la boîte" tombe déjà sur la pince ou l'aile : crabe et chauve-souris ne sont pas mesurables par cette méthode. Les ailes qui battent changent aussi la boîte. Non vérifié pour ces deux formes.

## L'attrape du curseur refaite : le corps entier (2026-10-26)

- Retour de l'utilisateur : "la main automatique pour le curseur c'est pas bien fait, c'est pas logique". Codex (fenêtre tmux `WORK:codex`, 16 min, scripts dans `/tmp/codex-calc2/`) a mesuré que le lobe précédent ne couvrait presque jamais le curseur (0 à 68% à 1.5 rayon au sol, 0% dès 15 degrés ou 2 rayons) et faisait une lame (25 à 49 px d'épaisseur) : ma mesure précédente ("pointe à 5 px du curseur") ne vérifiait que la pointe, pas la couverture. Leçon : mesurer la couverture réelle (le curseur est dans le contour) et l'épaisseur, pas la distance d'une pointe.
- Nouvelle logique (`crates/slimy-core/src/catching.rs`, machine à états pure) : déclenchement seulement pour un curseur connu et immobile (6.4 px pendant 0.15 s) dont la hauteur au dessus du sol est de 4 à 48 px et la distance horizontale d'au plus 96 px (la hauteur prime, jamais de lobe vers le haut) ; slime au sol, posé, affection > 0.55, éveillé, ni bouderie ni saisie. Phases : acquisition 0.35 s, micro-bond (accroupi doux 5*dt*60 par pas sur 0.16 s, puis `leap_level` : vx = signe(dx)*(10.980972*|dx| - 6.478247), vy = -110, cible 0.4 rayon avant le curseur, erreur de portée au plus 3.6 px), contact, aplatissement de tout le corps (profil `q = 0.8`, `ki = 1/sqrt(q^2 cos^2 + sin^2 / q^2)`, aire conservée par `sqrt(A/A_k)`, transition 1 s, maintien 1.2 s, retour 1.4 s), retrait par bonds de 70.4 px au plus vers `curseur.x - côté * 80`. Annulation : curseur déplacé de plus de 12.8 px ou sorti de la bande (h > 56), hors de portée (112 px) avant le bond ; saisie ou bouderie : fin immédiate. Repos de 45 s après une attrape (choix d'expérience : Codex disait 10 s, mais sauter toutes les 10 s sur un curseur au repos serait un tic), 10 s après une annulation. Calibré pour le rayon 64 seulement (la puissance est mise à l'échelle par `sqrt(rayon/64)` comme les autres bonds, non validé).
- Corps : `SoftBody::set_flatten(0 à 1)` (se compose avec le profil de forme, aire conservée), `leap_level` (saut sans le couple HOP_LEAN), exposés au wasm. Sortie du décideur : `flatten`, `gentle`, `catching` (phase), `Intent::Catch`. Page : `roam.js` envoie `known` séparément de `active` (un curseur posé est connu mais pas "actif"), applique `set_flatten` une seule fois par changement, crouch doux 300*dt, `leap_level` ; une expression par phase (curious, focus ou brace, leap, brace, happy, fond, calm) dans `expressions.js`. Code du lobe retiré de la page (la `Arm` du corps reste, testée).
- Vérifié sur le vrai corps (`slimy-wasm/tests/roam.rs`, 4 positions, des deux côtés) : le curseur est dans le contour pendant tout le maintien, 45 px de gel au moins au dessus de lui, aire 0.85 à 1.2, impact d'au plus 150 px/s, curseur libre ensuite. Vérifié dans le navigateur avec de vrais événements souris (`dev/catch-test.mjs`) : à droite 66 px de gel, à gauche 100 px, juste dessous 72 px, forme à membres (baleine) 107 px, un curseur à 120 px du sol ne déclenche rien. Non vérifié : curseur qui se déplace pendant l'approche, tous les rayons autres que 64, l'overlay réel avec la vraie souris et le pont GNOME.
- Piège : le banc de test doit exiger que le slime soit au sol (il peut être collé à un mur) avant de mesurer.

## CPU : le réveil permanent à 60 Hz (2026-10-26)

- Méthode : `strace -c -f -p` sur le processus et sur `WebKitWebProcess`, échantillonnage de piles avec `gdb -p ... -batch -ex "thread apply 1 bt"` (pas de `perf`), mesure des ticks par thread dans `/proc/PID/task/TID/stat` (script jetable). Au sommeil : le thread principal est en attente (19 échantillons sur 24 dans un appel bloquant, 5 dans `iris_bo_map`, attente GPU Intel), mais le processus est très bavard : environ 700 lectures et 660 écritures par seconde côté Tauri, 1 600 écritures par seconde côté WebKit.
- Cause : au repos la boucle sautait l'image mais redemandait `requestAnimationFrame` à chaque rafraîchissement d'écran (60 fois par seconde), ce qui réveille le processus web et le système de fenêtres pour rien. Correctif dans `main.js` : au repos, `later(secondes)` attend par `setTimeout`, et `wake()` (appelé par les événements souris du canvas et par le relevé du curseur) rend la main tout de suite ; relevé du curseur à 500 ms pendant la somnolence (`POINTER_POLL_DOZING_MS`).
- Mesures (build release, une fenêtre de 10 s trois fois, sommeil, même environnement `SLIMY_POINTER_BUS=org.slimy.NoSuchBus` pour supprimer l'activité de souris) : avant 24.9%, après 7.9% d'un cœur. Mesure sans cette variable, avec le vrai pont GNOME, avant : 6.5% au sommeil ; après : non mesurable proprement, la souris de l'utilisateur réveille le slime. Éveillé et en balade : 30 à 35% (presque 60 images par seconde en continu), pas encore traité. L'objectif de 3% n'est pas atteint.
- Piège : un contrôle de syntaxe (`node --check`) ne détecte pas une variable non déclarée dans `main.js` ; une première mesure à 4% était fausse car la boucle d'animation ne démarrait pas (`ReferenceError`, visible dans le journal `[slimy web]`). Toujours vérifier le journal et capturer la fenêtre de l'overlay (`import -window`) pour constater que le slime est dessiné avant de mesurer.

- Formes lama et baleine : le lama a un cou (lobe haut) et une tete avec museau et oreilles courtes (5 polygones), la baleine un front arrondi a l'avant. Rendu verifie via dev/preview.html. Le harnais `?nosoul=1` et `dev/birth-test.mjs` verifient la naissance de bout en bout.

- Pouvoirs et outils : `slimy-perms` refuse tout par defaut, duree obligatoire (max 1 h) pour ecran, presse-papiers et pointeur, journal de 200 entrees. Le backend decide (`gated` dans main.rs), jamais la page. `mcp.json` (dossier de donnees) est le seul endroit ou une commande de serveur peut etre declaree ; la page ne donne que le nom. Une session MCP = lancer, handshake, demander, tuer (delai 20 s). Piege de test : un Chrome headless reste sur son port apres un test qui plante, `fuser -k PORT/tcp` avant de relancer.
- Outils dans le chat : le modele demande un outil par un champ `tool` de sa reponse JSON (pas d'appel de fonction natif, donc valable pour les 9 fournisseurs). `parse_reply_with` ne garde la demande que si (serveur, outil) fait partie des offres. Rien ne s'execute sans le Oui de l'utilisateur ; le resultat revient comme un tour utilisateur etiquete donnees (risque d'injection par le contenu de l'outil, d'ou l'etiquette et la borne de 4000 caracteres).
- Pouvoir fichiers : le dossier se declare dans `files.json` (`{"root":"/chemin/absolu"}`) dans le dossier de donnees, jamais par la page. `files::inside` refuse composants non normaux puis canonicalise et verifie `starts_with(root)`, ce qui bloque aussi les liens symboliques qui sortent. Les tests de fichiers temporaires doivent avoir un nom de dossier par test (ils tournent en parallele).
- Pont GNOME : le code d'une extension ne se recharge pas sous Wayland sans reconnexion, donc toute nouvelle methode du pont n'est visible qu'apres un nouveau login (`bridge-test` repond `None` avant). Tester le cote Rust avec un faux service (`SLIMY_POINTER_BUS=org.slimy.Test`, service python-gi). Ne jamais ajouter au pont une methode qui agit (deplacer le curseur, etc.) : le bus de session est ouvert a tous les processus de l'utilisateur.
- Cadence : `renderer.busy()` (visage en transition) est presque toujours vrai en marchant, donc ne pas s'en servir pour decider d'une cadence reduite. Mesurer des images dessinees par seconde (`dev/fps-test.mjs`, compteur sur `drawArrays`) plutot que le CPU du vrai overlay, qui depend du hasard des deplacements.
- RemoteDesktop (ashpd 0.13) : features `tokio`, `remote_desktop` et `screencast` (obligatoire a la compilation) ; `NotifyPointerMotion` est relatif et 1:1 sous GNOME. Le test `pointer-test` ouvre la vraie fenetre de consentement : a lancer seulement quand l'utilisateur peut cliquer. Ne pas lire la position juste apres l'acceptation : l'utilisateur a bouge la souris pour cliquer.
- Cadence : un seuil de temps fixe sur requestAnimationFrame donne `f / ceil(f x seuil)` images/s (30 sur un ecran 60 Hz pour un seuil de 23 ms) ; reinitialiser `last` a chaque dessin perd le temps en trop. Pour une cadence reguliere, sauter un nombre entier d'images de l'ecran (`frameStride`).
- Attrape : le test `catch-test.mjs` echoue parfois a l'etape de preparation ("never held still", deux fois sur une vingtaine d'essais), relance seul il passe ; le test Rust `run_catch(seed, dx, radius, profile)` est la mesure fiable. Les crates wasm changent : reconstruire `web/pkg` avec `cargo build --release -p slimy-wasm --target wasm32-unknown-unknown` puis `wasm-bindgen --target web --out-dir apps/desktop/web/pkg target/wasm32-unknown-unknown/release/slimy_wasm.wasm` (pkg est ignore par git).
- Capture d'ecran : une capture noire (moyenne 1, ecart type 0) peut venir d'un ecran verrouille (`gdbus call --session --dest org.gnome.ScreenSaver --object-path /org/gnome/ScreenSaver --method org.gnome.ScreenSaver.GetActive`), pas du code ; verifier cela avant de conclure. Sous Wayland, xcap passe par X11 et ne voit pas les fenetres Wayland : utiliser le portail. Le modele choisi doit savoir lire les images (Claude, GPT-4o, un modele vision de Groq ou d'Ollama), sinon le fournisseur repond une erreur.
- Test reel pilote par RemoteDesktop (`slimy-desktop drive CMD OUT`, debug seulement) : ne JAMAIS taper sans avoir verifie par capture que la bulle de Slimy est ouverte et a le focus, sinon les touches partent dans la fenetre qui a le focus (le terminal de Claude Code a recu une phrase de test et une touche Entree). Le menu s'ouvre decale selon la position du slime : toujours capturer entre le clic droit et le clic sur Chat. Echelle : ecran logique 1920x1200, captures a 1280x800 (x1.5). Le shell de l'agent doit utiliser le fichier `.mutter-Xwaylandauth.*` le plus recent comme XAUTHORITY pour lancer l'app.
- Defauts trouves seulement en vrai avec gpt-oss-20b : (1) le mot `tool` dans le prompt donne l'erreur Groq \"Tool choice is none, but model called a tool\" (champ renomme `ask`, reprise sans offres) ; (2) le modele ecrit `server:\"slimy/files\"` quand la liste dit `slimy/files` (liste reformulee, decodage tolerant) ; (3) il invente le contenu d'un fichier sans le demander (regle stricte dans le prompt, encore imparfaite) ; (4) JSON mal forme affiche en brut (extraction tolerante de `say`). `wl-paste` bloque sur GNOME (pas de protocole data-control) : lecture arboard avec delai de 2 s. `SLIMY_DEBUG_RAW=1` affiche la reponse brute du modele ; `slimy-desktop brain-test \"message\"` presente les offres integrees.
- Voix : `sherpa-onnx` 2.0 a retire espeak-ng (`data_dir` ignore) : le francais ne marche que si on fournit `phoneme_codepoints` (espeak-rs, licence GPL-3 d'espeak-ng a garder en tete pour la distribution) ; sans cela les mots francais sont `OOV word skipped`. `espeak-rs` supprime la ponctuation et colle les mots autour : phonetiser proposition par proposition et remettre `. , ! ? ; :`. Whisper perd le debut et la fin d'un clip sans silence autour : 0,4 s de silence de chaque cote dans `voice::transcribe`. Essai aller-retour sans oreilles : `XDG_DATA_HOME=... slimy-desktop voice-test fr 30 \"phrase\"` (synthese puis retranscription). Les voix Kokoro v1.0 : ff_siwis (30) est la seule francaise ; af_bella 2, af_heart 3, af_nicole 6, af_sky 10 ; jf_* japonaises 37 a 40 non utilisees. Modeles dans `<donnees>/voice/`, telechargements `voice/dl`. `wl-paste` bloque sous GNOME, ne pas s'en servir pour les tests.
- Index vecgrep du projet : `index_codebase` et `index_graph` sur `<repo>` (86 fichiers indexes, mise a jour incrementale aux appels suivants). Controle qualite : `cargo clippy --workspace --all-targets` doit rester sans avertissement.
- Essai reel avec l'utilisateur present : il peut utiliser la souris, le clavier et le panneau Bluetooth en meme temps que le pilotage ; verifier la position du pointeur (`pos`) avant chaque clic, ne jamais taper quand du texte s'ecrit dans le terminal. Au branchement d'casque Bluetooth le volume peut passer a 100 % : le baisser (`pactl set-sink-volume @DEFAULT_SINK@ 35%`) avant toute lecture. Le slime se promene : pour un test stable choisir `Roam or stay still` dans son menu. Apres un redemarrage de la machine `/tmp` est vide (ames de test, modeles de voix).
- Vitesse de la voix : mesurer avec `SLIMY_STREAM=1 SLIMY_DEBUG_RAW=1 slimy-desktop voice-test fr 30 \"phrases\"` (build release, modele charge a part). Sur CPU AVX2 sans VNNI, le modele int8 est ~3 fois plus lent que le complet. Le build de debug est trop lent pour juger de la latence. Le banc `/tmp/claude-1000/bench.sh` compare synthese et retranscription sur trois phrases.
- Apprentissage : `slimy-learn` ne remplace pas le cerveau (LLM) et ne change que des reglages bornes (jamais du texte ni des pouvoirs). Ajouter une decision = une constante `Decision` dans `slimy-learn/src/lib.rs` + son entree dans `DECISIONS` + une recompense cote page ; la page nomme la decision, jamais la valeur. Les recompenses vivent dans `learn.js` (fenetres de 90 s pour les remarques, 25 s pour la parole). Pour voir ce qui est appris : menu Pouvoirs, ou `learn.json` dans le dossier de donnees.
- Affichage : `slimy-display` decide, `src-tauri/src/display.rs` lit le systeme. Sous Wayland/GNOME l'app tourne en XWayland (`GDK_BACKEND=x11`) donc les ecrans sont ceux d'X11. Les modules `cfg(not(target_os = "linux"))` (boucle de clics traversants, position du curseur par Tauri) sont non compiles et non essayes ici : premiere chose a lancer sur Windows ou macOS (`cargo check` puis un essai a la souris).

## Rappels et mémoire
- `remember` et `remind` sont toujours offerts (ils n'écrivent que les notes du slime) ; chaque usage passe par la bulle Oui/Non avec les arguments visibles.
- Les rappels sont sondés toutes les 20 s par la page et annoncés un par un seulement quand le slime est libre (pas de chat, panneau ou question en cours).
- Orchestration : le skill `orchestrate` est global (`~/.agents/skills/orchestrate`), référencé par `project-workflow` et `~/.claude/CLAUDE.md`.

## Skills et secours
- Un skill est `<data>/skills/NOM.md` avec en-tête `name` (a-z0-9-), `description`, puis le corps (2000 caractères, 20 skills au plus). L'outil `slimy/skill` n'est offert que s'il y en a ; le texte du skill revient comme résultat d'outil.
- Le secours ne rejoue que les pannes réseau, délais, limites et 5xx (pas une mauvaise clé), une fois, avec la clé du fournisseur de secours et son modèle par défaut.

## Espaces de travail
- La fenêtre overlay est collante (`set_visible_on_all_workspaces(true)` dans `setup`) : sinon le slime et les rappels n'apparaissent que sur l'espace de naissance. Vérifié par `xprop` (`_NET_WM_DESKTOP` 0xFFFFFFFF, `_NET_WM_STATE_STICKY`), pas encore en changeant d'espace.
- `drive` (RemoteDesktop) attend le consentement du portail : il bloque sans l'utilisateur devant l'écran.

## Fusion d'agents en worktrees
- Les agents en worktree ne committent pas : récupérer leurs changements par `git -C WT add -N .` puis `git -C WT diff > patch` et `git apply -C1 --exclude=docs/TASKS.md patch` dans le dépôt principal (appliquer `--3way` échoue si le fichier est déjà modifié). Fusionner `docs/TASKS.md` à la main.
- Whisper small : `sherpa-onnx-whisper-small.tar.bz2` existe dans la release `asr-models` (639 Mo).

- Voix francaise = feature Cargo `french-voice` de `slimy-desktop` (espeak-rs, GPL-3), desactivee par defaut pour que le binaire par defaut ne contienne aucun code GPL. Build par defaut : `cargo build -p slimy-desktop` (le francais est refuse avec le code `voice_french_missing`, `voice_status.french = false`). Build avec francais : `cargo build -p slimy-desktop --features french-voice`. Verifier les deux : `cargo clippy -p slimy-desktop --all-targets -- -D warnings` et `cargo test -p slimy-desktop`, avec et sans `--features french-voice`.

## Site slimy.okymi.org (deploye le 2026-10-29)
- Source : `site/` (HTML, CSS, JS statiques, pages en et fr, 404). Deploiement : `rsync -a --delete site/ debian@92.222.80.118:/var/www/slimy/` (aucun redemarrage necessaire).
- Serveur : le Caddy de production `cti-prod-caddy-1` tient 80 et 443 de l'IP publique (admin off, donc recharger = `docker restart`). Il relaie `slimy.okymi.org` vers le conteneur `slimy-web` (image caddy:2.11.4-alpine, reseau `cti-prod_edge`, fichiers montes en lecture seule depuis `/var/www/slimy`, config dans `/home/debian/slimy-web/Caddyfile`). Certificat Let's Encrypt automatique (HTTP-01 via Cloudflare). Sauvegarde du Caddyfile d'origine : `/home/debian/Caddyfile.cti-backup-20261029-2012`.
- Le binaire Caddy a besoin de `--cap-add NET_BIND_SERVICE` meme en `--cap-drop ALL`, sinon `exec /usr/bin/caddy: operation not permitted`.
- Constat : le conteneur `cti-prod-app-1` etait arrete depuis 12 jours (code 143) avant l'intervention ; non relance, le bloc `cti` du Caddyfile est conserve tel quel.
