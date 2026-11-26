# Conception : etat-de-session

## Context

- `vue-table` a mis tout l'état de la table dans `TableView` (`ortelia-presentation`) : colonnes dans l'ordre affiché, avec leur largeur ; tri par attribut (`Vec<(AttrId, SortOrder)>`) ; texte du filtre. Les intentions `MoveColumn`, `ResizeColumn`, `SortBy` et `SetFilter` le modifient. Il ne reste qu'à le lire et à l'écrire.
- `AppState::open(path)` reçoit le chemin du fichier de l'application (`cli::app_file` a déjà résolu un dossier en `<dossier>/app.json`), ouvre l'application et calcule la fenêtre (`WindowViewModel` : titre, largeur, hauteur) depuis `window`. `ortelia-ui-egui` en tire les options de la fenêtre native (`with_inner_size`), une seule fois, au lancement.
- Les tests de l'interface et les captures d'images ouvrent `examples/annuaire` par `AppState::open`.
- eframe 0.37 et egui 0.37, vérifiés dans les sources des crates (`~/.cargo/registry/src/`) :
  - `eframe::App::on_exit(&mut self)` est appelé une fois à la fermeture, quelle qu'en soit la cause (bouton de la fenêtre, `ViewportCommand::Close`), après la fin de la boucle d'événements. Sa signature prend un `Option<&glow::Context>` avec la feature `glow` d'eframe ; Ortelia utilise les features par défaut (`wgpu`, sans `glow`), donc `fn on_exit(&mut self)` ;
  - `App::save(&mut self, storage)` n'est appelé qu'avec la feature `persistence`, absente (pas de `ron` dans `Cargo.lock`) ;
  - `ViewportInfo` (lu par `ctx.input(|i| i.viewport())`) donne `maximized`, `fullscreen` et `minimized` en `Option<bool>`, et `inner_rect`, qui vaut **toujours `None` sous Wayland** (donc sous WSLg) ; `Context::viewport_rect()` donne en revanche la surface de la fenêtre, en points, sur toutes les plateformes ;
  - egui_kittest : `Harness::new_eframe` crée l'application, `state_mut()` y donne accès, `set_size()` fixe la taille de la fenêtre simulée. La destruction du harnais n'appelle pas `App::on_exit` : un test l'appelle lui-même.

## Goals / Non-Goals

