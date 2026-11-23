# Architecture en couches : une interface interchangeable

*2026-11-23. Proposé, à valider. Objectif fixé par le porteur du projet : séparer fortement les couches modèle et service, pour pouvoir remplacer l'interface (egui ↔ GPUI), voire disposer de deux implémentations et choisir laquelle utiliser. Questions Q18 à Q20 tranchées le même jour : egui est la seule interface réalisée, sans essai comparatif (§ 8).*

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
  ortelia-model              métamodèle, stockage dynamique, API de modification, révisions

  ortelia-geo                moteur cartographique sans interface : tuiles, projections,
                             sélection, préparation de la scène (le rendu est dans ui-*)
  ortelia-plugin-sdk         interface stable pour les plugins (ne dépend que du modèle)
```

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

Le globe illustre la règle : son moteur (`ortelia-geo` : tuiles, projections, sélection, préparation de la scène) ne dépend d'aucune interface. Seul le rendu final est propre à chacune.

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
