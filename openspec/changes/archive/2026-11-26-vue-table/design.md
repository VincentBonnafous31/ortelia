# Conception : vue-table

## Context

- `AppContext::open` (`ortelia-service`) ouvre une application : `app.json` vérifié (`CheckedApp`), Document alimenté, état de chaque source. Une erreur de `app.json` ou du modèle donne un `OpenError` ; une source en échec laisse sa branche vide (`SourceStatus::Failed`).
- Le Document tient une révision (`revision()`) : la présentation la compare pour savoir qu'un cache est périmé, sans observateurs. Dans ce changement, rien ne modifie le Document après l'ouverture, mais les caches sont construits pour l'être.
- `ortelia-presentation` ne contient qu'un `AppState` à deux textes. `ortelia-ui-egui` dessine un `CentralPanel` ; `ortelia` n'accepte que `--version` et `--help`.
- egui 0.37 (vérifié dans les sources de la crate) :
  - `egui_extras::TableBuilder` redimensionne les colonnes mais ne les réordonne pas, et garde leurs largeurs **par position** sous son `id_salt` ; `TableBuilder::reset()` efface cet état, et `TableBody::widths()` donne les largeurs courantes ; `body.rows(..)` ne dessine que les lignes visibles ;
  - `egui::Modal::new(id).show(ctx, ..)` affiche une boîte modale ;
  - le glisser-déposer passe par `Response::dnd_set_drag_payload`, `dnd_hover_payload` et `dnd_release_payload` ;
  - `Context::send_viewport_cmd(ViewportCommand::Close)` ferme la fenêtre ;
  - egui_kittest simule Maj+clic (`Node::click_modifiers`) et le glisser (`drag_at`, `hover_at`).

## Goals / Non-Goals

**Goals :**
- Toute la logique de la table (colonnes, libellés, formats, tri, filtre, largeurs, ordre) dans `ortelia-presentation`, testée sans interface ; `ortelia-ui-egui` ne fait que dessiner et émettre des intentions.
- Un état de la table complet dans la présentation (ordre, largeurs, tri, filtre), pour qu'`etat-de-session` n'ait qu'à l'enregistrer et le restaurer.
- Tenir 100 000 lignes : tri sous 200 ms, rien de proportionnel au nombre de lignes à chaque image.

**Non-Goals :**
- Un mécanisme général de vues et de conteneurs : la fenêtre a une seule vue, et le type `Content` n'a qu'une variante, prête à recevoir `Split` et les piles d'onglets.
- La traduction des textes propres à Ortelia : titres et boutons des boîtes sont des constantes françaises de la présentation, regroupées pour l'incrément 4 ; le détail des erreurs reste celui des couches basses, en anglais.

## Decisions

### D1. Répartition par crate

```
ortelia-config        app.json 0.4 : types serde, vérification contre le modèle,
                      libellés vérifiés, motifs de date analysés, avertissements
ortelia-service       AppContext::warnings()
ortelia-presentation  AppState (application ouverte ou en erreur), Intent, TableView,
                      Labels, formats des valeurs, pliage des textes
ortelia-ui-egui       dessin : table, en-têtes (tri, glisser-déposer), filtre,
                      boîtes modales, barre d'état, options de la fenêtre
ortelia               argument de chemin, ouverture, code de sortie
xtask                 gen-data --model annuaire
```

La règle des couches est respectée : la présentation ne dépend que de `ortelia-service` (et, à travers lui, de `ortelia-config` et `ortelia-model`), et d'`icu_normalizer`, qui n'a rien d'une bibliothèque d'interface.

### D2. Format 0.4 de `app.json` (`ortelia-config`)

```json
{
  "formatVersion": "0.4", "id": "annuaire", "name": "Annuaire des personnes",
  "languages": ["fr", "en"],
  "labels": [ { "lang": "fr", "entries": [ { "key": "Personne.nom", "text": "Nom" } ] } ],
  "model": { "files": ["annuaire.model.json"], "root": "Racine" },
  "sources": [ { "id": "personnes", "format": "json", "file": "personnes.json",
                 "target": "/annuaire/personnes" } ],
  "presentations": [
    { "class": "Personne",
      "table": {
        "columns": [ { "path": "nom", "width": 140 },
                     { "path": "date_naissance", "format": "dd/MM/yyyy" } ],
        "sort": [ { "path": "nom" }, { "path": "prenom" } ] } } ],
  "window": { "title": "Annuaire", "width": 1000, "height": 700,
    "content": { "_type_": "View", "id": "personnes",
                 "widget": { "_type_": "Table" }, "data": "/annuaire/personnes" } }
}
```

