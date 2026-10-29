# Slimy : architecture

## Choix et raisons (appels du builder, à valider par l'auteur)

| Couche | Choix | Raison |
|---|---|---|
| Cœur et simulation | Rust, workspace Cargo | Une base de code pour le natif et le WASM, rapide, sûre |
| Rendu | WebGL2 dans une webview Tauri, simulation compilée en WASM | Rendu gel avec shaders, pas d'IPC à 60 Hz |
| Application | Tauri v2 | Fenêtre transparente, intégration système, backend Rust |
| Interface (réglages, chat, journal) | React, Vite, Tailwind, daisyUI | Composants prêts, thèmes clair et sombre |
| Mémoire locale | SQLite (rusqlite) | Local d'abord, un fichier, requêtable |
| Serveur Go | Reporté (relais de visites, sync) | Inutile tant que le slime vit en local |
| Frameworks full stack | Non utilisés | Rien à héberger dans le MVP |

## Découpage

```
slimy/
  crates/
    slimy-core/     âme (noms, ton, faits appris) faite ; puis génome, humeur, relation, règles d'harmonie, format .slime (pur, sans E/S)
    slimy-body/     soft body, collisions, interaction souris (pur, compile en WASM)
    slimy-wasm/     liaison navigateur de slimy-body
    slimy-brain/    fournisseurs de modèles (Groq, OpenAI, Anthropic, DeepSeek, OpenRouter, xAI, Bedrock, Ollama, personnalisé), requêtes et réponses pures, prompt de l'âme, sortie JSON validée ; MCP à venir
    slimy-memory/   SQLite, journal, événements de la journée (natif)
    slimy-perms/    registre de permissions, journal d'accès (natif)
  apps/desktop/
    src-tauri/      backend Tauri : fenêtres, événements système, orchestration
    web/            React : rendu WebGL2, réglages, chat
  docs/
```

Règle de dépendance : `slimy-core` et `slimy-body` ne dépendent d'aucune E/S ni horloge. Le temps arrive en paramètre, ce qui rend tout testable et reproductible par graine.

## Flux de données

1. Frame : `slimy-body` (WASM, webview) calcule les points du corps, le shader dessine. Les entrées souris vont directement au WASM.
2. Événements discrets (clic, message, absence, application active si permis) : webview vers backend par IPC, stockés par `slimy-memory`.
3. Cerveau : le backend envoie au modèle un contexte borné (âme, humeur, mémoire récente, événement). La réponse est validée contre un schéma : texte, émotion (valence, excitation), intention (parler, se taire, demander). Toute réponse invalide est ignorée, le slime reste vivant avec ses animations de base.
4. Digestion nocturne : événements du jour, puis résumé structuré (modèle, ou règles si pas de modèle), puis `slimy-core` applique des deltas bornés et le test d'harmonie, puis journal d'une phrase. Le modèle propose, le code décide.
5. Permissions : chaque lecture (écran, application active, fichier, presse-papiers, outil) passe par `slimy-perms`, qui refuse par défaut et écrit un journal consultable.

## Génome et évolution

- Environ 40 gènes en [0,1] : teinte, saturation, transparence, viscosité, taille, aplatissement, écart et forme des yeux, position du reflet, intensité de chaque matière (rayures, miel, cristal, fourrure, lave, étoiles), croissance des accessoires.
- Palette générée en OKLCH avec règles d'harmonie : écart de teinte borné entre peau, noyau et reflet, contraste minimal du visage.
- Étapes de vie (bébé, enfant, ado, adulte, ancien) : changent proportions et vitesse de plasticité, jamais de branche.
- Nuit : delta maximal 0.03 par gène, mutations rares (probabilité fixée, graine enregistrée), tout enregistré dans l'historique pour rejouer l'évolution.

Le branchement actuel conserve le génome, son historique et les compteurs du jour dans le fichier d’âme. La page applique le modèle dérivé du génome au lancement. Les interactions sont regroupées par date locale et enregistrées toutes les 60 secondes ainsi qu’avant une fermeture par le menu. Un changement de jour, au lancement ou à minuit, digère la journée précédente par règles locales. Le journal attend que le dialogue soit libre ; les nouvelles proportions prennent effet au prochain lancement. Un arrêt forcé peut perdre les interactions depuis la dernière sauvegarde.

## Format `.slime`

