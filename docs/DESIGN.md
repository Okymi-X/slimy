# Slimy : design

Principe : chaque élément visuel porte une information sur l'état du slime. Rien n'est décoratif. Le slime reste un slime, un corps de gel, jamais un sticker ni un humanoïde.

## Ce que le corps dit

| Élément | Ce qu'il exprime |
|---|---|
| Transparence | Clarté d'esprit : reposé = limpide, fatigué = trouble, boude = ternie |
| Noyau | Cœur et affection : taille et éclat, traîne derrière le corps avec retard |
| Bulles | Bulles d'air piégées : naissent au fond, montent plus vite quand elles sont grosses, ondulent, prennent du retard quand le corps accélère et éclatent en haut. Plus nombreuses et plus vives quand il réfléchit au modèle (à venir) |
| Contenu avalé | Ce qu'il a absorbé dans la journée : particules abstraites qui flottent, se dissolvent en heures et teintent le gel (le lien visible entre journée et évolution) |
| Viscosité (gène) | Façon de bouger : fluide et rapide, ou lourd et filant |
| Fils de gel | Quand on l'arrache d'un bord de fenêtre, il file un fil qui se rompt |
| Pseudopode | Il tend un bras pour demander, pointer ou toucher le curseur |
| Mimétisme | Prend brièvement une forme simple (coeur, bulle de parole) puis se détend |
| Sommeil | S'affaisse en flaque, noyau faible, respiration lente, visage qui sombre |
| Visage | Émerge du gel : yeux en lentilles avec parallaxe interne, jamais collés sur la surface |

## Expressions

Dix poses (`expressions.js`), choisies par ce qui arrive au slime et mélangées en douceur (jamais de saut). Les yeux et la bouche changent de forme par un court rétrécissement puis regonflement.

| Pose | Déclencheur | Yeux et bouche |
|---|---|---|
| calme | au repos | yeux ovales, petit sourire |
| content | souris qui bouge sur lui | yeux ovales un peu plus grands, joues, sourire large |
| amour | souris qui le caresse (en mouvement sur lui) pendant 1.8 s, reste 1.4 s ; un curseur immobile ne compte jamais | yeux en coeur, joues, coeurs qui montent |
| surpris | tenu | yeux grands, bouche en o |
| joie | tenu et secoué vite, ou lancé en l'air | yeux en arche, bouche grande ouverte avec langue |
| aïe | atterrissage dur | yeux en arche, bouche ondulée |
| étourdi | secoué plusieurs fois | yeux en spirale, bouche ondulée |
| somnolent | 45 s sans rien autour | yeux en trait, petit sourire, z qui montent |
| bâillement | au moment de s'endormir | bouche ovale ouverte, yeux fermés |
| espiègle | réponse explicite du cerveau uniquement | langue tirée |
| boude | deux questions ignorées en 30 min | yeux mi clos qui regardent ailleurs, petite moue, gel délavé ; une vraie caresse (1 s de mouvement doux) le réconcilie, sinon il se calme après 45 min |

Le visage reçoit l’intention motrice : concentration en balade, préparation pendant l’accroupissement, élan pendant le saut, affection pendant le suivi, curiosité quand il regarde hors de son coin. Un accroupissement ne déclenche pas de douleur et un saut autonome ne déclenche pas de vertige. La douleur suit un choc mesuré, jamais la seule compression du corps. Le trajet vers le lit reste éveillé ; le sommeil commence une fois posé dans le coin. Interrompre une balade ne déclenche pas de sourire de réussite. Le regard de déplacement est converti du repère écran vers celui du corps, y compris sur les murs.

À venir : triste, timide.

## Modèle de base et skins

Un slime est décrit par un fichier de données (`apps/desktop/web/models/base-slime.json`), validé et borné au chargement (`model.js`). Un skin ou une espèce est un autre fichier du même format, jamais du code.

| Bloc | Contenu |
|---|---|
| `body` | rayon, silhouette (hauteur du dôme, hauteur de la base, largeur de la base), respiration (vitesse, amplitude) |
| `material` | cinq couleurs (haut, milieu, bas, lumière, noyau), opacité, opacité des bords, halo, brillance, frémissement, épaisseur, rayon et force du noyau |
| `bubbles` | nombre (jusqu'à 12), échelle, vitesse de montée |
| `face` | yeux (largeur, allongement, écart, hauteur, couleur), bouche, joues, suivi du regard (portée, vitesse), retard (traînée selon la vitesse, inertie, raideur, amortissement), clignement (min, max) |
| `motion` | regard qui flâne quand rien n'est à suivre |
| `accessories` | emplacements réservés (oreilles, feuille, ailes), vides pour l'instant |

Ressenti : tout est lent et doux. Respiration de 3.7 s, regard qui flâne par saccades douces, clignements espacés, somnolence progressive (paupières qui tombent, sourire qui se réduit) après 45 s sans rien autour de lui, réveil immédiat au toucher.

## Matières (couches de shader, intensité 0 à 1, mélangeables)

Gel pur, rayures de tigre, cellules de miel, facettes de cristal, fourrure en frange, veines de lave, étoiles. Chaque matière a une cartographie propre sur le corps et réagit à la lumière interne. Les six intensités sont stockées dans `material.layers`, bornées de 0 à 1. Les motifs suivent le repère et les dimensions du corps ; leur mélange est normalisé pour préserver le gel et ses reflets. La frange est dessinée à l’intérieur du contour, sans changer la silhouette physique. L’aperçu `web/dev/genome-preview.html?materials` compare le gel pur, les six matières et leur mélange avec les mêmes bulles.

## Rendu gel

- Corps à volume constant, bord épais plus lumineux, ombrage sous la peau vers le bas.
- Reflet principal qui suit la surface et se déforme avec elle, reflet secondaire plus petit, croissant de lumière réfléchie en bas.
- Ombre de contact douce ancrée à la surface réelle et faible teinte réfléchie, sans halo émissif ni bande bleue sous le corps. La base absorbe davantage de lumière au contact ; ombre et teinte disparaissent quand le slime se soulève. Reflets en fenêtre, éclats et caustiques internes discrets conservent l’aspect mouillé.
- Contour franc plus sombre que la peau, pour la lisibilité sur n'importe quel fond de bureau.

## Couleur

Palette générée en OKLCH par règles d'harmonie. Tokens : peau haute, peau milieu, peau basse, lumière, noyau, contour. Lisible sur bureau clair et sombre : le contour et l'ombre de contact assurent la séparation.

## Mouvement

Ressort amorti à pression constante. La locomotion décide à chaque pas physique de 120 Hz, indépendamment du rendu. Les bonds ordinaires couvrent au plus 110 px ; préparation et élan conservent les yeux ovales et une bouche discrète. Le regard suit directement une cible lissée, séparée des ressorts d’inertie du gel. Les yeux gardent leur écart sous forte accélération et leur taille varie moins que la largeur du corps. Réponse au toucher immédiate (moins de 50 ms). Respiration permanente de faible amplitude. Respect de la préférence "réduire les animations" : amplitudes réduites, pas d'arrêt.

## Interface de réglages

React et daisyUI, sobre : permissions, cerveau, journal, génome en lecture. Thèmes clair et sombre, focus visibles, navigation clavier, contraste AA. Aucun emoji, icônes en composants.

## Prototype de référence

Le prototype jetable de l'étape de brainstorming (canvas 2D) valide la sensation de toucher. Il ne fait pas partie du produit.
