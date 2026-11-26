# Proposition : vue-table

## Why

Depuis l'incrément 1, une application s'ouvre et ses données se chargent, mais rien ne s'affiche : la fenêtre principale ne montre que le nom et la version d'Ortelia, et l'exécutable n'ouvre aucune application. Ce changement branche la première vraie vue : `ortelia` ouvre l'application d'un dossier, et sa fenêtre affiche la table des personnes, triable, filtrable, avec ses libellés et ses formats.

C'est le deuxième des trois changements de l'**incrément 2** de `docs/feuille-de-route-annuaire.md`, après `captures-images` et avant `etat-de-session`. Il remplit les critères « Fini quand » :
- `app.json` affiche les personnes ;
- 50 000 personnes défilent de façon fluide.

Le critère « la disposition est retrouvée au redémarrage » revient à `etat-de-session` ; celui du choix de la plateforme des captures a été rempli par `captures-images`.

## What Changes

- **Lancement d'une application** (`ortelia`) : `ortelia [chemin]`, où le chemin est un dossier (qui contient `app.json`) ou un fichier ; sans argument, `app.json` du dossier courant.
  - **BREAKING** : sans argument, la fenêtre ne montre plus le nom et la version d'Ortelia, mais l'application du dossier courant, ou une erreur si elle ne s'ouvre pas.
- **Erreurs à l'écran** :
  - une application qui ne s'ouvre pas (fichier introuvable, `app.json` ou modèle fautif) : boîte modale qui liste les erreurs (vingt au plus, puis le total), avec un bouton « Quitter » ; le processus se termine alors avec le code 1 et écrit aussi les erreurs sur la sortie d'erreur ;
  - une source en échec : l'application s'ouvre, sa branche reste vide, et une boîte modale liste les erreurs, avec un bouton « Continuer » ;
  - les avertissements (diagnostics des données, libellés) ne bloquent rien : une barre d'état en donne le nombre, et un clic en ouvre la liste.
- **`app.json` complété** (format 0.4, **BREAKING**) :
  - `languages` : les langues de l'application ; la langue courante est, pour l'instant, la première ;
  - `labels` : les libellés par langue, en listes de paires clé / texte ;
  - `presentations` : la présentation d'une classe, ici sa table (colonnes, avec largeur, alignement et format ; tri initial) ;
  - `window` : titre, taille initiale, et contenu, une vue (`View`) qui affiche une table branchée sur un chemin de données (`/annuaire/personnes`).
  - Tout est vérifié contre le modèle à l'ouverture, et les erreurs sont rassemblées.
