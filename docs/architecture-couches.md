# Architecture en couches : une interface interchangeable

*2026-11-23. Proposé, à valider. Objectif fixé par le porteur du projet : séparer fortement les couches modèle et service, pour pouvoir remplacer l'interface (egui ↔ GPUI), voire disposer de deux implémentations et choisir laquelle utiliser. Questions Q18 à Q20 tranchées le même jour : egui est la seule interface réalisée, sans essai comparatif (§ 8). Complété le 2026-11-25 : lecture des données (§ 9, D28 à D30), puis fragment, Document et sources (D30 révisée, D31, D32).*

---

## 1. Le constat : séparer modèle et service ne suffit pas

Ce qui rend coûteux un changement d'interface, ce n'est pas l'accès aux données : c'est **toute la logique d'affichage qui n'est pas du dessin**.
- Table : colonnes visibles, tri multiple, filtre, regroupement, texte formaté de chaque cellule.
- Formulaire : choix du widget pour chaque attribut, libellés, valeurs, messages de validation, copie en cours d'édition, Appliquer / Annuler.
- Arbre : nœuds, libellés, enfants.
- Disposition : vues, piles d'onglets, sliders, onglet actif, ratios.
- Broker de sélection, actions et menus, raccourcis, palette de commandes.
- Libellés traduits, formats de date, unités.
- État de session, et toute la logique du mode design (ajouter un container, configurer une vue…).

Si cette logique vit dans le code egui, une seconde interface devrait tout réécrire. Il faut donc une **couche de présentation** qui calcule *quoi* afficher, sans savoir *comment* c'est dessiné : c'est le principe MVVM (modèle-vue-vue-modèle), ou des composants « sans tête » (*headless*). Les implémentations d'interface deviennent de simples moteurs de rendu.

---

## 2. Les crates et la règle de dépendance

```
ortelia  (exécutable)        choisit l'interface (features Cargo, option --ui)
  ├── ortelia-ui-egui   ┐    dessin uniquement : affiche les view-models
  └── ortelia-ui-gpui   ┘    et traduit clics et touches en intentions
          │
  ortelia-presentation       view-models (table, formulaire, arbre, disposition),
          │                  broker, intentions et actions, libellés, formats, unités,
          │                  état de session, logique du mode design
          │
  ortelia-service            sources (JSON…), tâches de fond, validation, hôte des plugins
          │
  ortelia-config   ortelia-expr      format de app.json et migrations ; expressions
          │
  ortelia-model              métamodèle, stockage dynamique, constructeur, API de modification, révisions

  ortelia-plugin-sdk         interface stable pour les plugins (ne dépend que du modèle)
```

Le globe 3D et la carte seront une bibliothèque externe, dans son propre dépôt (voir `feuille-de-route.md`) : son cœur sans interface sert la présentation, son rendu wgpu et son widget egui servent `ortelia-ui-egui`.

**Règle** : une couche ne connaît que les couches situées en dessous d'elle. Aucune crate sous `ortelia-ui-*` ne dépend d'egui ou de GPUI.

- Le compilateur la fait respecter : une crate ne peut utiliser que les dépendances déclarées dans son `Cargo.toml`. Pour un développeur Java, c'est l'équivalent de modules dont les dépendances sont vérifiées à la compilation.
- La CI le vérifie explicitement : l'arbre de dépendances de `ortelia-presentation` ne doit contenir ni egui ni GPUI.

---

## 3. Le contrat entre la présentation et l'interface

Trois sens de circulation :

| Sens | Mécanisme |
|---|---|
| **Lire** | L'interface lit les view-models. Pour les gros volumes, elle ne demande que ce qui est visible (lignes 120 à 160 de la table) : rien n'est recopié à chaque image. |
| **Écrire** | Toute action de l'utilisateur devient une **intention** envoyée à la présentation : sélectionner, trier, modifier un champ, appliquer, lancer une action, redimensionner une colonne… Les menus, les raccourcis, la palette de commandes et le mode design passent par ce même canal. |
| **Être prévenu** | La présentation expose un numéro de révision et appelle un « demandeur de rafraîchissement » fourni par l'interface quand quelque chose change (y compris à la fin d'une tâche de fond). |

Esquisse en Rust (les traits jouent le rôle des interfaces Java) :

