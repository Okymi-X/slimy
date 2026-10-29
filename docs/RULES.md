# Slimy : règles de code

## Générales

- Valider toute entrée à la frontière : réponses de modèle, fichiers `.slime` importés, messages IPC.
- Aucun secret en dur ni dans les journaux. Clés d'API dans le trousseau du système.
- Refuser par défaut : toute lecture de l'écran, des applications, des fichiers ou du presse-papiers passe par `slimy-perms`.
- Une responsabilité par module et par fonction. Séparer simulation, rendu, E/S et interface.
- Chercher avant d'écrire : réutiliser les helpers, composants et tokens existants. Pas de dépendance en double.
- Pas d'emoji dans le code, les commentaires, les docs ou l'interface.
- Pas de tirets cadratins ni demi cadratins dans la prose, les commentaires et l'interface.
- Pas de commentaire qui répète le code. Pas de code mort ni de TODO d'échafaudage.

## Rust

- `slimy-core` et `slimy-body` sans E/S, sans horloge système, sans aléa global : le temps et la graine arrivent en paramètre.
- `clippy` sans avertissement, `rustfmt`, pas de `unwrap` hors tests.
- Toute valeur de génome est bornée à l'entrée et après chaque modification.

## Cerveau

- Le modèle propose, le code décide : sorties validées par schéma, deltas bornés, budget de prises de parole par heure.
- Toujours un repli sans modèle : le slime reste animé et expressif.

## Interface

- Suivre `docs/DESIGN.md`, composants daisyUI, tokens de couleur, thèmes clair et sombre.
- Accessibilité : HTML sémantique, libellés, clavier, contraste AA, focus visible.

## Tests

- Tests unitaires sur génome, harmonie, deltas nocturnes, format `.slime`, permissions.
- Tests de propriété : aucune séquence de nuits ne sort un gène de [0,1] ni du pas maximal.
- Test de non fuite : sans permission, aucune source de données n'est lue.

## Git

- Aucune ligne d'attribution IA dans les commits ni les pull requests, sauf demande explicite de l'auteur.
- Ne pas committer ni pousser sans demande.

## Orchestration
- L'agent principal orchestre (skill `orchestrate`) : il planifie, rédige les briefs, vérifie et rapporte. Des sous-agents neufs du même modèle font les lectures, éditions et tests bornés.
- Un brief donne l'objectif, les fichiers autorisés, le contexte collé, le livrable court. Pas de commit ni de push par un sous-agent. Travaux parallèles : fichiers disjoints, trois au plus.
- Un rapport de sous-agent n'est jamais une preuve : relire le diff et observer une sortie de test.
