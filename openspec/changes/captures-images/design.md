# Conception : captures-images

## Context

- `ortelia-ui-egui` a déjà des tests egui_kittest 0.37 sans rendu (`Harness::new_eframe`, `get_by_label`), dans le module de tests de `src/lib.rs`. L'application egui (`OrteliaApp`) y est privée. Les tests qui la lancent sont sérialisés par le verrou `APP_TESTS`, à cause du cache d'intérêt de `tracing` (`CLAUDE.md`).
- La CI Linux installe déjà lavapipe (`mesa-vulkan-drivers`) et Xvfb, mais après l'étape des tests, pour le seul test de fumée. Le job `coverage` relance tous les tests, sans ces paquets.
- WSL2, sur le poste de développement : Ubuntu 26.04.1, Mesa 26.0.8, lavapipe installé (`lvp_icd.json`). Le runner `ubuntu-latest` de GitHub n'a pas forcément la même version de Mesa.
- Ce qu'apporte egui_kittest 0.37, vérifié dans Context7 et dans les sources de la crate :
  - avec les features `wgpu` et `snapshot`, `Harness::snapshot(name)` rend l'image et la compare à `tests/snapshots/<name>.png`, chemin relatif au dossier courant, c'est-à-dire la racine de la crate pendant `cargo test` ;
  - `try_snapshot` et `try_image_snapshot_options` rendent un `Result`. L'erreur `SnapshotError::Diff` donne le nombre de pixels fautifs, et une série `failing_pixels_by_threshold` qui compte ces pixels pour plusieurs seuils ; le message d'échec la reproduit ;
  - un échec écrit `<name>.new.png` et `<name>.diff.png` à côté de la référence ; une mise à jour garde l'ancienne image en `<name>.old.png` ;
  - `UPDATE_SNAPSHOTS` vaut `1` (remplace les références en échec, et le test réussit) ou `force` (les remplace toutes). C'est le seul moyen de choisir le mode : il n'y a pas d'option dans `SnapshotOptions` ;
  - le sélecteur d'adaptateur wgpu de la crate préfère les rendus logiciels (« *CPU is the best for our purposes* ») : lavapipe sous Linux, WARP sous Windows ; Metal sous macOS, faute de rendu logiciel ;
  - `kittest.toml`, cherché depuis le dossier courant en remontant, règle `threshold` (0,6 par défaut), `max_failed_pixels` (0 par défaut) et `output_path`, avec des sections `[windows]`, `[mac]` et `[linux]`. Le README de la crate écrit `[macos]`, mais le code ne lit que `[mac]` ;
  - le `Harness` d'egui_kittest n'installe pas le fournisseur de polices du système : seules les polices intégrées à egui servent, quel que soit le poste.
- Motivation : voir `proposal.md`, section Why. Comportements attendus : voir les spécifications `ui-snapshots` et `continuous-integration`.

## Goals / Non-Goals

**Goals :**
- Des captures reproductibles : même image pour le même code, sur la plateforme retenue et dans WSL2.
- Une mise à jour des références sur le poste de développement, sans passer par la CI.
- Un essai qui décide sur des mesures : nombres de pixels fautifs par seuil, sur quatre configurations, contre des références produites dans WSL2.
- À la fin, rien qui ne servait qu'à l'essai : ni scène d'essai, ni workflow de comparaison.

**Non-Goals :**
- Tester le comportement d'egui_kittest lui-même (mode de mise à jour, images écrites en cas d'échec) : la procédure est décrite dans `CLAUDE.md` et vérifiée une fois, à la main, dans les tâches.
- Capturer l'interface sous plusieurs thèmes ou échelles.

## Decisions

### 1. Où va quoi

| Élément | Crate ou dossier | Fichier |
|---|---|---|
| Tests de capture de la fenêtre principale | `ortelia-ui-egui` | `src/snapshot_tests.rs` (module de test) |
| Images de référence | `ortelia-ui-egui` | `tests/snapshots/*.png` |
| Scène d'essai (temporaire) | `ortelia-ui-egui` | `src/snapshot_trial.rs` (module de test) |
| Tolérances | racine du dépôt | `kittest.toml` |
| Comparaison sur quatre configurations (temporaire) | — | `.github/workflows/snapshots-trial.yml` |
| Compte rendu de l'essai | — | `docs/essais/captures-images.md` |

