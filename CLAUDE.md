# CLAUDE.md

Ce fichier guide Claude Code (claude.ai/code) dans ce dépôt.

@docs/cadrage-prive/CLAUDE-prive.md

## Projet

**Ortelia** : outil desktop (Windows, Linux) pour construire **sans code** des applications de visualisation et d'édition de données structurées. Un ingénieur métier décrit, dans un mode design, un modèle de données (métamodèle inspiré d'Ecore), les sources, la disposition des fenêtres et les vues ; le mode run exécute l'application. Le code spécifique (imports de fichiers particuliers, calculs) passe par des plugins natifs.

**Priorités** : rapidité et faible consommation mémoire. Fichiers jusqu'à environ 500 Mo, tables jusqu'à 100 000 lignes.

**État** : incrément 0 en cours. Le socle est posé (changement OpenSpec `socle-workspace`) : workspace Cargo en couches, crates presque vides, fenêtre principale minimale. Les essais techniques S1 à S3 restent à faire.

## Documents

| Fichier | Contenu |
|---|---|
| `docs/feuille-de-route.md` | Incrément 0, puis trois applications jalons : annuaire, éditeur de modèles, mode design |
| `docs/architecture-couches.md` | Couches, couche de présentation sans dépendance UI, interfaces egui / GPUI interchangeables (D25–D27) |
| `examples/annuaire/` | Application 1 : `app.json` (format 0.2), `annuaire.model.json`, `personnes.json` |
| `examples/model-editor/metamodel.model.json` | Ébauche du métamodèle décrit avec lui-même (application 2) |

`docs/cadrage-prive/` contient des notes de conception privées : ne jamais le versionner, ne jamais citer son contenu dans un fichier versionné.

## Workspace Cargo

```
Cargo.toml              workspace virtuel : versions, dépendances et lints communs
Cargo.lock              versionné (application)
rust-toolchain.toml     version de Rust fixée (1.99), avec rustfmt et clippy
.cargo/config.toml      alias `cargo xtask`
crates/
  ortelia-model/        couche 0 : métamodèle, stockage dynamique
  ortelia-config/       couche 1 : format de app.json
  ortelia-expr/         couche 1 : expressions
  ortelia-service/      couche 2 : sources, tâches de fond, validation, plugins
  ortelia-presentation/ couche 3 : view-models, logique d'affichage
  ortelia-ui-egui/      couche 4 : dessin egui
  ortelia/              couche 5 : exécutable (ligne de commande, journalisation)
xtask/                  outil du dépôt : vérification des couches
```

- Une crate hérite des réglages communs (`version.workspace = true`, `[lints] workspace = true`…) et déclare ses dépendances par `nom.workspace = true` : les versions ne s'écrivent que dans le `Cargo.toml` racine.
- `unsafe_code = "deny"` pour tout le workspace : une exception éventuelle s'écrit localement avec `#![allow(unsafe_code)]`.
- Pour changer de version de Rust : modifier `rust-toolchain.toml` et `rust-version` dans un même commit.

## Règles

### Langues

- Code, identifiants et commentaires du code : **anglais**.
- Spécifications et documents de conception : **français**.
- Interface d'Ortelia : français et anglais, au choix de l'utilisateur.

### Architecture

- Règle des couches (`docs/architecture-couches.md`) : `ortelia-model` ← `ortelia-config` / `ortelia-expr` ← `ortelia-service` ← `ortelia-presentation` ← `ortelia-ui-*` ← `ortelia`. Aucune crate sous `ortelia-ui-*` ne dépend d'egui ou de GPUI.
- Vérification : `cargo xtask check-layers` (depuis la racine, dans WSL2 avec `CARGO_TARGET_DIR=~/target/ortelia`, et dans la CI). Code 0 sans violation, 1 sinon. Elle signale :
  - une dépendance (normale, de développement ou de compilation) vers une crate du workspace de même niveau ou de niveau supérieur ;
  - une crate d'une couche qui dépend d'un outil (`xtask`) ;
  - une crate du workspace absente de la table des couches ;
  - une crate des couches 0 à 3 qui atteint, même indirectement, une bibliothèque d'interface (préfixes `egui`, `eframe`, `epaint`, `winit`, `gpui`), avec le chemin de dépendance.
