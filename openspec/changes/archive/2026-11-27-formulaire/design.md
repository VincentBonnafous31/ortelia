# Conception : formulaire

## Context

La motivation et le périmètre sont dans `proposal.md`. Les exigences sont dans `specs/grid-layout` et `specs/form-view`, et dans les deltas de `app-config`, `labels`, `window-layout`, `app-launch`, `config-schemas` et `ui-snapshots`.

État actuel, qui contraint l'approche :
- **`ortelia-config`**
  - `PresentationDef` n'a qu'une `table`. `Widget` (le widget d'une vue) a deux variantes, `Table` et `Empty`, étiquetées par `_type_` (`#[serde(tag = "_type_", deny_unknown_fields)]`). Le même schéma sert à `Content` (`View`, `Split`), récursif à travers un `Vec`.
  - La vérification (`check.rs`) rassemble toutes les erreurs dans un `Vec<ConfigError>`. Ses variantes sont plates et citent la classe et le chemin.
  - Les libellés sont vérifiés en premier (`labels.rs`, `check_labels`). Une clé qui ne désigne ni classe, ni attribut, ni littéral donne l'avertissement `UnknownLabelKey`.
- **`ortelia-presentation`**
  - `LayoutNode::build` construit les vues à l'ouverture, avec un seul `LabelLookup`, qui garde chaque clé manquante une fois.
  - `TableView::new` résout la présentation par `std::iter::once(class).chain(ancestors()).find_map(..)` : la présentation la plus proche s'applique, et sans `table` les colonnes sont déduites du modèle.
  - `ValueFormat::cell` formate une valeur en `Cell`, sur une ligne. `ViewRef` est ce que l'interface lit pour dessiner une vue.
- **`ortelia-ui-egui`** : `layout.rs` dessine chaque tuile d'après `state.view(id)`, avec `Table` ou rien.
- **`ortelia-model`**
  - Tous les attributs sont monovalués.
  - Un type de donnée nommé (`TexteLong`) est compilé en son type de base. Le formulaire ne peut donc pas savoir qu'un texte est long : la zone de texte se déclare.
  - `Store::get(obj, attr)` donne la valeur, `Store::is_alive` dit si l'objet existe encore, et `Model::is_subclass_of` sert au contrôle de classe.

Bibliothèques : **egui 0.37.0** et **egui_extras 0.37.0**, déjà dans le workspace. Aucune nouvelle dépendance. L'API utilisée a été vérifiée dans Context7, puis dans les sources de la 0.37.0, car Context7 ne sert que la documentation courante :
- `Ui::new_child(UiBuilder::new().max_rect(rect).id_salt(..))` et `Ui::allocate_rect` ;
- `Ui::add_enabled` et `Ui::add_enabled_ui` ;
- `TextEdit::singleline` / `multiline`, avec `desired_width`, `desired_rows` et `align(Align2::RIGHT_CENTER)` (`horizontal_align` est obsolète en 0.37). `TextBuffer` est implémenté pour `&str` : une zone de texte en lecture seule reste sélectionnable ;
- `Checkbox::without_text(..).indeterminate(..)`, `ComboBox::from_id_salt(..).selected_text(..).width(..)`, `Ui::radio` ; une case indéterminée est signalée `Toggled::Mixed` dans l'arbre d'accessibilité ;
- `Frame::group(style)`, `Ui::separator` (la ligne couvre le `max_rect`), `ScrollArea::both().auto_shrink(false)` ;
- `WidgetText::into_galley(ui, Some(TextWrapMode::Extend), f32::INFINITY, TextStyle::Body)` pour mesurer un texte ;
- `Response::labelled_by(id)`, qui relie un widget à son libellé dans l'arbre d'accessibilité.

`egui::Grid` ne fusionne pas de cellules : `Grid::show` n'a aucune notion d'étendue. Il réutilise aussi les largeurs de colonnes de l'image précédente. D'où une grille propre à Ortelia.

## Goals / Non-Goals

**Goals:**
- **Une grille sans lien avec les formulaires.** Placement et largeurs de colonnes sont des fonctions pures de `ortelia-presentation`, testées sans interface. L'interface ne fait que mesurer les cellules et les dessiner dans les rectangles calculés. On pourra l'enrichir (hauteurs, `rowSpan`, alignements) sans toucher aux formulaires.
- **Un formulaire résolu une fois, à l'ouverture.** Widgets par défaut, étendues par défaut, libellés et placement sont calculés à l'ouverture. À chaque image, il ne reste que la lecture des valeurs de l'objet montré.
- **Un format descriptible par le métamodèle.** Contenance (`elements`), polymorphisme par `_type_`, aucun dictionnaire, et des noms de `_type_` uniques dans tout `app.json`.
- **Le point d'entrée du broker de sélection.** `AppState::set_form_object` existe dès maintenant, pour que le changement 3 n'ait plus qu'à l'appeler.

**Non-Goals:**
- Aligner les colonnes d'un groupe sur celles du formulaire qui le contient. Chaque grille calcule ses largeurs, comme deux panneaux MigLayout imbriqués.
- Construire les formulaires de toutes les classes : seuls ceux des vues formulaire le sont, à l'ouverture. Le changement 3 les construira à la demande, selon la classe de l'objet sélectionné.
- Mettre les valeurs en cache : un formulaire a quelques dizaines de champs, lus à chaque image.

## Decisions

### D1. Format 0.6 de `app.json` (`ortelia-config`, module `app`)

```rust
pub struct PresentationDef { class, table: Option<TableDef>, form: Option<FormDef> }
pub struct FormDef { columns: Option<Vec<GridColumnDef>>, elements: Vec<Element> }
pub struct GridColumnDef { grow: Option<f64> }
pub struct CellDef { span: Option<u32>, #[serde(default)] new_row: bool }

#[serde(tag = "_type_", deny_unknown_fields)]
pub enum Element {
    Field(FieldDef),          // path, widget?, label?, label_cell?, cell?
    Group(GroupDef),          // label?, columns?, elements (default []), cell?
    Separator(SeparatorDef),  // cell?
    Text(TextDef),            // label, cell?
}

#[serde(tag = "_type_", deny_unknown_fields)]
pub enum FieldWidget {
    TextField(NoOptions), TextArea(TextAreaDef /* rows? */), NumberField(NoOptions),
    CheckBox(NoOptions), ComboBox(NoOptions), RadioGroup(NoOptions),
    DateField(DateFieldDef /* format? */), DateTimeField(DateFieldDef),
}

pub enum Widget { Table(TableWidget), Empty(EmptyWidget), Form(FormWidget /* class */) }
```

- **Pourquoi des structures vides** (`NoOptions {}` avec `deny_unknown_fields`) plutôt que des variantes unitaires : c'est la même raison que pour `EmptyWidget`. Une clé de trop est refusée, et une option ajoutée plus tard ne change pas la forme du JSON.
- **Récursivité** : `GroupDef::elements` est un `Vec<Element>`. Le `Vec` range ses éléments sur le tas, comme un champ `List<Element>` en Java : il n'y a pas besoin de `Box`. schemars décrit le type récursif par une définition nommée et des `$ref`, comme `Content`.
- **Noms de `_type_` uniques dans tout `app.json`** : `View`, `Split`, `Table`, `Empty`, `Form`, `Field`, `Group`, `Separator`, `Text`, `TextField`… Quand la configuration sera décrite par un modèle (designer construit avec Ortelia), chaque `_type_` y sera une classe d'un même paquetage. D'où `TextField` et non `Text`, qui est l'élément texte fixe, et `DateField` et non `Date`, qui est le type de base. Le widget d'une vue et celui d'un champ sont deux énumérations Rust, chacune listant ce que son contexte accepte. Quand une table entrera dans un formulaire (application 2), la même structure `TableWidget` deviendra une variante des deux. Ce sera la même classe, avec les mêmes options : c'est le catalogue de widgets partagé.
- **Bornes** : `TEXT_AREA_ROWS = 1..=50`, 4 par défaut.
- **Version** : `APP_FORMAT_VERSION` passe à `0.6`.

### D2. Vérification du formulaire (`ortelia-config`, nouveau module `form`)

`check_presentations` appelle `check_form(model, class, &FormDef, errors) -> Option<CheckedForm>` après la table. Le résultat est un arbre dont tous les défauts sont résolus, pour que la présentation n'ait plus de règle de format à connaître :

```rust
pub struct CheckedGrid { pub grow: Vec<f32>, pub elements: Vec<CheckedElement> }
pub enum CheckedElement {
    Field(CheckedField),   // attr, widget: CheckedWidget, label: Option<String>, label_cell, cell
    Group { label: Option<String>, grid: CheckedGrid, cell: CheckedCell },
    Separator { cell: CheckedCell },
    Text { label: String, cell: CheckedCell },
}
pub struct CheckedCell { pub span: usize, pub new_row: bool }
pub enum CheckedWidget { TextField, TextArea { rows: u32 }, NumberField, CheckBox,
                         ComboBox, RadioGroup, Date(Option<DatePattern>), DateTime(Option<DatePattern>) }
```

- **Défauts résolus ici** :
  - grille `[0, 1]` sans `columns` ;
  - étendue 1 pour les cellules d'un champ et d'un texte ;
  - toute la ligne, c'est-à-dire le nombre de colonnes de la grille qui contient l'élément, pour un groupe et un séparateur ;
  - widget par défaut du type de l'attribut. `pub fn default_widget(AttrType) -> CheckedWidget` est publique : la présentation s'en sert pour le formulaire déduit.
- **Compatibilité widget / type** : une seule fonction `accepts(widget, attr_type) -> bool`. Elle sert à la vérification, et un test la confronte à `default_widget`, qui doit toujours être accepté.
- **Motifs de date** : `DatePattern::parse(pattern, with_time)`, comme pour les colonnes. `with_time` vient du widget (`DateField` faux, `DateTimeField` vrai), et la compatibilité avec le type de l'attribut est vérifiée à part.
- **Position des erreurs** : la récursion passe la position de l'élément sous forme de texte (`form.elements[0].elements[2]`). C'est le plus lisible pour qui corrige `app.json` à la main, et cela ne demande pas d'identifiant aux éléments.
- **Nouvelles erreurs** :
  - `UnknownField { class, location, path }` ;
  - `WidgetMismatch { class, location, path, widget, attr_type }` ;
  - `SpanOutOfGrid { class, location, span, columns }` et `NoGridColumn { class, location }` ;
  - `GrowValue { class, location, index }` et `TextAreaRows { class, location, rows }` ;
  - `FieldPattern { class, location, path, pattern, error }` ;
  - `UnknownFormClass { view, class }`.
- **Erreur rendue plus générale** : `UnexpectedData { view }` reçoit le genre du widget (`"empty"`, `"form"`), pour que son message convienne aux deux vues.
- **Vues** : `CheckedView` gagne `Form { id, class: ClassId }`. `check_view` refuse `data`, et vérifie la classe par `model.class_id`. Une classe abstraite est acceptée : ses objets auront une sous-classe concrète.
- **Libellés** : avant `check_labels`, `form_label_keys(&self.presentations)` parcourt les `FormDef` bruts et rassemble les clés citées (titres de groupe, textes, `label` des champs). `check_labels` ne signale `UnknownLabelKey` que pour une clé qui ne désigne rien **et** n'est pas dans cet ensemble. Le parcours se fait sur les définitions brutes, pas sur les formulaires vérifiés : une clé reste reconnue même si son formulaire a une autre erreur. On évite ainsi un avertissement trompeur à côté de l'erreur.

### D3. La grille (`ortelia-presentation`, nouveau module `grid`)

```rust
pub struct CellSpec { pub span: usize, pub new_row: bool }
pub struct Placement { pub row: usize, pub col: usize, pub span: usize }
pub struct Grid { grow: Vec<f32>, cells: Vec<Placement>, rows: usize }
pub struct ColumnWidths { pub widths: Vec<f32>, pub positions: Vec<f32>, pub total: f32 }

impl Grid {
    pub fn new(grow: Vec<f32>, specs: &[CellSpec]) -> Grid;   // placement
    pub fn min_widths(&self, cell_min: &[f32], spacing: f32) -> Vec<f32>;
    pub fn widths(&self, cell_min: &[f32], available: f32, spacing: f32) -> ColumnWidths;
}
```

- **Non générique.** La grille ne connaît que des placements. Les contenus vivent dans un `Vec` parallèle, à la même position, chez son utilisateur (le formulaire). Un `Grid<T>` qui porterait les contenus serait la solution « classe générique » de Java, mais la règle du dépôt est de ne pas ajouter de généricité superflue. Deux tableaux de même index suffisent, et la grille resservira à d'autres widgets sans changer.
- **Placement**, en un seul passage sur les cellules, avec un curseur `(row, col)` :
  1. si `new_row` et `col > 0`, passer à la ligne suivante ;
  2. si `col + span > colonnes`, passer à la ligne suivante ;
  3. placer la cellule, puis avancer `col` de `span`.

  Le nombre de lignes est la ligne de la dernière cellule plus un. La vérification a garanti `1 <= span <= colonnes` : une cellule tient toujours sur une ligne vide.
- **Largeurs** (exigences de `grid-layout`) :
  1. minimum de chaque colonne : le plus grand des minimums de ses cellules d'étendue 1 ;
  2. pour chaque cellule d'étendue supérieure à 1, dans l'ordre des cellules : le manque, s'il y en a un, est ajouté aux colonnes de poids positif de son étendue, au prorata des poids, ou sinon à sa dernière colonne ;
  3. le surplus de largeur disponible est donné aux colonnes de poids positif, au prorata ;
  4. les positions sont les sommes cumulées des largeurs et des espacements.

  Tout est en `f32`, comme les parts des splits.
- **Mesures fournies par l'interface** : `cell_min` est indexé comme les cellules. La grille ne sait rien des polices. C'est le seul point où l'interface participe au calcul.

### D4. Le formulaire dans la présentation (`ortelia-presentation`, nouveau module `form`)

```rust
pub struct FormView { id: String, class: ClassId, form: FormGrid, object: Option<ObjectId> }
pub struct FormGrid { pub grid: Grid, pub cells: Vec<FormCell> }
pub enum FormCell {
    Label(String),                                  // field label, unit included
    Widget(FieldCell),                              // attr, kind, label_cell: usize, format, choices
    Group { title: Option<String>, grid: Box<FormGrid> },
    Separator,
    Text(String),
}
pub struct FormRef<'a> { view: &'a FormView, store: &'a Store }
pub enum FieldValue { Text(String), Check(Option<bool>), Choice(Option<usize>) }
```

- **Construction**, dans `LayoutNode::build`, pour chaque `CheckedView::Form`. La présentation de la classe est résolue par la même chaîne que `TableView::new` : la classe, puis ses ancêtres. La règle est la même que pour la table : la présentation la plus proche s'applique, et sans `form` le formulaire est déduit. Le formulaire déduit est un `CheckedGrid` construit par la présentation (un `CheckedField` par attribut, `default_widget`, grille `[0, 1]`) : ensuite, un seul chemin de code.
- **Aplatissement**, de `CheckedGrid` vers `FormGrid` :
  - chaque champ donne deux `FormCell`, `Label` puis `Widget`, et deux `CellSpec` ;
  - un groupe donne une cellule qui porte sa propre `FormGrid`, dans une `Box` : comme pour `ViewContent::Table`, cela garde `FormCell` petit ;
  - `FieldCell::label_cell` garde l'index de la cellule du libellé, pour relier le widget à son libellé (accessibilité, D5).
- **Libellés**, cherchés une fois, par le `LabelLookup` de l'ouverture, ce qui ne compte chaque clé manquante qu'une fois même quand la table et le formulaire la montrent :
  - attribut : `lookup.attribute(class, attr)` ;
  - clé libre : nouvelle méthode `lookup.key(key)`, qui rend `#clé#` si la clé manque ;
  - unité ajoutée entre parenthèses, par la fonction déjà utilisée pour les en-têtes de la table, déplacée dans `labels` pour être partagée ;
  - libellés des littéraux d'un énuméré : gardés dans `FieldCell::choices`.
- **Valeurs** : `FormRef::value(cell) -> FieldValue`, calculée à chaque appel, comme `TableRef::cell`. Sans objet, si l'objet est mort (`!store.is_alive`) ou s'il n'est pas de la classe de la vue (`!model.is_subclass_of`), le résultat est la valeur vide du widget : `Text("")`, `Check(None)` ou `Choice(None)`. Le texte vient d'une nouvelle méthode `ValueFormat::text(value, keep_lines)`, extraite de `ValueFormat::cell`, qui l'appelle avec `keep_lines = false`. La zone de texte l'appelle avec `true`.
- **Révisions** : rien à faire. Les valeurs sont lues dans le `Store` courant à chaque image, et un objet retiré par le remplacement d'une branche est détecté par `is_alive`. Le contrôle est nécessaire : une fois l'objet mort, son `ObjectId` peut désigner un autre objet. Il est fait à chaque lecture. Si le `Store` réutilise un jour les identifiants, `set_form_object` devra aussi retenir la révision de l'objet ; les tests de l'objet retiré le diraient.
- **API pour l'interface et le futur broker** :
  - `ViewContent::Form(Box<FormView>)` et `ViewRef::Form(FormRef)` ;
  - `AppState::set_form_object(view: &str, object: Option<ObjectId>) -> bool` (faux pour une vue inconnue ou qui n'est pas un formulaire) ;
  - `TableRef::object(row) -> Option<ObjectId>` ;
  - réexport de `ObjectId`, pour que les tests de `ortelia-ui-egui` trouvent la personne `Dupont` par la table sans dépendre de `ortelia-model`.

  Ce n'est pas une intention : le choix de l'objet viendra de la présentation elle-même (le broker), jamais d'un clic interprété par l'interface.

### D5. Dessin (`ortelia-ui-egui`, nouveau module `form`)

- **Défilement** : `ScrollArea::both().auto_shrink(false)`, avec un `id_salt` qui contient l'identifiant de la vue. La largeur disponible est celle de la zone de défilement.
- **Deux passes par image sur une grille**, toutes deux sans état gardé d'une image à l'autre :
  1. **Mesure** : une largeur minimale par cellule.
     - Un texte (libellé, texte fixe, titre de groupe) est mesuré par `WidgetText::into_galley(..)`, sans retour à la ligne. egui met les mises en page de texte en cache : remesurer à chaque image ne coûte presque rien.
     - Un widget prend un minimum fixe par genre, constante de `ortelia-ui-egui` : texte 80, nombre 60, case `interact_size.x`, liste 100, date 90, date-heure 150, zone de texte 120 points. Pour des boutons radio, c'est la somme des libellés mesurés.
     - Un groupe prend le minimum de sa grille (`Grid::min_widths`, appel récursif), plus les marges et le trait de son cadre.
     - Un séparateur : 0.
  2. **Dessin**, ligne par ligne :
     - chaque cellule reçoit un `Ui` enfant par `ui.new_child(UiBuilder::new().max_rect(rect).id_salt(index))`. `rect` a la position et la largeur de ses colonnes, espacements compris, et une hauteur ouverte ;
     - la hauteur de la ligne est la plus grande des hauteurs dessinées, et la ligne suivante commence en dessous, après l'espacement vertical ;
     - à la fin, `ui.allocate_rect(rect_total, Sense::hover())` réserve dans le parent la place de toute la grille, pour la zone de défilement.

     `new_child` plutôt que `scope_builder`, qui réserverait l'espace de chaque cellule dans le parent et décalerait le curseur. L'`id_salt` par cellule donne à chaque widget un identifiant stable, même quand deux champs montrent le même attribut.
- **Alignement vertical** : un libellé est dessiné dans une boîte de hauteur `interact_size.y`, centré. Il est ainsi aligné sur les widgets d'une ligne (texte, nombre, liste, case). Face à une zone de texte ou un groupe, il reste en haut. C'est sans mesure des hauteurs et sans passe de plus.
- **Widgets en lecture seule** :

  | Genre | Dessin |
  |---|---|
  | texte, nombre, date, date-heure | `TextEdit::singleline(&mut text)` avec `text: &str` (sélectionnable, non modifiable), sur la largeur de la cellule, avec une bordure fine (`TextEdit::frame`, trait des cadres de groupe, sans fond) : egui dessine un champ non modifiable sans fond ni bordure visible, et le formulaire vide ne montrerait que ses libellés. Alignement donné par la présentation (`FieldCell::align`, celui du type de l'attribut, comme dans la table : nombres à droite, dates centrées), appliqué par `TextEdit::align` |
  | zone de texte | `TextEdit::multiline(&mut text).desired_rows(rows)`, avec la même bordure |
  | case | `ui.add_enabled(false, Checkbox::without_text(..).indeterminate(value.is_none()))` |
  | liste | `ComboBox` dans `ui.add_enabled_ui(false, ..)`, `selected_text` = libellé du littéral |
  | boutons radio | `ui.radio(..)` par littéral, dans un `Ui` enfant (`scope_builder`) de disposition gauche-droite avec retour à la ligne, désactivé, de rôle d'accessibilité `RadioGroup` : c'est lui que `labelled_by` relie au libellé |
  | groupe | `Frame::group(ui.style())`, titre en `strong`, puis sa grille sur la largeur intérieure |
  | séparateur | `ui.separator()` |

  Le texte non modifiable garde l'aspect normal et peut se copier. Les cases, listes et boutons radio sont grisés : egui n'a pas de mode « lecture seule non grisé » pour eux. L'incrément 5 rendra les widgets éditables, avec Appliquer / Annuler.
- **Accessibilité** : la réponse du libellé donne son `Id`, et le widget est dessiné avec `.labelled_by(id)`. Les tests trouvent ainsi « le champ de texte de libellé `Nom` » (`get_by_role_and_label(Role::TextInput, "Nom")`).
- **Branchement** : `layout.rs`, `pane_ui`, ajoute `Some(ViewRef::Form(view)) => form::draw_form_view(ui, view)`, sans intention émise. La vue est dessinée dans un `Ui` enfant de rôle d'accessibilité `Form` : un lecteur d'écran, et les tests, y trouvent ses widgets à part de ceux des autres vues (les cases de la table sont nommées par leur colonne, `Contact`). Ce nœud n'a pas de rectangle : les tests déduisent les bords de la vue de ceux de la fenêtre.

### D6. L'annuaire

Le formulaire de `Personne` reste sur la grille par défaut (`[0, 1]`) : la vue de droite n'a que 350 points de large à l'ouverture, et deux paires de colonnes y seraient à l'étroit. `situation` reste en liste déroulante, pour la même raison. Les boutons radio, la grille de quatre colonnes et les groupes côte à côte sont vérifiés par les tests, sur des applications écrites pour eux.

```json
"form": {
  "elements": [
    { "_type_": "Group", "label": "Personne.identite", "elements": [
        { "_type_": "Field", "path": "nom" },
        { "_type_": "Field", "path": "prenom" },
        { "_type_": "Field", "path": "date_naissance",
          "widget": { "_type_": "DateField", "format": "d MMMM yyyy" } } ] },
    { "_type_": "Field", "path": "nombre_enfants" },
    { "_type_": "Field", "path": "taille" },
    { "_type_": "Field", "path": "contact_autorise" },
    { "_type_": "Field", "path": "situation" },
    { "_type_": "Field", "path": "inscription" },
    { "_type_": "Separator" },
    { "_type_": "Field", "path": "notes",
      "labelCell": { "span": 2 }, "cell": { "newRow": true, "span": 2 },
      "widget": { "_type_": "TextArea", "rows": 5 } }
  ]
}
```

Libellés ajoutés : `Personne.identite` (`Identité`, `Identity`). La vue `proprietes` devient `{ "_type_": "Form", "class": "Personne" }`.

### D7. Tests, par niveau

| Niveau | Ce qui est testé |
|---|---|
| `ortelia-config` | un test par scénario du delta d'`app-config` (lecture, erreurs situées, défauts résolus) ; `accepts(default_widget(t), t)` pour chaque type ; clés libres de `labels` |
| `ortelia-presentation`, `grid` | un test par scénario de `grid-layout` ; propriétés simples : les largeurs ne sont jamais sous les minimums, et égalent la largeur disponible dès qu'une colonne s'élargit et que la place suffit |
| `ortelia-presentation`, `form` | scénarios de `form-view` hors interface : résolution, héritage, formulaire déduit, cellules, libellés, valeurs (Dupont, Martin, notes sur deux lignes), formulaire vide, objet d'une autre classe, objet retiré (remplacement de branche sur une copie de l'annuaire), valeur modifiée par une transaction |
| `ortelia-ui-egui`, egui_kittest | affichage (valeurs relues par l'accessibilité), champs alignés et zone de texte sous son libellé (par les rectangles des nœuds, jamais en coordonnées écrites en dur), lecture seule (saisie, clic sans effet ni intention), défilement par la molette ; la vue vide sur une application de test |
| captures | `main_window` renouvelée, nouvelle `form` (parts 0,4 / 0,6 par `Intent::SplitResized`, Dupont par `set_form_object`) |

## Risks / Trade-offs

- **[Libellés d'un groupe non alignés sur ceux du formulaire]** Chaque grille calcule ses colonnes : dans l'annuaire, les champs du groupe « Identité » n'ont pas la même colonne de libellés que les champs suivants. → C'est le comportement de MigLayout avec des panneaux imbriqués. On pourra plus tard offrir des colonnes partagées entre grilles, comme enrichissement du composant.
- **[Minimums fixes des widgets]** Les minimums de largeur des widgets sont des constantes de l'interface, pas des mesures. → Ils ne servent qu'à fixer un plancher quand la vue rétrécit. Le défilement horizontal prend le relais en dessous.
- **[Aspect grisé des cases et des listes]** En lecture seule, cases, listes et boutons radio sont grisés, et les textes non. → Le contraste est voulu : seuls les textes se copient. L'édition (incrément 5) supprimera l'écart.
- **[`new_child` et zone de défilement]** Les `Ui` enfants ne réservent pas d'espace : si la réservation finale est oubliée ou fausse, la zone de défilement ne connaît pas la taille du contenu. → Le test de défilement le détecte. La réservation est faite à un seul endroit, à la fin du dessin d'une grille.
- **[Règle d'héritage]** Une sous-classe qui a une présentation sans `form` reçoit un formulaire déduit, et non celui de sa classe mère. C'est la règle de la table, gardée pour la cohérence. → Elle est écrite dans `form-view`. La compléter (formulaire qui étend celui de la classe mère) est hors périmètre.
- **[Changement 3]** Le broker montrera des objets de classes diverses, alors que les formulaires ne sont construits qu'à l'ouverture, pour la classe de la vue. → La construction est une fonction de la classe. Le changement 3 la rendra paresseuse, et devra décider quand signaler les libellés manquants d'un formulaire construit tard.

## Migration Plan

`app.json` passe de 0.5 à 0.6, sans migration, comme à chaque version : `examples/annuaire/app.json` et les textes `app.json` écrits dans les tests (constantes, ou `APP_FORMAT_VERSION`) changent dans le même commit. Les fichiers d'état `app.session.json` ne changent pas de format : une vue formulaire n'y écrit rien. Retour arrière : revenir au commit précédent, l'annuaire 0.5 y est intact.