Les tests de capture sont des modules de test de la crate (`#[cfg(test)] mod snapshot_tests;`), et non des tests d'intégration sous `tests/`. Comme en Java, où un test du même package voit les classes de portée package, un module de test voit les éléments privés de la crate : `OrteliaApp` et `draw_main_window` restent privés. Le dossier `tests/snapshots/` ne contient que des images ; aucun fichier `.rs` n'y crée de test d'intégration.

### 2. Dépendances

- egui_kittest 0.37.0, en dépendance de développement de `ortelia-ui-egui`, passe des features `eframe` aux features `eframe`, `wgpu` et `snapshot`. Il ajoute `dify` (comparaison d'images) ; `image` 0.25.10 et wgpu 30.0.2 sont déjà dans `Cargo.lock`, par eframe. Une dépendance de développement est l'équivalent du scope `test` de Maven : l'exécutable `ortelia` n'en hérite pas.
- toml 1.2.2, en dépendance de développement, pour le test qui lit `kittest.toml` (Decision 6). C'est la version qu'egui_kittest utilise déjà : `Cargo.lock` n'en gagne pas une seconde.
- Les features sont activées sur tous les systèmes, pas seulement sous Linux. Écarté : des dépendances de développement propres à Linux (`[target.'cfg(target_os = "linux")'.dev-dependencies]`). Il faudrait alors retirer le code des tests de la compilation ailleurs, et l'essai ne pourrait pas tourner sous macOS et Windows. Le coût sous Windows est faible : wgpu y est déjà compilé pour eframe.

### 3. Tests ignorés hors de Linux

Chaque test de capture porte :

```rust
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "snapshots are compared on Linux only, with lavapipe (docs/essais/captures-images.md)"
)]
```

`cfg_attr` n'applique l'attribut que si la condition est vraie : c'est un `#ifdef` posé sur une annotation. Hors de Linux, le test est compilé et listé comme ignoré, avec sa raison ; `cargo test -- --include-ignored` le lance quand même, ce que fait l'essai sous macOS et Windows.

Écartés :
- `#[cfg(target_os = "linux")]` : le test n'existerait pas ailleurs, et l'essai ne pourrait pas le lancer ;
- un retour anticipé dans le test : il serait affiché comme réussi sans rien avoir vérifié.

### 4. Conditions de rendu fixées

Chaque capture construit son `Harness` par `Harness::builder()` :
- taille fixe, `with_size(egui::vec2(640.0, 400.0))` ;
- échelle 1, `with_pixels_per_point(1.0)`, pour ne pas dépendre de l'écran ;
- thème sombre, `with_theme(egui::Theme::Dark)` : c'est le thème qu'egui prend quand il ne connaît pas celui du système. Le fixer rend la capture indépendante du poste.

La capture de la fenêtre principale lance l'application complète (`build_eframe` avec `OrteliaApp::new(AppState::new())`), comme le test « texte central affiché ». Elle prend donc le verrou `APP_TESTS` : elle journalise « main window shown ».

Le texte central contient le numéro de version : la référence changera à chaque nouvelle version. C'est accepté. Remplacer le texte par un texte fixe ne testerait plus le vrai contenu, et la version change rarement.

### 5. Le test « rendu différent »

Ce test prouve que la comparaison n'est pas vide : un seuil trop large ou une capture blanche le feraient échouer. Il rend la fenêtre principale par `draw_main_window`, avec un view-model écrit dans le test, dans des `Harness` construits comme ceux des captures (`build_ui`), et lit chaque image par `Harness::render()` :
1. rendre le texte central d'origine, et enregistrer l'image comme référence dans un dossier temporaire ;
2. rendre le même texte une seconde fois et le comparer à cette référence, par `egui_kittest::try_image_snapshot_options(&image, "main_window", &SnapshotOptions::new().output_path(dossier_temporaire))` : la comparaison doit réussir, ce qui montre que l'écart suivant vient du texte et non du rendu ;
3. rendre un autre texte central et le comparer de même : le résultat doit être `SnapshotError::Diff`, avec un nombre de pixels fautifs supérieur à zéro.

Le test ne lit ni n'écrit les références du dépôt : il ne dépend pas du numéro de version. En mode mise à jour (`UPDATE_SNAPSHOTS` défini), egui_kittest remplacerait la référence temporaire et rendrait `Ok` : le test lit alors la variable (lire l'environnement est sûr ; seul le modifier est `unsafe` en Rust 2024) et s'arrête aussitôt, avec un commentaire qui explique pourquoi. Ainsi, `UPDATE_SNAPSHOTS=1 cargo test` réussit, comme egui_kittest le promet.

