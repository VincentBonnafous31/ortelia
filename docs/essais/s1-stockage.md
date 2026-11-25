# Essai S1 : stockage dynamique des objets

Résultats de l'essai S1 de l'incrément 1 (`docs/feuille-de-route-annuaire.md`), changement OpenSpec `stockage-dynamique`. Deux dispositions en mémoire ont été comparées derrière la même API et la même suite de tests. Elles partagent l'arène des chaînes, l'encodage des dates et les contrôles :

- **V1, un bloc par objet** (`BoxedStore`) : chaque objet a son propre tableau de cases typées de 16 octets ;
- **V2, un tas d'objets** (`HeapStore`) : les objets sont des enregistrements contigus, rangés à la suite dans des blocs de 8 Mio. Leurs cases de 8 octets n'ont pas d'étiquette : c'est la classe qui les décrit, et un masque distingue « non renseigné » de 0.

Deux points de repère encadrent les mesures : le **plancher** (lecture JSON par serde sans rien construire, `IgnoredAny`) et **V0** (`serde_json::Value`).

## Conditions

| | |
|---|---|
| Date | 2026-11-25 |
| Processeur | AMD Ryzen 7 7800X3D (8 cœurs, 16 fils) ; chaque cas tourne sur un seul fil |
| Mémoire | 15,5 Gio attribués à WSL2 |
| Système | WSL2, Ubuntu, noyau 6.18.58.1-microsoft-standard-WSL2 |
| Rust | 1.99.1, compilation release (profil par défaut) |
| Code | branche `stockage-dynamique`, sur le commit `f5d6f81` |
| Fichiers | système de fichiers Linux (`~/s1/`), dans le cache de pages : `all` lit le fichier une fois avant les cas |

Chaque cas tourne dans un processus neuf. Pour les mesures A et E, on fait 5 passes et on retient la médiane ; les tirages au hasard ont une graine fixe.

Pour reproduire :

```bash
cargo xtask gen-data --size 500M --text 1 --out ~/s1/reseau-texte-500M.json
cargo xtask gen-data --size 500M --text 0 --out ~/s1/reseau-numerique-500M.json
cargo xtask gen-data --size 500M --late-type 0.1 --out ~/s1/reseau-tardif-500M.json
cargo xtask gen-data --size 100M --out ~/s1/reseau-100M.json
cargo run --release -p ortelia-service --example s1 -- all ~/s1/reseau-texte-500M.json --only floor,v1,v2
cargo run --release -p ortelia-service --example s1 -- all ~/s1/reseau-100M.json
```

## Fichiers

Modèle d'essai : `examples/reseau-meteo/reseau.model.json`. Graine 1 pour tous les fichiers.

| Classe | texte 500 Mo | numérique 500 Mo | `_type_` tardif 500 Mo | 100 Mo |
|---|---:|---:|---:|---:|
| Station | 4 067 | 4 925 | 4 071 | 794 |
| Thermometre | 5 915 | 7 150 | 6 013 | 1 174 |
| Anemometre | 5 962 | 7 256 | 5 987 | 1 212 |
| Pluviometre | 5 998 | 7 102 | 5 802 | 1 176 |
| Camera | 4 615 | 5 392 | 4 463 | 901 |
| Releve | 3 890 688 | 4 671 680 | 3 890 596 | 778 489 |
| Lacune | 120 830 | 144 395 | 119 913 | 24 114 |
| Annotation | 40 154 | 48 146 | 40 082 | 8 101 |
| Maintenance, Panne, Etalonnage | 12 129 | 14 482 | 12 269 | 2 419 |
| ZoneAlerte | 2 794 | 2 673 | 2 725 | 510 |
| Polygone / Cercle | 2 226 / 568 | 2 151 / 522 | 2 184 / 541 | 404 / 106 |
| Maille | 14 484 | 12 929 | 14 837 | 3 998 |
| Point | 2 336 542 | 2 340 389 | 2 334 325 | 465 871 |
| **Total** (avec la racine) | **6 446 973** | **7 269 193** | **6 443 809** | **1 289 270** |

