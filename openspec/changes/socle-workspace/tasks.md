# Tâches : socle-workspace

Toutes les commandes `cargo` se lancent dans WSL2, avec `CARGO_TARGET_DIR=~/target/ortelia` (voir `CLAUDE.md`). Chaque tâche laisse le workspace compilable et les tests au vert.

## 1. Workspace Cargo et couches basses

- [x] 1.1 Créer `rust-toolchain.toml` (1.99, rustfmt, clippy, profil minimal) et le `Cargo.toml` racine : workspace virtuel `resolver = "3"`, `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]` avec `unsafe_code = "deny"`. Vérifier : `cargo fmt --version` et `cargo clippy --version` répondent dans WSL2.
- [x] 1.2 Créer `ortelia-model`, `ortelia-config`, `ortelia-expr` et `ortelia-service` : un `lib.rs` avec un commentaire `//!` sur le rôle de la crate, et les dépendances internes de design.md (Decision 4). Vérifier : `cargo build --workspace` réussit et produit un `Cargo.lock`.
- [x] 1.3 Mettre à jour `CLAUDE.md` (état du projet, arborescence `crates/` et `xtask/`) et la ligne de statut de `README.md`. Vérifier : la mention « pas encore de code ni de workspace Cargo » a disparu, et la nouvelle arborescence est décrite.

## 2. Présentation minimale

- [x] 2.1 Créer `ortelia-presentation` (dépend de `ortelia-service`), avec `AppState::new()`, `AppState::main_window()` et `MainWindowViewModel { title, heading }`. Vérifier : un test unitaire passe, qui contrôle `title == "Ortelia"` et `heading == format!("Ortelia {}", env!("CARGO_PKG_VERSION"))`.

## 3. Vérification des couches (`xtask`)

- [x] 3.1 Créer la crate `xtask`, l'alias `cargo xtask` dans `.cargo/config.toml`, et `layers.rs` : graphe simple, table des couches, `check()` pour les règles internes (couche inférieure acceptée, couche supérieure ou de même niveau refusée, dépendance de développement comprise, dépendance vers un outil refusée, crate non classée). Vérifier : un test unitaire par scénario de `layered-workspace` passe.
- [x] 3.2 Ajouter la règle « pas de bibliothèque d'interface sous les interfaces » : parcours en largeur avec mémorisation des parents, chemin affiché, préfixes `egui`, `eframe`, `epaint`, `winit`, `gpui`. Vérifier : passent les tests de la dépendance directe, de la dépendance transitive (avec le chemin attendu), de `ortelia-ui-egui` autorisée et de deux violations signalées ensemble.
- [x] 3.3 Écrire `main.rs` : `cargo_metadata` → graphe simple, affichage des violations, code de sortie (0 ou 1). Vérifier : `cargo xtask check-layers` affiche zéro violation et se termine avec 0. Vérifier aussi à la main qu'une dépendance temporaire de `ortelia-model` vers `ortelia-service` fait échouer la commande avec 1, puis retirer cette dépendance.
- [x] 3.4 Documenter `cargo xtask check-layers` et la table des couches dans `CLAUDE.md`. Vérifier : la commande documentée fonctionne telle qu'elle est écrite.

## 4. Interface egui

- [x] 4.1 Créer `ortelia-ui-egui` avec `native_options`, `draw_main_window`, `OrteliaApp` (journalisation unique de la première image) et `run`. Dépendances : `ortelia-presentation`, eframe 0.37, egui 0.37 et tracing ; en développement, egui_kittest 0.37 avec sa feature `eframe`, et tracing-subscriber. Vérifier :
  - un test contrôle `native_options(..).viewport.title == Some("Ortelia")` ;
  - un test `Harness::new_eframe` trouve le libellé `Ortelia <version>` ;
  - un test `Harness::new_eframe` avec un subscriber en mémoire compte une seule fois `main window shown` après plusieurs images ;
  - `cargo xtask check-layers` affiche toujours zéro violation.
