# Spec Delta

## MODIFIED Requirements

### Requirement: Contenu de l'état de session
L'état de session d'une application SHALL contenir la taille de la fenêtre, en points ; pour chaque vue table, désignée par son identifiant : l'ordre des colonnes, désignées par leur chemin, la largeur de chacune, le tri (chemin et sens de chaque critère, dans l'ordre) et le texte du filtre ; et pour chaque split, désigné par son identifiant, les parts de ses enfants, dans l'ordre. Il SHALL être calculé par la couche de présentation, sans interface.

#### Scenario: Disposition modifiée
- **WHEN** l'annuaire est ouvert, qu'on déplace la colonne `date_naissance` en première position, élargit `nom` à 200 points, trie par `situation` croissant puis `nom` décroissant, filtre sur `dup`, que la fenêtre mesure 1200 × 800 points et que la présentation reçoit pour le split `principal` les parts 0,7 et 0,3
- **THEN** l'état de session donne une fenêtre de 1200 × 800 points ; pour la vue `personnes` : les colonnes `date_naissance`, `nom`, `prenom`, `nombre_enfants`, `taille`, `contact_autorise`, `situation`, `inscription`, `nom` à 200 points et les autres à leur largeur courante, le tri `situation` croissant puis `nom` décroissant, et le filtre `dup` ; et pour le split `principal`, les parts 0,7 et 0,3

#### Scenario: Disposition inchangée
- **WHEN** l'annuaire est ouvert sans état de session, puis que son état est calculé sans aucune interaction
- **THEN** l'état donne une fenêtre de 1000 × 700 points, les colonnes, largeurs et tri de la présentation de `Personne`, avec un filtre vide, et pour le split `principal` les parts 0,65 et 0,35

#### Scenario: Vue vide
- **WHEN** l'état de session de l'annuaire est calculé
- **THEN** il ne contient rien pour la vue vide `proprietes`

### Requirement: Enregistrement à la fermeture
À la fermeture de la fenêtre principale, l'état de session de l'application ouverte SHALL être écrit dans son fichier d'état. Rien n'est écrit pour une application qui n'a pas pu être ouverte, ni pour une application ouverte sans état de session (tests, captures d'images). `app.json` SHALL NOT être modifié.

#### Scenario: Fermeture puis réouverture
- **WHEN** l'annuaire, copié dans un dossier temporaire, est ouvert dans l'interface avec état de session, qu'on élargit `nom` à 200 points et trie par `prenom`, que la fenêtre se ferme, puis que l'annuaire est rouvert avec état de session
- **THEN** la colonne `nom` a 200 points de large et la table est triée par `prenom` croissant

#### Scenario: Séparation retrouvée
- **WHEN** l'annuaire, copié dans un dossier temporaire, est ouvert dans l'interface avec état de session, dans une fenêtre de 1000 points de large, qu'on glisse la séparation de 100 points vers la gauche, que la fenêtre se ferme, puis que l'annuaire est rouvert avec état de session dans l'interface
- **THEN** la première part du split `principal` vaut environ 0,55, et la table occupe environ 550 points de large

#### Scenario: Application qui ne s'ouvre pas
- **WHEN** la fenêtre d'une application dont l'ouverture a échoué se ferme
- **THEN** aucun fichier n'est écrit dans le dossier de l'application

#### Scenario: `app.json` intact
- **WHEN** l'annuaire, copié dans un dossier temporaire, est ouvert avec état de session, que sa disposition est modifiée puis son état enregistré
- **THEN** le contenu de son `app.json` est inchangé, octet pour octet

### Requirement: Restauration à l'ouverture
À l'ouverture d'une application avec état de session, si son fichier d'état existe, la présentation SHALL restaurer la taille de la fenêtre ; pour chaque vue table de l'application dont l'identifiant y figure, l'ordre des colonnes, leurs largeurs, le tri et le filtre enregistrés ; et pour chaque split dont l'identifiant y figure, les parts enregistrées. Sans fichier, elle s'ouvre avec la disposition de `app.json`, sans avertissement. L'exécutable `ortelia` ouvre toujours avec état de session.

