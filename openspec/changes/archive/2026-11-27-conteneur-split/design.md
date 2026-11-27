# Conception : conteneur-split

## Context

Motivation et périmètre : voir `proposal.md`. Exigences : `specs/window-layout`, et les deltas de `app-config`, `session-state`, `app-launch`, `config-schemas` et `ui-snapshots`.

État actuel, qui contraint l'approche :
- `ortelia-config` : `Content` est une énumération étiquetée par `_type_` (`#[serde(tag = "_type_")]`), avec une seule variante, `View(ViewDef)`. `ViewDef` a un `data` obligatoire, `Widget` une seule variante, `Table`. La vérification contre le modèle (`check.rs`, `check_window`) donne un `CheckedWindow` qui porte **une** `CheckedView`.
- `ortelia-presentation` : `AppState` porte une seule `TableView` (`OpenedApp::table`). `MainContent::Table(TableRef)` est ce que l'interface dessine, et `Intent::Table(TableIntent)` ne dit pas à quelle vue l'intention s'adresse. Le module `session` lit et écrit `app.session.json` (format `"1"`, lecture tolérante : clés inconnues ignorées, clés facultatives).
- `ortelia-ui-egui` : `draw_main_window` dessine la barre des avertissements (panneau du bas), puis la table dans un `CentralPanel`, puis la boîte de dialogue éventuelle.
- La règle des couches range d'office toute crate de préfixe `egui` parmi les bibliothèques d'interface : `egui_tiles` ne peut entrer que dans `ortelia-ui-egui`.