- [x] 4.2 Écrire la convention de tests de l'interface dans `CLAUDE.md` (niveaux, logique testée dans la présentation, tests egui_kittest pour chaque widget) et ajouter à l'incrément 1 de `docs/feuille-de-route.md` l'essai des captures d'images (macOS ou lavapipe). Vérifier : les deux documents décrivent les niveaux de la Decision 12 de design.md, sans contradiction.

## 5. Exécutable : ligne de commande

- [x] 5.1 Créer la crate `ortelia` (binaire) avec `cli.rs` (`Command::{Run, Version, Help}` et l'erreur d'usage) et `main() -> ExitCode` pour `--version`, `--help` et un argument inconnu. Pour l'instant, `Run` se termine sans rien faire. Vérifier : les tests unitaires de l'analyse passent, ainsi que les tests d'intégration de `tests/cli.rs` (`--version` → `ortelia <version>`, code 0 ; `--help` → mentionne les deux options, code 0 ; `--frobnicate` → sortie d'erreur avec l'argument et `--help`, code 2).

## 6. Journalisation

- [x] 6.1 Écrire `logging.rs` : `log_dir()`, qui prend `ORTELIA_LOG_DIR` s'il est non vide, sinon `dirs::state_dir()`, sinon `dirs::data_local_dir()`, suivi de `ortelia/logs`. Vérifier : passent les tests unitaires du remplacement, de la variable vide ignorée et du dossier par défaut calculé à partir d'un dossier d'état donné.
- [x] 6.2 Ajouter la construction du subscriber : `RollingFileAppender` (quotidien, préfixe `ortelia`, suffixe `log`, 7 fichiers au plus), `EnvFilter` avec `info` par défaut et repli sur `info` pour une valeur invalide, `fmt` sans couleurs. Vérifier, par des tests avec `with_default` dans un dossier temporaire :
  - le fichier s'appelle `ortelia.<date UTC>.log` ;
  - `debug` est filtré par défaut et écrit avec le filtre `debug` ;
  - une valeur invalide (`ortelia=verbeux`) donne `info` et un avertissement ;
  - sur 8 anciens fichiers, il en reste 7 au plus après création.
- [x] 6.3 Brancher la journalisation dans `main` : initialisation après l'analyse des arguments, garde conservé jusqu'à la fin, ligne de démarrage (version, système, architecture), ligne d'arrêt, avertissement sur la sortie d'erreur si le journal est indisponible. Vérifier : passent les tests unitaires du chemin d'échec, et un test d'intégration montre que `--version` ne crée aucun fichier dans `ORTELIA_LOG_DIR`.
- [x] 6.4 Installer le hook de panique, qui journalise au niveau `error` puis appelle l'ancien hook. Vérifier : un test passe, qui installe le hook, provoque une panique dans `catch_unwind`, contrôle la ligne `ERROR` (message et fichier source), puis restaure l'ancien hook.
- [x] 6.5 Documenter dans `CLAUDE.md` l'emplacement des journaux, `ORTELIA_LOG`, `ORTELIA_LOG_DIR` et la règle de confidentialité (aucune valeur de donnée de l'utilisateur). Vérifier : les chemins documentés correspondent à ceux que testent les tâches 6.1 et 6.2.

## 7. Exécutable : fenêtre principale

- [x] 7.1 Brancher `Run` : créer `AppState`, appeler `ortelia_ui_egui::run`, et en cas d'erreur écrire le message sur la sortie d'erreur et dans le journal, puis renvoyer le code 1. Vérifier par des tests d'intégration réservés à Linux, sans `DISPLAY`, `WAYLAND_DISPLAY` ni `WAYLAND_SOCKET` :
  - code 1 et message d'échec sur la sortie d'erreur ;
  - avec `ORTELIA_LOG_DIR` vers un dossier inexistant : le dossier est créé, et le journal contient la ligne de démarrage `INFO` avec la version ainsi que l'erreur ;
  - avec `XDG_STATE_HOME` vers un dossier temporaire : le journal est dans `<dossier>/ortelia/logs` ;
  - avec `ORTELIA_LOG_DIR` sous un fichier ordinaire : avertissement sur la sortie d'erreur qui cite le chemin, et poursuite du démarrage.
