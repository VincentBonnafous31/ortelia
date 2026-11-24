# Conception : socle-workspace

## Context

Le dépôt ne contient encore que des documents (voir `proposal.md`, section Why). Contraintes qui orientent la conception :

- Sur le poste de développement, `cargo` ne tourne que dans WSL2 : l'application y est compilée pour Linux et affichée par WSLg. La version Windows n'est compilée que par la CI.
- La toolchain de WSL2 est en Rust 1.99.1, profil minimal, sans rustfmt ni clippy.
- egui 0.37 exige Rust 1.95 au minimum. Un prototype jetable a déjà compilé et lancé eframe 0.37 dans ce WSL2 : les paquets système nécessaires y sont installés.
- Le porteur du projet débute en Rust : la structure doit rester lisible et les choix non évidents doivent être expliqués.

## Goals / Non-Goals

**Goals :**
- Une arborescence et des réglages Cargo qui serviront tels quels aux incréments suivants : on ajoutera des crates, sans réorganiser.
- Un premier exemple concret du motif « la présentation calcule, l'interface dessine », avec un test de chaque côté.
- Des tests qui couvrent tous les scénarios des spécifications, la plupart exécutés sous Linux et sous Windows.

**Non-Goals :**
- Le contrat complet entre présentation et interface (intentions, `dispatch`, numéro de révision, demandeur de rafraîchissement) : il sera posé à l'incrément 1, quand il y aura des intentions à transmettre.
- Masquer la console qui s'ouvre avec l'exécutable sous Windows (voir Open Questions).
- Les tests par captures d'images et le pilotage de l'exécutable par l'accessibilité du système (voir Decision 12).
- Optimiser la taille ou le temps de compilation des binaires.

## Decisions

### 1. Arborescence : `crates/` et `xtask/`

```
Cargo.toml              workspace virtuel (pas de [package] à la racine)
Cargo.lock              versionné : c'est une application, pas une bibliothèque
rust-toolchain.toml
.cargo/config.toml      alias `cargo xtask`
crates/
  ortelia-model/        couche 0
  ortelia-config/       couche 1
  ortelia-expr/         couche 1
  ortelia-service/      couche 2
  ortelia-presentation/ couche 3
  ortelia-ui-egui/      couche 4
  ortelia/              couche 5, exécutable
xtask/                  outil du dépôt (vérification des couches)
.github/workflows/ci.yml
```

- La racine est un **workspace virtuel** : `members = ["crates/*", "xtask"]`, `resolver = "3"`. Dans un manifeste virtuel, Cargo ne déduit pas le résolveur de l'édition : il faut l'écrire.
- Un package `ortelia-model` s'importe en Rust sous le nom `ortelia_model` : Cargo remplace les tirets par des soulignés.
- Alternative écartée : les crates à la racine du dépôt. Elles s'y mélangeraient avec `docs/`, `examples/` et `openspec/`.

### 2. Réglages communs hérités par toutes les crates

Dans le `Cargo.toml` racine :
- `[workspace.package]` : `version = "0.1.0"`, `edition = "2024"`, `rust-version = "1.99"`, `license = "MIT OR Apache-2.0"`, `repository`, `publish = false`. Chaque crate écrit `version.workspace = true`, etc. Toutes les crates partagent donc la version de l'application.
- `[workspace.dependencies]` : les versions de toutes les dépendances, externes et internes (`ortelia-model = { path = "crates/ortelia-model" }`). Une crate écrit seulement `ortelia-model.workspace = true`. C'est l'équivalent du `<dependencyManagement>` de Maven ou d'un catalogue de versions Gradle : une version se change à un seul endroit.
- `[workspace.lints]`, hérité par `[lints] workspace = true` : `unsafe_code = "deny"`. Le futur hôte des plugins aura besoin d'`unsafe` pour l'interface C : `deny` (et non `forbid`) lui permettra de l'autoriser localement par `#![allow(unsafe_code)]`, à un endroit visible. Les lints clippy restent ceux par défaut ; la CI traite les avertissements comme des erreurs.
- `publish = false` évite une publication accidentelle sur crates.io. Réserver le nom `ortelia` sur crates.io est une décision séparée.

