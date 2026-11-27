# Proposition : conteneur-split

## Why

La fenêtre de l'annuaire ne montre qu'une vue, la table des personnes. L'incrément 3 doit afficher à côté d'elle un panneau de propriétés : il faut d'abord savoir scinder la fenêtre. Ce changement apporte le premier conteneur, `Split`, qui partage la fenêtre entre plusieurs vues, et une vue vide qui tiendra la place du futur panneau de propriétés.

C'est le premier des trois changements de l'**incrément 3** de `docs/feuille-de-route-annuaire.md` (« Split et panneau de propriétés en lecture seule ») :
1. `conteneur-split` (celui-ci) : essai S3 et container `Split`, avec une vue vide à droite de la table ;
2. le formulaire déduit du modèle, affiché dans la vue de droite ;
3. le broker de sélection : la sélection d'une personne dans la table l'affiche dans le formulaire.

Il remplit deux des trois critères « Fini quand » de l'incrément : **le choix de S3 est tranché**, et **la séparation est retrouvée au redémarrage** (la disposition de la table l'est depuis `etat-de-session`). Le troisième, la sélection d'une personne affichée dans le formulaire avec un widget par type d'attribut, revient aux deux changements suivants.

L'essai S3 vient en tête parce que le choix de la bibliothèque des conteneurs fixe la forme du reste. La bibliothèque retenue d'avance est `egui_tiles` (version 0.18, qui suit egui 0.37) : elle sait déjà imbriquer des conteneurs, et elle offre des onglets et le glisser-déposer des vues, qui serviront plus tard (mode run, mode design). L'essai vérifie qu'elle se plie à une disposition fixe, décrite par `app.json`. S'il échoue, le repli est un `Split` écrit pour Ortelia, sur les panneaux d'egui.

## What Changes

- **Essai S3** : `egui_tiles` avec egui 0.37, jugé sur des critères posés d'avance (split sans onglets ni glisser-déposer, poids restaurés exactement, splits imbriqués, table dans une tuile, séparation déplacée dans un test sans écran, capture identique sous lavapipe). Compte rendu dans `docs/essais/s3-conteneurs.md`.
- **`app.json` au format 0.5** (**BREAKING** : le format 0.4 est refusé, sans migration, comme à chaque version) :
  - nouveau contenu de fenêtre `Split` : un identifiant (`id`), une direction (`horizontal`, enfants côte à côte, ou `vertical`, enfants l'un au-dessus de l'autre), des poids facultatifs (`weights`, un par enfant, strictement positifs, parts égales à défaut) et au moins deux enfants (`children`), vues ou splits ;
  - nouveau widget `Empty` : une vue qui ne dessine rien ;
  - `data` devient facultatif sur une vue : obligatoire pour une table, refusé pour une vue vide ;
  - les identifiants des vues et des splits d'une fenêtre sont uniques, dans un même espace de noms.
- **Disposition de la fenêtre** (`ortelia-presentation`) : un arbre de disposition, fait de splits et de vues, calculé depuis `app.json` et l'état de session ; il porte les poids courants de chaque split, et reçoit les déplacements de séparation par une intention.
- **Dessin** (`ortelia-ui-egui`) : l'arbre de disposition est dessiné par `egui_tiles`, sans onglets, sans glisser-déposer et sans simplification de l'arbre ; seules les séparations bougent. Les proportions sont gardées quand la fenêtre change de taille.
- **État de session** : les poids de chaque split, désigné par son identifiant, sont enregistrés à la fermeture et restaurés à l'ouverture suivante, avec la même indulgence que pour les tables (split disparu, nombre de poids qui ne correspond plus, valeur invalide : les poids de `app.json` s'appliquent). La version du format du fichier d'état ne change pas : la clé est nouvelle et facultative.
- **Annuaire** : `examples/annuaire/app.json` passe au format 0.5, avec un split horizontal : la table des personnes à gauche (65 %), une vue vide `proprietes` à droite (35 %).
- **Schémas** : `schemas/app.schema.json` régénéré.

### Hors périmètre

- Le **glisser-déposer** des vues, les **onglets**, et toute réorganisation de la disposition par l'utilisateur : la structure reste celle de `app.json`. `egui_tiles` les permettra plus tard, en mode run ou en mode design ; il faudra alors enregistrer l'arbre entier dans l'état de session.
- Le **formulaire** et la **sélection** : changements 2 et 3 de l'incrément. La vue de droite reste vide.
- Des tailles minimales ou maximales de tuile dans `app.json`, le repli d'une tuile, le déplacement d'une séparation au clavier.
- Un titre ou une barre de titre par vue.
- Plusieurs tables dans une même fenêtre de l'annuaire : le format le permet, mais l'annuaire n'en a qu'une.

## Capabilities

### New Capabilities
- `window-layout` : la disposition de la fenêtre en conteneurs et en vues (splits, poids, vue vide), calculée par la présentation, dessinée sans réorganisation possible, et le suivi des séparations déplacées par l'utilisateur.

### Modified Capabilities
- `app-config` : format 0.5 ; contenu de la fenêtre `View` ou `Split` ; widget `Empty` ; `data` facultatif selon le widget ; vérification des splits ; identifiants uniques des vues et des splits.
- `session-state` : les poids des splits entrent dans l'état de session, avec leurs règles de restauration.
- `app-launch` : le contenu de la fenêtre principale fourni par la présentation est l'arbre de disposition, et non plus une seule table.
- `config-schemas` : le schéma de `app.json` décrit les contenus `View` et `Split` et les widgets `Table` et `Empty`.
- `ui-snapshots` : la capture de la fenêtre principale montre le split de l'annuaire, la table à gauche et la vue vide à droite.

## Impact

- `ortelia-config` : `Content::Split`, `Widget::Empty`, `data` facultatif ; vérifications (direction, poids, nombre d'enfants, identifiants uniques, `data` selon le widget) ; `APP_FORMAT_VERSION` passe à `0.5`.
- `ortelia-presentation` : arbre de disposition (splits et vues), intention de déplacement de séparation, routage des intentions de table vers leur vue ; module `session` (liste `containers`) ; `AppState` n'a plus une seule table mais des vues désignées par leur identifiant.
- `ortelia-ui-egui` : dessin de l'arbre de disposition par `egui_tiles` ; nouvelle dépendance `egui_tiles` 0.18 (qui apporte `ahash`, `itertools` et `log`), réservée à cette crate par la règle des couches.
- `ortelia-presentation`, exemple `table-bench` : il trouve la table dans l'arbre de disposition.
- Tests : la présentation (lecture, poids, normalisation, restauration et cas dégradés de l'état de session), egui_kittest (largeurs des tuiles selon les poids, séparation glissée traduite en intention, enregistrement puis restauration), capture `main_window` renouvelée.
- Fichiers : `examples/annuaire/app.json`, `schemas/app.schema.json`, `docs/essais/s3-conteneurs.md` (nouveau).
- Documents : `CLAUDE.md` (état du projet, format 0.5, état de session), `docs/feuille-de-route-annuaire.md` (découpage de l'incrément 3, essai S3 fait), `docs/architecture-couches.md` si la disposition y demande une décision nouvelle.