#### Scenario: Disposition retrouvée
- **WHEN** l'état de session de l'annuaire enregistre les colonnes `prenom` puis `nom` en tête, `nom` à 180 points, le tri `nom` décroissant, le filtre `dup`, une fenêtre de 1200 × 800 points et les parts 0,7 et 0,3 pour le split `principal`, et que l'annuaire est ouvert avec état de session
- **THEN** les deux premières colonnes sont `prenom` et `nom`, `nom` a 180 points de large, la table ne montre que Dupont, son compteur vaut `1 / 10`, le filtre vaut `dup`, la fenêtre mesure 1200 × 800 points et les parts du split `principal` valent 0,7 et 0,3

#### Scenario: Pas de fichier
- **WHEN** l'annuaire, copié dans un dossier temporaire sans fichier d'état, est ouvert avec état de session
- **THEN** la table a les colonnes, les largeurs et le tri de la présentation de `Personne`, la fenêtre mesure 1000 × 700 points, les parts du split `principal` valent 0,65 et 0,35, et il n'y a aucun avertissement de plus

#### Scenario: Ouverture sans état de session
- **WHEN** l'annuaire, copié dans un dossier temporaire avec un fichier d'état qui trie par `prenom`, est ouvert sans état de session
- **THEN** il a la disposition de `app.json`, et l'enregistrement de son état n'écrit rien

### Requirement: Éléments disparus ignorés
`app.json` a pu changer depuis l'enregistrement : une vue, un split, une colonne ou un critère de tri enregistrés qui ne désignent aucune vue table de l'application, aucun split, ou aucune colonne de sa table, SHALL être ignorés, de même qu'une vue, un split, une colonne ou un critère en double (le premier compte). Le reste de l'état SHALL être restauré, sans erreur ni avertissement à l'écran.

#### Scenario: Colonne disparue
- **WHEN** l'état enregistré de la vue `personnes` cite une colonne `surnom`, que la présentation n'a pas
- **THEN** la table s'ouvre sans colonne `surnom`, les autres colonnes enregistrées gardent leur ordre et leur largeur

#### Scenario: Tri sur une colonne disparue
- **WHEN** l'état enregistré trie par `surnom` croissant puis `nom` décroissant
- **THEN** la table est triée par `nom` décroissant seulement

#### Scenario: Vue inconnue
- **WHEN** l'état enregistré décrit une vue `contacts`, que l'application n'a pas, et la vue `personnes`
- **THEN** la vue `personnes` est restaurée, et la vue `contacts` est ignorée

#### Scenario: Colonne en double
- **WHEN** l'état enregistré cite la colonne `nom` deux fois, à 180 puis à 300 points
- **THEN** la table n'a qu'une colonne `nom`, de 180 points

#### Scenario: Split inconnu
- **WHEN** l'état enregistré donne des parts à un split `lateral`, que l'application n'a pas, et les parts 0,7 et 0,3 au split `principal`
- **THEN** les parts du split `principal` valent 0,7 et 0,3, et le split `lateral` est ignoré

#### Scenario: Split devenu une vue
- **WHEN** l'état enregistré donne des parts à `personnes`, qui est une vue et non un split
- **THEN** ces parts sont ignorées, et la vue `personnes` est restaurée par ailleurs

## ADDED Requirements

### Requirement: Parts enregistrées vérifiées
Les parts enregistrées d'un split SHALL être ignorées, sans avertissement à l'écran, si leur nombre n'est plus celui des enfants du split ou si l'une n'est pas un nombre fini strictement positif : les parts de `app.json` s'appliquent. Des parts retenues SHALL être ramenées à une somme de 1.

#### Scenario: Split qui a gagné un enfant
- **WHEN** l'état enregistré donne deux parts au split `principal`, qui a maintenant trois enfants dans `app.json`
- **THEN** le split `principal` a les parts de `app.json`

#### Scenario: Part négative
- **WHEN** l'état enregistré donne au split `principal` les parts -0,2 et 1,2
- **THEN** les parts du split `principal` valent 0,65 et 0,35

#### Scenario: Parts dont la somme n'est pas 1
- **WHEN** l'état enregistré, modifié à la main, donne au split `principal` les parts 3 et 1
- **THEN** les parts du split `principal` valent 0,75 et 0,25
