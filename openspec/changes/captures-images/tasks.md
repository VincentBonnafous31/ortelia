# Tâches : captures-images

Toutes les commandes `cargo` se lancent dans WSL2, avec `CARGO_TARGET_DIR=~/target/ortelia` (voir `CLAUDE.md`). Le travail se fait sur la branche `captures-images`, créée depuis `main`. Chaque tâche laisse le workspace compilable, Clippy sans avertissement et les tests au vert. Les tâches qui demandent une exécution de la CI supposent une poussée de la branche : la demander d'abord au porteur du projet.

## 1. Mise en place

- [x] 1.1 Ajouter les features `wgpu` et `snapshot` à la dépendance de développement egui_kittest de `ortelia-ui-egui`, et toml 1.2.2 au `Cargo.toml` racine puis aux dépendances de développement de `ortelia-ui-egui` (design.md, § 2). Créer `kittest.toml` à la racine (`threshold = 0.6`, `max_failed_pixels = 0`), et compléter `.gitignore` et `.gitattributes` (design.md, § 9). Vérifier :
  - `cargo test --workspace` réussit, et `cargo xtask check-layers` n'affiche aucune violation ;
  - `cargo tree -p ortelia -i dify` ne trouve pas `dify` : l'exécutable n'en dépend pas ;
  - `git check-ignore crates/ortelia-ui-egui/tests/snapshots/x.new.png` cite la règle du `.gitignore`.

## 2. Captures de la fenêtre principale

- [x] 2.1 Écrire le module de test `src/snapshot_tests.rs` : la construction commune des `Harness` (taille 640 × 400, échelle 1, thème sombre) et la capture `main_window` de l'application complète, sous le verrou `APP_TESTS`, ignorée hors de Linux (design.md, §§ 3 et 4). Produire `tests/snapshots/main_window.png` dans WSL2 avec `UPDATE_SNAPSHOTS=1`, et regarder l'image. Vérifier : sans `UPDATE_SNAPSHOTS`, le test des scénarios « Fenêtre principale conforme » et « Rendu conforme » passe dans WSL2.
- [x] 2.2 Écrire le test « rendu différent » : référence temporaire, second rendu conforme, rendu d'un autre texte en échec, arrêt immédiat en mode mise à jour (design.md, § 5). Vérifier : le test du scénario « Rendu différent » passe ; lancé avec `UPDATE_SNAPSHOTS=1`, il passe aussi, et `git status` ne montre aucun fichier modifié sous `tests/snapshots/`.
- [x] 2.3 Écrire le test qui lit `kittest.toml` : `max_failed_pixels` au plus 10 à la racine et dans `[windows]`, `[mac]` et `[linux]`, et pas de section `[macos]` (design.md, § 6). Vérifier : le test du scénario « Tolérances lues à la racine » passe ; il échoue si l'on met temporairement `max_failed_pixels = 11`, puis si l'on ajoute une section `[macos]` (deux modifications annulées ensuite).
- [x] 2.4 Essayer une fois la procédure de mise à jour (design.md, § 10) : changer temporairement la taille des captures, constater l'échec et la présence de `main_window.new.png` et `main_window.diff.png`, absents de `git status` ; relancer avec `UPDATE_SNAPSHOTS=1`, constater la réussite et `main_window.old.png` ; puis tout annuler (`git checkout` de la référence, suppression des images produites). Vérifier : après l'annulation, `git status` est propre et la capture `main_window` passe.

## 3. Essai