### 3. Version de Rust fixée par `rust-toolchain.toml`

```toml
[toolchain]
channel = "1.99"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

- rustup lit ce fichier dans WSL2 comme dans la CI, et installe au besoin la version et les composants. La première commande `cargo` dans WSL2 ajoutera donc rustfmt et clippy.
- `"1.99"` désigne la dernière version corrective 1.99.x. Pour changer de version de Rust, on modifie ce fichier et `rust-version` dans un même commit.
- Alternative écartée : `channel = "stable"`. Une nouvelle version de Rust apporte de nouveaux lints clippy, ce qui ferait échouer la CI sans aucune modification du dépôt.

### 4. Dépendances internes : le graphe des couches dès maintenant

| Crate | Dépend de |
|---|---|
| `ortelia-model` | — |
| `ortelia-config` | `ortelia-model` |
| `ortelia-expr` | `ortelia-model` |
| `ortelia-service` | `ortelia-model`, `ortelia-config`, `ortelia-expr` |
| `ortelia-presentation` | `ortelia-service` |
| `ortelia-ui-egui` | `ortelia-presentation`, eframe, egui |
| `ortelia` | `ortelia-presentation`, `ortelia-ui-egui`, tracing… |

- Les crates des couches 0 à 2 ne contiennent qu'un `lib.rs` avec un commentaire `//!` qui décrit leur rôle futur. Pas de code fictif.
- Leurs dépendances sont déclarées même si elles ne servent pas encore. Le graphe matérialise ainsi l'architecture, et la vérification des couches porte sur un vrai graphe. rustc ne signale pas une dépendance inutilisée (le lint `unused_crate_dependencies` est désactivé par défaut).
- `ortelia-config` et `ortelia-expr` sont au même niveau et ne dépendent pas l'une de l'autre. Si `app.json` doit un jour analyser des expressions, on décidera alors de placer `expr` sous `config`, en modifiant la table des couches.

### 5. Présentation minimale (`ortelia-presentation`)

```rust
pub struct AppState { main_window: MainWindowViewModel }
impl AppState {
    pub fn new() -> Self;
    pub fn main_window(&self) -> &MainWindowViewModel;
}
pub struct MainWindowViewModel {
    pub title: String,    // "Ortelia"
    pub heading: String,  // "Ortelia 0.1.0"
}
```

- La version vient de `env!("CARGO_PKG_VERSION")`, évaluée à la compilation, un peu comme un `#define` passé au compilateur C. La version étant commune au workspace, c'est celle de l'application.
- Champs publics, mais l'interface ne reçoit qu'une référence partagée `&MainWindowViewModel`. Comme un `const&` en C++, cette référence interdit toute modification : un view-model en lecture seule n'a donc pas besoin d'accesseurs.
- `AppState` est le nom prévu dans `docs/architecture-couches.md` (§ 3). L'incrément 1 y ajoutera `dispatch`, `revision` et les view-models de vues.
- Tests unitaires dans la crate : les textes de la fenêtre (scénario « Textes de la fenêtre principale »).

### 6. Interface egui (`ortelia-ui-egui`)

