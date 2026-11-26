# Essai : plateforme des captures d'images

Résultats de l'essai du changement OpenSpec `captures-images`, premier changement de l'incrément 2 (`docs/feuille-de-route-annuaire.md`). Il s'agissait de choisir où rendre les captures d'images de l'interface (niveau 3 de la stratégie de tests de l'interface, `CLAUDE.md`). Le rendu doit être identique d'une exécution à l'autre, et l'on voulait pouvoir mettre à jour les images de référence sur le poste de développement, sans passer par la CI.

Les images de référence ont été produites dans WSL2, puis les mêmes tests ont tourné contre elles sur quatre configurations de la CI :
- Linux sur le runner `ubuntu-latest` (lavapipe) ;
- Linux dans un conteneur `ubuntu:26.04` (lavapipe, même version de Mesa que WSL2) ;
- macOS (Metal) ;
- Windows (WARP, rendu logiciel de Direct3D 12).

## Conditions

| | |
|---|---|
| Date | 2026-11-26 |
| Code | branche `captures-images`, commit `b737363` (exécution 45775469867 du workflow temporaire `snapshots-trial.yml`) |
| Bibliothèques | egui 0.37.0, egui_kittest 0.37.0 (features `eframe`, `wgpu`, `snapshot`), egui-wgpu 0.37.0, wgpu 30.0.2, dify 0.8.0 (comparaison d'images) |
| Rust | 1.99 (`rust-toolchain.toml`), compilation de test (profil `dev`) |
| Références | produites dans WSL2 (Ubuntu 26.04.1, Mesa 26.0.8-1ubuntu0.3, LLVM 21.1.8) par `UPDATE_SNAPSHOTS=1 cargo test -p ortelia-ui-egui` |
| Tolérances | `kittest.toml` : seuil par pixel 0,6, aucun pixel fautif toléré |
| Rendu | 640 × 400 pixels, échelle 1, thème sombre, polices intégrées à egui |

L'adaptateur est celui que choisit egui_kittest : il préfère les rendus logiciels (`DeviceType::Cpu`). Un test de la scène d'essai l'affichait dans le journal.

| Configuration | Image du runner | Système | Adaptateur choisi |
|---|---|---|---|
| WSL2 (poste de développement) | — | Ubuntu 26.04.1 | llvmpipe (LLVM 21.1.8, 256 bits), Vulkan, Mesa 26.0.8-1ubuntu0.3 |
| `ubuntu-latest` | `ubuntu-26.04`, 20261116.88.1 | Ubuntu 26.04.1 LTS | llvmpipe (LLVM 21.1.8, 256 bits), Vulkan, Mesa 26.0.8-1ubuntu0.3 |
| Conteneur `ubuntu:26.04` | `ubuntu-26.04`, 20261122.9.1 | Ubuntu 26.04.1 LTS | llvmpipe (LLVM 21.1.8, 256 bits), Vulkan, Mesa 26.0.8-1ubuntu0.3 |
| `macos-latest` | `macos-26-arm64`, 20261117.0516.1 | macOS 26.7.2 (25G332) | « Apple Paravirtual device », Metal (GPU virtualisé, aucun rendu logiciel disponible) |
| `windows-latest` | `windows-2025-vs2026`, 20261116.315.1 | Windows Server 2025 Datacenter (10.0.26100) | « Microsoft Basic Render Driver » (WARP), Direct3D 12 |

Le runner `ubuntu-latest` est passé à Ubuntu 26.04 : il a donc la même version de Mesa que WSL2, ce que la proposition ne supposait pas.

## Captures mesurées

- `main_window` : la fenêtre principale actuelle, rendue par l'application egui complète (un titre centré).
- `trial_scene`, réservée à l'essai : une scène qui rend ce que rendront les tables et les formulaires. Elle comprend :
  - des textes de plusieurs tailles, dont du texte à chasse fixe et des lettres accentuées ;
  - une grille rayée de 10 lignes sur 4 colonnes, avec des nombres et des dates ;
  - une case à cocher, un bouton, un champ de texte avec son étiquette et des séparateurs ;
  - une zone défilante, avec sa barre de défilement pleine et une ligne coupée par le bord.

La stabilité est mesurée par deux rendus de suite de `trial_scene`, dans deux `Harness`, comparés pixel par pixel.

## Mesures

Pixels fautifs par seuil, tirés des messages d'échec d'egui_kittest (série `failing_pixels_by_threshold`). Une comparaison réussie ne donne pas cette série : elle signifie qu'aucun pixel ne dépasse le seuil 0,6.

| Configuration | `main_window` | `trial_scene` | Stabilité (pixels différents entre deux rendus) |
|---|---|---|---|
| `ubuntu-latest` | réussie : 0 pixel au-delà de 0,6 | réussie : 0 pixel au-delà de 0,6 | 0 |
| Conteneur `ubuntu:26.04` | réussie : 0 pixel au-delà de 0,6 | réussie : 0 pixel au-delà de 0,6 | 0 |
| `macos-latest` | réussie : 0 pixel au-delà de 0,6 | **échec** : 2 pixels au-delà de 0,6 | 0 |
| `windows-latest` | réussie : 0 pixel au-delà de 0,6 | **échec** : 2 pixels au-delà de 0,6 | 0 |

Détail de l'échec de `trial_scene`, identique sous macOS et sous Windows :

| Seuil | 0 | 0,1 | 0,2 | 0,4 | 0,6 | 1,0 | 1,5 | 2,0 | 3,0 | 5,0 | 10,0 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Pixels fautifs | 1 548 | 1 548 | 1 548 | 1 548 | 2 | 2 | 2 | 2 | 0 | 0 | 0 |

Environ 1 500 pixels du bord des caractères diffèrent très peu : c'est l'anticrénelage, calculé autrement par Metal et WARP que par lavapipe. Les 2 pixels plus éloignés sont à la ligne coupée de la zone défilante. Le titre seul de `main_window` ne montre pas cet écart : seule une scène plus riche le révèle.

Durées des jobs, sans cache Cargo (le workflow d'essai n'en utilisait pas) :

| Configuration | Job | Mise en place (paquets, conteneur) | Compilation des tests | Tests de capture |
|---|---|---|---|---|
| `ubuntu-latest` | 2 min 0 s | 26 s (lavapipe) | 68 s | 5 s |
| Conteneur `ubuntu:26.04` | 4 min 41 s | 3 min 3 s (paquets, rustup) | 71 s | 2 s |
| `macos-latest` | 48 s | — | 35 s | 2 s |
| `windows-latest` | 2 min 1 s | — | 95 s | 4 s |

## Analyse, selon les critères dans leur ordre

1. **Une référence produite dans WSL2 passe-t-elle dans la CI, avec un seuil d'au plus 0,6 et au plus 10 pixels fautifs ?** Oui, sur les deux configurations Linux : aucun pixel fautif au seuil 0,6, sur les deux captures, avec `max_failed_pixels = 0`. La mise à jour des références reste donc locale. Sous macOS et Windows, la scène d'essai échoue avec 2 pixels fautifs. Elle passerait avec `max_failed_pixels = 2` ou un seuil de 3,0, mais les références devraient alors être produites, ou au moins vérifiées, par la CI, et l'écart grandira avec le nombre de widgets rendus.
2. **Stabilité** : deux rendus de suite donnent la même image sur les quatre configurations. Le critère ne départage pas.
3. **Coût** : sur `ubuntu-latest`, il suffit d'installer `mesa-vulkan-drivers` (26 s), et les tests de capture prennent quelques secondes. Le conteneur ajoute 3 min de mise en place à chaque exécution, sans rien apporter de plus, puisque le runner a déjà la même version de Mesa. Mettre à jour une référence ne demande qu'une commande dans WSL2 (`UPDATE_SNAPSHOTS=1 cargo test -p ortelia-ui-egui`), puis de regarder l'image avant de la versionner.

## Choix

**Lavapipe sous Linux, dans le job `checks` de la CI, avec des références produites dans WSL2.** C'est le résultat attendu par la proposition : les spécifications et la conception restent valables. Les tolérances restent celles d'egui_kittest par défaut (`threshold = 0.6`, `max_failed_pixels = 0`), sans section propre à un système.

- L'image du runner Linux des jobs qui exécutent les tests est fixée à `ubuntu-26.04`, et non plus `ubuntu-latest`. Le jour où `ubuntu-latest` passera à une autre version d'Ubuntu, donc de Mesa, les captures pourraient échouer sans aucune modification du code. Avec l'image fixée, ce changement deviendra une décision : changer d'image dans la CI et de version dans WSL2 en même temps, puis régénérer les références.
- Écartés :
  - le conteneur `ubuntu:26.04` : même rendu, 3 min de plus à chaque exécution ;
  - macOS et Windows : l'anticrénelage y diffère de celui de WSL2, et les références ne pourraient plus être produites sur le poste de développement.
- Sous Windows (job `checks`), les tests de capture sont compilés et signalés comme ignorés.

Après l'essai, la scène d'essai (`crates/ortelia-ui-egui/src/snapshot_trial.rs`), sa référence `trial_scene.png` et le workflow `.github/workflows/snapshots-trial.yml` ont été supprimés. On les retrouve dans le commit `b737363`, celui de l'exécution mesurée.

À surveiller :
- une mise à jour de Mesa dans WSL2 (`apt upgrade`) ou sur le runner peut décaler le rendu. Si les captures échouent en local alors que la CI passe, ou l'inverse, comparer d'abord les versions de Mesa (`dpkg-query -W mesa-vulkan-drivers`) à celles de ce compte rendu ;
- une montée de version d'egui change le rendu de toutes les captures : `UPDATE_SNAPSHOTS=force`, puis revue des images avant de les versionner.