- [x] 3.1 Écrire le module de test temporaire `src/snapshot_trial.rs` : la scène d'essai, sa capture `trial_scene` et le test de stabilité (deux rendus identiques au pixel près), ignorés hors de Linux comme les autres captures (design.md, § 7). Produire `tests/snapshots/trial_scene.png` dans WSL2 et regarder l'image. Vérifier : les deux tests passent dans WSL2.
- [ ] 3.2 Écrire `.github/workflows/snapshots-trial.yml` : déclenchement par une poussée sur `captures-images`, matrice des quatre configurations, version du rendu affichée, tests de capture lancés avec `--include-ignored` sans faire échouer le job, journal et images publiés en artefact (design.md, § 7). Vérifier, après une poussée acceptée par le porteur du projet : les quatre jobs se terminent, chacun avec son artefact, et le workflow `CI` s'exécute comme avant.
- [ ] 3.3 Relever les mesures des quatre configurations (pixels fautifs par seuil pour `main_window` et `trial_scene`, stabilité, durée des jobs, version de Mesa ou du système, adaptateur choisi quand le journal le montre) et écrire `docs/essais/captures-images.md` : conditions, mesures, coût de la mise à jour des références, choix et raisons (design.md, § 7). Si le choix n'est pas lavapipe sur `ubuntu-latest`, réviser les spécifications, la conception et les tâches du groupe 4 par `/openspec-update-change` avant de continuer. Vérifier : le compte rendu donne une mesure par configuration et par capture, et le choix suit les critères dans leur ordre.

## 4. Application du résultat

- [ ] 4.1 Modifier `.github/workflows/ci.yml` (design.md, § 8) : paquets du rendu logiciel installés avant l'étape « Tests » du job `checks` (Linux), étape qui publie l'artefact `snapshots-linux` après un échec, `mesa-vulkan-drivers` installé dans le job `coverage`. Fixer l'image du runner si l'essai l'a demandé, et régler `kittest.toml` d'après les mesures. Vérifier, après une poussée acceptée par le porteur du projet :
  - le workflow `CI` réussit sous Linux et sous Windows, sans artefact `snapshots-linux` ;
  - le journal des tests sous Windows liste les captures comme ignorées ;
  - un commit temporaire qui change le rendu de la fenêtre principale sans mettre à jour sa référence fait échouer `checks (ubuntu-latest)`, et l'artefact `snapshots-linux` contient `main_window.new.png` et `main_window.diff.png` ; ce commit est ensuite annulé.
- [ ] 4.2 Supprimer `src/snapshot_trial.rs`, `tests/snapshots/trial_scene.png` et `.github/workflows/snapshots-trial.yml`, et citer dans le compte rendu le commit qui les contenait encore. Vérifier : `cargo test --workspace` réussit, `git grep snapshot_trial` ne trouve rien hors du compte rendu, et ce compte rendu cite le commit.
- [ ] 4.3 Mettre à jour `CLAUDE.md` (table des documents ; niveau 3 de la stratégie de tests de l'interface : où sont les captures, plateforme retenue, `kittest.toml`, procédure de mise à jour de la design.md, § 10 ; paquets du rendu logiciel nécessaires dans WSL2), `docs/feuille-de-route-annuaire.md` (incrément 2 : résultat de l'essai) et `docs/feuille-de-route.md` (état : l'incrément 2 est en cours). Vérifier : les trois documents décrivent la plateforme retenue sans contredire les spécifications ni le compte rendu, et la procédure décrite fonctionne telle qu'elle est écrite (tâche 2.4).

## 5. Vérification finale

- [ ] 5.1 Lancer `/code-review` sur le diff complet et corriger les problèmes retenus. Vérifier : aucun problème important restant.
- [ ] 5.2 Lancer `/simplify` et appliquer les simplifications retenues. Vérifier : le diff simplifié compile et les tests passent.
- [ ] 5.3 Lancer `/verifier` (`tools/check.sh` : format, Clippy pour Linux et pour la cible Windows, tests avec les captures, règle des couches, schémas, test de fumée). Vérifier : toutes les étapes réussissent.

## Workflow follow-up

- Committer, pousser et ouvrir la pull request à la demande du porteur du projet. Après la fusion et une CI verte, archiver avec `/openspec-archive-change` : la spécification `ui-snapshots` entre dans `openspec/specs/`, et le delta de `continuous-integration` y est fusionné.
- Le critère « la plateforme des captures d'images est choisie » de l'incrément 2 est alors rempli. Prochain changement : `vue-table`, qui ajoutera la capture de la table.
