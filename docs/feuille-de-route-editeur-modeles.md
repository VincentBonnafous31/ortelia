# Feuille de route — Application 2 « Éditeur de modèles »

*2026-11-24. Proposé, à valider. Vue d'ensemble et principes : [`feuille-de-route.md`](feuille-de-route.md).*

Ses données sont des fichiers de modèle ; son modèle est le métamodèle (`examples/model-editor/metamodel.model.json`). Son `app.json` reste écrit à la main (ou généré par Claude).

Découpage à affiner avant de commencer, comme pour l'application 1.

---

## Incrément 7 — Arbre et actions

- Widget arbre (nœuds et libellés issus de la présentation par classe).
- Actions de menu contextuel : créer un package, une classe, un énuméré, un type de donnée, un attribut, une référence ; supprimer. Les actions possibles se déduisent du modèle (contenances de l'objet sélectionné).
- Panneau de propriétés : le formulaire généré suit la **classe réelle** de l'objet sélectionné.
- Nouveau / Ouvrir / Enregistrer / Enregistrer sous : le fichier de données est choisi par l'utilisateur, et non plus fixé dans `app.json`.

**Fini quand** : on crée de zéro le modèle de l'annuaire, et on l'enregistre.

---

## Incrément 8 — Références et validation

- Widgets de référence par identifiant : `refCombo` (type d'un attribut, super-classe), `refPicker`.
- `valueList` (littéraux d'un énuméré), `optional`.
- Contraintes du métamodèle : identifiants valides, bornes cohérentes, unicité des noms, références introuvables après une suppression.

**Fini quand** : un modèle édité dans l'application 2 est directement utilisé par l'application 1 (boucle complète).