```rust
// Dans ortelia-presentation : aucune dépendance vers egui ou GPUI.
pub enum Intent {
    Select { view: ViewId, objects: Vec<ObjectId> },
    SortBy { view: ViewId, column: usize, add_to_sort: bool },
    EditField { form: FormId, field: FieldId, value: Value },
    Apply { form: FormId },
    RunAction { action: ActionId, target: Option<ObjectId> },
    ResizeColumn { view: ViewId, column: usize, width: f32 },
    // ...
}

/// Fourni par l'interface : egui appelle `request_repaint()`, GPUI notifie sa vue.
pub trait RepaintRequester: Send + Sync {
    fn request_repaint(&self);
}

impl AppState {
    pub fn table(&self, view: ViewId) -> &TableViewModel; // lire
    pub fn dispatch(&mut self, intent: Intent);          // écrire
    pub fn revision(&self) -> u64;                        // a-t-il changé ?
}
```

Ce contrat convient aux deux bibliothèques :
- **egui** (mode immédiat) relit les view-models à chaque image et compare la révision ;
- **GPUI** (mode hybride, la vue se redessine quand on la notifie) s'abonne au demandeur de rafraîchissement.

### Ce qui reste dans chaque interface

- Le dessin, la mise en page fine, le thème.
- L'état éphémère : position de défilement, survol, focus.

L'état **persistant** (largeurs et ordre des colonnes, ratios des sliders, onglet actif, tri) est tenu par la présentation, dans l'état de session (D18). Il survit donc à un changement d'interface. Petite contrainte : egui mémorise lui-même les largeurs de colonnes ; l'interface egui devra les synchroniser par des intentions `ResizeColumn`.

### Le catalogue des widgets devient une donnée

La présentation décrit chaque champ par un type de widget (`text`, `combo`, `date`…) et ses options. Chaque interface fournit un rendu par type. Si une interface ne sait pas encore rendre un type, elle affiche « non pris en charge » au lieu de planter : une seconde interface peut donc être construite progressivement.

Mis en place par le changement `formulaire` (incrément 3) :
- **Un catalogue partagé.** Dans `app.json`, le widget d'une vue (`Table`, `Empty`, `Form`) et celui d'un champ de formulaire (`TextField`, `TextArea`, `NumberField`, `CheckBox`, `ComboBox`, `RadioGroup`, `DateField`, `DateTimeField`) sont deux énumérations, chacune limitée à ce que son contexte accepte. Quand une table entrera dans un formulaire, la même définition deviendra une variante des deux : c'est le même widget, avec les mêmes options.
- **Des noms de `_type_` uniques dans tout `app.json`** (`View`, `Split`, `Field`, `Group`, `Text`, `TextField`…) : quand la configuration sera décrite par un modèle, chacun sera une classe d'un même paquetage. D'où `TextField`, et non `Text`, qui est le texte fixe d'un formulaire.
- **La grille dans la présentation.** Le placement des cellules (en flux, avec `span` et `newRow`) et les largeurs des colonnes (minimums, puis le reste partagé selon les poids `grow`) sont des fonctions pures de `ortelia-presentation` (module `grid`), testées sans interface et sans lien avec les formulaires.
- **Les mesures par l'interface.** Seule l'interface connaît les polices : à chaque image, elle mesure la largeur minimale de chaque cellule (textes mesurés, minimums fixes par genre de widget) et les donne à la grille, qui rend les positions et les largeurs ; l'interface dessine chaque cellule dans son rectangle. Aucune mesure n'est gardée d'une image à l'autre.

---

## 4. Choisir l'interface

| Option | Avantages | Inconvénients |
|---|---|---|
| **À la compilation** (features Cargo : un exécutable par interface) | exécutable plus léger, compilation plus rapide | deux exécutables à distribuer si l'on veut les deux |
| **À l'exécution** (les deux compilées, `ortelia --ui gpui`) | un seul exécutable | plus gros, compilation plus longue (GPUI est lourd) |

Retenu (Q19) : des features Cargo, avec par défaut une seule interface compilée. Si les deux le sont, l'option `--ui` choisit au lancement. On obtient les deux modes sans rien imposer. Tant qu'egui est la seule interface, l'option `--ui` n'est pas réalisée.

---

## 5. egui ou GPUI : ce que l'on sait

D'après la documentation consultée (GPUI Kit 0.10, ex-« gpui-component » de Longbridge, et GPUI lui-même) :