### 6. Tolérances dans `kittest.toml`

```toml
# Tolerances of the snapshot tests (egui_kittest); see docs/essais/captures-images.md.
threshold = 0.6
max_failed_pixels = 0
```

Les valeurs finales viennent de l'essai, avec une section `[linux]` si besoin. Un test lit le fichier (`concat!(env!("CARGO_MANIFEST_DIR"), "/../../kittest.toml")`, par toml) et vérifie que `max_failed_pixels` ne dépasse 10 ni à la racine ni dans les sections `[windows]`, `[mac]` et `[linux]`. Il refuse aussi une section `[macos]`, que le code d'egui_kittest ne lirait pas, malgré son README.

### 7. L'essai

**Références** : produites dans WSL2, par `UPDATE_SNAPSHOTS=1 cargo test -p ortelia-ui-egui snapshot`, puis versionnées sur la branche du changement.

**Scène d'essai** (`snapshot_trial.rs`) : la fenêtre principale n'affiche qu'un titre, trop peu pour mesurer quoi que ce soit. La scène rend, avec les widgets d'egui seuls :
- des textes de plusieurs tailles ;
- une grille rayée de 10 lignes sur 4 colonnes, avec des nombres et des dates ;
- une case à cocher, un bouton, un champ de texte, un séparateur ;
- une zone défilante dont la barre de défilement est visible.

C'est ce que rendront les tables et les formulaires. La scène est rendue deux fois, dans deux `Harness`, et les deux images doivent être identiques au pixel près : c'est la mesure de stabilité.

**Workflow `snapshots-trial.yml`** : déclenché par chaque poussée sur la branche `captures-images` (`on: push: branches: [captures-images]`). Écarté : `workflow_dispatch`, que GitHub ne propose que pour un workflow présent sur la branche par défaut ; or ce workflow est supprimé avant la fusion et n'atteint jamais `main`. Une matrice de quatre configurations :

| Configuration | Rendu | Mise en place |
|---|---|---|
| `ubuntu-latest` | lavapipe | `apt-get install mesa-vulkan-drivers` |
| `ubuntu-latest`, conteneur `ubuntu:26.04` | lavapipe, même version de Mesa que WSL2 | paquets et rustup installés dans le conteneur |
| `macos-latest` | Metal (vrai GPU) | aucune |
| `windows-latest` | WARP (Direct3D 12 logiciel) | aucune |

Chaque configuration affiche d'abord la version de son rendu (paquet Mesa sous Linux, version du système ailleurs), puis lance `cargo test -p ortelia-ui-egui --locked snapshot -- --include-ignored` sans faire échouer le job (`continue-on-error`), et publie comme artefact le journal du test et les images `*.new.png` et `*.diff.png`. Les nombres de pixels fautifs par seuil se lisent dans les messages d'échec.

**Critères, dans l'ordre** :
1. une référence produite dans WSL2 passe dans la CI avec un seuil d'au plus 0,6 et au plus 10 pixels fautifs : la mise à jour reste locale ;
2. la stabilité : deux rendus de suite donnent la même image ;
3. le coût : durée du job, et ce que demande la mise à jour d'une référence.

**Résultat attendu et repli** :
- si `ubuntu-latest` passe : les captures restent dans le job `checks`, tel qu'il est ;
- si seul le conteneur Ubuntu 26.04 passe : un job à part exécute les tests de `ortelia-ui-egui` dans ce conteneur, et le job `checks` les ignore sous Linux aussi ;
- si aucune configuration Linux ne passe sans relever le seuil au-delà de 0,6 ou les pixels fautifs au-delà de 10 : on choisit entre ce relèvement et macOS, avec des références produites par la CI. Les spécifications sont alors révisées (`/openspec-update-change`) avant la suite des tâches.

