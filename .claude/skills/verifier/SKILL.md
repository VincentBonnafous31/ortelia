---
name: verifier
description: Lance en local les vérifications de la CI d'Ortelia (format, Clippy pour Linux et pour la cible Windows, tests, règle des couches, test de fumée) et en fait le bilan. À utiliser avant un commit ou un push, à la fin d'un changement OpenSpec, ou quand l'utilisateur demande de « vérifier », de « lancer la CI en local » ou de contrôler que tout passe.
argument-hint: "[--no-smoke] [--no-windows]"
---

# Vérifier comme la CI

`tools/check.sh` reproduit le job `checks` de `.github/workflows/ci.yml`, plus Clippy pour la cible Windows, que la CI ne voit que sur son exécuteur Windows. Toutes les étapes s'exécutent, même après un échec ; un bilan termine la sortie ; le code de sortie vaut 1 si une étape a échoué.

## Lancer

Depuis la racine du dépôt :
- sous Windows, cargo ne tourne que dans WSL2 : `wsl -- bash -l tools/check.sh $ARGUMENTS` (outil PowerShell) ;
- sous Linux : `bash tools/check.sh $ARGUMENTS`.

Options :
- `--no-smoke` : sans le test de fumée (qui ouvre brièvement une fenêtre) ;
- `--no-windows` : sans Clippy pour la cible Windows.

Prévoir un délai long (600000 ms) : la première compilation après un changement de dépendances prend plusieurs minutes.

## Lire le résultat

1. Commencer par le bilan (`=== summary`), en fin de sortie.
2. Pour chaque étape `FAILED`, retrouver sa sortie au-dessus du bilan, sous `=== <étape>`, et en tirer la cause. Un échec de `format` se corrige par `cargo fmt --all`.
3. Rendre compte à l'utilisateur : le bilan, puis, pour chaque échec, la cause et la correction proposée. Signaler toute étape sautée (`skipped`) : ne pas annoncer que tout passe si une étape n'a pas tourné.

Ne jamais faire passer une vérification en la contournant (`#[allow]` sans raison, test ignoré ou supprimé) : corriger la cause, ou demander à l'utilisateur.