- Table des couches : constantes `LAYERS`, `UI_CRATE_PREFIX` et `TOOLS` de `xtask/src/layers.rs`. Niveaux : 0 `ortelia-model` ; 1 `ortelia-config`, `ortelia-expr` ; 2 `ortelia-service` ; 3 `ortelia-presentation` ; 4 `ortelia-ui-*` ; 5 `ortelia`. Une nouvelle crate (`ortelia-geo`, `ortelia-plugin-sdk`…) doit y être ajoutée en même temps qu'au workspace.
- Toute la logique d'affichage (tri, filtre, formulaires générés, broker, intentions, formats, état de session, mode design) va dans `ortelia-presentation`, testée sans interface.
- Les formats de configuration doivent être descriptibles par le métamodèle : contenance, références par nom (`refersTo`), polymorphisme par `_type_`, pas de dictionnaires.

### Tests de l'interface

egui décrit chaque widget dans un arbre d'accessibilité (AccessKit) : l'interface se teste dans le processus de test, sans fenêtre ni GPU. Cinq niveaux :

| Niveau | Ce qu'il vérifie | Où |
|---|---|---|
| 1. Tests de `ortelia-presentation` | toute la logique d'affichage | tests unitaires, sans interface |
| 2. egui_kittest sans rendu | l'interface affiche le view-model ; les clics et les touches produisent les bonnes intentions | tests de `ortelia-ui-egui` (`Harness::new_eframe`, `get_by_label`) |
| 3. Captures d'images | le rendu réel, comparé à une image de référence | pas encore : essai à l'incrément 1 (macOS en CI ou lavapipe) |
| 4. Test de fumée de l'exécutable | la vraie fenêtre s'ouvre et dessine sans erreur | `crates/ortelia/tests/smoke.rs` |
| 5. Test manuel | l'impression générale, la version Windows | WSLg ; bac à sable Windows |

- Toute logique se teste dans `ortelia-presentation`.
- Chaque widget de `ortelia-ui-egui` a ses tests egui_kittest : affichage du view-model et, dès qu'il y a des intentions, traduction des interactions en intentions.
- Les tests de l'interface ne recalculent rien : ils vérifient ce qui est affiché et ce qui est émis.
- Test de fumée : marqué `#[ignore]` (il faut un affichage), donc absent de `cargo test`. Le lancer avec `cargo test -p ortelia --test smoke -- --ignored` : dans WSLg en local, sous `xvfb-run -a` dans la CI Linux. Il attend `main window shown` dans le journal (20 s au plus) puis vérifie qu'Ortelia n'y a écrit aucune ligne `ERROR` (les lignes relayées des bibliothèques tierces ne comptent pas).
- Les tests qui lancent l'exécutable sans argument (`crates/ortelia/tests/startup.rs`) ne tournent que sous Linux, sans affichage : sous Windows, la fenêtre s'ouvrirait vraiment.
- `tracing` met en cache l'intérêt de chaque point de journalisation : les tests qui vérifient un événement émis par du code que d'autres tests exécutent en parallèle sans subscriber doivent être sérialisés (voir `APP_TESTS` dans `ortelia-ui-egui`).

### Journalisation

- Seul l'exécutable `ortelia` initialise la journalisation (`crates/ortelia/src/logging.rs`) ; les bibliothèques n'utilisent que les macros de `tracing` (`info!`, `warn!`…).
- Un fichier par jour, `ortelia.<AAAA-MM-JJ>.log` (date UTC), 7 fichiers conservés au plus, horodatages UTC. Dossier :
  - Linux : `$XDG_STATE_HOME/ortelia/logs` (par défaut `~/.local/state/ortelia/logs`) ;
  - Windows : `%LOCALAPPDATA%\ortelia\logs` ;
  - `ORTELIA_LOG_DIR`, si elle est définie et non vide, remplace ce dossier.
- `ORTELIA_LOG` règle le niveau : `info` par défaut, `debug`, ou par crate (`ortelia_service=trace`, les autres crates restant en `info`). Une valeur invalide est ignorée, avec un avertissement dans le journal.
- Les avertissements et erreurs des bibliothèques qui passent par la crate `log` (eframe, wgpu…) sont relayés dans le même journal ; leurs messages `info` ne le sont pas.
- `--version` et `--help` ne journalisent rien. Si le dossier est inutilisable, un avertissement s'affiche sur la sortie d'erreur et l'application continue sans journal fichier. Les paniques sont journalisées au niveau `ERROR`.
- **Confidentialité** : jamais de valeur de donnée de l'utilisateur dans les journaux (contenu des fichiers chargés, valeurs saisies). Types, identifiants techniques, chemins de fichiers et compteurs sont permis.