- Le polymorphisme passe par `_type_`, comme dans les données : `#[serde(tag = "_type_")]` sur les énumérations Rust `Content` (variante `View`) et `Widget` (variante `Table`). C'est l'équivalent d'un `@JsonTypeInfo` de Jackson. `Widget::Table` est une structure vide pour l'instant : elle recevra les surcharges propres à la vue (la présentation par classe surchargeable dans chaque vue).
- Pas de dictionnaire : libellés et présentations sont des listes, et les références (classe, attribut, colonne du tri) sont des noms.
- `path` plutôt qu'`attribute` dans les colonnes : la clé restera la même quand les chemins de navigation (`adresse.ville`) viendront ; pour l'instant, elle n'accepte qu'un nom d'attribut.
- Vérification (`AppFile::check`), qui rassemble toutes les erreurs comme aujourd'hui : chemin `data` (même parcours que les cibles, dernière référence obligatoirement multiple), présentations, colonnes, tri, formats, libellés. Le résultat, `CheckedApp`, gagne `languages`, `labels`, `presentations` (indexées par `ClassId`), `window` et `warnings`.
- Alternative écartée : un widget écrit comme une chaîne (`"widget": "table"`, comme dans les premières esquisses). Un objet à `_type_` accueillera ses options sans changer de forme.

### D3. Motifs de date : analysés par la configuration, rendus par la présentation

- `ortelia-config` analyse le motif (`DatePattern`, une liste de champs et de textes littéraux) à la vérification : un motif invalide est une erreur d'ouverture, avec les autres.
- `ortelia-presentation` le rend avec les noms des mois et des jours de la langue courante.
- Un sous-ensemble écrit à la main plutôt qu'une bibliothèque :
  - jiff (déjà utilisé) ne connaît que `strftime` et les noms anglais ;
  - ICU4X ne formate que par squelettes sémantiques, et ses motifs libres sont expérimentaux.

  Le sous-ensemble tient en environ 200 lignes testées, sans dépendance.
- Conventions par langue : `fr` (virgule décimale, noms français, `dd/MM/yyyy`) ; toute autre langue suit pour l'instant les conventions anglaises. Les tables de noms (mois et jours, complets et abrégés, d'après CLDR) sont des constantes de la présentation.

### D4. État de l'application dans la présentation

```rust
pub struct AppState {
    window: WindowViewModel,        // title, initial size
    app: Option<OpenedApp>,         // None if the opening failed
    dialog: Option<Dialog>,         // OpenFailed / SourcesFailed / Warnings
    warnings: Vec<String>,          // config + missing labels + source diagnostics
    quit_requested: bool,
    exit_code: u8,                  // 1 if the opening failed
}

struct OpenedApp { context: AppContext, table: TableView }

pub enum Intent {
    Table(TableIntent),             // SortBy { column, add }, SetFilter(String),
                                    // MoveColumn { from, to }, ResizeColumn { column, width }
    ShowWarnings, CloseDialog, Quit,
}
```

- `AppState::open(path)` ne peut pas échouer : un échec d'ouverture devient l'état « en erreur », avec la boîte `OpenFailed`. L'exécutable lit `exit_code()` avant de lancer la fenêtre : le code ne dépend pas de la façon dont la fenêtre est fermée.
- Chaque image suit trois temps : `prepare()` (`&mut self` : met les index à jour si la révision a changé), dessin (`&self`), puis `dispatch(intent)` des intentions recueillies. En Rust, l'interface ne peut pas garder une référence partagée vers l'état pendant qu'elle le modifie : séparer lecture et écriture évite de se battre avec le vérificateur d'emprunts, et c'est exactement le contrat « lire les view-models, émettre des intentions » d'`architecture-couches.md`.
- Les fonctions de dessin **renvoient** leurs intentions (`Vec<Intent>`) au lieu d'appeler l'état : les tests egui_kittest vérifient ce qui est émis sans état réel.

