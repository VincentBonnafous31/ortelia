# Proposition : formulaire

## Why

La vue de droite de l'annuaire est vide depuis `conteneur-split`. L'incrément 3 doit y afficher les propriétés de la personne sélectionnée. Il faut d'abord savoir décrire un formulaire dans `app.json`, le construire dans la présentation et le dessiner.

Un formulaire n'est pas une simple liste d'attributs. C'est une hiérarchie d'éléments : des champs, dont chacun montre un attribut avec un widget adapté à son type, et des éléments de mise en page (groupes, séparateurs, textes). Ces éléments sont placés dans une grille. Le formulaire est décrit par la configuration et interprété à l'exécution : aucun code n'est généré par classe. Il doit rester descriptible par le métamodèle, pour qu'un futur designer, lui-même construit avec Ortelia, puisse l'éditer avec des formulaires et des arbres ordinaires.

C'est le deuxième des trois changements de l'**incrément 3** de `docs/feuille-de-route-annuaire.md` (« Split et panneau de propriétés en lecture seule ») :
1. `conteneur-split` (fait) : essai S3 et container `Split`, avec une vue vide à droite de la table ;
2. `formulaire` (celui-ci) : la description du formulaire dans `app.json`, son modèle dans la présentation, et son dessin, d'abord vide, dans la vue de droite ;
3. le broker de sélection : la sélection d'une personne dans la table l'affiche dans le formulaire.

Il prépare le critère « Fini quand » restant de l'incrément : **la sélection d'une personne l'affiche dans le formulaire, avec un widget par type d'attribut**. Ce changement apporte le formulaire et un widget par type d'attribut. Les valeurs d'une personne s'y affichent dès qu'on lui donne un objet : c'est ce que vérifient ses tests, et ce que fera le broker du changement 3. Dans l'application, la vue de droite montre le formulaire vide de `Personne`.

## What Changes

- **`app.json` au format 0.6** (**BREAKING** : le format 0.5 est refusé, sans migration, comme à chaque version) :
  - la présentation d'une classe peut déclarer un formulaire (`form`), à côté de sa table. Le formulaire a une grille (`columns`, une liste de colonnes, chacune avec un poids d'élargissement `grow`) et des éléments (`elements`) ;
  - les éléments sont polymorphes par `_type_` :
    - `Field` : un attribut (`path`), un widget facultatif (`widget`), une clé de libellé facultative (`label`), et la mise en page de ses deux cellules, le libellé (`labelCell`) et le widget (`cell`) ;
    - `Group` : un titre facultatif (`label`), sa propre grille (`columns`), ses éléments (`elements`) et sa cellule (`cell`) ;
    - `Separator` : une barre de séparation ;
    - `Text` : un texte fixe, donné par une clé de libellé ;
  - la mise en page d'une cellule : nombre de colonnes occupées (`span`), début d'une nouvelle ligne (`newRow`) ;
  - les widgets d'un champ, polymorphes par `_type_` : `TextField`, `TextArea` (`rows`), `NumberField`, `CheckBox`, `ComboBox`, `RadioGroup`, `DateField` et `DateTimeField` (`format`). Chacun n'accepte que les types d'attribut qui lui conviennent ;
  - nouveau widget de vue `Form`, qui déclare la classe dont il montre le formulaire (`class`), sans `data`.
- **Grille en flux** (`ortelia-presentation`) : les cellules se placent dans l'ordre des éléments, de gauche à droite, et passent à la ligne quand la ligne est pleine, quand une cellule n'y tient plus, ou sur `newRow`. Les largeurs des colonnes se calculent à partir des largeurs minimales mesurées par l'interface et des poids `grow`. C'est un composant à part, sans lien avec les formulaires, qu'on enrichira peu à peu (hauteurs, alignements, cellules sur plusieurs lignes…).
- **Formulaire dans la présentation** :
  - formulaire de la classe, résolu à l'ouverture : celui de la présentation de la classe, ou de sa classe mère la plus proche qui en a une ;
  - formulaire déduit du modèle à défaut : un champ par attribut, dans l'ordre du modèle, avec le widget par défaut de son type, sur une paire de colonnes libellé / valeur ;
  - libellés cherchés une fois, à l'ouverture ;
  - valeurs de l'objet montré formatées comme dans la table, sauf la zone de texte, qui garde ses sauts de ligne ;
  - formulaire vide quand il n'y a pas d'objet.
- **Dessin** (`ortelia-ui-egui`) :
  - grille dessinée ligne par ligne, les groupes dans un cadre titré ;
  - champs en lecture seule : textes sélectionnables mais non modifiables, cases, listes et boutons radio désactivés ;
  - défilement quand le formulaire dépasse sa vue.
