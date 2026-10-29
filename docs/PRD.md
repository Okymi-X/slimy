# Slimy : PRD

## Problème et idée

Les compagnons numériques actuels sont des stickers animés ou des chatbots sans corps. Slimy est un slime de bureau qui a un vrai corps de gel (translucide, mou, touchable), une personnalité qui se forme au fil des jours, et un cerveau que l'utilisateur branche sur le modèle IA de son choix. Il reste un slime : jamais un humanoïde, jamais un sticker.

Références visuelles : Rimuru (gel translucide, volume, toucher), Slime Rancher (espèces reconnaissables par leur matière, gros yeux brillants), planches de slimes kawaii fournies par l'utilisateur (variété, accessoires, expressions).

## Utilisateurs

Personne qui travaille longtemps sur un ordinateur de bureau (développeur, créatif, étudiant) et veut un compagnon vivant, discret, privé. Premier utilisateur : l'auteur du projet, sous Fedora Linux.

## Objectifs

1. Un corps de gel crédible et agréable à toucher : on le pousse, on l'étire, il garde son volume et rebondit.
2. Un compagnon qui prend l'initiative avec mesure : il éclot, demande le nom de l'utilisateur et son propre nom, pose des questions au bon moment, boude si on l'ignore, puis revient.
3. Une évolution continue et unique : chaque journée d'interaction décale un génome continu. Pas de branches fixes, pas de classes.
4. Un cerveau branchable : Ollama local, API compatible OpenAI, Anthropic, MCP pour les outils.
5. Le slime n'a que les pouvoirs que l'utilisateur lui accorde, un par un, révocables.
6. Un fichier `.slime` partageable (corps, génome, âme) et croisable entre deux slimes.

## Non objectifs

- Pas d'assistant généraliste qui concurrence un chatbot : le slime est un compagnon avant d'être un outil.
- Pas de réseau social ni de marketplace dans la première version.
- Pas de mobile, pas de matériel ESP32 au départ.
- Pas de monétisation, pas de mécaniques d'addiction (séries quotidiennes, culpabilisation).

## Anti objectifs (échec même si ça fonctionne)

- Un slime bavard ou envahissant qu'on désinstalle en une semaine.
- Un rendu qui ressemble à un sticker plat ou à un décor sans fonction.
- Toute observation cachée de l'utilisateur ou envoi de données sans accord.
- Un slime punitif : il boude, il ne fait pas payer.
- Une évolution qui produit des slimes laids ou qui oscille d'un jour à l'autre.

## Fonctionnalités

| # | Fonction | MVP |
|---|---|---|
| F1 | Fenêtre transparente sans bordure, slime qui se déplace sur tout l'écran, clics traversants hors du corps | oui |
| F2 | Corps de gel : soft body à volume constant, rendu translucide, noyau, bulles, reflets, contact au sol | oui |
| F3 | Visage émergeant du gel, yeux qui suivent, clignement, états (calme, content, serré, surpris, dort, boude) | oui |
| F4 | Naissance : éclosion, nom de l'utilisateur, nom du slime, premières questions | oui |
| F5 | Humeur et relation (affection, confiance, rancune) avec boudeuse réversible | oui |
| F6 | Âme : fichier de personnalité et mémoire locale | oui |
| F7 | Cerveau branchable (Ollama, compatible OpenAI, Anthropic) avec sortie émotionnelle structurée | oui |
| F8 | Permissions par pouvoir, demandées par le slime, indicateur visible quand il observe | oui |
| F9 | Digestion nocturne : résumé de la journée, décalage borné du génome, journal d'une phrase | oui |
| F10 | Génome continu : couleur, matière, forme, yeux, accessoires, mélange de matières | oui |
| F11 | Contenu visible : ce qu'il a avalé flotte dans le gel et se dissout | après MVP |
| F12 | Pseudopodes et mimétisme (tendre un bras, prendre brièvement une forme simple) | après MVP |
| F13 | Outils via MCP | après MVP |
| F14 | Export et import `.slime`, croisement | après MVP |
| F15 | Voix et phonèmes | plus tard |
| F16 | Visites entre slimes, serveur Go de relais | plus tard |

## Critères d'acceptation du MVP

- Sous Fedora, le slime s'affiche sur le bureau sans fond, se laisse saisir et lancer, et ne bloque pas les clics ailleurs.
- 60 images par seconde en activité, 20 en sommeil, moins de 3% de CPU au repos sur la machine de l'auteur.
- Au premier lancement, il demande les deux noms et les mémorise après redémarrage.
- Sans permission accordée, aucune donnée de l'écran, des applications ou des fichiers n'est lue (vérifiable par test).
- Après trois nuits simulées, le génome a bougé, chaque gène d'au plus 0.03 par nuit, et le slime reste dans les règles d'harmonie.
- Le cerveau fonctionne avec Ollama local et avec une API distante, et le slime reste vivant (animé, expressif) si le cerveau est coupé.