- **texte** : `--text 1` (descriptions, comptes rendus et annotations).
- **numérique** : `--text 0`, sans texte long, donc environ 13 % d'objets de plus à taille égale.
- **tardif** : `--text 1 --late-type 0.1`, où 10 % des objets des listes polymorphes ont leur `_type_` après leur premier attribut.
- **100 Mo** : profil texte, pour V0, qui prend environ 11 fois la taille du fichier en mémoire.

## Mesures

V2/V1 est le rapport des deux valeurs. Sous 1, V2 est meilleure (plus rapide, ou moins de mémoire). Les fichiers de 500 Mo sont abrégés T (texte), N (numérique) et L (`_type_` tardif).

### Accès et navigation (critère 1)

| Code | Mesure | Unité | V1 T | V2 T | V1 N | V2 N | V1 L | V2 L | V2/V1 (T, N, L) |
|---|---|---|---:|---:|---:|---:|---:|---:|---|
| A1 | lecture au hasard d'un décimal | ns/accès | 64,9 | 63,2 | 54,2 | 62,4 | 49,9 | 59,1 | 0,97 ; 1,15 ; 1,18 |
| A2 | parcours de tout l'arbre, un attribut par objet | ns/objet | 16,3 | 15,2 | 16,3 | 15,0 | 16,3 | 15,3 | 0,93 ; 0,92 ; 0,94 |
| A3 | `numero_serie` sur toutes les listes polymorphes | ns/objet | 68,8 | 85,1 | 92,2 | 91,3 | 81,8 | 80,6 | 1,24 ; 0,99 ; 0,99 |
| A4 | extraction de tous les sommets en `f64` | ms/million | 8,85 | 11,6 | 8,73 | 11,6 | 9,15 | 11,7 | 1,31 ; 1,33 ; 1,28 |
| A5 | remontée jusqu'à la racine depuis un `Point` | ns/niveau | 19,8 | 28,4 | 18,4 | 26,9 | 20,2 | 25,5 | 1,43 ; 1,46 ; 1,26 |
| A6 | fenêtre de table : 50 stations × 8 attributs en texte | µs | 33,1 | 35,6 | 33,8 | 33,6 | 33,4 | 37,5 | 1,08 ; 0,99 ; 1,12 |
| A7 | modifier un coin, remonter, réextraire sa zone | µs | 20,7 | 29,4 | 13,2 | 18,4 | 15,9 | 20,2 | 1,42 ; 1,39 ; 1,27 |

En valeur absolue, l'extraction de tous les sommets (A4, environ 2,34 millions de sommets) prend 20 ms avec V1 et 27 ms avec V2.

### Mémoire (critère 2)

| Code | Mesure | Unité | V1 T | V2 T | V1 N | V2 N | V1 L | V2 L | V2/V1 (T, N, L) |
|---|---|---|---:|---:|---:|---:|---:|---:|---|
| M1 | pic, lecture en flux | Mio | 733 | 424 | 756 | 404 | 732 | 424 | 0,58 ; 0,53 ; 0,58 |
| M1 | pic, lecture depuis un tampon | Mio | 1 232 | 923 | 1 256 | 903 | 1 232 | 924 | 0,75 ; 0,72 ; 0,75 |
| M2 | après chargement, rapportée au fichier | × | 1,47 | 0,85 | 1,51 | 0,81 | 1,46 | 0,85 | 0,58 ; 0,53 ; 0,58 |
| M3 | allocations pendant le chargement | millions | 6,71 | 0,26 | 7,57 | 0,30 | 12,86 | 6,42 | 0,04 ; 0,04 ; 0,50 |
| M3 | octets alloués pendant le chargement | Mio | 985 | 542 | 976 | 523 | 1 535 | 1 092 | 0,55 ; 0,54 ; 0,71 |

En lecture en flux, le pic est égal à la mémoire après chargement : la lecture n'ajoute aucun surcoût temporaire. Depuis un tampon, le pic augmente de la taille du fichier (500 Mio).

### Chargement (critère 3)

| Code | Mesure | Unité | plancher T | V1 T | V2 T | V1 N | V2 N | V1 L | V2 L | V2/V1 (T, N, L) |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| C1 | en flux | ms | 929 | 3 136 | 2 809 | 3 166 | 2 958 | 3 504 | 3 178 | 0,90 ; 0,93 ; 0,91 |
| C1 | depuis un tampon (lecture du fichier comprise) | ms | 659 | 2 663 | 2 383 | 2 805 | 2 430 | 3 129 | 2 678 | 0,89 ; 0,87 ; 0,86 |

