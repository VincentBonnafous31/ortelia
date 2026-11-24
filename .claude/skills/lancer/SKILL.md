---
name: lancer
description: Compile et lance Ortelia (version Linux, affichée sur le bureau Windows par WSLg) pour le montrer à l'utilisateur, puis lit son journal à la fermeture ; ou vérifie, sans l'utilisateur, qu'il démarre sans erreur. À utiliser quand l'utilisateur demande de lancer, démarrer, ouvrir ou voir l'application, ou pour confirmer qu'une modification fonctionne dans la vraie application.
argument-hint: "[--release] [--check] [-- arguments d'Ortelia]"
---

# Lancer Ortelia

Depuis la racine du dépôt :
- sous Windows : `wsl -- bash -l tools/launch.sh <options>` (outil PowerShell) ; WSLg affiche la fenêtre sur le bureau Windows ;
- sous Linux : `bash tools/launch.sh <options>`.

Options :
- `--release` : compilation optimisée, pour juger des performances (défilement de 50 000 lignes…) ;
- `--check` : vérification automatique, voir plus bas ;
- `-- <arguments>` : arguments transmis à Ortelia (par exemple, plus tard, un `app.json`).

## Montrer l'application à l'utilisateur

`tools/launch.sh` compile Ortelia et le lance avec un journal dans un dossier neuf, qui ne contient donc que les lignes de ce lancement. À la fermeture de la fenêtre, il affiche les avertissements et erreurs de ce journal, puis le code de sortie.

1. Lancer la commande **en arrière-plan** (`run_in_background`) : elle ne rend la main qu'à la fermeture de la fenêtre.
2. Dire à l'utilisateur que la fenêtre va s'ouvrir et ce qu'il peut y regarder.
3. Quand la commande se termine, rendre compte : code de sortie, lignes `WARN` et `ERROR`.

## Vérifier le démarrage sans l'utilisateur

`--check` lance le test de fumée (`crates/ortelia/tests/smoke.rs`) : il démarre Ortelia, attend la ligne `main window shown` du journal (20 s au plus), et échoue si Ortelia s'arrête ou journalise une erreur. Une fenêtre s'ouvre brièvement sur le bureau de l'utilisateur. `/verifier` lance déjà ce test : inutile de refaire `--check` juste après, sauf avec `--release`.

## Lire le journal

Seules comptent les lignes dont la cible commence par `ortelia`. Les autres viennent des bibliothèques tierces (winit, wgpu, egui…) ; sous WSLg, plusieurs sont attendues et sans conséquence, de même que les messages `libEGL` et `MESA` sur la sortie d'erreur. Ne les signaler que si elles se rapportent à un problème observé.