### D5. Table : colonnes, cellules, index

- `TableView` garde, pour chaque colonne et dans l'ordre affiché : l'attribut (`AttrId`), l'en-tête calculé (libellé et unité), l'alignement, la largeur, le type de valeur et le motif éventuel. Le tri est une liste `(AttrId, Asc | Desc)`. Il désigne les colonnes par attribut, et non par position : un déplacement ne le touche pas.
- Largeurs par défaut, en points : texte 140, énuméré 120, entier 70, décimal 80, booléen 60, date 100, date-heure 160.
- Cellules : `enum Cell { Empty, Text(String), Check(bool) }`, calculées à la demande par `cell(row, column)` à partir du Store. egui ne les demande que pour les lignes visibles, environ 30 lignes × 9 colonnes par image.
- Index des lignes :
  1. `base` : les `ObjectId` de la liste, dans l'ordre du Document ;
  2. `sorted` : des positions dans `base`, triées par `sort_by`. Le tri de la bibliothèque standard est stable (tri fusion), ce qui donne l'égalité stable de la spécification. Les valeurs absentes sont rangées en dernier par le comparateur lui-même, avant l'inversion du sens ;
  3. `rows` : `sorted` filtré.

  Chaque étage porte la révision et les paramètres avec lesquels il a été calculé, comme une clé de cache : `prepare()` ne recalcule que ce qui est périmé.
- Clés de tri : pour une colonne de texte, les clés pliées de toutes les lignes sont calculées une fois par révision et gardées (`Vec<Box<str>>`). Les autres types sont lus directement dans le Store, ce que l'essai S1 a montré rapide (bloc de valeurs par objet). Un énuméré se compare par l'indice de son littéral.
- Filtre : le texte plié de toutes les colonnes filtrables est préparé à la première saisie, puis gardé jusqu'au changement de révision. Une seule chaîne par ligne, avec le séparateur `\u{1F}` entre les colonnes : un mot ne pouvant pas contenir ce caractère, « chaque mot dans au moins une colonne » revient à « chaque mot dans la chaîne de la ligne ». Le déplacement d'une colonne ne change pas l'ensemble des colonnes filtrables, donc ne périme pas ce texte.
- Un compteur interne, réservé aux tests (`#[cfg(test)]`), compte les lignes formatées : il vérifie que seules les lignes demandées le sont.

### D6. Pliage des textes (`ortelia-presentation`)

`fold(text)` met en minuscules (`char::to_lowercase`), décompose en NFD par `icu_normalizer::DecomposingNormalizerBorrowed::new_nfd()`, et retire les caractères de classe de combinaison canonique non nulle (`icu_normalizer::properties::CanonicalCombiningClassMap`). Un texte ASCII prend un raccourci, `to_ascii_lowercase`, sans normaliseur.
- `icu_normalizer` 2.4.0 (avec ses données compilées) est déjà dans `Cargo.lock`, tiré par une dépendance indirecte : il n'ajoute presque rien à la compilation.
- Alternative écartée : la collation ICU (`icu_collator`), plus juste pour les égalités et les langues autres que le français et l'anglais. Elle demande une dépendance et des données de plus, et elle compare sans clé précalculée, ce qui met en jeu les 200 ms. Le pliage reste derrière une seule fonction, qu'on pourra remplacer.

### D7. Libellés (`ortelia-config` et `ortelia-presentation`)

- `ortelia-config` vérifie la forme et le sens des clés contre le modèle :
  - langue non déclarée, groupe en double, clé en double : erreurs ;
  - clé qui ne désigne rien : avertissement.

  Il produit une table de recherche `(langue, clé) → texte`. C'est un `HashMap` dans le code : l'interdiction des dictionnaires vaut pour les formats de fichier, pas pour les structures internes.
