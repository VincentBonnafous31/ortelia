# Proposition : socle-workspace

## Why

Ortelia n'a encore aucun code. Avant le premier incrément fonctionnel, il faut un socle qui fasse respecter dès le départ la séparation en couches (décisions D25 à D27 de `docs/architecture-couches.md`) et qui vérifie chaque modification sous Linux et sous Windows. Sur le poste de développement, `cargo` ne fonctionne que dans WSL2, et la version Windows ne peut être compilée que par la CI : sans CI, cette version ne serait jamais vérifiée.

Ce changement réalise la partie « socle » de l'**incrément 0** de `docs/feuille-de-route.md`. Il remplit le premier critère « Fini quand » de cet incrément : **CI verte sur les deux OS**. Les deux autres critères (mesures de S1 consignées ; choix de S2 et S3 tranchés) relèvent des essais techniques S1 à S3, qui feront l'objet de changements séparés.

## What Changes

- **Workspace Cargo en couches** : les crates `ortelia-model`, `ortelia-config`, `ortelia-expr`, `ortelia-service`, `ortelia-presentation`, `ortelia-ui-egui` et l'exécutable `ortelia`. Elles sont presque vides, mais leurs dépendances suivent déjà la règle des couches. Réglages communs : édition 2024, version de Rust fixée, licence MIT OR Apache-2.0, lints partagés.
- **Vérification automatique de la règle des couches** par un outil du workspace (`cargo xtask check-layers`). Il signale toute crate qui dépend d'une couche de même niveau ou d'une couche supérieure, toute crate qui n'est pas située dans une couche, et toute crate sous `ortelia-ui-*` qui dépend, même indirectement, d'egui ou de GPUI.
- **Une première tranche verticale minimale** : `ortelia-presentation` fournit le titre de la fenêtre principale, `ortelia-ui-egui` l'affiche, et l'exécutable ouvre la fenêtre. Un test sans écran (egui_kittest) vérifie l'application egui complète. Cette tranche valide toute la chaîne de compilation, y compris eframe sous Windows, et pose le motif « la présentation calcule, l'interface dessine ».
- **Test de fumée de l'exécutable** : le vrai binaire est lancé avec un affichage, réel ou virtuel, et le test vérifie que la fenêtre dessine sa première image sans erreur. Ce test est bloquant dans la CI Linux (écran virtuel Xvfb, rendu logiciel Mesa). Sous Windows, il n'est pas bloquant tant que le rendu logiciel n'a pas fait ses preuves sur les machines de GitHub.
- **Ligne de commande minimale** : `ortelia --version` affiche la version et se termine sans ouvrir de fenêtre. La CI s'en sert pour vérifier que les binaires produits démarrent.
- **Journalisation dans un fichier** : un fichier par jour dans le dossier de données local de l'utilisateur, avec un nombre de fichiers conservés limité et un niveau réglable par une variable d'environnement. Règle de confidentialité : aucune valeur de donnée de l'utilisateur dans les journaux.
- **CI GitHub Actions sous Linux et Windows** : format, clippy sans avertissement, tests, vérification des couches, puis binaires de version publiés comme artefacts.
- **Documentation** : `CLAUDE.md` (état du projet, commandes, convention de journalisation) et `README.md` (statut).

### Hors périmètre

- Les essais techniques S1 (stockage dynamique, 500 Mo), S2 (Rhai ou CEL) et S3 (`egui_tiles` avec egui 0.37) : changements séparés.
- Tout contenu fonctionnel des crates : métamodèle, lecture de `app.json`, sources, présentation de la table, etc. (incrément 1 et suivants).
- L'état de session (taille de la fenêtre, disposition) et les libellés FR/EN : incrément 1.
- Les crates `ortelia-geo` et `ortelia-plugin-sdk` : elles seront ajoutées avec leurs incréments, et la table des couches sera complétée à ce moment-là.
- L'option `--ui` et toute seconde interface.
- La signature des exécutables Windows, la publication de versions (*releases*), la vérification des licences des dépendances.
- Les tests de performance en CI.
- Les tests par captures d'images (comparaison du rendu à une image de référence) : reportés à l'incrément 1, avec la table, qui est le premier widget dont le rendu vaut d'être figé. Un court essai choisira alors entre une machine macOS de la CI (vrai GPU) et le rendu logiciel lavapipe sous Linux.
- Le pilotage de l'exécutable par l'accessibilité du système (UI Automation, AT-SPI) : lourd et fragile ; les tests sans écran couvrent le même besoin.

## Capabilities

### New Capabilities

- `layered-workspace` : organisation du workspace en couches et vérification automatique de la règle de dépendance, y compris l'interdiction d'egui et de GPUI sous `ortelia-ui-*`.
- `app-launch` : lancement de l'exécutable `ortelia`, avec la ligne de commande (`--version`) et l'ouverture de la fenêtre principale dont le contenu vient de la couche de présentation.
- `logging` : journalisation de l'application dans des fichiers, avec emplacement, rotation, niveau réglable et règle de confidentialité.
- `continuous-integration` : vérifications automatiques à chaque modification (format, lint, tests, couches, test de fumée de l'exécutable) sous Linux et Windows, et production des binaires.

### Modified Capabilities

Aucune : `openspec/specs/` est encore vide.

## Impact

- **Nouveaux fichiers** : `Cargo.toml` (workspace), `Cargo.lock`, `rust-toolchain.toml`, `.cargo/config.toml` (alias `xtask`), `crates/*` (sept crates), `xtask/`, `.github/workflows/ci.yml`.
- **Fichiers modifiés** : `CLAUDE.md` (dont la convention de tests de l'interface), `README.md`, `docs/feuille-de-route.md` (essai des captures d'images ajouté à l'incrément 1).
- **Dépendances externes** : eframe et egui 0.37, egui_kittest 0.37 avec sa feature `eframe` (tests), tracing, tracing-subscriber, tracing-appender, dirs, cargo_metadata (outil xtask seulement), tempfile (tests).
- **Poste de développement** : `rust-toolchain.toml` demande les composants rustfmt et clippy, que rustup installera dans WSL2 à la première commande `cargo`.
- **GitHub** : premier workflow du dépôt public `VincentBonnafous31/ortelia` ; les binaires de chaque exécution sur `main` sont téléchargeables comme artefacts.