- **Table** (`ortelia-presentation` pour la logique, `ortelia-ui-egui` pour le dessin) :
  - colonnes de la présentation de la classe affichée, de sa classe mère la plus proche à défaut, sinon **déduites du modèle** (attributs monovalués, dans l'ordre du modèle) ;
  - **tri multiple** : un clic trie sur une colonne seule, Maj+clic ajoute ou fait avancer une colonne dans le tri ; cycle croissant, décroissant, sans tri ; tri stable, valeurs absentes en dernier, énumérés dans l'ordre de leurs littéraux, textes comparés sans casse ni accents ;
  - **filtre global** : chaque mot saisi doit apparaître, sans casse ni accents, dans le texte affiché d'au moins une colonne ; compteur des lignes affichées sur le total ;
  - **largeur et ordre des colonnes** modifiables pendant la session : redimensionnement et glisser-déposer des en-têtes ; la présentation les tient, pour qu'`etat-de-session` n'ait qu'à les enregistrer ;
  - seules les lignes visibles sont dessinées et formatées ; les index de tri et de filtre sont recalculés quand la révision du Document change.
- **Affichage des valeurs** : nombres selon la langue (`1,68` en français), booléens en cases cochées non modifiables, énumérés par leur libellé, dates et dates-heures en UTC, par défaut ou selon un motif LDML de la colonne (`dd/MM/yyyy`, `d MMMM yyyy`…), unité de l'attribut dans l'en-tête.
- **Libellés** : la clé est `Classe`, `Classe.attribut` ou `Énuméré.LITTÉRAL` ; un attribut hérité cherche d'abord la clé de la classe affichée, puis celles de ses classes mères. Un libellé absent de la langue courante s'affiche `#clé#`, avec un avertissement.
  - Contrôles au lancement : clé en double dans une langue ou langue non déclarée (erreurs) ; clé qui ne désigne rien dans le modèle, ou libellé manquant pour un élément affiché (avertissements).
- **Données d'essai** : `cargo xtask gen-data --model annuaire --count <n>` écrit une application de l'annuaire complète (`app.json`, modèle, `personnes.json` de `n` personnes), reproductible.
- **Captures d'images** : la fenêtre principale avec la table de l'annuaire, et la boîte d'erreur d'ouverture.

### Hors périmètre

- L'enregistrement et la restauration de la disposition (largeurs, ordre, tri, filtre) et de la taille de la fenêtre : changement `etat-de-session`.
- Le choix de la langue, la langue du système et la traduction des textes propres à Ortelia (titres des boîtes, boutons, messages d'erreur, qui restent en français pour les titres et en anglais pour le détail des erreurs) : incrément 4.
- Les filtres par colonne et toute configuration du filtrage : ils viendront avec le langage d'expressions (incrément 6).
- La sélection de lignes et le broker de sélection : incrément 3.
- Les colonnes masquables, les colonnes calculées et les chemins de navigation dans les colonnes (`adresse.ville`) : une colonne désigne un attribut de la classe affichée.
- Les fuseaux horaires (toutes les dates-heures s'affichent en UTC) et le changement d'unité d'affichage, qui viendront ensemble.
- Les formats des nombres (séparateur de milliers, nombre de décimales).
- La collation linguistique complète (ICU) : le tri compare des textes pliés (minuscules, sans accents).
- Les conteneurs (`Split`, piles d'onglets), le menu, plusieurs vues : la fenêtre contient une seule vue.
- La modification des données depuis la table.

## Capabilities

### New Capabilities

- `table-view` : la vue table : colonnes configurées ou déduites, tri multiple, filtre global, largeur et ordre des colonnes, index recalculés à chaque révision, dessin des seules lignes visibles, interactions traduites en intentions.
- `labels` : libellés traduisibles d'une application : clés, recherche dans la langue courante avec héritage, affichage `#clé#` d'un libellé manquant, contrôles au lancement.
- `value-formats` : texte affiché pour chaque type de valeur, selon la langue, et sous-ensemble des motifs de date LDML.

### Modified Capabilities

- `app-config` : format 0.4 ; sections `languages`, `labels`, `presentations` et `window` ; vérification de la vue, des présentations et des colonnes contre le modèle.
- `app-context` : les avertissements de la configuration sont rapportés avec l'application ouverte.
- `app-launch` : argument de chemin ; contenu de la fenêtre principale fourni par l'application ouverte ; boîtes d'erreur, barre d'état et code de sortie ; démarrage réel sur l'annuaire.
- `config-schemas` : le schéma de `app.json` décrit les nouvelles sections.
- `ui-snapshots` : la capture de la fenêtre principale montre la table de l'annuaire ; capture de la boîte d'erreur d'ouverture.
- `storage-benchmark` : le générateur produit aussi une application de l'annuaire d'un nombre donné de personnes.
- `continuous-integration` : le test de fumée ouvre l'application de l'annuaire.

## Impact

- **Code** :
  - `ortelia-config` : types de `languages`, `labels`, `presentations` et `window`, vérification contre le modèle, avertissements ;
  - `ortelia-service` : avertissements de la configuration dans `AppContext` ;
  - `ortelia-presentation` : état de l'application ouverte ou en erreur, intentions, view-model de la table (colonnes, tri, filtre, index), libellés, formats des valeurs, motifs LDML, pliage des textes ;
  - `ortelia-ui-egui` : dessin de la table (`egui_extras::TableBuilder`), glisser-déposer des en-têtes, boîtes modales (`egui::Modal`), barre d'état, taille et titre de la fenêtre ;
  - `ortelia` : argument de chemin, ouverture de l'application, code de sortie ;
  - `xtask` : modèle `annuaire` du générateur.
- **Tests existants adaptés** : texte central et capture `main_window` de la fenêtre principale ; test de fumée lancé sur `examples/annuaire` ; tests de la ligne de commande (un chemin est désormais accepté).
- **Fichiers d'exemple** : `examples/annuaire/app.json` en format 0.4, avec langues, libellés français et anglais, présentation de `Personne` et fenêtre.
- **Schémas** : `schemas/app.schema.json` régénéré.
- **Documents** : `docs/feuille-de-route-annuaire.md` (incrément 2, libellé manquant affiché `#clé#` sans repli sur l'autre langue) ; `CLAUDE.md` (état, lancement avec un dossier, commande du générateur de l'annuaire).
- **Dépendances** : `egui_extras` 0.37, nouvelle, pour la table (dans `ortelia-ui-egui` seulement) ; `icu_normalizer` 2.4 (déjà dans `Cargo.lock`, par une dépendance indirecte) pour la décomposition NFD du pliage des textes, ajoutée à `ortelia-presentation`.