**Compte rendu** (`docs/essais/captures-images.md`), sur le modèle de `docs/essais/s1-stockage.md` :
- les conditions (versions de Mesa, de wgpu et d'egui, images des runners, adaptateur choisi) ;
- les mesures par configuration ;
- le choix et ses raisons, et le commit de l'essai.

La scène d'essai et le workflow de comparaison sont ensuite supprimés. Le commit cité dans le compte rendu permet de les retrouver.

### 8. La CI après l'essai (lavapipe sous Linux)

- Job `checks`, sous Linux : l'étape d'installation des paquets (`xvfb`, `mesa-vulkan-drivers`, `libgl1-mesa-dri`, `libegl1`, `libxkbcommon-x11-0`) passe avant l'étape « Tests » et devient « Install the software rendering packages ». Les tests de capture tournent avec les autres, dans `cargo test --workspace`.
- Juste après l'étape « Tests », une étape `if: failure() && runner.os == 'Linux'` publie l'artefact `snapshots-linux` : les fichiers `crates/*/tests/snapshots/**/*.new.png` et `*.diff.png` (`if-no-files-found: ignore`, 30 jours). Sans échec, l'étape ne s'exécute pas et aucun artefact n'est publié.
- Job `coverage` : il installe `mesa-vulkan-drivers` avant ses tests, qu'il relance tous.
- Sous Windows, les tests de capture sont listés comme ignorés (Decision 3).
- `tools/check.sh` lance déjà `cargo test --workspace` dans WSL2, où lavapipe est installé : les captures en font partie sans autre changement.

### 9. Fichiers de git

- `.gitignore` : `**/tests/snapshots/**/*.new.png`, `**/tests/snapshots/**/*.diff.png`, `**/tests/snapshots/**/*.old.png`.
- `.gitattributes` : `*.png binary`. Avec `* text=auto eol=lf`, git détecte déjà les PNG comme binaires ; la ligne explicite évite qu'une conversion des fins de ligne les abîme un jour.

### 10. Mettre à jour une référence (`CLAUDE.md`)

Dans WSL2 :

```bash
UPDATE_SNAPSHOTS=1 cargo test -p ortelia-ui-egui      # remplace les références en échec
UPDATE_SNAPSHOTS=force cargo test -p ortelia-ui-egui  # les remplace toutes (montée de version d'egui)
```

Regarder la nouvelle image (et `*.old.png`, l'ancienne) avant de la versionner. Après un échec dans la CI : `gh run download <id> -n snapshots-linux`.

## Risks / Trade-offs

- [Mesa différent entre WSL2 (26.0.8) et le runner Ubuntu] → c'est ce que mesure l'essai ; le conteneur Ubuntu 26.04 est le repli qui aligne les versions.
- [L'image `ubuntu-latest` change de version d'Ubuntu, donc de Mesa, sans prévenir] → fixer l'image du job qui compare les captures (`ubuntu-24.04`, ou le conteneur) quand l'essai désigne une configuration Linux.
- [Une mise à jour de Mesa dans WSL2 (`apt upgrade`) décale le rendu local] → la version de Mesa est notée dans le compte rendu. Si les captures échouent en local alors que la CI passe, comparer d'abord les versions de Mesa.
- [Une montée de version d'egui change le rendu de toutes les captures] → `UPDATE_SNAPSHOTS=force`, puis revue des images de différence avant de versionner.
- [Dans WSL2, un second adaptateur Vulkan existe (Direct3D 12 par le pilote dzn)] → egui_kittest préfère l'adaptateur CPU, donc lavapipe ; le compte rendu note l'adaptateur réellement choisi.
- [La compilation sous macOS n'a jamais été essayée] → `ortelia-ui-egui` et les crates dont il dépend n'ont pas de code propre à un système ; un échec de compilation serait consigné dans le compte rendu, sans bloquer l'essai sur les autres configurations.
- [La référence de la fenêtre principale change à chaque version] → accepté (Decision 4).
- [Des images versionnées alourdissent le dépôt] → une capture de 640 × 400 en aplats pèse quelques kilo-octets ; à revoir si les captures se multiplient.
