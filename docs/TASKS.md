# Slimy : tâches

Statuts : todo, doing, done, blocked.

## Phase 0 : spike plateforme

- [done] T0.1 Fenêtre Tauri transparente, sans bordure, toujours au dessus, sous Fedora (Wayland puis XWayland)
- [done] T0.2 Petite fenêtre qui suit le slime sur tout l'écran (déplacement fluide piloté par le backend), saisie par le corps seulement
- [done] T0.2b Clics traversants hors du corps (région d'entrée GTK mise à jour par le corps), menu clic droit pour quitter
- [done] T0.3 Décision documentée dans `docs/MEMORY.md` : Wayland natif ou XWayland imposé

## Phase 1 : corps

- [done] T1.1 `slimy-body` : soft body à pression constante, sol, murs, tests
- [done] T1.2 Interaction souris : toucher, saisie, étirement, lancer
- [done] T1.3 Rendu gel par shader WebGL2 (dôme, spéculaires, fresnel, noyau, bulles) et visage 2D, contrôlé par captures headless
- [done] T1.5 Le slime se colle à toutes les surfaces (sol, murs, plafond), pivote et ne se déchire plus quand on le tire violemment
- [doing] T1.4 Budget de performance : objectif moins de 3% ; contrôle isolé du 26 octobre à 11.1% (Tauri + WebKit, Xvfb et rendu logiciel). Cadence actuelle 12 images/s au repos, 6 en somnolence ; essais de planification et de rejet hors silhouette sans gain, retirés. Attribution par threads : compositeur WebKit ~4.2 points CPU, thread principal ~3 ; alternative ~2.8 pour le compositeur. Xvfb sans DRI3 : mesure GNOME accélérée et profil de piles nécessaires avant le prochain changement. Passe web du 29 octobre : sous Chrome headless en rendu GL logiciel (environ 11 % de cœur par image/s, bruité), `RENDER_SCALE` 0.75 donne -18 % (9 contre 11) mais seulement en logiciel et adoucit les bords, rejeté ; aucun gain JS mesurable, aucun changement conservé.

- [done] T1.6 Modèle de base en données (corps, matière, bulles, visage, mouvement) chargé et validé, point de départ des skins

## Phase 2 : vie

- [done] T2.1 Visage à ressorts : suit le curseur, traîne et ballotte avec le corps, somnolence (états surpris, arche d'impact, content, calme)
- [done] T2.1b Dix expressions et machine à états (content, amour, surpris, joie, aïe, étourdi, somnolent, bâillement, espiègle)
- [todo] T2.1c Extension GNOME du pointeur global : écrite, à installer par l'auteur
- [done] T2.2 Relation (affection, confiance, rancune) et bouderie réversible (deux questions ignorées en 30 min, fin à la première caresse ou après 45 min)
- [done] T2.3 Moteur d'initiative : 4 questions par heure au plus, jamais en boudant, 12 questions, réponses gardées comme faits, "plus tard" sans pénalité
- [done] T2.4 Naissance : oeuf qui se fissure, éclosion, deux noms, âme enregistrée (premières questions de curiosité : à faire avec le moteur d'initiative)

## Phase 3 : esprit

- [done] T3.1 `slimy-brain` : neuf fournisseurs (Groq par défaut), clés dans le trousseau système, panneau Cerveau, commande `brain-test`
- [done] T3.2 Réponse structurée (texte court, expression connue, valence, éveil) validée et apprivoisée, repli en texte simple
- [todo] T3.3 `slimy-memory` : événements, mémoire, journal
- [todo] T3.4 `slimy-perms` : registre, demande en personnage, indicateur d'observation, journal d'accès

## Phase 4 : évolution

- [done] T4.1 Génome continu et règles d'harmonie OKLCH (`slimy-core`, pur ; branché à la page par T4.5)
- [done] T4.2 Matières en couches de shader mélangeables : six gènes branchés, motifs dans le repère du corps, mélange borné et aperçu déterministe
- [done] T4.3 Digestion nocturne : règles, deltas bornés, mutations à graine, historique, signaux du jour et journal (T4.5)
- [done] T4.4 Tests de propriété sur les nuits (300 nuits x 12 vies avec signaux extrêmes : aucun gène hors de [0,1], aucun pas au dessus de 0.03, harmonie toujours bonne)

- [done] T1.8 Déplacements cohérents : coin de repos à 1.5 rayon du bord (le slime n'est plus plaqué au mur), saut penché vers sa direction, impact d'atterrissage mesuré, expressions liées à l'intention (marche, suivi, accroupi, vol, regard curieux, sieste), plus aucune expression aléatoire
- [done] T3.5 Remarques spontanées : au retour de l'utilisateur ou à une caresse, le cerveau dit une phrase (3 par heure, 15 min d'écart, jamais en boudant, silence 30 min si le cerveau échoue)
- [done] T1.7 Initiative motrice : le slime se balade en sautant, grimpe aux murs, se cache dans un coin quand il boude, se roule en boule pour dormir, regarde autour de lui, s'approche du curseur quand il t'aime (menu : se balader ou rester tranquille)

- [done] T4.5 Brancher le génome : fichier d'âme (génome et historique), palette et gènes appliqués au modèle de la page, comptage des signaux du jour, nuit au premier lancement du jour, phrase du journal ; fermeture native avec sauvegarde et redémarrage le même jour vérifiés

## Après MVP

- [done] Attraper le curseur avec le corps entier : un curseur immobile près du sol (4 à 48 px) est approché par un micro-bond, recouvert par un aplatissement du corps (aire conservée), tenu 1.2 s puis relâché ; plus aucun lobe ni bras en l'air (le pseudopode `reach_toward` reste dans le corps pour plus tard)
- [todo] Contenu avalé visible, saisir des objets avec un pseudopode, mimétisme
- [done] T5.1 Registre de permissions `slimy-perms` : refus par defaut, pouvoirs accordes un par un, duree obligatoire pour ecran, presse-papiers et pointeur, revocation immediate, journal borne de chaque acces (pas encore branche a l'interface)
- [done] T5.2 Registre branche : `perms.json`, commandes `perms_state/grant/revoke` (duree bornee a 1 h), menu Pouvoirs avec panneau (accorder, duree, retirer, journal), badge visible tant qu'un pouvoir est accorde ; verifie en headless (`dev/perms-test.mjs`)
- [done] T5.3 Outils MCP : crate `slimy-mcp` (protocole stdio, liste de serveurs validee, reponses bornees) et commandes `mcp_servers/mcp_tools/mcp_call` derriere le pouvoir `tool`, journalisees ; serveurs declares uniquement dans `mcp.json` ecrit par l'utilisateur
- [done] T5.4 Le cerveau demande des outils : liste des outils (cache 5 min) donnee au modele seulement si le pouvoir `tool` est accorde, champ `tool` accepte uniquement pour un outil propose, confirmation Oui/Non dans la bulle avant tout appel, resultat rendu au cerveau comme donnee ; teste en unitaire (Rust et JS), pas encore avec un vrai serveur MCP dans l'overlay
- [done] T5.5a Pouvoir presse-papiers : commande `clipboard_read` (wl-paste puis xclip, 2 s, 2000 caracteres), refusee sans accord ou apres la duree, acces journalise sans le contenu ; le chat la propose au modele comme outil integre `slimy/clipboard` (meme Oui/Non), nom `slimy` reserve aux serveurs
- [done] T5.5b Pouvoir fichiers : lecture seule d'un dossier declare par l'utilisateur dans `files.json`, chemins relatifs sans `..`, liens symboliques resolus avant le controle, 64 Ko lus, 2000 caracteres rendus, outil integre `slimy/files` avec le meme Oui/Non
- [done] T5.5c Application active : methode `GetActiveApp` ajoutee au pont GNOME (classe de la fenetre au premier plan, rien d'autre, extension v2 installee, chargee a la prochaine connexion), commande `active_app` derriere le pouvoir, outil integre `slimy/active_app`, `slimy-desktop bridge-test` pour verifier ; teste contre un faux service D-Bus
- [done] T5.6 Ecran : `Message.image` (JPEG base64, une seule image, la plus recente) dans les trois dialectes (OpenAI, Anthropic, Bedrock), `screen_capture` (portail Screenshot sous Wayland, `xcap` ailleurs, 1280 px de large, JPEG 70, jamais stocke), outil integre `slimy/screen` avec Oui/Non et l'avertissement que l'image part chez le fournisseur ; non verifie sur une vraie image car l'ecran etait verrouille pendant le test (capture noire) : lancer `slimy-desktop screen-test` devant l'ecran, la moyenne et l'ecart type ne doivent pas etre (1, 0)
- [done] T5.7 Deplacement du curseur reel : `remote.rs`, portail RemoteDesktop sous Linux (fenetre de consentement du systeme une fois par lancement, delai 60 s, session fermee a la revocation), `enigo` sous Windows et macOS (non compile ici), 300 px au plus par appel, derriere le pouvoir `pointer` (duree obligatoire) et le Oui/Non du chat ; verifie en vrai sous GNOME avec `slimy-desktop pointer-test` (deplacement de -80 px exact, sans acceleration)
- [todo] Export, import et croisement `.slime`
- [todo] Voix, visites entre slimes, relais Go

- [done] T1.8 Coordonner expressions et mouvement, annuler les sauts périmés et éviter le dépassement des destinations proches.

- [done] T1.9 Stabiliser les coins, les transitions sol/mur et le regard dans le repère du corps ; vérifier les expressions pendant le trajet et au repos.

- [done] T1.10 Cadencer la locomotion avec la physique, raccourcir les bonds et stabiliser les yeux dans le gel.

- [done] T1.11 Remplacer la lumière artificielle sous le gel par une ombre de contact et un reflet discret ancrés à la surface. Contrôle natif Tauri sur Xvfb sans erreur ; composition sur fond GNOME restant à vérifier.

- [done] T1.12 Calibrer les bonds sur le vrai corps (distance réelle prévue = distance parcourue), supprimer les micro-bonds d'approche et les chocs que le slime s'inflige en redescendant d'un mur.

- [blocked] T1.13 Disparition et bandes pendant la saisie : reproduire sous GNOME/XWayland. Xvfb seul ne reproduit pas ; Weston imbriqué déplace la fenêtre et refuse l’injection du pointeur, comparaison des compositeurs non concluante.

- [x] Naissance verifiee en headless (birth-test.mjs) ; instance reelle lancee pour que l'utilisateur la fasse.
- [x] Silhouettes lama (cou, tete, museau) et baleine (front) refaites.

- [done] CPU eveille : la balade libre (intention `wander`, personne ne touche) est dessinee a 40 images par seconde au lieu de 60 (mesure par `dev/fps-test.mjs` : 60 a environ 35 images/s pendant la marche) ; les autres etats gardent la cadence complete. Mesure du CPU reel trop bruitee (12 a 16 % sur 40 s, selon la part de marche), pas de conclusion chiffree sur le gain en CPU.

- [done] Multi-OS : presse-papiers par `arboard` (Windows, macOS, X11, Wayland) avec wl-paste et xclip en secours ; application active par `active-win-pos-rs` sous Windows et macOS (nom seulement), pont GNOME sous Linux. Seul Linux a ete compile et essaye ; Windows et macOS restent a verifier sur ces systemes.
- [done] Cadence reguliere de la balade : `cadence.js` mesure l'intervalle d'ecran et saute un nombre entier d'images (60 Hz : 30, 75 : 37,5, 90 : 45, 120 : 40, 144 : 36, 240 : 40) au lieu d'un seuil fixe qui donnait 30 images/s sur 60 et 90 Hz ; calcul verifie avec Codex (`/tmp/codex-calc3`), test de simulation avec et sans gigue. Le mouvement du curseur en 12 parts est exact (601 cas, aucun contre-exemple).
- [done] Attraper le curseur pour toutes les tailles et formes : le recul apres l'attrape depend maintenant de la largeur reelle du corps (`Snapshot.reach`, `formReach`, entree 15 du Roamer) au lieu de 80 px fixes, donc les pinces du crabe et les ailes de la chauve-souris lachent le curseur. Releve Rust (`survey_of_sizes_and_silhouettes`, ignore) : rayons 40 a 80 avec corps de base, crabe, chauve-souris et forme haute tous valides, rayon 96 des formes larges frole le seuil d'impact (151 a 156 px/s pour 150). Test permanent sur les rayons 52 et 80. Verifie dans la page pour le crabe (Claude) et la chauve-souris (OpenRouter) : le curseur est recouvert puis lache.
- [done] Remarques avec le contexte : si le pouvoir application active est accorde, la remarque spontanee cite le nom de l'application (nettoye, 40 caracteres, guillemets retires, presente comme donnee) ; sans pouvoir rien n'est demande donc rien n'est journalise ; l'etat des pouvoirs est relu toutes les 30 s.
- [done] Accessoires d'evolution sans nouveau gene (le format de l'ame reste a 25 genes) : cornes pour la matiere lave, ailes pour les etoiles, feuilles pour les rayures, chacun a partir de 0.4 et de taille croissante, symetriques ; `dev/preview.html?lava=0.9&stars=0.9&stripes=0.9` pour les voir. Ce sont des formes dessinees sur le gel comme les oreilles et les cristaux, pas des masses de gel ; un vrai gene dedie reste possible plus tard avec une migration des ames.
- [done] Baleine redessinee : corps profile en virgule dont la queue s'arque vers le haut (lobe a -0.5 rad) et deux nageoires de queue en gel epais au bout, plus une nageoire ventrale ; elle se lit comme une baleine dans `dev/preview.html?form=whale`. Saisie par le haut et par le cote verifiee en headless.
- [done] Essai reel de bout en bout avec Groq (gpt-oss-20b) dans le vrai overlay, pilote par RemoteDesktop : menu, chat, application active, fichiers (contenu exact lu), outil MCP (meteo), badge "I am watching" ; correctifs de prompt, d'identifiants et de JSON tolerant. A refaire en vrai : presse-papiers (texte place via X11), ecran, curseur dans le chat.

## Voix (inspiree de la dictee d'Omarchy, Voxtype + Whisper)

- [done] V1 `slimy-voice` (pur) : voix choisie selon le caractere (energie, curiosite, affection) et la langue, hauteur selon le gene de taille (petit = plus aigu, style Kawaii plus haut et plus vif, style Calme plus grave), texte rendu prononcable (sans emoji, markdown ni adresses, coupe a une phrase), detection de fin de parole (bruit de fond mesure, silence d'une seconde, 25 s au plus) ; 8 tests
- [done] V2 Moteur local par sherpa-onnx 2.0 : Whisper base (198 Mo) pour entendre, Kokoro int8 multilingue v1.0 (126 Mo, 54 voix) pour parler, telechargement a la demande depuis les releases officielles k2-fsa avec progression, extraction protegee contre les chemins qui sortent du dossier, rien n'est envoye hors de l'ordinateur. Francais : phonemes par espeak-ng (le moteur ne les fournit plus), ponctuation remise pour les pauses ; l'anglais utilise le lexique du modele
- [done] V3 Pouvoir `microphone` (duree obligatoire, journalise), commandes `voice_status/install/config_save/listen/stop_listening/speak/stop_speaking`, evenements `voice-level` (bouche) et `voice-install` (progression)
- [done] V4 Page : bouton micro dans la bulle (rouge et pulse pendant l'ecoute, deuxieme appui = arret), la phrase entendue part comme un message, les reponses sont dites a voix haute et la bouche suit le volume de la voix, section Voix du menu Pouvoirs (installer, reponses parlees, style) ; 8 tests
- [done] V5 Essais reels du moteur : aller-retour synthese puis transcription (francais quasi exact, anglais exact), lecture par les vrais haut-parleurs, ecoute par le vrai micro (boucle acoustique : le slime parle, le micro l'entend, Whisper retranscrit la phrase presque en entier). Correctifs trouves en vrai : decalage continu au demarrage du micro (filtre passe-haut), bruit de fond mesure par la mediane et pas le maximum, 0,25 s de demarrage ignore, parole trop courte = bruit, silence initial coupe (Whisper perd la premiere phrase sinon), niveau normalise (micro regle a 27 %)
- [done] V6 Page verifiee en headless (`dev/voice-test.mjs`) : bouton micro, ecoute, phrase envoyee, reponse dite, bouche ouverte avec le volume, section Voix et style
- [todo] V7 Essai dans le vrai overlay avec le bouton micro (demande ta presence pour la fenetre de consentement du pilotage) ; modele Whisper plus grand en option (small, 610 Mo) pour une meilleure precision ; selection de la voix japonaise `jf_*` pour un timbre plus kawaii
- [done] Modele par defaut de Groq : `openai/gpt-oss-120b` (6 demandes de pouvoir correctes sur 6 contre quelques ecarts du 20b) ; un `brain.json` existant garde son modele
- [done] Qualite : `cargo clippy --workspace --all-targets` sans aucun avertissement (6 simplifications `map_or`, `return` inutiles, constante morte retiree, initialisation de la config Whisper) ; prechargement des modeles de voix 8 s apres le demarrage (`voice_warm`) pour que le premier mot ne soit pas lent ; projet indexe avec vecgrep (114 fichiers, 1 025 morceaux, graphe de 1 076 noeuds)
- [done] Essai reel de la voix dans l'overlay avec le casque Bluetooth de l'utilisateur : installation des modeles par le panneau (faite par l'utilisateur), bouton micro, voix humaine retranscrite exactement ("Yo what's up what's up, hello hello, test micro test"), reponse du cerveau (gpt-oss-120b) dite a voix haute ; attente de la premiere parole portee de 7 a 10 s (le premier essai a echoue faute de parole dans les 7 s)
- [todo] Essais reels par la voix du chat avec presse-papiers, ecran et curseur (le clavier etait utilise par l'utilisateur pendant l'essai)
- [done] Voix plus naturelle : prosodie selon l'emotion de la reponse (visage, valence, arousal : joyeux plus haut et vif, endormi ou boudeur plus grave et lent), symboles lus (24 degres, 50 pour cent, et), tempo compense quand la voix est plus aigue, clips rognes, fondus de 12 ms, niveau regulier, pause de 0,18 s entre les phrases, parole phrase par phrase (la premiere commence pendant que les suivantes se calculent), 14 tests de plus
- [done] Vitesse : le modele Kokoro complet (334 Mo) tourne 3 fois plus vite que le compact int8 sur un processeur sans VNNI (i7-10750H : 0,6 s de calcul par seconde de voix au lieu de 2) ; premier son 1,7 s au lieu de 5,5 s ; il devient le modele installe par defaut, le compact reste accepte. Plus de fils ne change rien (4 suffisent).

## Modele de comportement qui s'ameliore seul

- [done] L1 `slimy-learn` (pur, local, gratuit) : bandit contextuel a echantillonnage de Thompson, quelques nombres par option, un contexte par moment de la journee (nuit, matin, apres-midi, soir), oubli progressif (memoire de 30 resultats par option) pour suivre les changements d'habitude, options bornees a une courte liste ecrite dans le code (espacement des remarques 600 / 900 / 1500 / 2400 s, rythme de la parole 0,92 / 1,0 / 1,08), fichier abime ou trafique refuse (retour aux valeurs par defaut), deterministe ; 8 tests dont convergence (plus de 70 % des derniers choix sur la meilleure option), adaptation a un changement, apprentissage separe par moment
- [done] L2 Branche : `learn.json` (ecriture atomique), commandes `learn_choose/reward/report/reset` ; la page choisit l'espacement des remarques spontanees (le budget de 3 par heure reste le plafond) et le rythme de la parole, puis observe la reaction : une caresse, un clic ou un message dans les 90 s apres une remarque = bien, silence = non ; reponse a l'oral : repondre apres = bien, repondre par-dessus = un peu, couper la parole = clairement non ; section "Ce que j'ai appris" dans le menu Pouvoirs avec bouton Tout oublier ; 9 tests
- [todo] L3 Autres choix a apprendre (envie de se promener, accueil, style de voix) une fois les premiers resultats observes en usage reel ; mesure sur plusieurs jours
- [done] Verification reelle des pouvoirs sans toucher au clavier de l'utilisateur (build release du 2026-10-27) : presse-papiers (arboard par X11, 46 caracteres lus), application active (pont GNOME, `google-chrome`), capture d'ecran (portail, 1280x800, luminosite moyenne 137 et ecart type 99, donc du vrai contenu), curseur (portail RemoteDesktop, deux deplacements acceptes) ; commandes de controle `clipboard-test`, `bridge-test`, `screen-test`, `pointer-test`, `listen-test`, `voice-test`

## S'adapter a tous les bureaux et appareils

- [done] D1 `slimy-display` (pur, 7 tests) : choix de l'ecran (principal, suivre le pointeur apres 2,5 s de pause, ecran fixe, repli sur le principal si l'ecran est debranche), zone utile de l'ecran choisi, signature qui change quand la configuration change (ecran ajoute, retire, deplace, redimensionne, echelle, barre qui apparait), taille du slime selon la hauteur logique de l'ecran (0,8 a 1,4 par pas de 0,05), zone cliquable avec marge de 6 px
- [done] D2 Branche : le backend surveille les ecrans toutes les 1,5 s (`display::watch`), recouvre la nouvelle zone et envoie `world-changed` ; la page garde le slime dans la nouvelle zone sans recharger (`resize_world`, `set_world`), recharge seulement si la densite de pixels ou la taille change d'au moins 0,15 ; choix de l'ecran dans le menu Pouvoirs (principal, la ou je suis, ecran N) ; zone de jeu de chaque ecran par Tauri (`work_area`), affinee par `_NET_WORKAREA` sur X11 pour l'ecran principal ; tests headless `dev/display-test.mjs` (tailles 0,8 / 1 / 1,3 et zone qui retrecit)
- [done] D3 Windows et macOS : `set_hit_region` ne faisait rien, la fenetre transparente plein ecran aurait avale tous les clics ; une boucle de 25 ms bascule maintenant `set_ignore_cursor_events` selon la position du pointeur (marge de 6 px, jamais de masquage si la position est inconnue). Ce code n'a PAS pu etre compile ni essaye (pas de rustup ici, seulement Linux) : a verifier sur ces systemes

## Cerveau et outils (inspiration OpenClaw, Hermes, Duet, Dots)
- [x] Arguments exacts dans la bulle Oui/Non, budget de 4 outils par message.
- [x] Mémoire visible : outil `slimy/remember`, panneau "Ce que je retiens" avec oubli.
- [x] Rappels : crate `slimy-remind`, outil `slimy/remind`, annonce proactive quand le slime est libre, panneau.
- [x] Orchestration par défaut : skill `orchestrate` liée à `project-workflow`.
- [x] Skills du slime (`skills/*.md` dans le dossier de données, outil `slimy/skill`) et fournisseur de secours (`fallback`, une seule reprise sur panne, limite ou 5xx).
- [ ] Vérification réelle dans l'overlay (rappel, skill, secours), scénarios headless.

## Prochaines étapes (reprise en nouvelle session)
- [ ] Vérifier à l'œil : le slime suit le changement d'espace de travail ; bulle "Rappel : ..." visible ; skill `standup` lu via le chat ; secours de fournisseur (couper le réseau ou clé invalide sur le principal).
- [x] Scénarios headless pour le select de secours, les panneaux mémoire/rappels et l'annonce d'un rappel : `dev/panels-test.mjs` (9 contrôles ; le harnais renvoie deux fournisseurs et des rappels dus via `window.__due`).
- [x] Bilan du matin proactif : `briefing.js` (une fois par jour local, au premier moment libre, rappels du jour et nombre de souvenirs, sans appel au modèle, jour mémorisé dans localStorage) ; 3 tests unitaires et un contrôle de `dev/panels-test.mjs`.
- [ ] Vérifier le code non Linux (affichage, click-through) sur Windows et macOS.
- [ ] Apprentissage : observer sur plusieurs jours réels.
- [ ] Whisper small optionnel (fait, URL du modèle vérifiée, 639 Mo) : vérifier à l'oreille le gain de précision après téléchargement réel.
- [x] Apprentissage L3 : `walk_gap` (pause après une promenade) et `greeting_style` (silence, discret, joyeux au retour), récompenses dans `learn.js`, affichés dans "Ce que j'ai appris".
- [ ] Export/import .slime (menu Exporter/Importer mon âme, dossier exports/ du dossier de données) : à essayer en vrai ; la page recharge après import, le backend relit le fichier à chaque appel (aucun cache), seule une écriture de sentiments de la page dans les 1,5 s avant le rechargement pourrait remettre l'ancienne relation.
- [x] Audit du code non-Linux par lecture contre les sources enigo 0.6.1, active-win-pos-rs 0.9.1, xcap 0.9.8, tauri : une erreur certaine corrigee (`shell_pointer` absent hors Linux dans `pointer-test`) ; reste a compiler sur Windows et macOS (`cargo check -p slimy-desktop`), avec les permissions macOS (Accessibilite) et les clics traversants a essayer.
- [done] Contenu avalé : silhouette de curseur dans le coeur pendant l'attrape (`swallowed.js`, fondu 1,2 s), vérifiée en aperçu seulement ; elle chevauche un peu la bouche (décalage vers le bas à essayer), `catch-test.mjs` instable (étape de préparation) donc non vérifiée dans une vraie attrape ; saisir des objets avec un pseudopode reste à faire.
- [x] Test headless export/import .slime (`web/dev/slimefile-test.mjs`, faux backend du harnais) : menu, Oui/Non, erreurs lisibles, aperçu mal formé, rechargement ; un aperçu invalide efface désormais la question en attente.
- [x] Test de scénario `skill-test.mjs` (outil `slimy/skill` oui/non, cerveau limité en débit) sur la page réelle, avec `skill_read` et un script `window.__brain` ajoutés au harnais.
- [x] Flèche avalée : placée sous la bouche (0,1 rayon sous le coeur, échelle 0,8), vérifiée à rayon 40, 64 et 80 par `dev/swallow-test.mjs` (aperçu `preview.html?radius=N`, `window.__swallow`) ; captures dans /tmp/claude-1000/shots.
- [done] Voix francaise derriere la feature Cargo `french-voice` (espeak-rs optionnel, build par defaut sans GPL, `voice_status.french`, erreur `voice_french_missing`)
- [done] Site vitrine `site/` (en, fr, 404) deploye sur https://slimy.okymi.org (Caddy existant, conteneur slimy-web, TLS Let's Encrypt, verifie : pages, ressources, 404, redirection HTTP).
- [todo] Publication GitHub : LICENSE MIT, champ license des crates, README, CONTRIBUTING, historique neuf (auteur noreply, sans docs/MEMORY.md local), depot Okymi-X/slimy (le site pointe deja dessus).