- **Libellés** : les clés citées par un formulaire (titre de groupe, texte, libellé d'un champ) peuvent être libres. Elles ne produisent pas l'avertissement « clé qui ne désigne rien », mais un libellé manquant est signalé comme les autres.
- **Annuaire** : `examples/annuaire/app.json` passe au format 0.6, avec un formulaire de `Personne` (un groupe « Identité », les autres attributs, un séparateur, les notes en zone de texte sous leur libellé). La vue `proprietes` devient une vue `Form` de la classe `Personne`.
- **Schémas** : `schemas/app.schema.json` régénéré.
- **Captures** : `main_window` renouvelée (formulaire vide à droite) et nouvelle capture `form`, le formulaire montrant une personne.

### Hors périmètre

- La **sélection** et le **broker** : changement 3. D'ici là, l'objet montré par une vue formulaire n'est donné que par les tests. La classe déclarée par la vue `Form` pourra servir ensuite au formulaire montré quand rien n'est sélectionné.
- L'**édition**, et donc les sélecteurs de date, les contrôles de saisie et Appliquer / Annuler : incrément 5. Les widgets sont en lecture seule.
- Les **conditions** (visible, éditable selon les valeurs) : à l'incrément des contraintes. Elles seront des objets structurés, descriptibles par le métamodèle, et non des expressions en texte. Aucune clé n'est réservée d'ici là.
- Les **onglets**, les **boutons** (ils attendent les actions), le **repli des groupes** et l'état de session qui irait avec.
- Les widgets de **références et de contenance** (liste déroulante de références, objet contenu, table dans un formulaire, formulaire de l'objet contenu) : application 2.
- Les cellules sur **plusieurs lignes** (`rowSpan`), l'élargissement en hauteur, l'alignement d'une cellule, les tailles minimales de colonne, et un champ sans cellule de libellé.
- Des **chemins** de champ à travers des références (`adresse.ville`) : `path` ne désigne qu'un attribut de la classe, hérité compris.
- Le formulaire d'une sous-classe qui **complète** celui de sa classe mère : le formulaire hérité est pris tel quel.

## Capabilities

### New Capabilities
- `grid-layout` : la grille en flux, réutilisable par tout widget. Elle comprend des colonnes avec poids d'élargissement, le placement des cellules (`span`, `newRow`, passage à la ligne automatique) et le calcul des largeurs de colonnes depuis les largeurs minimales mesurées et la largeur disponible.
- `form-view` : le formulaire d'une classe. Il couvre :
  - la résolution par la présentation et l'héritage, et le formulaire déduit du modèle ;
  - les éléments et leurs cellules, les widgets par défaut et les valeurs de l'objet montré ;
  - le formulaire vide, le dessin en lecture seule et le défilement.

### Modified Capabilities
- `app-config` : format 0.6 ; `form` dans une présentation, avec ses éléments, ses cellules et ses widgets, vérifiés contre le modèle ; widget de vue `Form` et sa classe ; `data` refusé pour une vue formulaire.
- `labels` : clés libres citées par un formulaire, sans avertissement « clé qui ne désigne rien » ; libellés manquants d'un formulaire affiché signalés.
- `window-layout` : la vue de droite de l'annuaire est une vue formulaire, et la vue vide se vérifie sur une autre application.
- `app-launch` : le contenu de la fenêtre de l'annuaire comprend la vue formulaire `proprietes`.
- `config-schemas` : le schéma de `app.json` décrit le widget de vue `Form`, les éléments d'un formulaire et les widgets d'un champ.
- `ui-snapshots` : la capture de la fenêtre principale montre le formulaire vide à droite ; nouvelle capture du formulaire qui montre une personne.

## Impact

- `ortelia-config` :
  - `PresentationDef::form`, `FormDef`, `GridColumnDef`, `Element`, `CellDef`, `FieldWidget`, `Widget::Form` ;
  - vérifications et erreurs (attribut inconnu, widget incompatible, `span` hors de la grille, poids, `rows`, motif de date, classe d'une vue formulaire, `data` refusé) ;
  - clés de libellé des formulaires transmises à la vérification des libellés ;
  - `APP_FORMAT_VERSION` passe à `0.6`.
- `ortelia-presentation` :
  - nouveaux modules `grid` (placement, largeurs) et `form` (formulaire résolu, view-model `FormRef`, valeurs) ;
  - `ViewContent::Form` et `ViewRef::Form` ;
  - `AppState::set_form_object`, point d'entrée du futur broker ;
  - `TableRef::object(row)`, l'objet d'une ligne ;
  - réexport de `ObjectId`, pour que l'interface ne dépende pas de `ortelia-model`.
- `ortelia-ui-egui` : nouveau module `form` (mesure des cellules, dessin de la grille, widgets en lecture seule, cadre des groupes, défilement), branché dans le dessin des vues. Aucune nouvelle dépendance.
- Tests :
  - présentation : placement, largeurs, résolution, formulaire déduit, valeurs, libellés ;
  - configuration : un test par erreur ;
  - egui_kittest : libellés, valeurs et widgets affichés, lecture seule ;
  - captures `main_window` et `form`.
- Fichiers : `examples/annuaire/app.json`, `schemas/app.schema.json`.
- Documents :
  - `CLAUDE.md` : état du projet, format 0.6 ;
  - `docs/feuille-de-route-annuaire.md` : changement 2 de l'incrément 3 fait ;
  - `docs/architecture-couches.md` : grille dans la présentation, catalogue des widgets partagé entre vues et champs.
