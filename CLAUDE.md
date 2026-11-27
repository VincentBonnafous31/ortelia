# CLAUDE.md

Ce fichier guide Claude Code (claude.ai/code) dans ce dépôt.

@docs/cadrage-prive/CLAUDE-prive.md

## Projet

**Ortelia** : outil desktop (Windows, Linux) pour construire **sans code** des applications de visualisation et d'édition de données structurées. Un ingénieur métier décrit, dans un mode design, un modèle de données (métamodèle inspiré d'Ecore), les sources, la disposition des fenêtres et les vues ; le mode run exécute l'application. Le code spécifique (imports de fichiers particuliers, calculs) passe par des plugins natifs.

**Priorités** : rapidité et faible consommation mémoire. Fichiers jusqu'à environ 500 Mo, tables jusqu'à 100 000 lignes.

**État** : incrément 0 terminé (changement OpenSpec `socle-workspace`) : workspace Cargo en couches, crates presque vides, fenêtre principale minimale, version Windows validée dans le bac à sable. Incrément 1 (application 1 « Annuaire des personnes ») terminé, en deux changements OpenSpec, tous deux archivés. Le premier, `stockage-dynamique` (essai S1 : métamodèle, Store, constructeur, lecteur JSON, mesures), est terminé et archivé (spécifications `metamodel`, `object-store`, `data-loading`, `storage-benchmark`) : le Store retenu est un bloc de valeurs typées par objet (`docs/essais/s1-stockage.md`). Le second, `modele-et-donnees` (spécifications `change-tracking`, `app-config`, `app-context`, `config-schemas`), apporte `app.json` (format 0.3, `ortelia-config`), Document avec révision et journal des changements, fragments greffés sur une branche, erreurs rassemblées et diagnostics, ouverture d'une application (`AppContext::open`, `ortelia-service`), JSON Schema des fichiers de configuration (`schemas/`). Incrément 2 (vue avec une table) terminé, en trois changements OpenSpec. Le premier, `captures-images`, est terminé et archivé (spécification `ui-snapshots`, delta de `continuous-integration`) ; il retient lavapipe sous Linux pour les captures d'images (`docs/essais/captures-images.md`). Le deuxième, `vue-table`, est terminé et archivé (spécifications `table-view`, `labels`, `value-formats`, deltas de `app-config`, `app-context`, `app-launch`, `config-schemas`, `continuous-integration`, `storage-benchmark` et `ui-snapshots`) : `app.json` au format 0.4 (langues, libellés, présentations, fenêtre), table de l'annuaire (tri, filtre, colonnes, formats), boîtes d'erreur, barre des avertissements, `ortelia [chemin]`. Le troisième, `etat-de-session`, est terminé et archivé (spécification `session-state`, deltas de `table-view` et `app-launch`) : il enregistre à la fermeture la taille de la fenêtre et la disposition de la table dans `app.session.json`, à côté de `app.json`, et la restaure à l'ouverture. Incrément 3 (split et panneau de propriétés en lecture seule) commencé, en trois changements OpenSpec. Le premier, `conteneur-split`, est terminé et archivé (spécification `window-layout`, deltas de `app-config`, `session-state`, `app-launch`, `config-schemas` et `ui-snapshots`) : essai S3 (`docs/essais/s3-conteneurs.md`, egui_tiles retenu), `app.json` au format 0.5 (container `Split`, widget `Empty`), table de l'annuaire à gauche et vue vide à droite, parts des splits dans l'état de session. Le deuxième, `formulaire`, est terminé et archivé (spécifications `grid-layout` et `form-view`, deltas de `app-config`, `app-launch`, `config-schemas`, `labels`, `ui-snapshots` et `window-layout`) : `app.json` au format 0.6 (formulaire `form` d'une présentation, éléments `Field`, `Group`, `Separator` et `Text`, huit widgets de champ, widget de vue `Form`), grille en flux dans la présentation, formulaire résolu par classe ou déduit du modèle, dessiné en lecture seule ; la vue de droite de l'annuaire montre le formulaire vide de `Personne`.

## Documents