**Goals :**
- Capture et restauration dans `ortelia-presentation`, testées sans interface, comme le reste de la logique d'affichage ; l'interface ne fait que signaler la taille de la fenêtre et appeler l'enregistrement à la fermeture.
- Un format de fichier extensible (langue, séparation d'un `Split`, plusieurs vues), lu avec indulgence.
- Des tests qui ne dépendent jamais d'un `app.session.json` laissé dans `examples/` par un lancement manuel, et qui n'y écrivent rien.

**Non-Goals :**
- Un service général de préférences de l'utilisateur : il viendra, s'il le faut, avec le menu de configuration de l'incrément 4.
- La restauration de l'état d'egui lui-même (défilement, focus) : seul l'état de la présentation est enregistré.

## Decisions

### D1. Répartition par crate

```
ortelia-config        `Serialize` sur `SortOrder`
ortelia-presentation  module `session` : format du fichier (types serde), chemin du fichier,
                      lecture, écriture sûre ; `TableView::session` / `restore` ;
                      `AppState::open_with_session`, `save_session`,
                      intention `WindowResized`
ortelia-ui-egui       intention de taille de la fenêtre ; `on_exit` → `save_session`
ortelia               `AppState::open_with_session` au lieu de `AppState::open`
```

La règle des couches est respectée : rien de nouveau ne descend, et `ortelia-presentation` ne gagne que `serde` et `serde_json`, déjà dans le workspace (`serde` 1.0.229 avec `derive`, `serde_json` 1.0.154).

**Pourquoi la lecture et l'écriture du fichier dans la présentation**, et non dans `ortelia-service` : le contenu du fichier n'a de sens que pour la présentation (identifiants de vues, ordre de colonnes, filtre), et le service n'a pas à connaître les vues. Un service générique de « fichiers de l'utilisateur » ne servirait aujourd'hui qu'à ce fichier ; il sera temps de l'extraire si d'autres en ont besoin, par exemple quand l'état déménagera dans le dossier de l'utilisateur. Les entrées-sorties restent rassemblées dans `session.rs`, en deux fonctions (`load`, `save`), et le reste du module est pur, donc testé sans disque.

### D2. Emplacement du fichier

- `session::file_for(app_file: &Path) -> PathBuf` : même dossier, même nom, extension `.json` remplacée par `.session.json` (`app.json` → `app.session.json`). Un fichier d'application sans extension `.json` reçoit `.session.json` en plus de son nom complet (`app` → `app.session.json`, `app.txt` → `app.txt.session.json`), pour ne jamais écraser le fichier de l'application.
- Le chemin est calculé depuis le chemin donné à l'ouverture, sans le rendre absolu : `ortelia examples/annuaire` lit et écrit `examples/annuaire/app.session.json`, relatif au dossier courant, qui ne change pas pendant l'exécution.
- `.gitignore` : `*.session.json`, pour les exemples du dépôt et toute application versionnée avec lui.

### D3. Format du fichier (`ortelia-presentation`, module `session`)

```json
{
  "formatVersion": "1",
  "window": { "width": 1200, "height": 800 },
  "views": [
    {
      "id": "personnes",
      "table": {
        "columns": [ { "path": "prenom", "width": 120 }, { "path": "nom", "width": 180 } ],
        "sort": [ { "path": "nom", "order": "desc" } ],
        "filter": "dup"
      }
    }
  ]
}
```

- Mêmes conventions que `app.json` : clés en camelCase, listes d'objets désignés par leur `id` ou leur `path`, jamais de dictionnaire (le format reste descriptible par le métamodèle). `path` est le nom de l'attribut de la colonne, comme dans la présentation ; il deviendra un chemin quand les colonnes en accepteront.
- `views[].table` est facultatif : une vue d'un autre widget (formulaire, arbre) aura sa propre clé à côté.
- Types serde **sans** `deny_unknown_fields`, contrairement à `app.json` : un fichier écrit par une version plus récente garde ce que la version courante comprend (exigence « Clés inconnues ignorées »). Toutes les clés sauf `formatVersion` sont facultatives (`#[serde(default)]`) ; largeurs et tailles en `Option<f32>`, pour qu'une valeur absente ou `null` s'ignore au lieu d'invalider le fichier.
- Lecture en deux temps : d'abord `formatVersion` seul (une structure qui n'a que ce champ ; serde ignore le reste), puis, s'il vaut `"1"`, le fichier entier. Un fichier de version `"2"` est ainsi signalé comme tel, et non comme une erreur de type sur une clé que la version 2 aurait changée.
- Écriture en JSON indenté (`serde_json::to_string_pretty`) : on peut le lire, ou le supprimer pour revenir à `app.json`.
- Le sens du tri réutilise `ortelia_config::SortOrder` (`asc`, `desc`), qui gagne `Serialize`. La présentation a déjà ce type dans son état ; un second énuméré identique ne ferait que des conversions.

### D4. Capture et restauration d'une table (`ortelia-presentation`, `table.rs`)

- `TableView::session(&self, model) -> TableSession` : les colonnes dans l'ordre affiché (nom de l'attribut, largeur), le tri (nom, sens), le filtre. Les colonnes et le tri sont gardés par `AttrId` ; le modèle donne leur nom.
- `TableView::restore(&mut self, saved: &TableSession, model)`, appelée une fois, juste après `TableView::new` :
  1. **Ordre** : on parcourt les colonnes enregistrées ; chacune qui correspond à une colonne de la table, pas encore placée, est placée à la suite, avec sa largeur enregistrée bornée (20 à 2 000 points), ou sa largeur courante si elle manque ou n'est pas finie. Les colonnes de la table non citées suivent, dans leur ordre de présentation, avec leur largeur de présentation.
  2. **Tri** : la liste enregistrée remplace le tri de la présentation, même vide ; les critères dont la colonne n'est pas dans la table, ou déjà vue, sont écartés.
  3. **Filtre** : le texte enregistré remplace le filtre vide.
  Les index ne sont pas touchés : le premier `prepare` les calcule, comme à toute ouverture. `layout` (numéro de disposition) n'a pas à changer : egui n'a encore rien mémorisé.
- La recherche d'une colonne par nom est un parcours linéaire des colonnes (moins de cent) : une table de hachage n'apporterait rien.

### D5. État de l'application (`ortelia-presentation`, `lib.rs`)