### Style Rust

Le porteur du projet débute en Rust et connaît bien C, C++ et Java.
- Écrire un Rust idiomatique mais lisible : pas de généricité ni de macros superflues, et des commentaires sur ce qui n'est pas évident.
- Expliquer les choix par analogie avec C++ ou Java quand c'est utile.

### Documentation des bibliothèques

Toujours consulter **Context7** (serveur MCP déclaré dans `.mcp.json`) avant d'écrire du code qui utilise une bibliothèque. En particulier, egui 0.37 est plus récent que les connaissances du modèle et son API change à chaque version mineure.

## Méthode : OpenSpec

- Développement piloté par les spécifications avec **OpenSpec** 2.x (`npm install -g @fission-ai/openspec`), initialisé par `openspec init --tools claude --language "French (fr-FR)" --scope project --profile core`. Les skills sont dans `.claude/skills/` (copie dans `.agents/skills/`) ; `openspec update` les régénère.
- `openspec/config.yaml` déclare le français pour les artefacts, en gardant en anglais les titres structurels (`### Requirement:`, `#### Scenario:`) et les mots-clés `SHALL` / `MUST`. Il porte aussi le contexte du projet et des règles par artefact.
- Cycle d'un incrément : `/openspec-explore` (réfléchir), `/openspec-propose <nom>` (proposition, spécifications, conception, tâches, à faire valider), `/openspec-apply-change` (réaliser), `/openspec-archive-change` (fusionner dans `openspec/specs/`). Autres skills : `/openspec-update-change` (modifier les artefacts d'un changement), `/openspec-sync-specs` (fusionner sans archiver).
- Après une modification non triviale, et avant de la considérer terminée : `/code-review` sur le diff, puis `/simplify`, puis recompiler.

## Git

- Branche `main`, dépôt public `https://github.com/VincentBonnafous31/ortelia` (remote `origin`).
- CI GitHub Actions (`.github/workflows/ci.yml`), à chaque poussée sur `main`, à chaque pull request vers `main` et à la demande :
  - job `checks`, sous Linux et Windows, tout en `--locked` : `cargo fmt --all --check` (Linux), `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo xtask check-layers`, puis le test de fumée (Linux sous Xvfb, bloquant ; Windows non bloquant) ;
  - job `binaries`, hors pull requests : compilation release, `ortelia --version`, puis artefacts `ortelia-linux-x86_64` et `ortelia-windows-x86_64` (30 jours), téléchargeables depuis la page de l'exécution (`gh run download <id>`). C'est la seule source de la version Windows.
  - Seul `main` enregistre le cache Cargo (`Swatinem/rust-cache`).
- Avant de pousser, lancer la même séquence en local.
- `gh` (GitHub CLI) est installé et connecté.
- Ne committer ou pousser que sur demande.

## Conventions egui 0.37

L'API d'egui change à chaque version mineure : vérifier dans Context7 ou dans les sources des crates (`~/.cargo/registry/src/*/egui-0.37.*`) plutôt que de supposer.

- Mode immédiat : l'interface est redessinée à chaque image par `eframe::App::ui(&mut self, ui, frame)` (et non plus `update(ctx)`). L'état vit dans l'application, pas dans les widgets.
- Panneaux : `egui::Panel::left(..)` / `Panel::bottom(..)` puis `CentralPanel`, tous affichés avec `.show(ui, ..)`. `SidePanel`, `TopBottomPanel` et `show_inside` sont obsolètes.
- Table : `egui_extras::TableBuilder`. `body.rows(..)` ne dessine que les lignes visibles (testé avec 50 000 lignes). egui mémorise les largeurs sous l'`id_salt` de la table et repart de `Column::initial` quand le nombre de colonnes change.
- `DatePickerButton` travaille sur des `jiff::civil::Date` (pas chrono).
- Les polices emoji et symboles (⏷, 📆…) ne sont plus incluses par défaut : activer la feature `monochrome_emoji_fonts` de la crate `egui`, sinon elles s'affichent en carrés vides sous Linux/WSL.
- Décorations de fenêtre natives ; titre via `ViewportBuilder::with_title`.