| Fichier | Contenu |
|---|---|
| `docs/feuille-de-route.md` | Vue d'ensemble : socle (terminé), applications jalons, pistes après l'application 2 (mode design, globe 3D en bibliothèque séparée…) |
| `docs/feuille-de-route-annuaire.md` | Application 1 « Annuaire des personnes » : incréments 1 à 6 |
| `docs/feuille-de-route-editeur-modeles.md` | Application 2 « Éditeur de modèles » : incréments 7 et 8 |
| `docs/architecture-couches.md` | Couches, couche de présentation sans dépendance UI, interfaces egui / GPUI interchangeables (D25–D27) ; lecture des données : lecteur de format, correspondance, constructeur, fragment greffé sur sa branche (D28–D30) ; Document seul à modifier le Store (D31) ; racine et sources de `app.json` (D32) |
| `examples/annuaire/` | Application 1 : `app.json` (format 0.6 : modèle, langues `fr` et `en`, libellés, racine `Racine`, source `personnes` de cible `/annuaire/personnes`, présentation de `Personne` avec sa table et son formulaire, fenêtre partagée par le split `principal` entre la vue table `personnes` et la vue formulaire `proprietes`), `annuaire.model.json`, `personnes.json` (10 personnes) |
| `schemas/` | JSON Schema (draft 7) de `app.json` et des `*.model.json`, générés par `cargo xtask gen-schemas` ; les exemples y renvoient par leur clé `$schema` |
| `examples/model-editor/metamodel.model.json` | Ébauche du métamodèle décrit avec lui-même (application 2) ; ne compile pas encore (attribut multivalué `literals`) |
| `docs/essais/s1-stockage.md` | Essai S1 : conditions, mesures des deux Stores comparés, analyse, seuils et choix du Store |
| `docs/essais/s3-conteneurs.md` | Essai S3 : egui_tiles pour les conteneurs de la fenêtre, critères et tests qui les vérifient, écarts rencontrés, choix d'egui_tiles |
| `docs/essais/captures-images.md` | Essai des captures d'images : quatre configurations de la CI, pixels fautifs par seuil, choix de lavapipe sous Linux (`ubuntu-26.04`) |
| `examples/reseau-meteo/` | Essai S1 : modèle d'essai `reseau.model.json` (réseau de stations météo, profondeur 4, listes polymorphes, zones découpées en mailles) et petit fichier conforme `reseau-exemple.json` |

`docs/cadrage-prive/` contient des notes de conception privées : ne jamais le versionner, ne jamais citer son contenu dans un fichier versionné.

## Workspace Cargo

```
Cargo.toml              workspace virtuel : versions, dépendances et lints communs
Cargo.lock              versionné (application)
rust-toolchain.toml     version de Rust fixée (1.99), avec rustfmt et clippy
.cargo/config.toml      alias `cargo xtask`
crates/
  ortelia-model/        couche 0 : métamodèle, stockage dynamique (Store), Document, constructeur
  ortelia-config/       couche 1 : format de app.json, vérification contre le modèle
  ortelia-expr/         couche 1 : expressions
  ortelia-service/      couche 2 : ouverture d'une application, sources (lecteur JSON), plus tard tâches de fond et plugins
  ortelia-presentation/ couche 3 : view-models, logique d'affichage
  ortelia-ui-egui/      couche 4 : dessin egui
  ortelia/              couche 5 : exécutable (ligne de commande, journalisation)
xtask/                  outil du dépôt : vérification des couches, générateur de données d'essai, JSON Schema, tableau de couverture
schemas/                JSON Schema générés (versionnés)
```

- Une crate hérite des réglages communs (`version.workspace = true`, `[lints] workspace = true`…) et déclare ses dépendances par `nom.workspace = true` : les versions ne s'écrivent que dans le `Cargo.toml` racine.
- `unsafe_code = "deny"` pour tout le workspace : une exception éventuelle s'écrit localement avec `#![allow(unsafe_code)]`.
- Pour changer de version de Rust : modifier `rust-toolchain.toml` et `rust-version` dans un même commit.

## JSON Schema des fichiers de configuration

- `cargo xtask gen-schemas` écrit `schemas/app.schema.json` et `schemas/model.schema.json`, générés par schemars depuis les types serde qui lisent ces fichiers (`AppFile`, `ModelFile`). Après toute modification de ces types, le relancer et versionner le résultat.
- `cargo xtask gen-schemas --check` compare sans écrire : code 1 en citant le fichier périmé, 0 sinon. La CI (Linux) et `tools/check.sh` le lancent.
- Les dérivations `JsonSchema` sont derrière la feature `json-schema` de `ortelia-model` et `ortelia-config`, que seul `xtask` active : l'exécutable est compilé sans schemars (`cargo tree -p ortelia -i schemars` ne le trouve pas).
- Les commentaires de doc (`///`) des types de fichiers deviennent les descriptions des schémas, montrées par l'éditeur : les écrire pour l'utilisateur, et mettre les détails d'implémentation en commentaires `//`.