La lecture du fichier en mémoire (C0) prend environ 205 ms. Le plancher montre ce que coûte serde_json seul : la construction des objets représente les deux tiers environ du temps de chargement.

### Modification

| Code | Mesure | Unité | V1 T | V2 T | V1 N | V2 N | V1 L | V2 L |
|---|---|---|---:|---:|---:|---:|---:|---:|
| E1 | `set` au hasard (décimal, entier, texte) | ns/set | 80,8 | 90,7 | 72,6 | 87,5 | 68,5 | 83,5 |
| E1 | croissance de la mémoire pendant 5 millions de `set` | Mio | 39,4 | 39,4 | 39,4 | 39,4 | 39,4 | 39,4 |
| E2 | insérer puis supprimer 1 000 objets au milieu de 100 000 | ms | 28,9 | 16,9 | 29,0 | 16,6 | 29,1 | 16,8 |
| E3 | supprimer une station et son sous-arbre | ms | 0,019 | 0,006 | 0,022 | 0,008 | 0,029 | 0,008 |
| E4 | créer 1 000 mailles de 4 coins | ms | 0,286 | 0,290 | 0,283 | 0,281 | 0,298 | 0,294 |

La croissance de E1 vient de l'arène des chaînes, qui ne récupère pas la place des textes remplacés. Elle est identique pour les deux Stores.

Depuis le changement « modèle et données », l'exemple `s1` modifie les données par les transactions du document (une par passe de mesure, et une par modification pour A7) : les mesures E et A7 comptent désormais le journal des changements, anciennes valeurs comprises. Les chiffres de ce document restent ceux de l'essai, mesurés directement sur le Store.

### Point de repère V0, sur 100 Mo

| Mesure | plancher | V0 | V1 | V2 |
|---|---:|---:|---:|---:|
| C1 en flux (ms) | 189 | 1 188 | 608 | 558 |
| C1 depuis un tampon (ms) | 120 | 1 015 | 489 | 441 |
| M2 après chargement (Mio) | 3 | 1 066 | 149 | 87 |
| M2 rapportée au fichier | 0,03 | 10,7 | 1,49 | 0,87 |

Un arbre `serde_json::Value` prend 7 fois la mémoire de V1 et 12 fois celle de V2, et il se charge deux fois plus lentement.

## Analyse

**Accès et navigation.** V1 est plus rapide pour suivre les liens de navigation : remontée vers le conteneur (A5, 1,26 à 1,46 fois), extraction des sommets (A4, environ 1,3 fois), modification puis réextraction d'une zone (A7, 1,27 à 1,42 fois). Les deux variantes sont à égalité sur le parcours complet (A2, où V2 a même 6 à 8 % d'avance grâce à la contiguïté), sur l'attribut hérité dans les listes polymorphes (A3, sauf un écart de 1,24 sur le profil texte) et sur la fenêtre de table (A6). En lecture au hasard (A1), les défauts de cache dominent : 50 à 65 ns par accès dans les deux cas.

D'où vient le surcoût de V2 ? Pour atteindre un objet, il faut lire deux mots qui dépendent l'un de l'autre : l'emplacement dans la table, puis l'en-tête de l'enregistrement. V1 n'en lit qu'un, l'entrée de l'objet, qui contient déjà la classe et le conteneur. Sur un accès au hasard, cela fait deux défauts de cache au lieu d'un. C'est le prix de la table des emplacements, qui permettra de compacter le tas sans changer les identifiants. La remontée vers le conteneur (A5), qui ne fait rien d'autre que ces sauts, y est la plus sensible. Le surcoût mémoire de V1 est structurel lui aussi : une allocation par objet, avec l'en-tête de malloc, et des cases de 16 octets au lieu de 8.

En valeur absolue, les écarts restent petits devant les besoins de l'affichage. Les opérations interactives (A6, A7) prennent moins de 40 µs dans les deux cas. L'extraction de tous les sommets d'un fichier de 500 Mo, qui sera faite une fois puis mise en cache, prend 20 ms ou 27 ms.

