# Proposition : modele-et-donnees

## Why

L'essai S1 a fixé la disposition du Store, mais pas encore la façon dont une application vit avec ses données. Une application Ortelia a un seul arbre d'objets pour toute la session, sous une racine. Au démarrage, des fichiers alimentent chacun une branche de cet arbre ; plus tard, des appels REST en alimenteront d'autres ; pendant l'exécution, des actions ajoutent, suppriment ou modifient des branches. Chaque modification doit être tracée, pour que la présentation sache qu'un cache est périmé et, plus tard, pour l'indicateur « modifié » et l'enregistrement.

Ce changement est le second et dernier de l'**incrément 1** de `docs/feuille-de-route-annuaire.md`. Il remplit les critères « Fini quand » qui restent :
- le modèle, `app.json` et `personnes.json` se chargent, et des fichiers fautifs produisent des erreurs claires ;
- les tests de création, modification, suppression et notification passent.

Le premier critère (mesures de S1 consignées et choix du stockage arrêté) a été rempli par le changement `stockage-dynamique`.

## What Changes

- **Un Store brut, un Document qui le modifie** (`ortelia-model`).
  - Le `Document` possède le Store de l'application. Il tient un numéro de révision et un journal de transactions : valeur modifiée (ancienne et nouvelle valeur), objet créé, objet supprimé, branche remplacée.
  - La présentation compare la révision et lit les changements survenus depuis une révision donnée ; il n'y a pas d'observateurs.
  - **BREAKING** : les méthodes de modification du `Store` ne sont plus accessibles hors de `ortelia-model`. Toute modification passe par le `Document`.
- **Un fragment, greffé sur une branche** (`ortelia-model`).
  - Un lecteur remplit toujours un `Fragment` neuf : des objets détachés, conformes au type de la branche visée. Le fragment peut être lu dans un autre thread.
  - L'intégration remplace le contenu de la branche par le fragment, en une seule transaction. Elle absorbe le fragment sans recopier ses objets ni ses textes. Seuls les objets de la branche remplacée disparaissent : les identifiants des autres branches restent valables.
  - **BREAKING** : le constructeur et le lecteur JSON produisent un fragment pour une branche, et non plus un Store entier.
- **Lecture de `app.json`** (`ortelia-config`), format 0.3 (**BREAKING**) :
  - `model` (fichiers et classe de la racine) et `sources` (identifiant, format, fichier, branche visée par `target`) ;
  - les autres sections (libellés, présentation, fenêtre) sont retirées de l'exemple et reviendront avec les incréments 2 et suivants ;
  - les erreurs sont claires : clé inconnue, version non prise en charge, classe ou référence introuvable ;
  - les cibles sont vérifiées contre le modèle avant toute lecture de données. Une cible est un chemin de références de contenance, ou `/` pour la racine. Chaque fichier alimente sa propre branche, et deux sources ne peuvent viser ni la même branche ni deux branches imbriquées.
- **Ouverture d'une application** (`ortelia-service`).
  - La racine est créée d'après `model.root`, puis le squelette déduit des cibles : `/annuaire` pour la cible `/annuaire/personnes`.
  - Chaque source est lue, puis intégrée dans sa branche. Une source qui échoue n'empêche pas l'ouverture : sa branche reste vide et son erreur est rapportée.
  - Le lien entre chaque source et sa branche est conservé, pour l'enregistrement et l'indicateur « modifié » des incréments suivants.
- **Validation des données par le modèle** :
  - les erreurs sont toutes rassemblées, avec un plafond et le total, chacune avec son chemin depuis la racine de l'application ;
  - un attribut obligatoire absent ne refuse pas le chargement : l'objet est chargé avec un diagnostic ;
  - le message destiné à l'utilisateur peut citer la valeur fautive, le journal jamais.
- **Métamodèle** :
  - la racine de l'annuaire devient une classe ordinaire du modèle, `Racine`, qui contient l'`Annuaire` ;
  - `Personne` gagne un attribut par type de donnée (entier, décimal, booléen, énuméré, date-heure) et un texte long, et les données d'exemple suivent ;
  - les bornes des attributs et des références sont vérifiées, les littéraux en double et les versions de format inconnues sont refusés, et les erreurs de lecture citent le fichier.
- **Store** : parcours d'un sous-arbre, et suppression en lot qui ne parcourt la liste qu'une fois.
- **JSON Schema** de `app.json` et des `*.model.json`, générés depuis les types Rust par `cargo xtask gen-schemas`, versionnés dans `schemas/`, et vérifiés à jour par la CI. Une clé `$schema` est acceptée dans ces fichiers, pour l'autocomplétion dans tout éditeur.