## Lancer Ortelia

```bash
ortelia [chemin]
```

- `chemin` : un dossier qui contient un `app.json`, ou le fichier `app.json` lui-même ; sans argument, le `app.json` du dossier courant. `tools/launch.sh` (skill `/lancer`) ouvre `examples/annuaire` par défaut, et `tools/launch.sh -- <chemin>` une autre application (chemin relatif à la racine du dépôt).
- Une application qui ne s'ouvre pas affiche ses erreurs dans une boîte (bouton « Quitter »), les écrit sur la sortie d'erreur et dans le journal, et le code de sortie est 1.
- État de session : à la fermeture normale de la fenêtre, la taille de la fenêtre (ni maximisée, ni en plein écran ; suivie à partir de la première action de l'utilisateur dans la fenêtre, pour ne pas retenir une taille provisoire), pour chaque table l'ordre et la largeur des colonnes, le tri et le filtre, et les parts de chaque split sont écrits dans le fichier d'état, à côté du fichier de l'application : `app.session.json` pour `app.json` (`.json` devient `.session.json`). Il est restauré à l'ouverture suivante ; `app.json` n'est jamais écrit. Les fichiers `*.session.json` sont ignorés par git ; supprimer `app.session.json` ramène la disposition de `app.json`. Format, lecture et écriture : module `session` de `ortelia-presentation`.

## Annuaire généré et mesure de la table

```bash
cargo xtask gen-data --model annuaire --count 100000 --out ~/annuaire-100k
cargo run --release -p ortelia-presentation --example table-bench -- ~/annuaire-100k
```

- `gen-data --model annuaire` écrit dans le dossier `--out` (créé au besoin) une application complète : `app.json` et `annuaire.model.json` recopiés de `examples/annuaire`, et `personnes.json` de `--count` personnes (noms accentués, particules, attributs facultatifs absents). `--seed` (1 par défaut) : mêmes options, mêmes fichiers.
- `table-bench` ouvre l'application, puis mesure le tri initial, le tri par `nom`, la préparation du filtre et le filtre `dup`, dans un tableau Markdown. Toujours en release ; seuils de la spécification : tri et filtre sous 200 ms, préparation du filtre sous une seconde. `cargo test` exécute son test sur `examples/annuaire`.
- Pour voir 50 000 personnes : `cargo xtask gen-data --model annuaire --count 50000 --out ~/annuaire-50k`, puis `tools/launch.sh --release -- ~/annuaire-50k`.

## Essai S1 : données générées et mesures

Générateur de fichiers conformes au modèle d'essai `examples/reseau-meteo/reseau.model.json` :

```bash
mkdir -p ~/s1
cargo xtask gen-data --size 10M --out ~/s1/reseau-10M.json
```

- `--size` (obligatoire) : taille visée, avec les suffixes `K`, `M` et `G` (puissances de 1024). Le fichier la dépasse de moins d'un objet.
- `--out` (obligatoire) : fichier à écrire.
- `--seed` (1 par défaut) : graine ; les mêmes options donnent un fichier identique, octet pour octet.
- `--text` (1.0 par défaut) : facteur sur la longueur des textes longs (`description`, `compte_rendu`, `texte`) ; à 0, ils sont omis (profil « numérique »).
- `--late-type` (0.0 par défaut) : part d'objets des listes polymorphes dont `_type_` vient après le premier attribut (ils passent par un arbre intermédiaire à la lecture).
- Les fichiers vont dans `~/s1/`, dans le système de fichiers Linux de WSL2, et jamais sous `/mnt/c` : le disque Windows y est lent (il fausserait les temps de chargement), et des fichiers de 500 Mo n'ont rien à faire dans le dépôt.

Programme de mesure (exemple `s1` de `ortelia-service`), toujours en compilation release :

```bash
cargo run --release -p ortelia-service --example s1 -- all ~/s1/reseau-10M.json
```

- `all <fichier> [--only floor,v0,store]` : lance chaque cas dans un processus à part (le programme se relance lui-même), pour que le pic de mémoire d'un cas ne fausse pas le suivant, puis affiche un tableau Markdown (une ligne par mesure et par implémentation). `--only floor,store` écarte V0, qui prend environ 11 fois la taille du fichier en mémoire.
- `case <floor|v0|store> <stream|buffer|count> <fichier>` : un seul cas, une ligne par mesure. `stats <fichier>` : nombre d'objets par classe.
- Implémentations : plancher (`IgnoredAny`, sans construction), V0 (`serde_json::Value`, point de repère), et le Store. Le second Store comparé par l'essai a été supprimé ; ses mesures restent dans `docs/essais/s1-stockage.md`. Mesures : C (chargement), M (mémoire, lue dans `/proc/self/status`, « non disponible » hors Linux), A (accès et navigation), E (modification) ; leur liste est dans `crates/ortelia-service/examples/s1/`.
- `cargo test` exécute aussi les tests de l'exemple, sur `examples/reseau-meteo/reseau-exemple.json`.

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
- Table des couches : constantes `LAYERS`, `UI_CRATE_PREFIX` et `TOOLS` de `xtask/src/layers.rs`. Niveaux : 0 `ortelia-model` ; 1 `ortelia-config`, `ortelia-expr` ; 2 `ortelia-service` ; 3 `ortelia-presentation` ; 4 `ortelia-ui-*` ; 5 `ortelia`. Une nouvelle crate (`ortelia-plugin-sdk`…) doit y être ajoutée en même temps qu'au workspace.
- Toute la logique d'affichage (tri, filtre, formulaires générés, broker, intentions, formats, état de session, mode design) va dans `ortelia-presentation`, testée sans interface.
- Les formats de configuration doivent être descriptibles par le métamodèle : contenance, références par nom (`refersTo`), polymorphisme par `_type_`, pas de dictionnaires.

### Tests de l'interface

egui décrit chaque widget dans un arbre d'accessibilité (AccessKit) : l'interface se teste dans le processus de test, sans fenêtre ni GPU. Cinq niveaux :

| Niveau | Ce qu'il vérifie | Où |
|---|---|---|
| 1. Tests de `ortelia-presentation` | toute la logique d'affichage | tests unitaires, sans interface |
| 2. egui_kittest sans rendu | l'interface affiche le view-model ; les clics et les touches produisent les bonnes intentions | tests de `ortelia-ui-egui` (`Harness::new_eframe`, `get_by_label`) |
| 3. Captures d'images | le rendu réel, comparé à une image de référence | tests de capture de `ortelia-ui-egui` (`src/snapshot_tests.rs`, références dans `tests/snapshots/`), sous Linux avec lavapipe |
| 4. Test de fumée de l'exécutable | la vraie fenêtre s'ouvre et dessine sans erreur | `crates/ortelia/tests/smoke.rs` |
| 5. Test manuel | l'impression générale, la version Windows | WSLg (`/lancer`) ; bac à sable Windows (`/bac-a-sable`, ou `powershell -ExecutionPolicy Bypass -File tools\sandbox\test-windows.ps1` : exécutable de la CI pour la branche courante, annuaire copié dans `tools/sandbox/app`, bilan du journal et du fichier d'état ; vGPU désactivé car il fait planter wgpu) |