| Besoin | egui 0.37 | GPUI + GPUI Kit |
|---|---|---|
| Disposition à onglets et sliders | `egui_tiles` (compatibilité à vérifier) ou maison | **fournie** : panneaux redimensionnables, onglets déplaçables, imbrication |
| Table de 100 000 lignes | `egui_extras` (virtualisée, éprouvée dans le prototype) | **fournie** : défilement virtuel, colonnes fixes et redimensionnables, tri |
| Graphiques (N1) | `egui_plot` | **fournis** (courbes, barres, aires…) |
| Sélecteur de date | `egui_extras` | fourni |
| **Globe 3D** (rendu GPU personnalisé) | **natif** : rendu wgpu dans un panneau | **pas d'API publique trouvée** pour un rendu GPU personnalisé ; il faudrait rendre le globe hors écran puis l'afficher comme image, ce qui est plus coûteux |
| Simplicité pour le vibecoding | mode immédiat, très direct | modèle d'entités et de contextes, plus exigeant pour un débutant en Rust |
| Stabilité de l'API | change à chaque version mineure | issue de l'éditeur Zed, change aussi souvent |

**Lecture** : GPUI Kit fournit plus de composants prêts à l'emploi (disposition, table, graphiques) ; egui reste nettement plus simple pour le globe 3D, qui est le point le plus risqué du projet. Les deux options sont crédibles : c'est précisément pourquoi la séparation en couches est utile.

Le globe illustre la règle. Ce sera une bibliothèque séparée, dont le cœur (tuiles, projections, sélection, préparation de la scène) ne dépend d'aucune interface : seul le rendu final est propre à chacune.

---

## 6. Recommandation

1. **Concevoir pour deux interfaces, n'en réaliser qu'une : egui.** Deux interfaces complètes doubleraient tout le travail d'affichage. La couche de présentation rend une seconde interface possible plus tard, sans refonte.
2. **Tester la présentation sans interface.** L'essentiel du comportement (tri, filtre, formulaires générés, broker, mode design) se teste alors par des tests unitaires, sans fenêtre, ce qui est très favorable au vibecoding et à la CI. Ces tests prouvent aussi que la présentation ne dépend d'aucune interface.
3. **Pas d'essai comparatif** (Q20). L'essai S7 (même tranche avec egui et avec GPUI) n'est pas prévu. Le contrat de la section 3 sera validé par la réalisation de l'interface egui et par les tests de la présentation.

Même sans seconde interface, la couche de présentation reste utile : elle se teste sans fenêtre, et elle cantonne à `ortelia-ui-egui` les changements d'API qu'egui apporte à chaque version mineure.

---

## 7. Décisions proposées