Bibliothèque : **`egui_tiles` 0.18.0** (dépend de `egui` 0.37.0, Rust 1.95 au moins ; le dépôt est en 1.99). Context7 ne la référence pas : son API a été vérifiée dans ses sources (`~/.cargo/registry/src/*/egui_tiles-0.18.0`), et le sera de nouveau au début de l'essai. Points utiles :
- `Tree<Pane>` (`Tree::new(id, root, tiles)`, `Tree::ui(&mut dyn Behavior<Pane>, ui)`), `Tiles::insert_pane`, `Tiles::insert_container` ;
- `Linear { children, dir, shares }` : `LinearDir::Horizontal` met les enfants côte à côte ; `Shares` donne à chaque enfant une part relative (seuls les rapports comptent ; egui_tiles prend 1 par défaut, donc une somme égale au nombre d'enfants) ;
- trait `Behavior<Pane>` : `pane_ui` (dessin d'une tuile), `tab_title_for_pane` (obligatoire), `is_tile_draggable` (vrai par défaut), `simplification_options` (simplifications actives par défaut), `on_edit(EditAction::TileResized)` (séparation glissée, ou double-cliquée : le double-clic égalise les deux voisins), `gap_width` (1 point), `min_size` (32 points).
- dépendances : `ahash`, `itertools`, `log` (ses avertissements arrivent dans notre journal par le relais de `log`), `serde` en option (non activée).

## Goals / Non-Goals

**Goals:**
- Trancher l'essai S3 par des tests, pas par une impression : chaque critère est vérifié par un test automatisé, et le compte rendu cite ces tests.
- Un arbre de disposition tenu par la présentation, seul à savoir les parts des splits. L'interface ne fait que le dessiner et rapporter les séparations déplacées, pour qu'egui_tiles reste remplaçable (repli maison, ou GPUI) sans toucher à la présentation.
- Une seule fonction de normalisation des parts, utilisée pour `app.json`, pour les intentions et pour l'état de session.

**Non-Goals:**
- Exposer les concepts d'egui_tiles (tuiles, onglets, `TileId`) hors de `ortelia-ui-egui`.
- Prévoir dès maintenant l'enregistrement d'un arbre réorganisé : il viendra avec le glisser-déposer.
- Optimiser le parcours de l'arbre : une fenêtre a quelques nœuds, un parcours linéaire par identifiant suffit.

## Decisions

### D1. L'essai S3 se fait dans le vrai code, critère par critère

L'essai ne passe pas par un prototype jetable : la disposition est écrite directement dans les crates, et chaque critère devient un test (voir `tasks.md`, section 1) :

| Critère | Vérifié par |
|---|---|
| split sans onglets ni glisser-déposer | egui_kittest : aucun onglet, une vue glissée ne bouge pas |
| parts restaurées exactement | présentation, puis egui_kittest : fermeture et réouverture |
| split vertical dans un split horizontal | egui_kittest : positions des trois tuiles |
| table dans une tuile | tests existants de la table, inchangés, dans la tuile de gauche |
| séparation déplacée dans un test sans écran | egui_kittest : glisser, double-clic, intention émise |
| capture identique sous lavapipe | capture `main_window`, dans WSL2 puis dans la CI |

Compte rendu : `docs/essais/s3-conteneurs.md`, sur le modèle de `docs/essais/captures-images.md` (conditions, critères, résultats, décision).

**Repli** : si un critère échoue sans contournement raisonnable, le dessin du split est écrit pour Ortelia dans `ortelia-ui-egui` (allocation des rectangles selon les parts, séparation qui réagit au glisser). Grâce à D4, seul le module de dessin change ; la présentation, les spécifications et les tests de la présentation restent.

*Alternative écartée* : un exemple jetable (`examples/s3`) comparant deux implémentations, comme l'essai S1. La question n'est pas de mesurer des performances mais de vérifier un comportement, et la bibliothèque est retenue d'avance : un prototype referait le travail deux fois.

### D2. Format 0.5 de `app.json` (`ortelia-config`)

```rust
#[serde(tag = "_type_", deny_unknown_fields)]
pub enum Content {
    View(ViewDef),
    Split(SplitDef),
}

pub struct SplitDef {
    pub id: String,
    pub direction: Direction,          // "horizontal" | "vertical"
    #[serde(default)]
    pub weights: Option<Vec<f64>>,
    pub children: Vec<Content>,        // récursif
}

pub struct ViewDef {
    pub id: String,
    pub widget: Widget,                // Table | Empty
    #[serde(default)]
    pub data: Option<String>,
}
```

- **Récursion** : `SplitDef` contient des `Content`, qui peuvent être des `SplitDef`. En Rust, un type ne peut pas se contenir directement (sa taille serait infinie) ; il le peut à travers un `Vec`, qui range ses éléments sur le tas, comme une `List<Content>` dans une classe Java. Pas besoin de `Box`. schemars décrit la récursion par une définition nommée (`$ref`).
- **Vérifications** (dans `AppFile::check`, par `check_window`, comme celles de `languages` par `check_languages` ; `parse` ne vérifie que la version) : au moins deux enfants ; autant de poids que d'enfants, chacun fini et strictement positif ; `data` présent pour une table, absent pour une vue vide ; identifiants uniques dans tout l'arbre, vues et splits mêlés. Les erreurs de l'arbre entier sont rassemblées, et chacune cite le split ou la vue (`ConfigError::SplitChildren`, `SplitWeightCount`, `SplitWeightValue`, `MissingData`, `UnexpectedData`, `DuplicateContentId`).
- **`PartialEq` sans `Eq`** pour les types qui portent des poids (`AppFile`, `WindowDef`, `Content`, `SplitDef`, `CheckedApp`, `CheckedWindow`, `CheckedContent`, `CheckedSplit`) : `f64` n'est pas `Eq`, puisque NaN n'est pas égal à lui-même.
- **Poids en `f64`** dans la configuration, comme tout nombre JSON lu par serde_json ; la présentation passe en `f32`, l'unité d'egui.
- **Descriptible par le métamodèle** : `weights` est un attribut multivalué de type réel (en Ecore : `EFloat`, `upperBound = -1`). Le métamodèle d'Ortelia ne compile pas encore les attributs multivalués (la même limite bloque `literals` dans `examples/model-editor`) : c'est une dette déjà connue, pas une entorse à la règle.

*Alternatives écartées* :
- un split binaire avec une `fraction` : plus simple, mais trois panneaux côte à côte (arbre, table, propriétés) demanderaient d'imbriquer des splits, et un changement de format plus tard ;
- un poids porté par chaque enfant (comme les données de disposition d'un widget SWT) : il mêlerait à toute vue un attribut qui n'a de sens que dans un split ;
- `data` déplacé dans le widget `Table` : défendable, mais la future vue arbre aura aussi des données, et la vue des propriétés n'en aura pas ; garder `data` sur la vue, facultatif selon le widget, change moins de choses.

### D3. Vérification contre le modèle : un arbre vérifié (`ortelia-config`)

`CheckedWindow` porte désormais un `CheckedContent`, de même forme que `Content` : `Split { id, direction, weights, children }` ou `View(CheckedView)`, où `CheckedView` est `Table { id, data, class }` ou `Empty { id }`. `check_window` parcourt l'arbre en profondeur et vérifie le `data` de chaque vue table (même code qu'aujourd'hui), en rassemblant les erreurs de toutes les vues.

### D4. Arbre de disposition dans la présentation (`ortelia-presentation`, nouveau module `layout`)

```rust
pub enum LayoutNode {
    Split(SplitNode),
    View(ViewNode),
}
pub struct SplitNode {
    id: String,
    direction: Direction,
    children: Vec<LayoutNode>,
    parts: Vec<f32>,                   // une par enfant, somme 1
}
pub struct ViewNode {
    id: String,
    content: ViewContent,
}
pub enum ViewContent {
    Table(TableView),
    Empty,
}
```

- Une `enum` Rust à données joue le rôle d'une hiérarchie scellée en Java (`sealed interface LayoutNode permits SplitNode, ViewNode`) ou d'un `std::variant` en C++ : le `match` oblige à traiter chaque cas, et le compilateur signale un cas oublié quand une variante s'ajoute (onglets, plus tard).
- `OpenedApp` n'a plus de champ `table` : les tables vivent dans les `ViewNode`. `AppState::view(id)` les retrouve par un parcours de l'arbre.
- **Normalisation** : une seule fonction, `normalize_parts(values: &[f32], expected: usize) -> Option<Vec<f32>>`, qui renvoie `None` si le nombre ne correspond pas ou si une valeur n'est pas finie et strictement positive, et sinon les valeurs divisées par leur somme. Elle sert aux poids de `app.json` (déjà vérifiés, donc jamais `None`), aux intentions et à l'état de session.
- **View-models empruntés**, comme `TableRef` : `MainContent::Layout(&LayoutNode)` remplace `MainContent::Table`, avec des accesseurs en lecture seule ; l'interface lit l'arbre à chaque image sans rien copier.
- **Intentions** :
  - `Intent::Table { view: String, intent: TableIntent }` remplace `Intent::Table(TableIntent)` : la présentation applique l'intention à la vue désignée seulement ;
  - `Intent::SplitResized { split: String, shares: Vec<f32> }` : les parts brutes lues dans l'interface ; la présentation les normalise ou les ignore (voir `window-layout`, « Parts reçues par la présentation »).
- **Direction** : `ortelia_config::Direction` est réutilisé, comme `SortOrder`, et réexporté par la présentation pour que l'interface n'ait pas à dépendre de `ortelia-config`.

### D5. Dessin par egui_tiles (`ortelia-ui-egui`, nouveau module `layout`)

- **Construction** : à la création de l'application eframe, après l'ouverture (donc après la restauration de l'état de session), un `Tree<Pane>` est construit depuis l'arbre de disposition, une fois pour toutes : une tuile `Linear` par split (direction, puis `set_share` de chaque enfant à `part × nombre d'enfants`, pour garder la convention d'egui_tiles), une tuile par vue avec `Pane { view: String }`. La correspondance identifiant du split → `TileId` est gardée dans un `Vec`, à côté de l'arbre. L'identifiant egui de l'arbre est fixe (`"ortelia-layout"`) : les identifiants egui des tuiles, et donc la mémoire d'egui pour la table, sont stables d'une image à l'autre.
- **`Behavior`** : une structure à durée de vie courte, recréée à chaque image, qui emprunte l'état (`&AppState`) et la liste des intentions (`&mut Vec<Intent>`) :
  - `pane_ui` dessine la vue désignée : la table par le code existant (`table::draw_table_view`, qui émet des `TableIntent`, que `pane_ui` adresse à sa vue par `Intent::Table { view, .. }`), rien pour une vue vide. La charge du glisser-déposer d'un en-tête porte l'identifiant egui de sa table : elle est partagée par toute la fenêtre, et un en-tête lâché sur une autre table ne déplace rien ;
  - `is_tile_draggable` renvoie `false` : ni tuile ni onglet ne se déplace ;
  - `simplification_options` renvoie `SimplificationOptions::OFF` : sans cela, egui_tiles fusionnerait deux splits imbriqués de même direction (`join_nested_linear_containers`) ou supprimerait un split d'un seul enfant, et la correspondance avec nos identifiants serait perdue ;
  - aucun conteneur `Tabs` n'est créé : un `Linear` contient directement les tuiles des vues, et `all_panes_must_have_tabs` est faux ;
  - `on_edit(EditAction::TileResized)` lève un drapeau.
- **Après `tree.ui`** : si le drapeau est levé, une intention `SplitResized` est émise pour chaque split dont les parts d'egui_tiles diffèrent de celles de la présentation (à 0,00001 près, comme les largeurs de colonnes sont comparées à celles de la présentation), avec les parts de ses enfants dans l'ordre. La présentation reste la seule référence : l'interface n'en garde pas de troisième copie. egui_tiles lève le drapeau à chaque image tant que la séparation est tenue, même immobile : sans cette comparaison, l'application émettrait une intention et redemanderait une image à chaque fois, sans fin. Pendant un glisser, cela fait au plus une intention par image et par split, que la présentation absorbe sans coût (quelques nombres).
- **Emplacement** : l'arbre remplace la table dans le `CentralPanel`, dont la marge intérieure est retirée quand une disposition est dessinée : les tuiles vont jusqu'aux bords de la fenêtre, les séparations sur toute sa hauteur, et chaque vue a sa propre marge de 8 points (celle du panneau central d'egui), donc la table reste à la même distance des bords qu'avant. Les parts s'appliquent ainsi à la largeur de la fenêtre (650 points sur 1000 pour l'annuaire, à la séparation près). La barre des avertissements et les boîtes de dialogue ne changent pas.
- Analogie : le `Tree` d'egui_tiles est à notre arbre de disposition ce qu'un arbre de widgets SWT est à un modèle EMF qu'il affiche. La vérité est dans le modèle ; le widget rapporte les gestes de l'utilisateur, et le modèle se met à jour.

*Alternative écartée* : reconstruire le `Tree` à chaque image depuis la présentation, qui serait alors la seule source de vérité au sens strict. Les `TileId` seraient les mêmes (insertion dans le même ordre), mais cela alloue des tables de hachage à chaque image, et rien d'autre que l'utilisateur ne change les parts aujourd'hui. Si une commande « revenir à la disposition de `app.json` » arrive (incrément 4), un compteur de révision de la disposition dira à l'interface quand reconstruire.

### D6. Parts dans l'état de session (`ortelia-presentation`, module `session`)

```json
{
  "formatVersion": "1",
  "window": { "width": 1200, "height": 800 },
  "views": [ { "id": "personnes", "table": { ... } } ],
  "containers": [ { "id": "principal", "split": { "weights": [0.7, 0.3] } } ]
}
```

- `containers` suit le modèle de `views` : une liste d'objets désignés par leur `id`, et une clé par sorte de conteneur (`split` aujourd'hui, `tabs` plus tard). Pas de dictionnaire, comme le veut la spécification du fichier.
- La clé s'appelle `weights`, comme dans `app.json` : des parts normalisées sont des poids valides. À la relecture, elles passent par `normalize_parts`, ce qui couvre aussi un fichier modifié à la main (`[3, 1]` donne 0,75 et 0,25).
- `formatVersion` reste `"1"` : la clé est nouvelle et facultative, et un Ortelia plus ancien l'ignore (clés inconnues ignorées).
- Restauration par identifiant, le premier en double compte, et une entrée qui désigne une vue et non un split est ignorée (voir le delta de `session-state`). Le journal de restauration (`session state restored`) compte aussi les conteneurs.

### D7. Tests, par niveau

- **Niveau 1, présentation** : arbre de l'annuaire, parts calculées depuis les poids, splits imbriqués, intentions de parts (valides, nombre faux, valeur invalide, split inconnu), intentions de table adressées à leur vue (deux tables des mêmes données), état de session (calcul, restauration, cas dégradés). Les configurations de test à plusieurs vues sont des copies de l'annuaire avec un autre `app.json`, dans un dossier temporaire, comme les tests de l'état de session.
- **Niveau 2, egui_kittest** : largeur de la table selon les parts (bornes de son nœud d'accessibilité), positions dans un split imbriqué, fenêtre élargie, absence d'onglets, vue glissée sans effet, séparation glissée et double-cliquée. La séparation n'a qu'un nœud d'accessibilité sans nom (rôle `Unknown`, celui de sa zone de saisie) : sa position est calculée depuis le bord droit de la table, jamais écrite en dur, puis `Harness::drag_at` et `Harness::drop_at` (egui_kittest 0.37) la déplacent ; le double-clic passe par deux clics rapprochés envoyés comme événements de pointeur. Le nœud de la séparation sert à trouver le conteneur du split dans l'arbre, et donc la tuile de la vue vide, dont le sous-arbre ne doit contenir que des conteneurs sans texte (les colonnes de la table qui dépassent, rognées par son défilement horizontal, gardent des nœuds au-delà de la séparation : l'écran ne suffit pas à les distinguer). Les tests de dessin passent par un harnais d'application eframe (`recorder_of`, qui enregistre les intentions au lieu de les transmettre) : un harnais qui dessine une fonction ajouterait une marge extérieure de 8 points, que l'application n'a pas.
- **Niveau 3, capture** : `main_window` change d'image (la table n'a plus que 65 % de la largeur, et la séparation est visible). Nouvelle référence produite dans WSL2 (`UPDATE_SNAPSHOTS=1`), regardée avant d'être versionnée.
- **Niveau 4, fumée** : inchangé ; il ouvre l'annuaire, donc le split.

## Risks / Trade-offs

- [egui_tiles en retard sur une future version d'egui, qui bloquerait la montée de version] → egui_tiles est maintenu par la même équipe qu'egui (rerun), et la 0.18 a suivi egui 0.37 ; au pire, le repli de D1 ne touche que le module de dessin.
- [Séparation sans nœud d'accessibilité nommé : tests par coordonnées fragiles] → positions calculées depuis les bornes de la table et de la fenêtre, avec une tolérance de quelques points (épaisseur de la séparation, `gap_width` d'un point).
- [Deux copies des parts, celle de la présentation et celle d'egui_tiles, qui divergent] → seule l'interface les modifie aujourd'hui, et elle le rapporte à chaque image où une séparation bouge ; le test « Séparation retrouvée » vérifie l'aller-retour complet. Voir D5 pour le jour où la présentation les changera.
- [Deux tables dans une fenêtre qui partagent la mémoire d'egui (largeurs, défilement)] → l'`id_salt` de chaque table inclut l'identifiant de sa vue ; le test des deux tables des mêmes données le vérifie.
- [Parts minuscules restaurées depuis un fichier modifié à la main : une tuile de quelques points] → egui_tiles impose `min_size` (32 points) seulement pendant un glisser. On l'accepte : le fichier n'est pas fait pour être édité, et le premier glisser répare la disposition. Une borne pourra être ajoutée si le cas se présente.
- [Le format 0.4 n'est plus lu] → seules les applications du dépôt existent ; elles passent au format 0.5 dans ce changement, et `gen-data` recopie `examples/annuaire/app.json`, donc suit automatiquement.

## Migration Plan

- `examples/annuaire/app.json` passe au format 0.5 dans le même commit que le code qui le lit ; `schemas/app.schema.json` est régénéré (`cargo xtask gen-schemas`), et la CI vérifie qu'il est à jour.
- Un `app.session.json` existant reste valide : sans `containers`, les parts de `app.json` s'appliquent.
- Pas de retour arrière prévu au-delà de `git revert` : le format 0.4 n'a pas d'autre utilisateur que le dépôt.