- `AppState::open(path)` reste **sans état de session** : ni lecture, ni écriture. Les tests de l'interface, les captures d'images et `table-bench` l'utilisent, et ne dépendent donc jamais d'un `app.session.json` laissé dans `examples/annuaire` par `/lancer`.
- `AppState::open_with_session(path)` ouvre l'application puis, si elle s'est ouverte, lit `session::file_for(path)` et restaure la fenêtre et chaque vue citée ; il garde le chemin du fichier d'état dans l'état. Une seule vue existe aujourd'hui (`window.content`) : la restauration cherche dans `views` celle de son `id`. L'exécutable n'utilise que celle-ci.
  - Comme en Java, où l'on écrirait deux constructeurs ou une fabrique statique par cas, Rust n'a pas de surcharge : deux fonctions nommées disent ce qu'elles font.
- `AppState::save_session(&self)` calcule l'état et l'écrit (D7). Sans application ouverte, ou ouverte sans état de session, elle ne fait rien. Elle prend `&self` : enregistrer ne change pas l'état.
- **Taille de la fenêtre** : `Intent::WindowResized { width, height }` met à jour une taille « signalée » (`reported_size`), gardée telle quelle, non bornée ; une valeur non finie est ignorée. La taille de l'état de session est la taille signalée, sinon la taille initiale, bornée de 200 à 10 000 points à l'enregistrement. Garder la valeur reçue sans la borner évite que l'interface la voie toujours différente de la taille réelle, et la renvoie à chaque image (exigence « Taille de la fenêtre suivie », scénario « Taille hors bornes »).
- `WindowViewModel` garde le sens de « taille initiale » : la taille restaurée si l'état de session en donne une, bornée, sinon celle de `window`.

### D6. Signalement de la taille de la fenêtre (`ortelia-ui-egui`)

