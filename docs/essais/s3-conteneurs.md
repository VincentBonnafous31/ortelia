# Essai S3 : conteneurs de la fenêtre avec egui_tiles

Résultats de l'essai S3 du changement OpenSpec `conteneur-split`, premier des trois changements de l'incrément 3 (`docs/feuille-de-route-annuaire.md`). Il s'agissait de vérifier qu'egui_tiles, bibliothèque de tuiles pour egui, convient pour dessiner les conteneurs de la fenêtre. Le premier de ces conteneurs est le split, qui partage la fenêtre entre la table de l'annuaire et une vue vide. Plus tard viendront les piles d'onglets et le glisser-déposer des vues.

L'essai ne passe pas par un prototype jetable : la disposition est écrite directement dans les crates, et chaque critère est vérifié par un test du dépôt.

## Conditions

| | |
|---|---|
| Date | 2026-11-27 |
| Code | branche `conteneur-split` |
| Bibliothèques | egui 0.37.0, egui_tiles 0.18.0 (sans la feature `serde`), egui_kittest 0.37.0 (features `eframe`, `wgpu`, `snapshot`) |
| Rust | 1.99 (`rust-toolchain.toml`), compilation de test (profil `dev`) |
| Captures | WSL2 (Ubuntu 26.04), lavapipe, Mesa 26.0.8-1ubuntu0.3 ; 640 × 400 pixels, échelle 1, thème sombre |
| Disposition essayée | `examples/annuaire/app.json` au format 0.5 : split `principal` horizontal, poids `[65, 35]`, table `personnes` à gauche, vue vide `proprietes` à droite |

egui_tiles n'ajoute qu'un paquet au `Cargo.lock` : ses dépendances (ahash, itertools, log) étaient déjà dans l'arbre de l'application. Il n'est utilisé que par `ortelia-ui-egui` ; la présentation ne le connaît pas (`cargo xtask check-layers`). Sa documentation n'est pas dans Context7 : l'API a été lue dans les sources de la crate.

## Critères et tests

Les tests de l'interface sont dans `crates/ortelia-ui-egui/src/layout/tests.rs`, sauf mention contraire. Tous passent.

| Critère | Résultat | Vérifié par |
|---|---|---|
| Split sans onglets ni glisser-déposer | atteint | `no_tabs` : aucun onglet, aucun texte qui porte le nom d'une vue ou d'un split. `view_dragged` : la vue vide glissée jusqu'au bord gauche ne bouge pas, la table garde sa place et sa largeur, aucune intention de séparation. |
| Parts restaurées exactement | atteint | Présentation (`crates/ortelia-presentation/src/session_tests.rs`) : `layout_restored`, `file_without_containers`, `unknown_split`, `split_now_a_view`, `split_with_a_new_child`, `saved_parts_checked`. Interface : `separator_found_again` glisse la séparation de 100 points dans l'application entière, ferme la fenêtre (`on_exit`), rouvre l'annuaire avec état de session et retrouve une première part de 0,55 et une table de 550 points. |
| Split vertical dans un split horizontal | atteint | `nested_vertical_split` : la table `a` à gauche sur la moitié de la largeur et toute la hauteur, `b` à droite en haut, `c` à droite en bas, trois fois plus haute (poids `[1, 3]`). |
| Table dans une tuile | atteint | Les tests de la table de `crates/ortelia-ui-egui/src/tests.rs` (`table_of_the_directory_is_displayed`, `click_and_shift_click`, `filter_typed`, `filter_cleared`, `header_dragged_and_dropped`, `column_resized`, `closed_then_opened_again`, `idle_table_emits_no_intent`) passent sans changement de leurs vérifications : seul le harnais dessine maintenant la fenêtre par ses tuiles. |
| Séparation déplacée dans un test sans écran | atteint | `separator_dragged` : 100 points vers la gauche dans une fenêtre de 1000 points, intention du split `principal` de première part 0,55, table de 550 points. `separator_double_clicked` : parts 0,5 et 0,5. `no_separator_moved` : aucune intention en dix images. `window_widened` : de 1000 à 1500 points, la table passe à 975 points sans intention. `table_of_the_directory_on_the_left` : 650 points sur 1000. `empty_view_draws_nothing` : la tuile de la vue vide ne contient aucun texte ni widget. |
| Capture identique sous lavapipe | atteint dans WSL2 | `snapshot_tests::main_window`, nouvelle référence `crates/ortelia-ui-egui/tests/snapshots/main_window.png` : la table sur 65 % de la largeur, la séparation sur toute la hauteur, la vue vide à droite. `open_error.png` n'a pas changé. La CI la comparera à sa première exécution sur la pull request. |