- `ortelia-presentation` cherche les libellés dans la langue courante (la première de `languages`) :
  - pour un attribut, `Classe.attr` en remontant de la classe affichée jusqu'à la classe qui déclare l'attribut ;
  - à défaut, `#Classe.attr#`, avec la clé de la classe affichée.

  Les libellés de l'en-tête et des littéraux des énumérés affichés sont calculés **une fois**, à la création de la table. Les clés manquantes y sont relevées (une fois chacune), ajoutées aux avertissements et journalisées en `WARN`. Ce sont des noms de configuration, pas des données de l'utilisateur.

### D8. Interface egui (`ortelia-ui-egui`)

- **Table** : `TableBuilder` avec un `id_salt` fixe par vue, `Column::initial(largeur de la présentation).resizable(true).clip(true)`, en-tête collant, `body.rows(hauteur, nombre de lignes, ..)`.
  - Après le dessin, les largeurs lues par `TableBody::widths()` sont comparées à celles de la présentation : un écart produit `ResizeColumn`.
  - Le piège des largeurs gardées par position : la présentation incrémente un numéro de disposition à chaque `MoveColumn`. Quand l'interface le voit changer (elle garde le dernier vu dans la mémoire temporaire d'egui, `ui.data_mut`), elle appelle `TableBuilder::reset()`, et egui repart des largeurs de la présentation, qui ont suivi leurs colonnes.
  - Alternative écartée : un `id_salt` qui dépend de l'ordre. Revenir à un ordre déjà vu retrouverait des largeurs périmées.