- À chaque image, après le dessin : taille = `ctx.viewport_rect().size()` multipliée par `ctx.zoom_factor()` (le zoom d'egui, Ctrl+plus, réduit les points d'egui, alors que `with_inner_size` prend des points de la fenêtre), et non `ViewportInfo::inner_rect`, qui vaut `None` sous Wayland. Pas d'intention si `maximized`, `fullscreen` ou `minimized` vaut `Some(true)` (`None`, inconnu, compte pour faux), ni si la surface est nulle (une fenêtre réduite sous Windows peut mesurer 0 × 0 sans que `minimized` soit connu), ni si la taille diffère de moins de 0,5 point de `AppState::reported_window_size()` (ou de la taille initiale avant le premier signalement).
- Cette décision est une fonction pure, `window_intent(size, &ViewportInfo, reported) -> Option<Intent>`, testée directement en plus du test par egui_kittest : simuler une fenêtre maximisée dans le harnais dépend de la façon dont egui_kittest remplit `ViewportInfo`, à vérifier dans ses sources pendant la réalisation.
- La comparaison à la taille signalée, et non à la taille enregistrée, fait qu'une fenêtre restée sous 200 points n'émet qu'une intention (D5).
- **Rien n'est signalé avant la première action de l'utilisateur dans la fenêtre** (événement egui de pointeur, de molette, de touche ou de saisie ; le gain du focus ne compte pas). Sous WSLg, les premières images ont montré une taille provisoire de 640 × 480 points, avant que le système applique la taille demandée ; une fermeture juste après (ici, un plantage de WSLg qui termine la boucle d'événements, donc appelle `on_exit`) l'enregistrait, et la fenêtre rouvrait trop petite. Une fois l'utilisateur présent, la taille est celle qu'il voit. Le drapeau est gardé par l'interface (`WindowTracker`), comme `maximized` : c'est un fait de la fenêtre, pas une logique d'affichage. Écarté : attendre qu'une taille reste stable un certain temps, qui demande une horloge et des redessins programmés, et perd un redimensionnement fait juste avant la fermeture.

### D7. Enregistrement à la fermeture

- `OrteliaApp::on_exit(&mut self)` appelle `self.state.save_session()`. C'est le seul point d'enregistrement : `on_exit` est appelé pour toute fermeture normale. Un arrêt brutal (processus tué, panique) n'enregistre rien ; c'est accepté (proposition, hors périmètre), et le test de fumée, qui tue le processus, n'écrit donc rien.
- **Pourquoi pas la persistance d'eframe** (feature `persistence`, `App::save`) : elle range un fichier RON par nom d'application eframe (`"Ortelia"`, le même pour toutes les applications), dans un dossier qu'elle choisit, et mêle l'état d'egui à celui de l'application. Ortelia veut un fichier par application, dans son format, à côté de `app.json`.
- **Écriture sûre** (`session::save`) : écriture de `<nom>.session.json.<pid>.tmp` dans le même dossier (le numéro du processus : deux instances fermées ensemble n'écrivent pas dans le même fichier), puis `std::fs::rename` vers `<nom>.session.json`. Sous Linux, `rename(2)` remplace atomiquement ; sous Windows, la bibliothèque standard de Rust utilise `MoveFileExW` avec `MOVEFILE_REPLACE_EXISTING`, qui remplace aussi un fichier existant. En cas d'échec, le fichier temporaire est supprimé s'il existe, et un `WARN` cite le chemin et l'erreur d'entrée-sortie (`io::Error` ne contient pas de données de l'utilisateur). Pas de `fsync` : après une coupure de courant, on retrouve au pire l'état précédent ou aucun. Le dossier n'est pas créé : c'est celui de `app.json`, qui existe.
- Un enregistrement réussi est journalisé en `INFO` avec le chemin ; une restauration aussi, avec le nombre de vues restaurées.

### D8. Journal sans valeurs de l'utilisateur

- Le message d'erreur de serde_json ne doit **pas** être journalisé : pour une erreur de type, il cite la valeur (`invalid type: string "dup", expected …`). Le `WARN` d'un fichier ignoré donne le chemin, la catégorie (`serde_json::Error::classify()` : syntaxe, données, fin de fichier) et la position (`line()`, `column()`), ce qui suffit pour retrouver l'erreur dans le fichier.
- Le filtre n'apparaît dans aucun message, à aucun niveau ; les tests le vérifient avec un journal capturé au niveau `trace` (`tracing-subscriber`, déjà en dépendance de développement de la présentation).
- La version d'un fichier refusé est citée : ce n'est pas une donnée saisie par l'utilisateur, mais un champ écrit par Ortelia. Elle est tronquée à 16 caractères, au cas où le fichier aurait été modifié à la main.

## Risks / Trade-offs

- [Le dossier de l'application est en lecture seule (installation partagée, support amovible)] → L'enregistrement échoue avec un `WARN`, la fermeture se fait ; la disposition n'est pas retrouvée. Le passage au dossier de l'utilisateur, hors périmètre, réglera ce cas.
- [Un dossier partagé entre plusieurs personnes partage aussi leur état] → Accepté pour l'instant ; même remède.
- [`app.session.json` versionné par erreur avec une application, avec le texte du filtre] → `*.session.json` dans le `.gitignore` du dépôt ; pour les applications des utilisateurs, le dire dans la documentation quand elle existera.
- [Restaurer un filtre sur 100 000 lignes prépare le texte de toutes les lignes à la première image (90 à 113 ms mesurés par `table-bench`)] → Accepté : c'est le coût d'une première saisie, déjà sous la seconde exigée ; le démarrage reste dominé par le chargement des données.
- [Une fenêtre maximisée à la fermeture rouvre à sa dernière taille normale, non maximisée] → Accepté pour cet incrément ; l'état maximisé pourra s'ajouter au format sans changer sa version, grâce aux clés facultatives.
- [Les premières images peuvent précéder l'application de la taille demandée (640 × 480 points vus sous WSLg)] → Aucune taille n'est signalée avant la première action de l'utilisateur dans la fenêtre (D6). En contrepartie, une fenêtre redimensionnée par le gestionnaire de fenêtres puis fermée sans que le pointeur soit jamais entré dans la fenêtre garde sa taille précédente.
- [Deux instances de la même application : la dernière fermée écrase l'autre, ] → Chacune écrit son propre fichier temporaire (numéro du processus) et le renommage est atomique : le fichier final est toujours entier ; accepté.
- [Sous Windows, un antivirus qui tient le fichier ouvert peut faire échouer le renommage] → `WARN` et état précédent conservé ; rien de pire qu'une session non enregistrée.

## Migration Plan

Aucune migration : à la première ouverture, il n'y a pas de fichier d'état, et l'application s'ouvre comme aujourd'hui. Pour revenir à la disposition de `app.json`, supprimer `app.session.json`.