**Mémoire.** V2 prend 42 à 47 % de mémoire en moins (0,81 à 0,85 fois la taille du fichier, contre 1,46 à 1,51), soit 310 à 350 Mio de moins pour un fichier de 500 Mo. Elle fait 25 fois moins d'allocations pendant le chargement.

**Chargement.** V2 est 7 à 14 % plus rapide. La lecture en flux coûte 12 à 22 % de temps de plus que la lecture depuis un tampon, mais elle évite un pic de 500 Mio et permettra la progression et l'annulation.

**`_type_` tardif.** Avec 10 % d'objets dont `_type_` arrive tard, le chargement prend 12 à 18 % de temps de plus, et les allocations de V2 sont multipliées par 24, à cause de l'arbre intermédiaire. La mémoire finale ne change pas. C'est acceptable pour des fichiers tiers, et sans objet pour ceux qu'Ortelia écrira (`_type_` en premier).

**Modification.** À égalité pour la création (E4). V2 est plus rapide pour les suppressions (E3 : 2,2 à 3,6 fois), et V1 l'est un peu plus pour les `set` (E1 : 1,1 à 1,2 fois). E2 dépend surtout du décalage de la liste, commun aux deux, et varie d'une série à l'autre (29 ms pour V1 dans la première, 17 ms dans la seconde, sans changement du code de V1).

**Bruit de mesure.** D'une exécution à l'autre, un même binaire varie de quelques pour cent sur A2, A4, A6 et A7. Il varie bien plus sur A3 (de 56 à 98 ns par objet) et sur A5 (de 26 à 35 ns par niveau pour V2, de 17 à 25 pour V1). Ces deux mesures dépendent de la place des objets en mémoire physique et de l'état des caches, sous WSL2. Les rapports de A3 et A5 ne valent qu'à ±30 % près.

## Optimisation de V2

Après la première série, deux modifications de V2 ont été mesurées en alternance avec l'ancienne version, dans une même session, sur le fichier numérique de 500 Mo (3 exécutions de chacune, médianes ci-dessous).

1. **Les types des attributs rangés à côté de la disposition de chaque classe**, au lieu d'aller les chercher dans le modèle compilé : **écartée**. A1 se dégrade (77 à 81 ns, contre 68 à 70), et rien d'autre ne bouge. Le type tiré du modèle ne dépend que de l'identifiant de l'attribut : le processeur le lit pendant qu'il attend l'enregistrement. Le type tiré de la disposition dépend de la classe, lue dans l'enregistrement : il attend la mémoire.
2. **Chaque mot de masque placé juste avant les 64 cases qu'il décrit** : **retenue**. La place d'une case ne dépend plus que de l'indice de l'attribut, et plus de la classe : `get` et `set` ne consultent plus la disposition. La mémoire ne change pas pour moins de 64 attributs par classe.

| Code | V2 avant | V2 après | V1 | V2/V1 après |
|---|---:|---:|---:|---:|
| A1 (ns/accès) | 73,0 | 68,0 | 59,5 | 1,14 |
| A2 (ns/objet) | 15,8 | 15,4 | 16,5 | 0,93 |
| A4 (ms/million de sommets) | 12,4 | 11,1 | 8,7 | 1,28 |
| A5 (ns/niveau) | 35,3 | 35,4 | 17,8 | 2,0, à ±30 % |
| A6 (µs/fenêtre) | 33,5 | 34,0 | 32,6 | 1,04 |
| A7 (µs/opération) | 19,6 | 17,7 | 13,2 | 1,34 |

L'entrelacement gagne 7 à 10 % sur A1, A4 et A7. Il ne touche pas A5, qui ne lit que l'en-tête. L'écart sur A5 vient de la double indirection décrite plus haut. Le réduire voudrait dire ranger la classe et le conteneur dans la table des emplacements : 8 octets de plus par objet, soit environ 0,1 fois la taille du fichier.

La relecture du code faite pendant cette optimisation a aussi trouvé un défaut dans l'en-tête de V2 : l'indice local de la référence contenante y était combiné, par OU, aux mêmes bits que son numéro global. `container` pouvait donc renvoyer une mauvaise référence quand les deux différaient, ce qui n'arrivait pas avec le modèle d'essai. Le défaut est corrigé et couvert par un test (`container_reports_every_reference`). Il ne changeait pas le temps des mesures.