- **D25.** Une couche `ortelia-presentation` sans aucune dépendance vers une bibliothèque d'interface porte toute la logique d'affichage (view-models, broker, intentions, formats, état de session, mode design).
- **D26.** Les interfaces (`ortelia-ui-egui` ; une seconde, par exemple `ortelia-ui-gpui`, reste possible mais n'est pas planifiée) ne font que dessiner et traduire les événements en intentions. Le choix se fait par features Cargo, avec l'option `--ui` si plusieurs sont compilées.
- **D27.** La règle de dépendance est vérifiée par la CI, et la présentation est couverte par des tests sans interface.

---

## 8. Questions

**Q18.** Pourquoi le prototype est-il passé de GPUI à egui ? Qu'est-ce qui vous fait garder GPUI en option ?
> Le prototype a commencé avec GPUI, puis est passé à egui après avoir constaté que la suite ArtCraft utilise cette bibliothèque. Pas d'attachement fort à GPUI : c'est une possibilité, pas un objectif.

**Q19.** Choix de l'interface : à la compilation (un exécutable par interface), à l'exécution (un seul exécutable), ou les deux comme proposé ?
> Les deux, comme proposé (§ 4). L'option `--ui` ne sera réalisée que si une seconde interface existe.

**Q20.** Faut-il réaliser l'essai S7 (même tranche avec egui et GPUI) avant de choisir la première interface ?
> Non, et aucun essai S7 n'est prévu. egui est la seule interface réalisée.

---

## 9. Lecture des données : format, correspondance, constructeur

*2026-11-25. Décisions prises en préparant l'incrément 1 (changements OpenSpec `stockage-dynamique`, puis `modele-et-donnees`).*

```
  fichier JSON ---> lecteur JSON ---+
  (XML plus tard)   (ortelia-service)|
                                     +--> correspondance --+
                                          (identite pour   |
                                          l'instant)       v
  plugin de lecture ---------------------------------> constructeur ---> fragment
  (connait le modele)                                (ortelia-model)   (Store detache,
                                                                        objets de tete
                                                                        d'une branche)
                                                                           |
                                                       absorption, sans recopie des objets
                                                                           v
  app.json : racine, sources --> Document : Store + revision + journal
  (ortelia-config)               greffe du fragment sur sa branche (/, /stations,
                                 /annuaire/personnes...), en remplacant son contenu,
                                 en une transaction
```

- **Le lecteur de format** ne connaît que la syntaxe. Le lecteur JSON lit en flux, sans arbre intermédiaire.
- **La correspondance** relie la forme du fichier à celle du modèle. La forme du fichier ne dicte pas celle du modèle : des positions GeoJSON `[lon, lat]` peuvent devenir des objets `Point`. Par défaut, c'est la correspondance identité : nom de clé = nom d'attribut ou de référence, `_type_` pour la classe concrète. Des règles déclarées dans `app.json` viendront plus tard (`feuille-de-route.md`, pistes).
- **Le constructeur** est la seule API de création des objets. Le lecteur l'appelle dans l'ordre du fichier (début d'objet, valeurs, fin d'objet), comme un `ContentHandler` SAX. Il convertit les valeurs données en texte (XML, dates JSON) selon le type de l'attribut : il n'y a qu'une conversion, pour tous les formats. Il produit aussi les erreurs, avec le chemin de la donnée depuis la racine de l'application. Il ne s'arrête pas à la première : il les rassemble (les 100 premières et leur total), ainsi que les diagnostics des attributs obligatoires sans valeur.
- **Le fragment** est ce que remplit le constructeur : un Store détaché, avec les objets de tête d'une branche (un objet pour la racine ou une référence simple, une liste pour une référence multiple). Il peut changer de thread.
- **Le Document** possède le Store de l'application. Il est seul à le modifier, par des transactions qui augmentent sa révision et s'ajoutent à son journal.

**Décisions**

- **D28.** La lecture des données passe par trois étages : lecteur de format, correspondance, constructeur. Le constructeur, dans `ortelia-model`, est la seule voie de création des objets pour tous les lecteurs.
- **D29.** Un plugin de lecture connaît le modèle et appelle directement le constructeur. L'interface C du constructeur ne fait passer que des identifiants, des textes et des valeurs simples ; `ortelia-plugin-sdk` n'aura besoin que de `ortelia-model`.
- **D30.** Lire un fragment, puis le greffer sur sa branche, en remplaçant son contenu.
  - Un lecteur remplit toujours un fragment neuf, sans liste des changements. La lecture peut donc se faire en tâche de fond, sans toucher aux données affichées.
  - La greffe absorbe le Store du fragment dans celui du Document, sans recopier les objets ni les textes, et ne fait mourir que les objets de la branche. Elle s'enregistre comme un seul changement, qui donne la branche et ses nouveaux objets.
  - La fusion, par rapprochement sur l'identifiant métier (avec la source REST), donnera plus tard les vraies différences.
- **D31.** Un Store brut, un Document seul à le modifier.
  - Hors de `ortelia-model`, le Store ne se lit que par `Document::store()` ; ses méthodes de modification ne sont visibles que dans la crate.
  - Toute modification passe par une transaction (`Document::edit`), qui enregistre ses changements à sa fin : anciennes et nouvelles valeurs, créations, suppressions, remplacements de branche.
  - La présentation lit la révision du Document et les changements depuis la révision qu'elle a vue, sans observateurs ni rappels : c'est elle qui décide quand regarder.
- **D32.** Une racine désignée par `app.json`, un fichier par branche.
  - `model.root` désigne la classe de la racine ; chaque source a pour cible une branche, `/` ou `/r1/…/rn`, vérifiée contre le modèle avant toute lecture.
  - À l'ouverture, la racine et le squelette (les objets intermédiaires des cibles) sont créés dans une première transaction, puis chaque source est lue et greffée sur sa branche, dans l'ordre de `app.json`, en une transaction par source.
  - Deux sources n'alimentent jamais la même branche, ni une branche à l'intérieur de l'autre. Une source en échec n'empêche pas l'ouverture : sa branche reste vide, et son état donne ses erreurs.
