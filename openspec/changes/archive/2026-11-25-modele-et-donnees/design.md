# Conception : modele-et-donnees

## Context

Motivation et périmètre : voir `proposal.md`. Exigences : les spécifications du changement.

État au début du changement, issu de `stockage-dynamique` :
- `ortelia-model` a le métamodèle (`ModelFile`, `Model::compile`), le `Store` (bloc de valeurs typées par objet, arène des textes, identifiants jamais réutilisés), le constructeur (`Builder`) et les erreurs de chargement (`LoadError`, `DataPath`). Toutes les méthodes du `Store`, modification comprise, sont publiques.
- Le constructeur remplit un `Store` entier à partir de sa racine, et la première erreur arrête tout.
- `ortelia-service` a le lecteur JSON (`read_json`, `read_json_slice`), dont le fichier contient l'objet racine.
- `ortelia-config` est vide. `examples/annuaire/app.json` (format 0.2) esquisse déjà les sections des incréments 2 et 3 (libellés, présentation, fenêtre), et `model.root` y désigne `Annuaire`.
- Hors de `ortelia-model`, le Store est modifié par l'exemple `s1` (mesures E1 à E4) et par les tests `crates/ortelia-model/tests/store.rs`.

Contraintes : la règle des couches, la rapidité et la mémoire (fichiers d'environ 500 Mo), aucune valeur de donnée dans le journal.

## Goals / Non-Goals

**Goals :**
- Que le compilateur garantisse que toute modification passe par le `Document`.
- Que l'intégration d'une source coûte un passage linéaire sur ses objets, sans dupliquer ses blocs de valeurs ni ses textes.
- Une API que la présentation pourra interroger à chaque image, à un coût négligeable (une comparaison d'entiers quand rien n'a changé).
- Des types de fragment et de lecteur prêts pour la lecture en tâche de fond, sans la réaliser.

**Non-Goals :**
- Revenir en arrière dans une transaction en cas d'erreur : une opération refusée ne change rien, mais les opérations précédentes de la même transaction restent appliquées.
- Plafonner ou compacter le journal.
- Rendre une suppression réversible.

## Decisions

### 1. Où va quoi

| Élément | Crate | Fichier |
|---|---|---|
| `Document`, `Edit` (transaction en cours), `Transaction`, `Change`, `OwnedValue`, `Target` | `ortelia-model` | `src/document.rs` |
| `Fragment`, `Shape`, absorption | `ortelia-model` | `src/fragment.rs`, `src/store/absorb.rs` |
| `Store::descendants`, `delete_many` ; modification en `pub(crate)` | `ortelia-model` | `src/store/mod.rs` |
| Erreurs rassemblées, diagnostics, valeur fautive | `ortelia-model` | `src/builder.rs` |
| `Model::load` (fichiers nommés), version, bornes, littéraux, attributs obligatoires | `ortelia-model` | `src/metamodel/` |
| Types de `app.json`, vérification contre le modèle, `TargetPath` | `ortelia-config` | `src/lib.rs`, `src/file.rs`, `src/check.rs` |
| Lecteur JSON pour une branche, qui continue après une erreur | `ortelia-service` | `src/json.rs` |
| Ouverture d'une application (`AppContext`) | `ortelia-service` | `src/app.rs` |
| `cargo xtask gen-schemas` | `xtask` | `src/schemas.rs` |

Les dépendances restent dans la règle des couches : `ortelia-config` dépend de `ortelia-model`, pour résoudre les noms de `app.json` en identifiants, et `ortelia-service` des deux. `xtask` dépend de `ortelia-config` avec la feature `json-schema` : un outil peut dépendre des couches, pas l'inverse.

### 2. Un Store brut, un Document qui seul le modifie

Les méthodes de modification du `Store` (`create_root`, `create_child`, `set`, `delete`, `delete_many`, et la nouvelle `absorb`) passent en `pub(crate)`. Les méthodes de lecture restent publiques. Hors de `ortelia-model`, on ne peut donc que lire un `&Store`, que rend `Document::store()`, ou modifier par le `Document`. C'est l'équivalent d'une visibilité *package-private* en Java, sauf que la frontière est la crate et qu'aucune réflexion ne la contourne.

- Le `Builder` et le `Document`, tous deux dans `ortelia-model`, gardent l'accès complet.
- Le scénario « Modification directe du Store impossible » est vérifié par un doc-test `compile_fail` sur `Document::store()`. rustdoc compile ce bloc comme une crate extérieure, et le test échoue s'il compile.
- Les tests du Store (`tests/store.rs`, `tests/common/mod.rs`) passent dans la crate, dans `src/store/tests.rs` : un test d'intégration est une crate extérieure et ne verrait plus les méthodes `pub(crate)`. Les tests du constructeur (`tests/builder.rs`) restent où ils sont, car le `Builder` reste public.

Alternative écartée : un journal dans le Store lui-même, activé ou non. Le constructeur devrait le couper, chaque Store porterait un journal, même un fragment, et rien n'empêcherait un appel direct à `set` sans trace.

### 3. Transactions : un garde RAII

```rust
let mut edit = document.edit();            // Edit<'_> borrows the document mutably
edit.set(person, attrs.nom, Value::Str("Dupond"))?;
edit.delete_many(&[a, b, c])?;
drop(edit);                                 // or end of scope: the transaction is recorded
```

Opérations d'un `Edit` : `create_child`, `set`, `delete`, `delete_many` et `replace` (§ 7), avec les mêmes arguments et les mêmes erreurs que celles du Store, et `store()` pour lire en cours de transaction.

`Edit<'a>` emprunte le `Document` en écriture, applique chaque opération tout de suite, et accumule les `Change`. Son `Drop` enregistre la transaction si elle contient au moins un changement, et augmente alors la révision de un. C'est le `std::lock_guard` de C++, ou un *try-with-resources* Java dont le `close()` enregistre la transaction.

- **Pourquoi `Drop` et pas un `commit()` explicite :** les opérations sont déjà appliquées au Store. Une transaction oubliée sans `commit()` laisserait des changements sans trace, donc invisibles pour la présentation. Avec `Drop`, l'enregistrement est garanti, même lors d'une panique. Un `commit(self)` serait seulement un `drop` nommé : il n'en apporte pas.
- **Pourquoi pas une fermeture** (`document.transaction(|edit| { … })`) : la gestion du `?` et des emprunts dans la fermeture est plus déroutante pour un débutant en Rust que le garde, pour le même résultat.
- Pendant qu'un `Edit` existe, le Document ne peut pas être lu : le borrow checker l'interdit, car il est emprunté en écriture. On lit avant ou après, ou par `edit.store()`.
- Une opération refusée (`StoreError`) laisse le Store inchangé et n'enregistre rien. Les opérations précédentes de la même transaction restent appliquées : pas de retour arrière (Non-Goals).

Révision et journal :

```rust
pub struct Document {
    store: Store,
    revision: u64,
    journal: Vec<Transaction>,       // oldest first; one transaction per revision since 0
}
pub struct Transaction { pub revision: u64, pub changes: Vec<Change> }
```

`changes_since(rev)` rend la tranche des transactions de révision supérieure à `rev`. Le journal est trié par révision, donc `partition_point` (une recherche dichotomique) la trouve. La présentation garde la dernière révision vue et ne fait rien tant que `document.revision()` n'a pas bougé.

### 4. Les changements

```rust
pub enum Change {
    Set { obj: ObjectId, attr: AttrId, old: OwnedValue, new: OwnedValue },
    Created { obj: ObjectId, class: ClassId, parent: ObjectId, reference: RefId, index: usize },
    Deleted { obj: ObjectId, parent: ObjectId, reference: RefId, index: usize },
    Replaced { target: Target, objects: Vec<ObjectId> },
}
```

- `OwnedValue` est la version possédée de `Value<'a>` : `Str(String)` au lieu de `Str(&'a str)`. Le journal doit survivre aux modifications suivantes du Store, alors qu'un `&str` emprunté l'en empêcherait. Les textes sont donc copiés, l'ancien comme le nouveau.
- Un `set` qui donne la valeur déjà présente n'enregistre rien (scénario « Valeur inchangée »). Le formulaire de l'incrément 5 pourra appliquer tous ses champs sans fabriquer de faux changements. La comparaison se fait sur `Value` : un `NaN` remplacé par `NaN` est enregistré, ce qui est sans conséquence.
- Un objet créé est enregistré vide (`Created`). Les valeurs qu'on lui donne ensuite suivent, en `Set` dont l'ancienne valeur est absente.
- `Deleted.index` est la position avant la suppression. Dans une suppression en lot, chaque index est la position avant le lot. Un consommateur qui voudra rejouer les suppressions les traitera de la plus grande position à la plus petite.
- `Replaced` ne liste que les nouveaux objets de tête. Les anciens sont morts : `store.is_alive` le dit. Ce changement unique invalide tout ce qui dépend de la branche.
- La racine ne se supprime pas (`StoreError::RootNotDeletable`) : elle se remplace par `Replaced { target: Target::Root, .. }`.

### 5. Lire la révision plutôt que s'abonner

La présentation lit `revision()` puis `changes_since()` : c'est une lecture par révision (*pull*), pas un patron Observateur.
- En Rust, des écouteurs appelés pendant une modification du Store voudraient le lire alors qu'il est emprunté en écriture. Il faudrait des `Rc<RefCell<…>>` et des vérifications d'emprunt à l'exécution, sources de paniques.
- egui redessine à chaque image : comparer un `u64` par vue ne coûte rien.
- Le contrat de `architecture-couches.md` (§ 3 : révision et demandeur de rafraîchissement) reste valable. La présentation appellera le demandeur de rafraîchissement après une intégration en tâche de fond, quand celle-ci existera.

### 6. Fragment et forme

```rust
pub struct Shape { pub class: ClassId, pub many: bool }      // what a branch holds
pub struct Fragment { store: Store, heads: Vec<ObjectId>, shape: Shape }
```

- `Shape::root(class)` donne `{ class, many: false }` ; `Shape::of(model, reference)` donne le type déclaré de la référence et sa multiplicité.
- `Builder::new(model, shape, base: DataPath)` remplace `Builder::new(model)`. `begin_head(class)` remplace `begin_root(class)`. Il vérifie que `class` est concrète et est `shape.class` ou l'une de ses sous-classes, ce qui règle au passage la conformité de la racine, signalée par la revue de S1. Pour une forme simple, il refuse une seconde tête (`ReferenceAlreadySet`).
- Dans le Store d'un fragment, les têtes n'ont pas de conteneur, et la racine du Store reste vide (`root() == None`). Le fragment n'expose qu'une lecture (`store()`, `heads()`, `shape()`) et se consomme par l'intégration.
- `base` préfixe les chemins des erreurs : pour la cible `/annuaire/personnes`, l'erreur de la quatrième personne est `/annuaire/personnes/3/nombre_enfants`.
- `Fragment` est `Send`, comme le Store qu'il contient. Un test `assert_send::<Fragment>()` le garantit à la compilation.

### 7. Intégration : absorber le fragment au lieu de le recopier

`Edit::replace(target, fragment)` :
1. Vérifie la forme. `fragment.shape` doit être celle de la branche (`Shape::of(reference)`, ou `Shape::root(classe de la racine)`), au moins une tête pour la racine, et le même modèle (`Arc::ptr_eq`). Sinon, `IntegrateError`, et rien n'a changé.
2. Supprime l'ancien contenu de la branche : ses sous-arbres sont libérés, comme par `delete_many`.
3. Absorbe le Store du fragment dans celui du Document (`Store::absorb`). Soit `n` le nombre d'objets du Document, `l` son nombre de listes, `b` son nombre de blocs de textes :
   - `objects.extend(fragment.objects)` : les entrées sont déplacées. Chaque `Entry` garde son `Box<[Slot]>`, et les blocs de valeurs ne bougent pas en mémoire.
   - Un seul passage réécrit les identifiants des entrées déplacées : `ObjectId + n` dans `container` et `Slot::One` ; numéro de liste `+ l` dans `Slot::Many` ; bloc `+ b` dans chaque `StrRef` (sauf `StrRef::EMPTY`, qui ne désigne aucun bloc).
   - `lists.extend(…)` avec chaque `ObjectId + n` ; `strings.blocks.extend(…)` : les `String` des blocs sont déplacées, pas recopiées. Le bloc courant du Document reste le sien : le dernier bloc du fragment, partiellement rempli, ne se remplira plus, soit au plus 1 Mio perdu par intégration.
4. Rattache les têtes, renumérotées, à la branche, et enregistre `Replaced`.

Coût : un passage linéaire sur les objets et les listes du fragment, sans allocation par objet. Seules les tables de pointeurs du Document grandissent (`Vec<Option<Entry>>`, environ 32 octets par objet), et peuvent être réallouées une fois. Les identifiants obtenus sont tous neufs (`≥ n`), donc jamais réutilisés.

C'est un `std::list::splice` étendu à toutes les tables du Store. Alternatives écartées :

| Alternative | Pourquoi non |
|---|---|
| Échanger tout le Store | ne vaut que pour une source unique ; tous les identifiants meurent, même ceux des autres branches |
| Recopier objet par objet | double un instant la mémoire de la branche (500 Mo de plus pour un gros fichier) et réalloue chaque bloc |
| Construire directement dans le Store du Document | interdit la lecture en tâche de fond, car le Store affiché serait emprunté en écriture pendant toute la lecture |

### 8. Les cibles : chemin statique, branche à l'exécution

- `ortelia-config` résout chaque `target` en un `TargetPath { refs: Vec<RefId>, shape: Shape }`. Le chemin `/` donne `refs` vide et `Shape::root(racine)`. Les règles du scénario (segment inconnu, segment intermédiaire multiple ou de type abstrait) sont vérifiées pendant cette résolution.
- Le conflit entre deux sources se teste sur les `refs`. Deux sources sont en conflit si l'une est un préfixe de l'autre, égalité comprise. `/` est le préfixe de tout. Les `RefId` portent leur classe déclarante : la comparaison est exacte, même avec des noms de références identiques dans deux classes.
- À l'exécution, `ortelia-model` donne `Target::Root` ou `Target::Reference { parent: ObjectId, reference: RefId }`. `AppContext` obtient la cible d'une source en descendant depuis la racine le long des `refs` intermédiaires, toutes simples. Il crée au passage, dans la transaction du squelette, l'objet intermédiaire manquant, du type déclaré de sa référence (concret, vérifié à la lecture de `app.json`).

### 9. Le constructeur rassemble les erreurs

- Les méthodes du `Builder` ne rendent plus `Result<(), LoadError>`. Elles enregistrent l'erreur elles-mêmes. `begin_head` et `begin_object` rendent un `bool` : `false` quand l'objet est refusé, le lecteur doit alors ignorer son contenu. Les `set_*` ignorent la valeur refusée.
- Les positions des chemins sont celles du fichier : un objet refusé compte quand même dans sa liste (c'est déjà ce que fait `fail_object`). Les erreurs suivantes gardent ainsi le chemin qu'on lit dans le fichier.
- Plafond : `MAX_REPORTED = 100` erreurs gardées, et le total compté. Les diagnostics sont gardés et comptés de la même façon, séparément.
- Attributs obligatoires : `Class::required_attributes()` (liste précalculée à la compilation, héritage compris) est parcourue dans `end_object`. Un attribut non renseigné donne un `Diagnostic { path, kind: MissingRequired { attribute } }`. Le coût par objet est nul pour une classe sans attribut obligatoire.
- `finish()` rend `Result<Loaded, LoadFailure>`. `Loaded` contient le fragment, les diagnostics et leur total. `LoadFailure` contient les erreurs et leur total, les diagnostics et leur total. Dès la première erreur, le fragment est voué à être jeté. Le constructeur continue pourtant à créer les objets acceptés : c'est plus simple et cela ne coûte que dans un cas d'échec.
- Le lecteur JSON :
  - une clé inconnue donne une erreur, puis la valeur est consommée par `IgnoredAny` ;
  - un objet refusé (`_type_` inconnu, abstrait ou non conforme) voit tout son contenu consommé par `IgnoredAny`. Quand `_type_` vient tard, le contenu déjà mis en mémoire tampon est jeté au lieu d'être rejoué ;
  - le type de la valeur JSON est vérifié contre la forme : un objet pour une forme simple, un tableau pour une liste ;
  - seules les erreurs de syntaxe et d'entrée-sortie interrompent la désérialisation. Elles s'ajoutent aux erreurs rassemblées.

### 10. La valeur fautive, pour l'utilisateur seulement

`LoadError` gagne `value: Option<String>`. Le constructeur la remplit pour une erreur de type de valeur, réduite à 40 caractères (des `char`, pas des octets, pour ne jamais couper un caractère UTF-8), suivis de « … ».
- `Display` donne la forme destinée au journal, **sans** la valeur. C'est la forme qu'on obtient sans y penser (`warn!("{error}")`), donc la plus sûre par défaut.
- `LoadError::user_message()` donne la forme destinée à l'utilisateur, avec la valeur. Personne ne l'appelle encore : l'interface arrivera à l'incrément 2. Des tests vérifient les deux formes.
- Une clé inconnue et le nom de classe d'un `_type_` inconnu viennent aussi du fichier, et peuvent être des données (un dictionnaire employé par erreur) : `Display` dit seulement « unknown key » ou « unknown class name in `_type_` », et `user_message()` les cite, réduits à 40 caractères comme une valeur.

### 11. `app.json` (`ortelia-config`)

Les types serde suivent le modèle de `ModelFile` : `deny_unknown_fields`, `rename_all = "camelCase"`.

```rust
pub struct AppFile {
    #[serde(rename = "$schema", default)] pub schema: Option<String>,   // read, ignored
    pub format_version: String,
    pub id: String,
    pub name: String,
    pub model: ModelSection,          // files: Vec<String>, root: String
    #[serde(default)] pub sources: Vec<SourceDef>,
}
pub struct SourceDef { pub id: String, pub format: SourceFormat, pub file: String, pub target: String }
#[serde(rename_all = "lowercase")] pub enum SourceFormat { Json }
```

- `AppFile::parse(text)` lit le fichier, puis vérifie `formatVersion` (`0.3`), que serde ne sait pas contrôler. Les erreurs serde_json citent déjà la clé, la ligne et la colonne. Elles sont reprises dans `ConfigError::Syntax`.
- `AppFile::check(&self, &Model) -> Result<CheckedApp, Vec<ConfigError>>` vérifie la racine, les cibles et les conflits, et rassemble toutes les erreurs. `CheckedApp` contient la classe de la racine et, pour chaque source, son identifiant, son format, son fichier et son `TargetPath`.
- **Exemple réduit** : `examples/annuaire/app.json` ne garde que `$schema`, `formatVersion`, `id`, `name`, `model` et `sources`. Les sections `languages`, `labels`, `presentation` et `window` en sont retirées. L'historique git les garde, et les incréments 2 à 4 les reprendront en les concevant. Les conserver sans les lire aurait obligé à figer dès maintenant leur format dans des types Rust, ou à tolérer des clés inconnues.

### 12. Ouverture d'une application (`ortelia-service`)

```rust
pub struct AppContext { app: CheckedApp, document: Document, sources: Vec<SourceState> }
pub struct SourceState { pub id: String, pub file: PathBuf, pub target: Target, pub status: SourceStatus }
pub enum SourceStatus {
    Loaded { integrated_at: u64, diagnostics: Vec<Diagnostic>, diagnostic_count: usize },
    Failed(SourceError),              // Io { file, message } or Data(LoadFailure)
}
```

`AppContext::open(path)` :
1. lit `app.json`, puis les fichiers du modèle (relatifs au dossier de `app.json`), par `Model::load` ;
2. vérifie `app.json` contre le modèle. Une erreur à cette étape ou à la précédente donne `OpenError`, qui cite le fichier en cause ;
3. crée le `Document` de la racine, puis le squelette en une transaction (§ 8) ;
4. pour chaque source, dans l'ordre : ouvre le fichier, le lit en flux (`read_json`) avec sa forme et son chemin, puis intègre le fragment par `edit.replace`, en une transaction. Un échec donne `SourceStatus::Failed`, et la branche reste telle que le squelette l'a créée.

Les sources sont lues l'une après l'autre, dans le thread appelant. Le jour de la lecture en tâche de fond, l'étape 4 se coupera en deux : lire (dans un autre thread, le `Fragment` étant `Send`), puis intégrer (dans le thread principal, dans l'ordre de `app.json`).

Journalisation (`tracing`, jamais de valeur de donnée) :
- `info!` : application ouverte (identifiant, nombre de sources) ; source chargée (identifiant, fichier, nombre d'objets, nombre de diagnostics) ;
- `warn!` : source en échec (identifiant, fichier, nombre d'erreurs, et la première erreur sous sa forme `Display`, sans valeur).

Le test « Valeur fautive non journalisée » installe un subscriber qui capture le journal. Il est sérialisé comme les tests `APP_TESTS` de `ortelia-ui-egui`, à cause du cache d'intérêt de `tracing` (voir `CLAUDE.md`).

### 13. Le métamodèle complété

- `Model::load(files: &[NamedModelFile]) -> Result<Model, ModelLoadError>`, où `NamedModelFile { name: String, text: String }`. C'est le `Model::from_json` proposé par la revue de S1, avec le nom de chaque fichier. Il lit chaque fichier, vérifie sa version (`0.1`), puis compile. `ModelLoadError { file: Option<String>, kind }` : `kind` vaut `Syntax { line, column, message }`, `Version { found }` ou `Compile(ModelError)`.
- Pendant la compilation, chaque classe, chaque énuméré et chaque type de données retient le numéro de son fichier. Une erreur de compilation rend ce numéro, que `Model::load` traduit en nom. `TooLarge` n'a pas de fichier. `Model::compile(&[ModelFile])` reste, pour les tests.
- Nouvelles erreurs de compilation : `InvalidBounds { class, feature, lower, upper }` et `DuplicateLiteral { enum_name, literal }`.
- `Attribute::is_required()` (borne inférieure 1) ; `Class::required_attributes()`.
- `ModelFile` gagne la clé `$schema`, comme `AppFile`.

### 14. Le Store complété

- `descendants(obj) -> Vec<ObjectId>` : parcours en profondeur, parent d'abord, avec une pile explicite comme `delete` (pas de récursion, donc pas de débordement de pile sur un arbre profond). La pile reçoit les enfants à l'envers, pour les rendre dans leur ordre. Le résultat est un `Vec` plutôt qu'un itérateur : plus simple à écrire et à lire, pour un usage rare (suppression, enregistrement d'une branche).
- `delete_many(&[ObjectId]) -> Vec<(ObjectId, ObjectId, RefId, usize)>` : regroupe les objets par liste conteneur (`HashMap<u32, HashSet<ObjectId>>`). Chaque liste est filtrée par un seul `retain`, qui relève au passage les positions d'origine. Les sous-arbres sont ensuite libérés comme dans `delete`. Un objet dont un ancêtre est aussi dans le lot n'est traité qu'une fois. Le `Document` en tire les `Change::Deleted`.

### 15. JSON Schema : schemars, derrière une feature

- schemars 1.2.3 (vérifié dans Context7 : `deny_unknown_fields` donne `additionalProperties: false`, `rename_all`, `rename` et `default` sont pris en compte). Feature `preserve_order`, pour que les propriétés suivent l'ordre des champs : un schéma lisible et un diff stable.
- `ortelia-model` et `ortelia-config` déclarent une feature `json-schema = ["dep:schemars"]`, et `ortelia-config` active `ortelia-model/json-schema`. Les types de fichiers portent `#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]`. Seul `xtask` active la feature.
  - Unification des features : `cargo build --workspace` et `cargo test --workspace` compilent `ortelia-model` avec la feature, puisque `xtask` est membre du workspace. C'est sans effet sur le comportement.
  - `cargo build -p ortelia --release`, la commande des binaires de la CI, ne l'active pas. Une tâche le vérifie : `cargo tree -p ortelia -i schemars` ne trouve rien.
- `cargo xtask gen-schemas [--check]` :
  - génère en draft 7 (`SchemaSettings::draft07().into_generator().into_root_schema_for::<T>()`), la version la mieux reconnue par les éditeurs (VS Code, IntelliJ) ;
  - écrit du JSON indenté avec un saut de ligne final, ce qui le rend reproductible ;
  - avec `--check`, compare au lieu d'écrire, puis sort avec le code 1 en citant le fichier et la commande à lancer.
- La CI (`ci.yml`, job `checks`, étape Linux, après `check-layers`) et `tools/check.sh` lancent `cargo xtask gen-schemas --check`.
- Les fichiers de `examples/` reçoivent `"$schema": "../../schemas/…"`. VS Code résout ce chemin relativement au fichier.

### 16. Le modèle et les données de l'annuaire

```
Racine
  annuaire : Annuaire                (simple)
Annuaire
  personnes : Personne [*]
Personne
  nom : String  [obligatoire]        prenom : String        date_naissance : Date
  nombre_enfants : Int               taille : Float (unit "m")
  contact_autorise : Boolean         situation : SituationFamiliale
  inscription : DateTime             notes : TexteLong (type dérivé de String)
SituationFamiliale = CELIBATAIRE | MARIE | PACSE | DIVORCE | VEUF
```

- `app.json` : `model.root` vaut `Racine`, et la source `personnes` a pour cible `/annuaire/personnes`. `personnes.json` reste un tableau.
- Les dix personnes de `personnes.json` reçoivent des valeurs pour les nouveaux attributs. Certaines n'en ont pas pour les attributs facultatifs, pour que le formulaire de l'incrément 3 montre aussi des champs vides.
- `unit` est lu mais pas encore utilisé, comme dans `stockage-dynamique`.

### 17. L'exemple `s1` et le générateur

- L'exemple `s1` lit le fichier en un fragment de racine (`Shape::root`), crée un `Document`, puis l'intègre à la racine. Les mesures A lisent `document.store()`. Les mesures E passent par des transactions, une par mesure.
- Les mesures E incluent désormais le coût du journal (anciennes valeurs copiées). Les chiffres de `docs/essais/s1-stockage.md` restent ceux de l'essai. Une phrase le signale dans ce document, et la commande documentée doit toujours fonctionner.
- Le test de `gen-data` lit les fichiers générés en un fragment de racine.

### 18. Versions des bibliothèques

| Bibliothèque | Version | Où | Usage |
|---|---|---|---|
| schemars | 1.2.3 (features `derive`, `preserve_order`) | `ortelia-model`, `ortelia-config` (optionnelle), `xtask` | JSON Schema |
| serde, serde_json | 1.0.229, 1.0.154 (inchangées) | + `ortelia-config` | `app.json` |
| tracing | 0.1.44 (inchangée) | + `ortelia-service` | journal de l'ouverture |
| tracing-subscriber | 0.3.23 (inchangée) | dév. de `ortelia-service` | capture du journal dans un test |

schemars 1.2.3 est la dernière version publiée (`cargo search`). Son API a été vérifiée dans Context7 pour la version 1.2.3.

### 19. Documentation

- `docs/architecture-couches.md`, § 9 :
  - le schéma montre le fragment, l'absorption et la greffe dans une branche ;
  - D30 devient « lire un fragment, puis le greffer sur sa branche, en remplaçant son contenu : un seul changement, et seuls les objets de la branche meurent ; la fusion (REST) donnera les vraies différences » ;
  - deux décisions nouvelles : **D31** (un Store brut, un Document seul à le modifier, révision et journal lus par la présentation, sans observateurs) et **D32** (une racine du modèle désignée par `app.json`, squelette créé à l'ouverture, un fichier par branche).
- `docs/feuille-de-route-annuaire.md`, incrément 1 : la phrase sur `app.json` « correspond à la fin de l'incrément 2 » (l'exemple ne contient plus que l'incrément 1), « Lire puis intégrer » et « Notification des modifications » selon D30 et D31, et un résultat sous « Fini quand ».
- `CLAUDE.md` : état du projet, `schemas/` dans l'arborescence et le tableau des documents, commande `gen-schemas`.

## Risks / Trade-offs

- [Une erreur de renumérotation dans l'absorption corromprait l'arbre en silence] → Une fonction de test vérifie la cohérence d'un Store : chaque enfant a pour conteneur son parent, et chaque objet vivant est atteignable une seule fois depuis la racine. Elle est appliquée après chaque test d'intégration : listes, références simples, textes en bloc dédié, arbre profond, intégrations successives.
- [Le journal copie les textes, et une très grosse transaction le fait grossir d'autant] → Le remplacement d'une branche ne coûte qu'un changement, quelle que soit sa taille. Les modifications faites à la main restent petites. Un plafond, ou un journal compacté, viendra si une action en lot le demande.
- [Un Store qui vit longtemps ne rend pas sa mémoire] (arène, cases vides) → Sans effet pour l'annuaire, chargé une fois. Avec REST : un compteur de textes vivants par bloc permettrait de libérer les blocs d'une branche remplacée, puisque l'absorption garde les textes d'un fragment dans leurs propres blocs. La réutilisation des cases, avec un numéro de génération dans l'identifiant, serait l'autre moitié.
- [L'oubli d'un `drop(edit)` avant une lecture] → Le compilateur le signale : le Document est emprunté. Le message d'erreur d'emprunt peut dérouter au début. Un commentaire sur `Document::edit` l'explique.
- [Les mesures E de `s1` ne sont plus comparables à l'essai] → C'est voulu : elles mesurent maintenant le vrai chemin de modification. `s1-stockage.md` le signale.
- [L'exemple `app.json` perd son esquisse des incréments suivants] → L'historique git la garde, et chaque incrément concevra sa section au lieu d'hériter d'une esquisse non vérifiée.

## Migration Plan

Pas de données ni d'utilisateurs à migrer : `app.json` passe en format 0.3 sans conversion, et les fichiers d'exemple sont mis à jour dans ce changement. Le retour arrière est un revert de la pull request.