- Toute logique se teste dans `ortelia-presentation`.
- Les tests de l'interface, les captures d'images et `table-bench` ouvrent les applications par `AppState::open`, sans état de session : ils ne dépendent jamais d'un `app.session.json` laissé dans `examples/` par `/lancer`, et n'y écrivent rien. Seul l'exécutable utilise `AppState::open_with_session` ; les tests de l'état de session travaillent sur une copie de l'annuaire dans un dossier temporaire, comme le test de fumée.
- Chaque widget de `ortelia-ui-egui` a ses tests egui_kittest : affichage du view-model et, dès qu'il y a des intentions, traduction des interactions en intentions.
- Les tests de l'interface ne recalculent rien : ils vérifient ce qui est affiché et ce qui est émis.
- Test de fumée : marqué `#[ignore]` (il faut un affichage), donc absent de `cargo test`. Le lancer avec `cargo test -p ortelia --test smoke -- --ignored` : dans WSLg en local, sous `xvfb-run -a` dans la CI Linux. Il attend `main window shown` dans le journal (20 s au plus) puis vérifie qu'Ortelia n'y a écrit aucune ligne `ERROR` (les lignes relayées des bibliothèques tierces ne comptent pas).
- Les tests qui lancent l'exécutable sans argument (`crates/ortelia/tests/startup.rs`) ne tournent que sous Linux, sans affichage : sous Windows, la fenêtre s'ouvrirait vraiment.
- Captures d'images (egui_kittest, features `wgpu` et `snapshot`) : chaque capture rend l'interface à 640 × 400, échelle 1, thème sombre, et la compare à `tests/snapshots/<nom>.png`, versionnée. Elles ne sont comparées que sous Linux, avec lavapipe (rendu logiciel Vulkan de Mesa), où le rendu est identique entre WSL2 et le runner `ubuntu-26.04` de la CI (`docs/essais/captures-images.md`). Ailleurs, elles sont signalées comme ignorées. Dans WSL2, lavapipe vient du paquet `mesa-vulkan-drivers`, comme dans la CI, et la version de Mesa doit rester celle d'Ubuntu 26.04.
- Tolérances : `kittest.toml` à la racine (seuil par pixel 0,6, aucun pixel fautif ; sections `[windows]`, `[mac]`, `[linux]`, mais pas `[macos]`, qui fait échouer la lecture du fichier par egui_kittest). Un test vérifie que ce fichier est bien celui qu'egui_kittest utilise, et refuse un seuil au-delà de 0,6 ou plus de 10 pixels fautifs tolérés.
- Mettre à jour une référence, dans WSL2 :
  ```bash
  UPDATE_SNAPSHOTS=1 cargo test -p ortelia-ui-egui      # remplace les références en échec
  UPDATE_SNAPSHOTS=force cargo test -p ortelia-ui-egui  # les remplace toutes (montée de version d'egui)
  ```
  Regarder la nouvelle image avant de la versionner. L'ancienne est gardée en `<nom>.old.png` et l'écart en `<nom>.diff.png`. Un échec écrit `<nom>.new.png`, plus `<nom>.diff.png` si la taille n'a pas changé. Ces trois sortes d'images sont ignorées par git. Après un échec dans la CI : `gh run download <id> -n snapshots-linux`. Si une capture échoue en local alors que la CI passe, ou l'inverse, comparer d'abord les versions de Mesa (`dpkg-query -W mesa-vulkan-drivers`).
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
- Après une modification non triviale, et avant de la considérer terminée : `/code-review` sur le diff, puis `/simplify`, puis `/verifier`.