- **En-têtes** : une zone `Sense::click_and_drag()` qui laisse libre la poignée de redimensionnement au bord droit.
  - Un clic donne `SortBy { add: modifiers.shift }`.
  - Un glissement pose la charge `ColumnDrag(position)` ; l'en-tête survolé dessine un trait d'insertion ; le relâchement donne `MoveColumn { from, to }`.
  - L'indicateur de tri est un texte (`⏶`, `⏷`, suivi du rang s'il y a plusieurs critères), d'où la feature `monochrome_emoji_fonts` d'egui, qui fournit ces symboles et `⚠` (les triangles `▲` et `▼` n'y sont pas).
- **Cellules** : un texte aligné selon la colonne (`Layout`), ou une case à cocher désactivée (`ui.add_enabled(false, Checkbox::without_text(..))`).
- **Filtre** : une barre au-dessus de la table, avec un `TextEdit::singleline` (texte d'aide « Filtrer »), un bouton d'effacement et le compteur `lignes / total` à droite.
- **Boîtes** : `egui::Modal`, une par sorte de boîte. « Quitter » émet `Quit` ; quand `quit_requested()` est vrai, l'application envoie `ViewportCommand::Close`.
- **Barre d'état** : `egui::Panel::bottom`, affichée seulement s'il y a des avertissements. Elle contient un bouton « ⚠ n avertissements » qui émet `ShowWarnings`.
- **Fenêtre** : `ViewportBuilder::with_title(..).with_inner_size([largeur, hauteur])`.

### D9. Lancement (`ortelia`)

`cli::parse` renvoie `Command::Run { path: Option<PathBuf> }` : un argument qui ne commence pas par `-` est le chemin. `main` résout le fichier (un dossier donne `<dossier>/app.json` ; sans argument, `app.json` du dossier courant), appelle `AppState::open`, écrit les erreurs d'ouverture sur la sortie d'erreur et dans le journal (`ERROR`), lance la fenêtre, puis renvoie `exit_code()`. Un dossier sans `app.json` donne l'erreur d'entrée-sortie existante d'`AppContext::open`, qui cite le fichier attendu.

### D10. Générateur de l'annuaire (`xtask`)

`gen-data --model annuaire --count <n> --out <dossier>` :
- écrit `personnes.json` avec le même écrivain JSON en flux que le modèle d'essai et la même graine (`fastrand`) ;
- recopie `app.json` et `annuaire.model.json`, inclus à la compilation par `include_str!` (l'équivalent d'une ressource embarquée dans un jar). La clé `$schema` recopiée vise un chemin relatif qui n'existe plus dans le dossier généré : elle est sans effet sur Ortelia ;
- tire les noms et les prénoms de listes fixes qui comprennent des accents, des particules et des majuscules internes. Les attributs facultatifs sont absents dans une part fixe des personnes ;
- `--size` reste obligatoire pour le modèle d'essai, et `--count` l'est pour l'annuaire.

### D11. Mesure de la table

Un exemple de `ortelia-presentation`, `table-bench`, sur le modèle de `s1` :

```bash
cargo xtask gen-data --model annuaire --count 100000 --out ~/annuaire-100k
cargo run --release -p ortelia-presentation --example table-bench -- ~/annuaire-100k
```

Il ouvre l'application, puis mesure le tri par `nom`, la préparation du filtre et le filtre `dup`, et affiche un tableau Markdown. Son test, en debug sur `examples/annuaire`, vérifie seulement qu'il s'exécute. Les seuils de la spécification se vérifient en release, à la main, et le résultat sera consigné dans le bilan de l'incrément.

### D12. Tests

| Niveau | Contenu |
|---|---|
| 1. présentation | colonnes (configurées, héritées, déduites), en-têtes et unité, tri (cycle, Maj, ordre des valeurs, absents, énumérés, stabilité, pliage), filtre, compteur, largeurs et déplacements, recalcul par révision, lignes formatées, libellés et `#clé#`, formats (nombres, dates, motifs FR/EN), états d'ouverture et boîtes |
| 1. config | lecture et vérification du format 0.4, motifs, libellés (erreurs et avertissements) |
| 2. kittest | en-têtes et indicateurs affichés, intentions au clic, au Maj+clic, au glisser et à la saisie, boîtes et boutons, barre d'état |
| 3. captures | `main_window` (annuaire) et `open_error` |
| 4. fumée | `ortelia examples/annuaire` |

Le test de capture qui prouve qu'une image différente échoue compare désormais deux états de la table (filtre vide et filtre `dup`), puisque le titre central a disparu.

## Risks / Trade-offs

- [serde : `tag = "_type_"` et `deny_unknown_fields`] Un enum à étiquette interne met le contenu en mémoire tampon avant de le lire ; les erreurs qui en sortent citent la position de la fin de l'objet plutôt que celle de la clé fautive. → Vérifier dès la première tâche que les clés inconnues sont bien refusées dans `View` et `Table`, et que schemars produit un `oneOf` avec `_type_` constant. Accepter la position approximative, puisque le message cite la clé.
- [Clic et glissement sur le même en-tête] Un léger mouvement pendant un clic pourrait démarrer un glissement, ou la zone de l'en-tête masquer la poignée de redimensionnement. → egui ne démarre un glissement qu'au-delà d'un seuil ; la zone s'arrête avant la poignée. À vérifier à la main avec `/lancer`, en plus des tests kittest.
- [Préparation du filtre sur 100 000 lignes] Formater toutes les cellules prend un temps proportionnel au volume, au moment de la première frappe. → La préparation est faite une seule fois par révision, et sa durée est mesurée par `table-bench`. Si elle dépasse la seconde, on la fera en tâche de fond, plus tard.
- [Textes des boîtes en deux langues] Titres en français, détail des erreurs en anglais. → C'est accepté jusqu'à l'incrément 4, qui traduira les textes d'Ortelia.
- [Langues autres que `fr` et `en`] Elles prennent les conventions anglaises (nombres, noms des mois). → Documenté. Il faudra des données par langue (CLDR) quand une application en demandera une autre.
- [Taille de l'exécutable] La feature `monochrome_emoji_fonts` ajoute une police. → Quelques centaines de Ko, contre une police maison ou des symboles dessinés à la main.

## Migration Plan

- `examples/annuaire/app.json` passe au format 0.4. Il n'y a pas d'autre `app.json`, et pas de migration automatique : le format reste à 0.x, et la spécification refuse les autres versions.
- `schemas/app.schema.json` est régénéré par `cargo xtask gen-schemas`.
- Pour lancer Ortelia, `tools/launch.sh` reçoit le chemin de l'application, par défaut `examples/annuaire`.

## Open Questions

- Les largeurs par défaut et la hauteur des lignes seront ajustées au vu de la première capture : seules les références d'images changent.
