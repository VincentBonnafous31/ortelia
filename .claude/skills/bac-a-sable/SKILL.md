---
name: bac-a-sable
description: Teste la version Windows d'Ortelia compilée par la CI dans le bac à sable Windows (Windows Sandbox) - choisit l'exécution de la CI, télécharge ortelia.exe, ouvre le bac à sable, puis lit le journal. À utiliser seulement quand l'utilisateur demande de tester la version Windows, l'exécutable Windows ou le bac à sable.
argument-hint: "[branche | id d'exécution de la CI]"
---

# Tester la version Windows dans le bac à sable

La version Windows n'est compilée que par la CI : job `binaries`, artefact `ortelia-windows-x86_64`, conservé 30 jours. Ce job ne tourne pas pour les pull requests, seulement pour les poussées sur `main` et les exécutions lancées à la main (`workflow_dispatch`).

Toutes les commandes passent par l'outil Bash (Git Bash), depuis la racine du dépôt : Windows PowerShell 5.1 transmet mal les guillemets du filtre `--jq`.

## 1. Choisir l'exécution de la CI

- Si l'argument est un nombre, c'est l'id de l'exécution : passer à l'étape 2.
- Sinon, la branche visée est l'argument, ou `main` par défaut. Si l'utilisateur parle de « sa » version ou de la branche en cours, c'est la branche courante.

Dernière exécution réussie de la branche qui a produit l'exécutable (toutes sauf celles des pull requests) :

```
gh run list --workflow ci.yml --branch <branche> --status success --limit 50 --json databaseId,headSha,createdAt,event --jq '[.[] | select(.event != "pull_request")][0]'
```

Comparer son `headSha` au dernier commit poussé de la branche (`git fetch`, puis `git rev-parse origin/<branche>`) et dire à l'utilisateur quel commit sera testé.

S'il n'existe pas d'exécution pour ce commit, proposer à l'utilisateur de lancer la CI sur la branche, et attendre son accord avant de le faire :

```
gh workflow run ci.yml --ref <branche>
```

La commande affiche l'URL de l'exécution créée (`…/actions/runs/<id>`) : en tirer l'id. Si elle n'en affiche pas, attendre quelques secondes, puis prendre la dernière exécution `--event workflow_dispatch` de la branche, en vérifiant que son `createdAt` suit le lancement et que son `headSha` est le commit voulu : ne jamais prendre l'exécution précédente à sa place. Attendre la fin en arrière-plan avec `gh run watch <id> --exit-status` (compter une quinzaine de minutes). Si elle échoue, rendre compte de l'échec et s'arrêter.

## 2. Télécharger l'exécutable

```
gh run download <id> -n ortelia-windows-x86_64 -D tools/sandbox/build/<id>
```

`tools/sandbox/build/` n'est pas versionné. Si `tools/sandbox/build/<id>/ortelia.exe` existe déjà, le réutiliser. Si l'artefact a expiré, relancer la CI comme à l'étape 1.

## 3. Ouvrir le bac à sable

- Un seul bac à sable peut tourner à la fois : demander à l'utilisateur de fermer celui qui serait encore ouvert.
- Noter le nombre de lignes du journal du jour s'il existe (`tools/sandbox/logs/ortelia.<AAAA-MM-JJ>.log`, date UTC) : on ne lira ensuite que les lignes de ce lancement.

Puis :

```
powershell.exe -ExecutionPolicy Bypass -File tools/sandbox/run-sandbox.ps1 tools/sandbox/build/<id>
```

Le bac à sable démarre en une à deux minutes, puis `start.cmd` y lance `ortelia.exe` avec un journal en niveau `debug` dans `tools/sandbox/logs/` (dossier partagé avec le poste). Le vGPU est désactivé, car il fait planter wgpu : le rendu est logiciel (WARP), donc lent, et c'est normal.

## 4. Test manuel et compte rendu

1. Dire à l'utilisateur ce qu'il peut vérifier, d'après l'incrément en cours, et lui demander de prévenir quand il a fini.
2. Lire les nouvelles lignes du journal et rendre compte : démarrage (`main window shown`), lignes `WARN` et `ERROR` d'Ortelia (cibles `ortelia…`), panique éventuelle.
3. Rappeler que fermer le bac à sable efface tout son contenu ; le journal, lui, reste dans `tools/sandbox/logs/`.