### Skills du projet

Dans `.claude/skills/`, à côté de ceux d'OpenSpec :

| Skill | Rôle |
|---|---|
| `/verifier` | les vérifications de la CI en local (`tools/check.sh`), avec un bilan : avant chaque commit ou push |
| `/lancer` | compiler et lancer Ortelia, pour le montrer ou pour vérifier qu'il démarre sans erreur (`tools/launch.sh`) |
| `/bac-a-sable` | tester l'exécutable Windows de la CI dans le bac à sable Windows |

Sous Windows, les deux scripts tournent dans WSL2 : `wsl -- bash -l tools/check.sh`.

## Git

- Branche `main`, dépôt public `https://github.com/VincentBonnafous31/ortelia` (remote `origin`).
- CI GitHub Actions (`.github/workflows/ci.yml`), à chaque poussée sur `main`, à chaque pull request vers `main` et à la demande :
  - job `checks`, sous Linux (image fixée `ubuntu-26.04`, pour les captures d'images) et Windows, tout en `--locked` : `cargo fmt --all --check` (Linux), `cargo clippy --workspace --all-targets --locked -- -D warnings`, installation du rendu logiciel (Linux : lavapipe, Xvfb, Mesa), `cargo test --workspace --locked` (captures d'images comprises sous Linux ; après un échec, leurs images forment l'artefact `snapshots-linux`, 30 jours), `cargo xtask check-layers`, `cargo xtask gen-schemas --check` (Linux), puis le test de fumée (Linux sous Xvfb, bloquant ; Windows non bloquant) ;
  - job `coverage`, sous Linux (`ubuntu-26.04`, avec lavapipe), non bloquant pour les autres jobs : couverture du code par les tests, mesurée par cargo-llvm-cov. Le tableau par crate (lignes, fonctions, régions des fichiers de `src/`, modules de tests compris) s'affiche dans le résumé de l'exécution ; le rapport HTML, par fichier et par ligne, est l'artefact `coverage-html` (30 jours). Pas de seuil pour l'instant.
  - job `binaries`, hors pull requests : compilation release, `ortelia --version`, puis artefacts `ortelia-linux-x86_64` et `ortelia-windows-x86_64` (30 jours), téléchargeables depuis la page de l'exécution (`gh run download <id>`). C'est la seule source de la version Windows.
  - Seul `main` enregistre le cache Cargo (`Swatinem/rust-cache`).
- Avant de pousser, lancer la même séquence en local : `tools/check.sh` (skill `/verifier`). Il vérifie aussi les lints propres à Windows (code réservé à Linux par `#[cfg]`, donc mort sous Windows) par Clippy pour la cible `x86_64-pc-windows-msvc`, sans édition de liens, donc sans les outils MSVC. La cible s'installe une fois : `rustup target add x86_64-pc-windows-msvc`.
- Couverture en local (dans WSL2), après une installation unique (`rustup component add llvm-tools` et `cargo install cargo-llvm-cov --locked`) :
  ```bash
  cargo llvm-cov --workspace --locked --no-report
  cargo llvm-cov report --json --summary-only --output-path ~/target/ortelia/coverage.json
  cargo xtask coverage ~/target/ortelia/coverage.json   # tableau Markdown par crate
  cargo llvm-cov report --html                          # détail dans $CARGO_TARGET_DIR/llvm-cov/html
  ```
- `gh` (GitHub CLI) est installé et connecté.
- Ne committer ou pousser que sur demande.

## Conventions egui 0.37

L'API d'egui change à chaque version mineure : vérifier dans Context7 ou dans les sources des crates (`~/.cargo/registry/src/*/egui-0.37.*`) plutôt que de supposer.

- Mode immédiat : l'interface est redessinée à chaque image par `eframe::App::ui(&mut self, ui, frame)` (et non plus `update(ctx)`). L'état vit dans l'application, pas dans les widgets.
- Panneaux : `egui::Panel::left(..)` / `Panel::bottom(..)` puis `CentralPanel`, tous affichés avec `.show(ui, ..)`. `SidePanel`, `TopBottomPanel` et `show_inside` sont obsolètes.
- Table : `egui_extras::TableBuilder`. `body.rows(..)` ne dessine que les lignes visibles (testé avec 50 000 lignes). egui mémorise les largeurs sous l'`id_salt` de la table et repart de `Column::initial` quand le nombre de colonnes change.
- Conteneurs de la fenêtre : `egui_tiles` 0.18 (sans la feature `serde`), dans `ortelia-ui-egui` seulement (module `layout`). L'arbre `Tree<Pane>` est construit une fois, à l'ouverture, depuis la disposition de la présentation, qui reste la référence. `is_tile_draggable` renvoie faux et `simplification_options` renvoie `SimplificationOptions::OFF` (sinon egui_tiles fusionne deux splits imbriqués de même direction ou supprime un split d'un seul enfant). L'intention `SplitResized` n'est émise que si les parts d'egui_tiles diffèrent de celles de la présentation : egui_tiles signale un redimensionnement à chaque image tant que la séparation est tenue. Sa documentation n'est pas dans Context7 : lire les sources (`~/.cargo/registry/src/*/egui_tiles-0.18.*`).
- Formulaire (module `form`) : chaque cellule est dessinée dans un `Ui` enfant (`new_child`, qui ne réserve aucune place), et la place de toute la grille est réservée à la fin par `allocate_rect`, pour la zone de défilement. Un `TextEdit` sur un `&str` est sélectionnable mais non modifiable, et egui ne lui dessine ni fond ni bordure visible : le formulaire lui donne une bordure fine par `TextEdit::frame` (`read_only_frame`). `TextEdit::horizontal_align` est obsolète : `align(Align2::..)`. Le formulaire est dans un nœud d'accessibilité de rôle `Form`, sans rectangle : les tests y cherchent ses widgets (`get_by_role(Role::Form).get_by_role_and_label(..)`), car les cases de la table portent le nom de leur colonne.
- `egui::Id::new` est obsolète : `Id::unique` pour un identifiant global (table, arbre des tuiles).
- `DatePickerButton` travaille sur des `jiff::civil::Date` (pas chrono).
- Les polices emoji et symboles (⏷, 📆…) ne sont plus incluses par défaut : activer la feature `monochrome_emoji_fonts` de la crate `egui`, sinon elles s'affichent en carrés vides sous Linux/WSL. Même avec elle, certains symboles manquent (`▲`, `▼`) et egui_kittest panique sur un glyphe absent : vérifier la police (`fc-query` sur les polices de `epaint_default_fonts`). Les indicateurs de tri sont `⏶` et `⏷`.
- Décorations de fenêtre natives ; titre via `ViewportBuilder::with_title`.