- `pub fn run(state: AppState) -> Result<(), Box<dyn std::error::Error>>` ouvre la fenêtre avec `eframe::run_native`. `Box<dyn Error>` joue le rôle d'un `throws Exception` en Java. L'exécutable n'a qu'à afficher l'erreur, sans connaître les types d'eframe.
- `fn native_options(state: &AppState) -> eframe::NativeOptions` met le titre dans `ViewportBuilder::with_title`. Cette fonction est séparée pour être testée : le test lit `options.viewport.title`.
- `struct OrteliaApp { state: AppState, first_frame_logged: bool }` implémente `eframe::App`. Sa méthode `ui(&mut self, ui, frame)` (signature d'eframe 0.37, vérifiée dans les sources) appelle `draw_main_window(ui, self.state.main_window())`.
- Après la première image, `ui` journalise `info!("main window shown")` et passe `first_frame_logged` à `true`, pour ne pas répéter l'événement à chaque image. Cet événement sert au test de fumée (Decision 7) : il prouve qu'une image a vraiment été dessinée. `ortelia-ui-egui` dépend donc de `tracing`, ce qui est permis, puisque `tracing` n'est qu'une API de journalisation (Decision 7, « rôle des crates »).
- `fn draw_main_window(ui: &mut egui::Ui, vm: &MainWindowViewModel)` affiche `vm.heading` au centre d'un `egui::CentralPanel`, avec `CentralPanel::default().show(ui, ..)` (API 0.37). Cette fonction ne contient aucune logique, seulement du dessin.
- **Tests sans écran de l'application complète** : `egui_kittest::Harness::new_eframe(|cc| OrteliaApp::new(AppState::new()))`, grâce à la feature `eframe` d'egui_kittest. La signature `new_eframe(builder: impl FnOnce(&mut CreationContext) -> State)` est vérifiée dans les sources 0.37. On teste ainsi l'`App` réelle, et pas seulement la fonction de dessin.
  - Texte central : après `harness.run()`, `harness.get_by_label("Ortelia 0.1.0")`.
  - Première image : le test installe, pour son seul thread (`tracing::subscriber::with_default`), un subscriber `fmt` qui écrit dans un tampon en mémoire partagé : un petit type `SharedBuffer(Arc<Mutex<Vec<u8>>>)` qui implémente `io::Write`, fourni par une closure (une closure qui renvoie un writer est un `MakeWriter` ; `Arc<Mutex<Vec<u8>>>` seul n'en est pas un). Après plusieurs images, il compte une seule occurrence de `main window shown`. `tracing-subscriber` est donc une dépendance de développement de `ortelia-ui-egui`.
  - `tracing` met en cache, pour chaque point de journalisation, l'intérêt des subscribers. Si un test sans subscriber atteint `main window shown` pendant qu'un autre installe le sien, le cache peut retenir « aucun intérêt » et l'événement est perdu (constaté : un échec sur trois). Les tests qui exécutent `OrteliaApp` sont donc sérialisés par un verrou (`APP_TESTS`).
  - egui_kittest interroge l'arbre d'accessibilité (AccessKit) : il ne faut ni GPU ni affichage, et ces tests tournent dans la CI sous Linux et sous Windows. Les features `wgpu` et `snapshot` ne sont pas activées (pas de captures d'images, voir Decision 12).
- eframe est pris avec ses features par défaut (rendu wgpu, X11 et Wayland), qui ont fonctionné dans WSLg avec le prototype.

### 7. Exécutable (`ortelia`) : ligne de commande, journalisation, codes de sortie

Modules : `main.rs`, `cli.rs`, `logging.rs`.

**Ligne de commande** (`cli.rs`)
- Analyse manuelle de `std::env::args()` vers `enum Command { Run, Version, Help }`, ou une erreur d'usage. Avec deux options, une bibliothèque d'analyse n'apporte rien.
- Alternative pour plus tard : clap, quand arriveront le dossier d'application (incrément 1) et éventuellement `--ui`.

**Codes de sortie**
- `fn main() -> std::process::ExitCode`, l'équivalent du `int main` en C : 0 en cas de succès, 1 si la fenêtre ne s'ouvre pas, 2 pour une erreur d'usage.
- Sans affichage, winit 0.30.13 renvoie une erreur (vérifié dans ses sources) au lieu de paniquer. `run` la remonte, et `main` l'écrit sur la sortie d'erreur et dans le journal.

**Ordre dans `main`**
1. Analyser les arguments. `Version`, `Help` et les erreurs d'usage se terminent ici, sans journal.
2. Initialiser la journalisation, en gardant le garde de vidage jusqu'à la fin de `main`.
3. Installer le hook de panique.
4. Journaliser le démarrage.
5. Créer `AppState`, appeler `ortelia_ui_egui::run`.
6. Journaliser l'arrêt.

**Journalisation : rôle des crates**
- Une seule crate initialise la journalisation : l'exécutable, qui assemble les composants (*composition root*).
- Les bibliothèques n'utilisent que les macros de `tracing` (`info!`, `warn!`…), quand elles en auront besoin. C'est la séparation SLF4J / Logback de Java : `tracing` est l'API, `tracing-subscriber` et `tracing-appender` sont la configuration, présente seulement dans `main`.

**Journalisation : composants** (`logging.rs`)
- `fn log_dir(override_dir: Option<OsString>) -> Option<PathBuf>` : la variable `ORTELIA_LOG_DIR` si elle est non vide, sinon `dirs::state_dir()` (Linux : `$XDG_STATE_HOME`, `~/.local/state` par défaut), sinon `dirs::data_local_dir()` (Windows : `%LOCALAPPDATA%`), suivi de `ortelia/logs`.
- Le dossier d'état est choisi sous Linux parce que la spécification XDG y range les journaux. Sous Windows, `dirs::state_dir()` renvoie `None` : on passe alors à `%LOCALAPPDATA%`.
- Écriture des fichiers : `RollingFileAppender::builder()` avec `.rotation(Rotation::DAILY)`, `.filename_prefix("ortelia")`, `.filename_suffix("log")` et `.max_log_files(7)`, puis `.build(dir)`. On obtient `ortelia.2026-11-24.log`, avec des dates en UTC.
- Création du dossier et suppression des anciens fichiers : `build` supprime les fichiers en trop dès la création comme à chaque changement de jour (vérifié dans les sources de la version 0.2.5). Le dossier est créé par `std::fs::create_dir_all` avant `build` : `build` commence par lister le dossier pour le nettoyer, et écrit une erreur parasite sur la sortie d'erreur s'il n'existe pas encore.
- Écriture en arrière-plan : `NonBlockingBuilder::default().lossy(false)` place un thread entre l'application et le disque, comme l'`AsyncAppender` de Log4j, pour qu'un disque lent ne ralentisse pas l'interface. Le mode non perdant fait attendre une rafale d'événements quand la file est pleine, au lieu de perdre des lignes (dont celle d'une panique). Il renvoie un `WorkerGuard`, qui vide le tampon quand il est détruit (RAII, comme un destructeur C++).
- Piège à éviter : écrire `let _guard = …`, et non `let _ = …`, qui détruirait le garde immédiatement.
- Niveau : `EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).parse(format!("info,{spec}"))`. La directive par défaut ne s'applique qu'à un filtre vide : `ortelia_service=trace` seul ferait taire toutes les autres crates. Le préfixe `info,` la garde pour les autres crates, et un niveau global donné ensuite (`debug`) la remplace. Si l'analyse échoue (par exemple `ortelia=verbeux`), on prend le filtre `info` et on journalise un avertissement, une fois le subscriber en place. Attention : un mot seul comme `abc` est un filtre valide (un nom de module), il ne produit donc pas d'erreur.
- Format : `tracing_subscriber::fmt` avec `with_ansi(false)`, sans codes couleur dans le fichier. Les horodatages par défaut sont en UTC (RFC 3339).
- Bibliothèques tierces : eframe, wgpu et d'autres journalisent par la crate `log`, et non par `tracing`. `tracing_log::LogTracer::init_with_filter(Warn)` relaie leurs avertissements et leurs erreurs dans le même journal. Leurs messages `info` (liste des adaptateurs…) restent dehors. Certaines écrivent des lignes `ERROR` bénignes en sondant le système (EGL, portail XDG dans WSLg) : le test de fumée ne compte donc que les lignes `ERROR` dont la cible commence par `ortelia`.
- Une fonction construit le subscriber à partir d'un writer et d'un filtre. L'initialisation lui passe le writer non bloquant ; les tests lui passent directement le `RollingFileAppender`, qui est bloquant, et installent le subscriber pour le seul thread courant avec `tracing::subscriber::with_default`, sans état global.
- Échec de `build` : un avertissement sur la sortie d'erreur cite le dossier, puis l'application continue sans journal fichier.

**Hook de panique**
- `std::panic::set_hook` enveloppe le hook par défaut, récupéré avec `take_hook`. Le nouveau hook journalise d'abord le message et l'emplacement au niveau `error`, puis appelle l'ancien, qui affiche toujours la panique sur la sortie d'erreur.
- Le garde étant détruit pendant le déroulement de la pile, le tampon est vidé avant la fin du processus.

**Tests**
- Tests unitaires dans `logging.rs` et `cli.rs`.
- Tests d'intégration dans `crates/ortelia/tests/`, qui lancent le binaire réel grâce à `env!("CARGO_BIN_EXE_ortelia")` : Cargo fournit ce chemin aux tests d'intégration d'une crate qui a un binaire.
- Les tests qui lancent l'application sans argument ne s'exécutent que sous Linux (`#[cfg(target_os = "linux")]`). Ils retirent `DISPLAY`, `WAYLAND_DISPLAY` et `WAYLAND_SOCKET` de l'environnement du processus fils, et donnent `ORTELIA_LOG_DIR` ou `XDG_STATE_HOME` vers un dossier temporaire. Ils vérifient alors le code 1, le message d'erreur, la ligne de démarrage et l'avertissement de dossier impossible à créer.
- Sous Windows, l'application ouvrirait vraiment une fenêtre et le test ne se terminerait pas.
- **Test de fumée** (`crates/ortelia/tests/smoke.rs`, scénario « Démarrage réel ») :
  - il est marqué `#[ignore]` parce qu'il faut un affichage : `cargo test` ne le lance pas, il faut le demander avec `cargo test -p ortelia --test smoke -- --ignored` ;
  - il lance le binaire avec `ORTELIA_LOG_DIR` vers un dossier temporaire, puis relit le journal toutes les 200 ms, pendant 20 s au plus, jusqu'à trouver `main window shown` ;
  - il vérifie alors que le processus tourne encore (`Child::try_wait()` renvoie `None`) et que le journal ne contient aucune ligne `ERROR` émise par Ortelia (cible `ortelia*`), puis il tue le processus ;
  - les lignes sont déjà sur le disque quand le test les lit : le thread d'écriture du journal les écrit au fil de l'eau, sans attendre la destruction du garde ;
  - en local, il tourne dans WSLg ; dans la CI, sous Xvfb (Linux) et sur le bureau du runner (Windows), voir Decision 9.
- En édition 2024, `std::env::set_var` est `unsafe`, parce qu'il n'est pas sûr si d'autres threads lisent l'environnement. Les tests ne modifient donc jamais l'environnement de leur propre processus : ils passent les variables au processus fils (`Command::env`), ou appellent des fonctions pures avec des valeurs explicites.

**Confidentialité**
- Les journaux ne contiennent jamais de valeurs de données de l'utilisateur (contenu des fichiers chargés, valeurs saisies). On peut y écrire des types, des identifiants techniques, des chemins de fichiers et des compteurs.
- Cette convention entre dans `CLAUDE.md`. Elle deviendra une exigence testable quand l'application chargera des données (incrément 1).

### 8. Vérification des couches : outil `xtask`

- **Convention xtask** : un binaire du workspace lancé par l'alias `cargo xtask` (`.cargo/config.toml` : `xtask = "run --package xtask --"`). C'est l'équivalent d'une tâche Gradle définie dans `buildSrc`, écrite dans le langage du projet. Elle tourne à l'identique sous Linux et sous Windows, sans dépendre de bash.
- **Lecture du graphe** : `cargo_metadata::MetadataCommand::new().features(CargoOpt::AllFeatures).exec()` lance `cargo metadata` et fournit le graphe résolu. Toutes les features sont activées, pour qu'une bibliothèque d'interface tirée par une dépendance optionnelle soit vue aussi. Le graphe résolu (`resolve.nodes`) donne, pour chaque dépendance, ses `dep_kinds` (normale, développement, compilation). Ce graphe couvre toutes les plateformes cibles : une dépendance propre à Windows est donc vérifiée aussi sous Linux.
- **Découpage pour les tests** :
  - `main.rs` lit les métadonnées, les convertit en un graphe simple propre à l'outil (noms de crates et arcs typés), puis affiche les violations et choisit le code de sortie ;
  - `layers.rs` contient la table des couches et la fonction pure `check(&graph) -> Vec<Violation>`.

  Les tests de `layers.rs` construisent des graphes à la main pour chaque scénario, sans lancer `cargo`. Un test supplémentaire vérifie le workspace réel.
- **Table des couches** : une constante dans `layers.rs`. Les noms exacts reçoivent leur niveau, de 0 à 3 et 5 ; le préfixe `ortelia-ui-` donne le niveau 4. `xtask` est déclaré comme outil.
  - Bibliothèques d'interface interdites, par préfixe de nom : `egui`, `eframe`, `epaint`, `winit`, `gpui`.
  - Toute crate membre absente de la table est une violation.
- **Règles** :
  - Pour les arcs internes : niveau de la dépendance < niveau de la crate. Une crate d'une couche ne peut pas dépendre d'un outil.
  - Pour les couches 0 à 3 : parcours en largeur du graphe depuis la crate, en mémorisant le parent de chaque nœud. On peut ainsi afficher le chemin jusqu'à la bibliothèque interdite (`ortelia-service -> foo -> egui`).
  - Tous les types de dépendance comptent. Les dépendances de développement des crates tierces n'apparaissent pas dans le graphe résolu, ce qui est le comportement voulu.
- **Sortie** : une ligne par violation, en anglais comme les autres messages destinés aux développeurs, puis un résumé. Code 0 sans violation, 1 sinon.
- **Alternatives écartées** :
  - cargo-deny (`[bans]`) sait interdire egui, mais il faudrait lister toutes les crates autorisées à en dépendre (eframe, egui-winit, egui_extras…), et il ne vérifie pas l'ordre de nos couches.
  - Un script qui filtre `cargo tree` dépendrait de bash et d'un format de sortie non garanti.

### 9. CI GitHub Actions (`.github/workflows/ci.yml`)

- **Déclencheurs** : `push` sur `main`, `pull_request` vers `main`, `workflow_dispatch`. `permissions: contents: read`. `concurrency` annule une exécution devenue obsolète sur une pull request ; les exécutions sur `main` ne sont jamais annulées, pour que chaque commit de `main` ait ses binaires.
- **Job `checks`**, matrice `ubuntu-latest` et `windows-latest`, avec `fail-fast: false` pour voir les deux résultats :
  1. `actions/checkout@v7` ;
  2. `rustup toolchain install`, qui lit `rust-toolchain.toml` ;
  3. `Swatinem/rust-cache@v2`, avec `save-if: ${{ github.ref == 'refs/heads/main' }}` : seul `main` enregistre le cache, pour que les pull requests ne remplissent pas la limite de 10 Go par dépôt (pratique reprise du workflow d'egui) ;
  4. `cargo fmt --all --check` (Linux seulement : le résultat ne dépend pas du système) ;
  5. `cargo clippy --workspace --all-targets --locked -- -D warnings` ;
  6. `cargo test --workspace --locked` ;
  7. `cargo xtask check-layers` ;
  8. test de fumée :
     - Linux : installation de `xvfb`, `mesa-vulkan-drivers` (lavapipe, Vulkan logiciel), `libgl1-mesa-dri` et `libegl1` (OpenGL logiciel), puis `xvfb-run -a cargo test -p ortelia --locked --test smoke -- --ignored`. L'étape est bloquante ;
     - Windows : même commande, sans Xvfb, avec `continue-on-error: true`. Les runners Windows de GitHub ont une session de bureau, mais pas de GPU ; wgpu devrait se rabattre sur WARP, le rendu logiciel de Direct3D 12, ce qui reste à confirmer. Avec `continue-on-error`, un échec reste visible dans le détail de l'exécution sans la faire échouer.
- **Job `binaries`** :
  - il dépend de `checks` et ne tourne pas sur les pull requests (`if: github.event_name != 'pull_request'`) ;
  - même matrice, puis `cargo build --release --locked -p ortelia` et `target/release/ortelia[.exe] --version` ;
  - `actions/upload-artifact@v7` avec `ortelia-linux-x86_64` ou `ortelia-windows-x86_64`, `if-no-files-found: error`, conservation 30 jours.
- **Verrouillage** : `--locked` fait échouer Cargo si `Cargo.lock` manque ou ne correspond plus aux manifestes.
- **Paquets système Linux** : a priori aucun pour compiler. Avec eframe 0.37, les bibliothèques X11, Wayland, xkbcommon et fontconfig sont chargées dynamiquement à l'exécution, et seul le test de fumée ouvre une fenêtre (ses paquets sont installés à part, étape 8). Si la première exécution prouve le contraire, on ajoutera une étape `apt-get install`.

### 10. Versions des bibliothèques

API vérifiées dans Context7 et dans les sources des crates téléchargées :

| Bibliothèque | Version | Crate(s) | API utilisée |
|---|---|---|---|
| eframe | 0.37.0 | `ortelia-ui-egui` | `run_native`, `NativeOptions { viewport }`, `App::ui(&mut self, &mut Ui, &mut Frame)` |
| egui | 0.37.0 | `ortelia-ui-egui` | `CentralPanel::default().show(ui, ..)`, `ViewportBuilder::with_title` |
| egui_kittest | 0.37.0 (dev), feature `eframe` | `ortelia-ui-egui` | `Harness::new_eframe`, `run`, `get_by_label` |
| tracing | 0.1.44 | `ortelia`, `ortelia-ui-egui` | `info!`, `warn!`, `error!` |
| tracing-subscriber | 0.3.23, feature `env-filter` | `ortelia` ; `ortelia-ui-egui` (dev) | `fmt` layer, `EnvFilter::builder()`, `registry()`, `MakeWriter` |
| tracing-appender | 0.2.5 | `ortelia` | `RollingFileAppender::builder()`, `NonBlockingBuilder::lossy(false)`, `WorkerGuard` |
| tracing-log | 0.2.0 | `ortelia` | `LogTracer::init_with_filter` (relais de la crate `log`) |
| dirs | 6.0.0 | `ortelia` | `state_dir`, `data_local_dir` |
| tempfile | 3.27 (dev) | `ortelia` | dossiers temporaires des tests |
| cargo_metadata | 0.23.1 | `xtask` | `MetadataCommand`, `Metadata.resolve`, `NodeDep.dep_kinds` |
| actions/checkout | v7 | CI | |
| Swatinem/rust-cache | v2 (2.9.3) | CI | |
| actions/upload-artifact | v7 | CI | |

### 11. Documentation

- `CLAUDE.md` : état du projet, arborescence, commande `cargo xtask check-layers`, convention de journalisation (emplacement, `ORTELIA_LOG`, `ORTELIA_LOG_DIR`, confidentialité), convention de tests de l'interface (Decision 12) et commande du test de fumée. On y retire aussi la mention « pas encore de code ».
- `README.md` : ligne de statut.
- `docs/feuille-de-route.md` : l'essai des captures d'images (macOS ou lavapipe) est ajouté à l'incrément 1.

### 12. Stratégie de tests de l'interface

egui redessine toute l'interface à chaque image et décrit chaque widget dans un arbre d'accessibilité (AccessKit). On peut donc tester l'interface dans le processus de test, sans fenêtre ni GPU. Cinq niveaux :

| Niveau | Ce qu'il vérifie | Dans ce changement |
|---|---|---|
| 1. Tests de `ortelia-presentation` | toute la logique d'affichage | oui (textes de la fenêtre) |
| 2. egui_kittest sans rendu | l'interface affiche le view-model ; plus tard, les clics et les touches produisent les bonnes intentions | oui (application complète, première image) |
| 3. Captures d'images (egui_kittest, features `wgpu` et `snapshot`) | le rendu réel, comparé à une image de référence versionnée | non : incrément 1 |
| 4. Test de fumée de l'exécutable | la vraie fenêtre s'ouvre et dessine sans erreur | oui (Linux bloquant, Windows non bloquant) |
| 5. Test manuel | l'impression générale, la version Windows | bac à sable Windows (tâche 9.4) |

- **Convention**, écrite dans `CLAUDE.md` :
  - toute logique se teste dans `ortelia-presentation` ;
  - chaque widget de `ortelia-ui-egui` a ses tests egui_kittest : affichage du view-model et, dès qu'il y a des intentions, traduction des interactions en intentions ;
  - les tests de l'interface ne recalculent rien : ils vérifient ce qui est affiché et ce qui est émis.
- **Captures d'images, reportées** : elles sont surtout utiles pour les widgets visuels (table, graphique, carte), et il n'y en a pas encore.
  - Il leur faut un GPU, réel ou logiciel. egui exécute les siennes sur une machine macOS de la CI, qui a un vrai GPU, avec les images dans git LFS. L'autre solution est le rendu logiciel lavapipe sous Linux.
  - Les images de référence changent souvent à chaque version d'egui (polices, anticrénelage) et doivent alors être régénérées (`UPDATE_SNAPSHOTS=1`).
  - Un court essai à l'incrément 1 choisira entre ces deux solutions.
- **Écarté** : piloter l'exécutable par l'accessibilité du système (UI Automation sous Windows, AT-SPI sous Linux). C'est possible grâce à AccessKit, mais lourd et fragile, alors qu'egui_kittest couvre le même besoin dans le processus de test.

## Risks / Trade-offs

- [La première compilation Windows en CI est longue (wgpu, eframe), de l'ordre de 10 minutes] → `Swatinem/rust-cache` ; les exécutions suivantes réutilisent le cache.
- [Des paquets système manquent sur `ubuntu-latest`] → étape `apt-get install` ajoutée après la première exécution si nécessaire (Decision 9).
- [Sous Windows, une console s'ouvre à côté de la fenêtre] → acceptable pendant le développement : c'est même utile pour voir les messages. Traitement reporté (Open Questions).
- [Le test du hook de panique modifie un état global du processus, alors que les tests tournent en parallèle] → le test restaure l'ancien hook à la fin. Le hook n'écrit que vers le subscriber du thread qui panique : dans les autres tests, aucun subscriber global n'est installé, et leurs paniques restent sans effet sur le journal.
- [Sous Xvfb, wgpu ne trouve pas d'adaptateur graphique] → lavapipe (`mesa-vulkan-drivers`) est installé pour Vulkan, et Mesa pour OpenGL. Si cela ne suffit pas, on force le rendu OpenGL avec `WGPU_BACKEND=gl` dans l'étape du test de fumée.
- [Le test de fumée échoue sous Windows faute de rendu logiciel sur le runner] → l'étape est non bloquante. Si elle réussit sur plusieurs exécutions, un changement ultérieur la rendra bloquante ; sinon, ce niveau reste couvert par le test manuel dans le bac à sable Windows.
- [Le rendu logiciel est lent, et la première image tarde] → délai de 20 s, très au-delà du temps attendu ; le test lit le journal sans attendre un délai fixe.
- [Les tests sans affichage ne couvrent l'échec d'ouverture de la fenêtre que sous Linux] → sous Windows, on vérifie la ligne de commande, la présentation et l'interface sans écran, et le test de fumée tente le lancement réel (non bloquant). Le lancement de la fenêtre Windows se vérifie aussi à la main, dans le bac à sable Windows, avec l'artefact de la CI.
- [Une future dépendance demande une version de Rust plus récente que 1.99] → on modifie `rust-toolchain.toml` et `rust-version` ; la CI confirme.
- [Des dépendances internes déclarées mais encore inutilisées] → c'est voulu (Decision 4). Elles serviront dès l'incrément 1.

## Migration Plan

Pas de migration : c'est le premier code du dépôt. Le changement est livré par commits successifs sur `main`, chacun compilable et testé dans WSL2. Le workflow CI entre en dernier, une fois le workspace vert en local. Retour arrière : annuler les commits.

## Open Questions

- Sous Windows, masquer la console avec `#![windows_subsystem = "windows"]` tout en gardant la sortie de `--version` (rattachement à la console parente). À traiter avec la distribution des binaires, sans effet sur les spécifications actuelles.
- Réserver le nom `ortelia` sur crates.io : décision indépendante de ce changement.