### Hors périmètre

- Les modes d'intégration autres que le remplacement : l'ajout (avec les actions d'import) et la fusion par identifiant métier (avec la source REST).
- La lecture en tâche de fond, avec progression et annulation : un fragment est seulement transférable entre threads, et les sources sont lues l'une après l'autre.
- L'annulation d'une suppression, et l'annuler / rétablir en général : une suppression libère les objets.
- La récupération de la mémoire dans un Store qui vit longtemps : un texte remplacé ou supprimé garde sa place dans l'arène, et un objet supprimé laisse une case vide. Sans conséquence pour l'annuaire, chargé une fois ; à traiter avec les rafraîchissements REST.
- Les bornes des références, au-delà de leur cohérence (une liste obligatoire vide n'a pas de diagnostic), les contraintes, l'identifiant métier, les valeurs par défaut et les unités.
- L'enregistrement des fichiers et l'indicateur « modifié » (incrément 5), même si le lien entre source et branche est conservé dès maintenant.
- Le squelette initial déclaré dans `app.json` (valeurs initiales, sous-classe d'un objet intermédiaire) : les objets intermédiaires prennent le type déclaré de leur référence.
- Le chargement d'`app.json` par l'exécutable `ortelia` : l'ouverture d'une application est testée sans interface et viendra dans l'exécutable avec la table (incrément 2).
- La validation des fichiers d'exemple contre leur JSON Schema dans les tests.

## Capabilities

### New Capabilities

- `change-tracking` : le Document, seule voie de modification des données d'une application ; transactions, révision, journal des changements, intégration d'un fragment dans une branche.
- `app-config` : lecture et vérification de `app.json` (`model`, `sources`), cibles vérifiées contre le modèle, erreurs claires.
- `app-context` : ouverture d'une application : racine et squelette, lecture et intégration des sources, état de chaque source, lien entre source et branche.
- `config-schemas` : JSON Schema des fichiers de configuration, générés depuis les types Rust, clé `$schema`, vérification qu'ils sont à jour.

### Modified Capabilities

- `metamodel` : modèle de l'annuaire avec sa racine ; bornes vérifiées ; littéraux en double et versions inconnues refusés ; erreurs qui citent le fichier ; clé `$schema` acceptée.
- `object-store` : parcours d'un sous-arbre ; suppression en lot.
- `data-loading` : construction d'un fragment pour une branche ; fichier JSON qui contient un objet ou un tableau selon la branche ; toutes les erreurs rassemblées, avec un plafond et le total ; diagnostic d'attribut obligatoire absent ; valeur fautive absente du journal.
- `continuous-integration` : vérification que les JSON Schema générés sont à jour.

## Impact

- **Code** :
  - `ortelia-model` : `Document`, transactions et journal, `Fragment` et absorption, constructeur qui produit un fragment et rassemble les erreurs, métamodèle complété, méthodes de modification du `Store` réservées à la crate ;
  - `ortelia-config` : types et vérification de `app.json` ;
  - `ortelia-service` : lecteur JSON qui vise une branche et continue après une erreur, ouverture d'une application ;
  - `xtask` : commande `gen-schemas`.
- **Code existant adapté** : les tests du Store passent dans la crate (`src/store/`), car ils appellent des méthodes qui ne sont plus publiques. Les mesures de modification de l'exemple `s1` et le test de `gen-data` passent par le `Document`.
- **Fichiers d'exemple** : `examples/annuaire/` (racine et attributs ajoutés au modèle, données complétées, `app.json` en format 0.3) ; clé `$schema` ajoutée aux fichiers de modèle de `examples/`.
- **Nouveaux fichiers** : `schemas/app.schema.json` et `schemas/model.schema.json`.
- **Outillage** : la CI (`.github/workflows/ci.yml`) et `tools/check.sh` vérifient que les schémas sont à jour.
- **Documents** :
  - `docs/architecture-couches.md`, § 9 : le schéma, la décision D30 (la greffe d'une branche remplace l'échange du Store entier), et de nouvelles décisions sur le Document et les sources ;
  - `docs/feuille-de-route-annuaire.md` : incrément 1 ;
  - `CLAUDE.md` : état, arborescence, commande `gen-schemas`.
- **Dépendances externes** : schemars 1.2.3, seulement derrière une feature activée par `xtask` : l'exécutable ne l'embarque pas.