- [ ] 7.2 Lancer l'application dans WSLg : `timeout 10 ~/target/ortelia/debug/ortelia`. Vérifier : code 124 (elle tournait encore), et, en la lançant sans `timeout`, une fenêtre titrée « Ortelia » qui affiche `Ortelia 0.1.0`. Fermer la fenêtre fait sortir le programme avec le code 0 et écrit la ligne d'arrêt dans le journal.
- [x] 7.3 Écrire le test de fumée `crates/ortelia/tests/smoke.rs`, marqué `#[ignore]`. Il lance le binaire avec `ORTELIA_LOG_DIR` temporaire, attend `main window shown` dans le journal (20 s au plus), vérifie que le processus tourne encore et que le journal est sans `ERROR`, puis tue le processus. Vérifier :
  - `cargo test -p ortelia --test smoke -- --ignored` passe dans WSLg ;
  - `cargo test --workspace` ne lance pas ce test ;
  - avec l'accord du porteur du projet (installation de `xvfb` par `wsl -u root`), le test passe aussi sous `xvfb-run -a` dans WSL2, comme dans la CI ;
  - la commande du test est documentée dans `CLAUDE.md`.

## 8. Intégration continue

- [x] 8.1 Écrire `.github/workflows/ci.yml`, job `checks`, matrice Linux et Windows, tout en `--locked` :
  - toolchain, puis cache enregistré seulement sur `main` (`save-if`) ;
  - format sous Linux, clippy `-D warnings`, tests, `cargo xtask check-layers` ;
  - test de fumée : sous Linux, paquets Mesa et Xvfb puis `xvfb-run -a`, étape bloquante ; sous Windows, `continue-on-error: true`.

  Vérifier : en local, la même séquence de commandes réussit dans WSL2.
- [x] 8.2 Ajouter le job `binaries` : release, `--version`, artefacts `ortelia-linux-x86_64` et `ortelia-windows-x86_64` ; pas de binaires sur les pull requests. Vérifier : `cargo build --release --locked -p ortelia` puis `ortelia --version` réussissent dans WSL2.
- [ ] 8.3 Avec l'accord du porteur du projet, pousser sur une branche et ouvrir une pull request. Vérifier : le job `checks` est vert sous Linux et sous Windows, test de fumée Linux compris, et le job `binaries` ne s'exécute pas. Si des paquets système manquent sous Linux, ajouter l'étape `apt-get` et relancer. Si wgpu ne trouve pas d'adaptateur sous Xvfb, essayer `WGPU_BACKEND=gl`. Noter le résultat du test de fumée Windows dans la description de la pull request : s'il est fiable sur plusieurs exécutions, on le rendra bloquant dans un changement ultérieur.
- [x] 8.4 Documenter la CI dans `CLAUDE.md` (ce qu'elle vérifie, où trouver les binaires) et y retirer « CI prévue à l'incrément 0 ». Vérifier : la section Git de `CLAUDE.md` décrit le workflow réel.

## 9. Vérification finale

- [x] 9.1 Lancer `/code-review` sur le diff complet et corriger les problèmes retenus. Vérifier : aucun problème important restant.
- [x] 9.2 Lancer `/simplify` et appliquer les simplifications retenues. Vérifier : le diff simplifié compile.
- [x] 9.3 Dans WSL2 : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo xtask check-layers`, puis le test de fumée dans WSLg (`cargo test -p ortelia --test smoke -- --ignored`). Vérifier : les cinq commandes réussissent.
- [ ] 9.4 Après fusion sur `main` : vérifier que la CI est verte sur les deux OS et que les deux artefacts sont téléchargeables. Lancer l'exécutable Windows dans le bac à sable Windows (après avoir vérifié que Smart App Control n'y est pas actif) et constater que la fenêtre s'ouvre.

## Workflow follow-up

- Archiver le changement avec `/openspec-archive-change` une fois la CI verte sur `main`. Les spécifications `layered-workspace`, `app-launch`, `logging` et `continuous-integration` entrent alors dans `openspec/specs/`.
- Le premier critère « Fini quand » de l'incrément 0 (CI verte sur les deux OS) est alors rempli. Les critères restants passent par les changements des essais S1, S2 et S3.