Un fichier unique : version de format, génome, âme, historique des deltas, graine. L'export exclut la mémoire privée par défaut. L'import valide le schéma et borne chaque valeur.

## Valeurs de départ (appels du builder, ajustables)

**Plateforme MVP** : Linux (Fedora) seul. macOS et Windows plus tard, sans cible de CI au départ.

**Fenêtre** : une seule fenêtre transparente toujours au dessus, qui couvre la zone utile de l'écran principal (fixée au démarrage, XWayland). Elle ne bouge jamais : le slime est un sprite (deux canvas de 400 par 400 px, WebGL2 pour le gel et 2D pour le visage) que la page déplace par transformation CSS. Seule la boîte du slime reçoit la souris (région d'entrée GTK mise à jour quand elle bouge). Raison : déplacer une fenêtre est asynchrone et faisait trembler le slime. S'accrocher aux bords des autres fenêtres demande leurs positions, donc la permission "voir les fenêtres" (F8). Multi écran après le spike.

**Rendu du gel** : shader de fragment WebGL2 (`apps/desktop/web/gel.js`) qui lit le contour lissé (112 points), construit un dôme à partir de la distance au contour (direction adoucie pour éviter les plis), puis éclaire : diffus, deux spéculaires, fresnel, reflet du ciel, lueur sous la peau, noyau, bulles. `face.js` dessine le visage en 2D au dessus, `render.js` orchestre, `main.js` pilote la simulation à pas fixe (120 Hz). `web/dev/preview.html` rejoue le rendu dans un navigateur pour le contrôler (capture Chrome headless).

**Signaux sans aucune permission** (événements internes seulement) : caresses (survol lent), saisies et lancers, messages à l'utilisateur et réponses, questions ignorées, durée de session, heure d'usage, absences.

**Règles de digestion sans modèle** (signal normalisé, delta = signal x 0.02, borné à 0.03 par nuit et par gène) :

| Signal | Gènes poussés |
|---|---|
| Affection (caresses, questions répondues, moins les ignorées) | teinte vers le chaud, joues, viscosité vers doux |
| Énergie (lancers, sessions actives de jour) | raideur de rebond, taille plus petite, vivacité du noyau |
| Curiosité (réponses, messages) | taille des yeux, transparence, nombre de bulles |
| Usage nocturne | teinte vers bleu violet, matière étoiles |
| Ténacité (boudes terminées sans caresse) | épaisseur du contour, matières cristal et miel |

Avec un modèle : il propose un résumé structuré, le code applique les mêmes bornes.

**Harmonie** : écart de teinte OKLCH entre peau haute et noyau d'au plus 40 degrés, contraste du visage d'au moins 4.5 contre la peau locale. En cas d'échec le gène fautif est ramené dans la limite, la nuit n'est pas rejetée. Mutation : 3% par nuit, ajoute une matière ou un accessoire à 0.2.

**Initiative** : au plus 4 prises de parole spontanées par heure, seulement après un retour d'absence d'au moins 15 minutes, après une caresse, ou sur minuteur. Bulle courte (120 caractères) près du slime, fenêtre de chat pour le long. Bouderie après 2 questions ignorées en 30 minutes, fin à la première caresse ou après 45 minutes. Sans cerveau : banque scriptée d'une vingtaine de questions et réactions.

**Âme minimale** : nom de l'utilisateur, nom du slime, langue, date de naissance, quatre curseurs de ton (affection, curiosité, énergie, ténacité), liste de faits appris (clé, valeur, date), référence à la configuration du cerveau. Dix premières questions scriptées, les suivantes générées.

**Retour visuel du jour dans le MVP** : dérive de teinte du gel et du noyau liée aux signaux de la veille. Le contenu avalé visible (F11) reste après MVP.

**Clés d'API** : trousseau du système (Secret Service). Absent : demande à chaque lancement, jamais en clair sur disque.

**Mesure CPU** : somme du processus Tauri et du processus webview.

## Risques connus

- Wayland interdit de positionner une fenêtre et complique les clics traversants : tâche 0 = spike sous Fedora, repli prévu en XWayland (`GDK_BACKEND=x11`).
- La réfraction du vrai bureau derrière la fenêtre n'est pas lisible : le gel réfracte un environnement procédural et ses propres contenus.
- Un cerveau trop bavard : le moteur d'initiative impose un budget de prises de parole par heure, indépendant du modèle.