## Seuils

Le plan prévoyait de fixer les seuils au vu des résultats. Ils sont donc posés après coup, et leur justification compte plus que leur valeur :

| Critère | Seuil : un écart est décisif s'il… | Justification | Constaté |
|---|---|---|---|
| 1. Accès et navigation | dépasse 1,5 fois sur une mesure A, ou fait sortir une opération interactive (A6, A7) de 1 ms | En dessous de 1,5 fois, sur des durées de quelques dizaines de nanosecondes, l'écart ne se voit pas à l'affichage, et une optimisation locale peut le combler. 1 ms laisse une large marge dans une image de 16 ms. | A1 1,14, A4 1,28, A7 1,34 fois après optimisation ; A5 de 1,26 à 2,0 fois selon la série (mesure bruitée) ; A6 et A7 sous 40 µs |
| 2. Mémoire | dépasse 20 % | C'est l'une des deux priorités du projet. 20 % d'un fichier de 500 Mo, c'est environ 100 Mio. | 42 à 47 % en faveur de V2 |
| 3. Chargement | dépasse 20 % | Le chargement se fait une fois par session, et il passera en tâche de fond. | 7 à 14 % en faveur de V2 |

Seule la remontée vers le conteneur (A5) peut dépasser le seuil d'accès : c'est le point à trancher.

## Choix

**V1, un bloc par objet, retenu par le porteur du projet.** L'accès et la navigation sont le premier critère, et V1 est en tête sur toutes les mesures qui suivent les liens : A1, A4, A5, A7. Ce choix fait primer ce critère sur l'écart de mémoire, et il accepte donc :

- environ 1,5 fois la taille du fichier en mémoire, au lieu de 0,85 fois (700 à 760 Mio pour un fichier de 500 Mo, au lieu de 400 à 425) ;
- une allocation par objet pendant le chargement ;
- un chargement plus lent de 7 à 14 %.

V1 est aussi la variante la plus simple : pas de table des emplacements, pas de masque, une case étiquetée par valeur.

### Analyse proposée avant la décision

L'analyse soumise au porteur du projet proposait V2. Elle est gardée ici, pour mémoire :

**V2, le tas d'objets, avec les masques entrelacés.** Après optimisation, V2 reste en dessous du seuil sur toutes les mesures d'accès, sauf A5. Sur A5, l'écart va de 1,26 à 2 fois, mais il porte sur une trentaine de nanosecondes par niveau : une remontée complète depuis un sommet, de trois niveaux, prend environ 100 ns, ce qui reste invisible pour une sélection sur la carte ou dans un arbre. Les parcours et les tables sont à égalité. En revanche, l'écart de mémoire est décisif (42 à 47 %), et le chargement va dans le même sens. V2 est la variante la moins simple, mais la règle « à résultats proches, la plus simple » ne s'applique pas : les résultats ne sont pas proches sur la mémoire.

Si l'écart de A5 devait compter un jour, par exemple pour des expressions qui remontent beaucoup vers les conteneurs, la classe et le conteneur pourraient passer dans la table des emplacements (environ 0,1 fois la taille du fichier en plus) sans rien changer à l'API.

## Ce que le choix implique pour la suite

- `BoxedStore` devient le type concret `Store`. Le trait `ObjectStore`, V2, et la généricité du constructeur et du lecteur sont supprimés (tâche 6.3). Ce document garde la description de V2 et ses mesures, si la mémoire devait un jour primer.
- L'entrée d'un objet supprimé devient vide, et son identifiant n'est jamais réutilisé. La place des entrées vides n'est pas récupérée ; à revoir avec la liste des changements (second changement).
- Pistes, à mesurer avant de les adopter :
  - des cases de 8 octets au lieu de 16, en tirant le type du modèle et en gardant un masque pour « non renseigné » (c'était le principe de V2) : les cases prendraient deux fois moins de place ;
  - des accesseurs typés (`get_f64`…) pour les boucles chaudes de la carte.
- La lecture en flux reste le mode par défaut : son pic de mémoire est celui du Store seul.
- Les fichiers écrits par Ortelia mettront `_type_` en premier.