## Écarts rencontrés

1. **La séparation a un nœud d'accessibilité, sans nom.** La conception supposait qu'elle n'en avait pas. Elle en a un, de rôle `Unknown` : c'est la zone qui réagit au pointeur, 6 points de large sur toute la hauteur. Sa position reste calculée depuis le bord droit de la table ; le nœud sert à trouver le conteneur du split dans l'arbre, et donc la tuile de la vue vide.
2. **egui_tiles signale un redimensionnement à chaque image tant que la séparation est tenue**, même immobile. L'interface émettait alors une intention à chaque image, et l'application redemandait une image après chaque intention : elle ne se stabilisait plus tant que le bouton restait enfoncé. Le test d'aller-retour dans l'application entière l'a montré (les tests de dessin seul ne redemandent pas d'image). L'interface compare désormais les parts d'egui_tiles à celles de la présentation, comme pour les largeurs de colonnes, et n'émet une intention que si elles diffèrent.
3. **Marges.** Le panneau central d'egui a une marge de 8 points : les parts s'appliquaient à la largeur de la fenêtre moins 16 points, soit 639 points sur 1000 pour la table au lieu de 650. La marge du panneau est retirée autour des tuiles, et chaque vue a sa propre marge de 8 points : la table reste à la même distance des bords, et la séparation va du haut en bas de la fenêtre. De son côté, un harnais d'egui_kittest qui dessine une fonction (et non une application eframe) ajoute une marge extérieure de 8 points : les tests de dessin passent donc par un harnais d'application, dont l'état enregistre les intentions au lieu de les transmettre.
4. **Les colonnes de la table qui dépassent ont des nœuds au-delà de la séparation.** Elles sont rognées par le défilement horizontal de la table, mais l'arbre d'accessibilité ne le dit pas. « Rien dans la moitié droite » ne se vérifie donc pas par l'écran mais par l'arbre : le sous-arbre de la tuile vide ne contient que des conteneurs sans texte, alors que celui de la table contient son champ de filtre.
5. **Simplifications.** Par défaut, egui_tiles fusionne deux splits imbriqués de même direction et supprime un split d'un seul enfant : la correspondance entre ses tuiles et nos splits serait perdue. Elles sont désactivées (`SimplificationOptions::OFF`).
6. **Convention des parts.** egui_tiles donne 1 à chaque enfant par défaut, et son double-clic fait la moyenne des deux voisins d'une séparation. Les parts de la présentation (de somme 1) lui sont données multipliées par le nombre d'enfants : même total qu'à l'ordinaire, et le double-clic donne bien 0,5 et 0,5.
7. **`egui::Id::new` est obsolète dans egui 0.37** : l'identifiant de l'arbre est `Id::unique("ortelia-layout")`, comme celui de la table.
8. **Taille minimale.** egui_tiles n'impose sa taille minimale de tuile (32 points) que pendant un glisser : une part minuscule écrite à la main dans `app.session.json` donnerait une tuile de quelques points, que le premier glisser répare. Accepté, voir les risques de la conception.
9. **Glisser-déposer d'en-têtes entre deux tables.** La charge du glisser d'un en-tête ne portait que la position de sa colonne, alors qu'egui la partage dans toute la fenêtre : avec deux tables, un en-tête de l'une lâché sur un en-tête de l'autre réordonnait la seconde. Elle porte maintenant l'identifiant egui de sa table (`header_dropped_on_another_table`). L'annuaire, qui n'a qu'une table, n'était pas touché.

## Décision

**egui_tiles est retenu** pour les conteneurs de la fenêtre. Tous les critères sont atteints sans contournement de la bibliothèque : les écarts portent sur l'usage (comparaison des parts avec la présentation avant d'émettre, marges, simplifications désactivées) et sur la manière de tester. Le repli prévu, un split dessiné pour Ortelia, n'est pas nécessaire.

L'arbre d'egui_tiles est construit une fois, à l'ouverture, depuis la disposition de la présentation, qui reste la référence : l'interface ne fait que rapporter les déplacements de séparation par des intentions. Les piles d'onglets et le glisser-déposer des vues, prévus plus tard, s'appuieront sur les conteneurs `Tabs` et le glisser d'egui_tiles, aujourd'hui désactivés (`is_tile_draggable` renvoie faux, aucun conteneur `Tabs` n'est créé).
