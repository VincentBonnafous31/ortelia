# Conception : stockage-dynamique

## Context

- `ortelia-model` et `ortelia-service` sont vides, à part leur commentaire de crate. Les dépendances internes existent déjà : `ortelia-service` dépend de `ortelia-model`.
- Le format des fichiers de modèle existe : `examples/annuaire/annuaire.model.json` et `examples/model-editor/metamodel.model.json` (format 0.1). La clé `references` y désigne des références de **contenance** : un objet appartient à un seul conteneur.
- `unsafe_code = "deny"` vaut pour tout le workspace. Une exception s'écrit localement, à un endroit visible, avec `#![allow(unsafe_code)]`.
- `cargo` ne tourne que dans WSL2. Le système de fichiers Windows y est monté sous `/mnt/c`, et il est lent : les gros fichiers d'essai vont dans `~/s1/`.
- Motivation : voir `proposal.md`, section Why. Comportements attendus : voir les spécifications `metamodel`, `object-store`, `data-loading` et `storage-benchmark`.

## Goals / Non-Goals

**Goals :**
- Une API du Store et un constructeur qui ne changeront pas quand la disposition en mémoire changera. Le lecteur JSON et toutes les couches au-dessus ne voient que cette API.
- Deux implémentations comparées dans des conditions identiques : même API, mêmes tests, même arène des chaînes, mêmes contrôles.
- Des mesures reproductibles : données générées avec une graine, chaque cas dans un processus séparé, conditions consignées.
- À la fin, un seul Store concret, sans trait ni généricité qui ne servaient qu'à l'essai.

**Non-Goals :**
- Optimiser au-delà de ce que l'essai doit départager. Par exemple, ranger côte à côte les petits objets sans enfants, ou loger les chaînes courtes dans une case : à envisager seulement si les mesures le demandent.
- Récupérer la place des objets supprimés : le second changement en décidera avec la liste des changements, qui pourra encore avoir besoin de leurs valeurs (Decision 6).
- Rassembler plusieurs erreurs de données : la première erreur arrête le chargement (Decision 8).

## Decisions

### 1. Où va quoi

| Élément | Crate | Fichier |
|---|---|---|
| Format des fichiers de modèle (types serde) | `ortelia-model` | `src/metamodel/file.rs` |
| Modèle compilé, identifiants, compilation | `ortelia-model` | `src/metamodel/mod.rs`, `src/metamodel/compile.rs` |
| `Value`, encodage des dates | `ortelia-model` | `src/value.rs` |
| Arène des chaînes | `ortelia-model` | `src/strings.rs` |
| API commune du Store (trait d'essai), `ObjectId` | `ortelia-model` | `src/store/mod.rs` |
| V1 : un bloc par objet | `ortelia-model` | `src/store/boxed.rs` |
| V2 : tas d'objets | `ortelia-model` | `src/store/heap.rs` |
| Constructeur, erreurs de chargement, chemin | `ortelia-model` | `src/builder.rs` |
| Suite de tests commune aux deux Stores | `ortelia-model` | `tests/store.rs` |
| Lecteur JSON | `ortelia-service` | `src/json.rs` |
| Programme de mesure | `ortelia-service` | `examples/s1/` |
| Générateur de données | `xtask` | `src/gen_data.rs` |
| Modèle d'essai et petit fichier conforme | — | `examples/reseau-meteo/` |
| Résultats de l'essai | — | `docs/essais/s1-stockage.md` |

- Le constructeur est dans `ortelia-model`, et non dans `ortelia-service`, parce que les futurs plugins de lecture l'appelleront. Or `ortelia-plugin-sdk` ne dépendra que du modèle (`docs/architecture-couches.md`, § 2).
- Le lecteur JSON est dans `ortelia-service`, la couche des sources. Il n'utilise que l'API publique du constructeur, comme le ferait un plugin.
- `xtask` est un outil : il peut dépendre des crates des couches (l'inverse est interdit). Il n'en dépend qu'en développement, pour le test qui charge un fichier généré (Decision 10).

### 2. Le métamodèle : lecture puis compilation

**Lecture** (`file.rs`) : des structs serde qui reflètent le format 0.1 (`ModelFile`, `PackageDef`, `ClassDef`, `AttributeDef`, `ReferenceDef`, `EnumDef`, `DataTypeDef`).
- `#[serde(deny_unknown_fields)]` refuse une clé inconnue. L'erreur de serde_json cite la clé, avec la ligne et la colonne.
- `#[serde(rename_all = "camelCase")]` fait correspondre `lowerBound` au champ `lower_bound`.
- Les clés du format qui ne servent pas encore sont lues, puis ignorées par la compilation : `defaultValue`, `unit`, `identifier`, `refersTo`, ainsi que `pattern`, `min`, `max`, `minLength` et `maxLength` des types de données. Un fichier valide aujourd'hui le restera quand elles serviront.

**Compilation** (`compile.rs`) : `Model::compile(files: &[ModelFile]) -> Result<Model, ModelError>`, en quatre passes :
1. Indexer toutes les classes, tous les énumérés et tous les types de données par nom. Un nom de classe en double est une erreur.
2. Résoudre les super-classes, puis calculer pour chaque classe la chaîne de ses ancêtres, de la racine jusqu'à elle. Une super-classe introuvable ou un cycle (une classe déjà rencontrée en remontant) est une erreur.
3. Construire la disposition de chaque classe, en partant des classes sans ancêtre : d'abord les attributs de la super-classe, puis ceux de la classe ; de même pour les références. Un nom déjà présent dans la classe ou ses ancêtres est une erreur. Un attribut dont `upperBound` n'est pas 1 est refusé (« non pris en charge »).
4. Résoudre les types : type de base, énuméré, ou type de données ramené à son type de base (en remontant ses `baseType`).

**Identifiants** : de petits entiers enveloppés dans un type, ce qu'on appelle un *newtype*. `ClassId(u16)` ne peut pas être confondu avec `RefId(u16)` par erreur, alors que deux `typedef` C seraient interchangeables.

```rust
pub struct ClassId(u16);
pub struct AttrId { owner: ClassId, index: u16 } // owner = class that declares it
pub struct RefId { owner: ClassId, index: u16 }
pub enum AttrType { String, Int, Float, Boolean, Date, DateTime, Enum(EnumId) }
```

- `index` est la position dans la disposition. Elle est la même dans toutes les sous-classes (spécification `metamodel`, « Position fixe des attributs »).
- `owner` permet de vérifier, à chaque accès, que l'attribut appartient bien à la classe de l'objet. En héritage simple, ce contrôle se fait en temps constant : la classe `C` est une sorte de `A` si l'ancêtre de `C` au rang `profondeur(A)` est `A`. C'est le test de sous-typage des JVM pour les super-classes primaires (« display »). Utiliser l'attribut d'une autre classe est une erreur de programmation : le Store panique, comme un `ClassCastException` en Java.
- Recherche par nom : `model.class_id("Station")`, puis `model.class(id).attribute("code")` et `.reference("equipements")`, qui renvoient une `Option`. On les appelle une fois, pas à chaque accès.
- Pour chaque classe, le modèle compilé sait aussi si elle a des sous-classes. Le lecteur JSON s'en sert pour `_type_` (Decision 9).

### 3. Les valeurs

```rust
pub enum Value<'a> {
    Unset,
    Int(i64), Float(f64), Bool(bool),
    Str(&'a str),          // borrowed from the store's string arena
    Enum(u32),             // index of the literal in its enum
    Date(jiff::civil::Date),
    DateTime(jiff::Timestamp), // always a whole number of microseconds
}
```

- Le paramètre `'a` lie le `&str` rendu par `get` au Store. C'est l'équivalent d'un `const char*` interne qu'on ne doit plus utiliser après une modification, sauf qu'ici le compilateur l'interdit : on ne peut pas modifier le Store tant qu'une valeur empruntée est vivante.
- `set` reçoit le même type. `Value::Unset` remet l'attribut à l'état non renseigné.
- Encodage dans une case de 8 octets, commun aux deux Stores :
  - `Date` devient `année × 512 + mois × 32 + jour`. Cet entier garde l'ordre chronologique, y compris pour les années négatives (mois × 32 + jour < 512), et se reconvertit sans calcul de calendrier (`Date::new(année, mois, jour)`) ;
  - `DateTime` devient un nombre de microsecondes depuis 1970 (`Timestamp::as_microsecond` et `Timestamp::from_microsecond`).

### 4. L'arène des chaînes, commune aux deux Stores

- Des blocs de 1 Mio, chacun un `String` créé avec sa capacité finale et jamais agrandi. Rien n'est recopié quand l'arène grandit, et il n'y a pas de pic dû à un doublement de capacité.
- Une chaîne est désignée par un `StrRef(u64)`, qui contient le numéro du bloc, la position et la longueur.
- Une chaîne plus longue qu'un bloc reçoit un bloc à elle seule, à sa taille exacte.
- Lecture : `&bloc[début..fin]`, un `&str` obtenu sans `unsafe`. Comme le bloc est un `String`, Rust garantit l'UTF-8, et le découpage ne vérifie que les deux bornes, en temps constant. Un bloc d'octets bruts aurait demandé de revalider l'UTF-8 à chaque lecture, ou du `unsafe`.
- L'arène ne fait que grandir. Une chaîne remplacée reste lisible, ce dont la liste des changements aura besoin (second changement). La place perdue ne se récupère qu'en relisant le fichier enregistré.
- Pas de partage des chaînes identiques dans cet essai. Les énumérés n'en ont pas besoin : ce sont des numéros de littéral.

### 5. L'API commune : un trait, le temps de l'essai

```rust
pub struct ObjectId(u32); // index in the object table, never reused

pub trait ObjectStore: Send + Sized {
    fn new(model: Arc<Model>) -> Self;
    fn model(&self) -> &Model;
    fn create_root(&mut self, class: ClassId) -> Result<ObjectId, StoreError>;
    fn root(&self) -> Option<ObjectId>;
    fn create_child(&mut self, parent: ObjectId, reference: RefId,
                    position: Position, class: ClassId) -> Result<ObjectId, StoreError>;
    fn is_alive(&self, obj: ObjectId) -> bool;
    fn class_of(&self, obj: ObjectId) -> ClassId;
    fn container(&self, obj: ObjectId) -> Option<(ObjectId, RefId)>;
    fn children(&self, obj: ObjectId, reference: RefId) -> &[ObjectId];
    fn get(&self, obj: ObjectId, attr: AttrId) -> Value<'_>;
    fn set(&mut self, obj: ObjectId, attr: AttrId, value: Value<'_>) -> Result<(), StoreError>;
    fn delete(&mut self, obj: ObjectId) -> Result<(), StoreError>;
}
```

- Le trait joue le rôle d'une interface Java. Le constructeur, le lecteur JSON, les tests et le programme de mesure sont écrits une fois, de façon générique (`Builder<S: ObjectStore>`), et compilés pour chaque Store. Comme les templates C++, la généricité est résolue à la compilation (monomorphisation) : aucun appel virtuel ne fausse les mesures. Un `dyn ObjectStore` aurait ajouté un appel indirect à chaque accès, comme un appel d'interface Java.
- **Après l'essai**, le trait est supprimé : le Store retenu devient le type concret `Store`, et le constructeur et le lecteur ne sont plus génériques (Decision 12).
- `children` rend une tranche (`&[ObjectId]`) dans tous les cas. Pour une référence simple, elle contient zéro ou un élément.
- `Arc<Model>` partage le modèle compilé, immuable, entre le Store, le constructeur et plus tard la présentation. C'est un compteur de références atomique, comme un `std::shared_ptr` sur un objet constant. Il est `Send`, donc le Store l'est aussi (spécification « Transfert entre threads »). Un test le vérifie à la compilation, avec `fn assert_send<T: Send>()`.
- Accéder à un objet supprimé, ou utiliser l'attribut d'une autre classe, est une erreur de programmation qui panique. C'est pour cela qu'existe `is_alive`, par exemple pour une sélection conservée. Les erreurs que l'on doit pouvoir traiter (classe abstraite, non conforme, mauvais type, référence simple occupée) sont des `StoreError`.
- Pas d'accesseurs typés (`get_f64`…) au départ. Le `match` sur la valeur rendue par `get` disparaît en général une fois le code inliné. Si les mesures montrent un coût du décodage, on en ajoutera.

### 6. Les deux dispositions

**Commun** : la table des objets est indexée par `ObjectId`. Les listes de contenance sont dans une table à part, `Vec<Vec<ObjectId>>`. La case d'une référence multiple contient le numéro de sa liste, créée au premier enfant. Supprimer un objet :
- le retire de la liste de son conteneur (`Vec::remove`, qui décale les suivants) ;
- marque morts l'objet et tous ses descendants, en parcourant le sous-arbre avec une pile explicite plutôt que par récursion.

Les identifiants ne sont jamais réutilisés. La place des enregistrements morts n'est pas réutilisée non plus dans ce changement (voir Non-Goals).

**V1 : un bloc par objet** (`BoxedStore`). Même principe que `DynamicEObjectImpl` d'EMF, qui range ses valeurs dans un tableau propre à chaque objet.

```
 objets : Vec<Option<Entry>>          Entry { class, container, slots: Box<[Slot]> }
                                                                   |
                                      +-----------+-----------+----v------+-----------+
                                      | Str  16 o | Date 16 o | Float 16o | Many 16 o |
                                      +-----------+-----------+-----------+-----------+
```

- `enum Slot { Unset, Int(i64), Float(f64), Bool(bool), Str(StrRef), Enum(u32), Date(i64), DateTime(i64), One(Option<ObjectId>), Many(u32) }`, soit 16 octets : 8 de valeur, plus l'étiquette et l'alignement.
- Une allocation par objet. Supprimer un objet libère son bloc.

**V2 : un tas d'objets** (`HeapStore`). Même principe que le tas d'une JVM : chaque objet est un enregistrement dont les champs sont placés d'après sa classe, et les objets sont alloués les uns à la suite des autres.

```
 emplacements : Vec<u64>  (bloc, position)          blocs de mots de 8 octets (8 Mio chacun)
      ObjectId ---------------------------------->  +--------+--------+------+------+------+
                                                    | en-tete| masque | case | case | case | ...
                                                    +--------+--------+------+------+------+
   en-tete : classe (16 bits), reference du conteneur (16 bits), conteneur (32 bits)
   masque  : 1 bit par attribut renseigne (un mot par tranche de 64 attributs)
   cases   : attributs encodes selon leur type (Decision 3), puis references
```

- Les objets sont rangés dans l'ordre de leur création, donc dans l'ordre du fichier, en profondeur : un sous-arbre est d'un seul tenant.
- Une case n'a pas d'étiquette : son type se lit dans le modèle compilé. Le masque distingue « non renseigné » de « zéro ».
- Les blocs ont une capacité fixe et ne sont jamais réalloués, comme ceux de l'arène des chaînes. Un enregistrement ne chevauche jamais deux blocs.
- La table des emplacements fait le lien entre un `ObjectId` et la position de son enregistrement, comme les *handles* des premières JVM. Elle coûte 8 octets par objet, mais permettra de compacter le tas sans changer les identifiants.
- Aucune allocation par objet, à part les listes de contenance.

**Estimation pour un relevé** (3 attributs et une référence multiple) : en V1, environ 32 octets dans la table des objets et un bloc de 64 octets plus l'en-tête de malloc ; en V2, un enregistrement de 48 octets et un emplacement de 8 octets. Dans les deux cas, il faut ajouter 4 octets dans la liste du conteneur.

### 7. Le constructeur

```rust
pub struct Builder<S: ObjectStore> { store: S, stack: Vec<Frame> }
impl<S: ObjectStore> Builder<S> {
    pub fn new(model: Arc<Model>) -> Self;
    pub fn begin_root(&mut self, class: ClassId) -> Result<(), LoadError>;
    pub fn begin_reference(&mut self, reference: RefId) -> Result<(), LoadError>;
    pub fn begin_object(&mut self, class: ClassId) -> Result<(), LoadError>;
    pub fn set_text(&mut self, attr: AttrId, text: &str) -> Result<(), LoadError>;
    pub fn set_int(&mut self, attr: AttrId, value: i64) -> Result<(), LoadError>;
    pub fn set_float(&mut self, attr: AttrId, value: f64) -> Result<(), LoadError>;
    pub fn set_bool(&mut self, attr: AttrId, value: bool) -> Result<(), LoadError>;
    pub fn set_null(&mut self, attr: AttrId) -> Result<(), LoadError>;
    pub fn end_object(&mut self);
    pub fn end_reference(&mut self);
    pub fn current_class(&self) -> ClassId; // lets a reader resolve keys
    pub fn finish(self) -> S;
}
```

- **Appels poussés** : le lecteur appelle le constructeur dans l'ordre du fichier, comme un `ContentHandler` SAX. La pile (`Frame::Object` ou `Frame::Reference` avec la position courante dans la liste) donne le contexte. Elle donne aussi le chemin d'une erreur, sans qu'il soit calculé tant qu'il n'y a pas d'erreur.
- **Conversion des textes** (`set_text`), selon le type de l'attribut (spécification « Conversion des valeurs texte ») :
  - `str::parse` pour `i64`, `f64` et `bool` ;
  - `jiff::civil::Date` et `jiff::Timestamp` par `FromStr`. `Timestamp` exige un décalage (`Z` ou `+01:00`). Un instant dont `subsec_nanosecond() % 1000 != 0` est refusé : il n'est pas représentable à la microseconde, et le tronquer perdrait de l'information ;
  - le littéral d'un énuméré est cherché par nom dans la liste de l'énuméré, en général courte.

  C'est la seule conversion de texte du système : un lecteur XML l'utilisera telle quelle.
- **Valeurs typées** (`set_int`, `set_float`, `set_bool`) : un entier est accepté pour un `Float`. Toute autre combinaison est une erreur de type, sans conversion implicite vers le texte (spécification « Valeurs déjà typées »).
- **Erreurs** : `LoadError { path: DataPath, kind: LoadErrorKind }`.
  - `kind` vaut `UnknownKey`, `WrongValueType { attribute, expected }`, `UnknownClass`, `AbstractClass`, `NonConformingClass`, `ReferenceAlreadySet` ou `Syntax { line, column }`.
  - `DataPath` s'affiche comme `/stations/0/equipements/1/mesures/2/valeur`.
  - L'erreur ne contient jamais la valeur fautive. Le second changement l'ajoutera au message destiné à l'utilisateur, sans qu'elle aille dans le journal.

### 8. Le lecteur JSON (`ortelia-service`)

```rust
pub fn read_json<S: ObjectStore>(reader: impl Read, model: Arc<Model>, root: ClassId) -> Result<S, LoadError>;
pub fn read_json_slice<S: ObjectStore>(bytes: &[u8], model: Arc<Model>, root: ClassId) -> Result<S, LoadError>;
```

- **Une chaîne de `DeserializeSeed`** : un `Deserialize` qui transporte un état, ici la référence au constructeur et la classe attendue. On a `ObjectSeed` (un objet), `KeySeed` (une clé), `ValueSeed` (la valeur d'un attribut) et `ReferenceSeed` (un tableau d'objets, ou un seul objet).
- **Clés** : `KeySeed` les reçoit par `visit_str`, comme un `&str` provisoire, sans allocation. Il les résout dans la classe courante en `_type_`, attribut, référence ou clé inconnue.
- **Valeurs** : `ValueSeed` aiguille selon ce que serde_json reçoit :
  - `visit_str` vers `set_text`, `visit_i64` et `visit_u64` vers `set_int` (au-delà de `i64::MAX`, c'est une erreur de type) ;
  - `visit_f64` vers `set_float`, `visit_bool` vers `set_bool`, `visit_unit` (null) vers `set_null` ;
  - un objet ou un tableau à la place d'une valeur est une erreur de type.
- **Erreurs du constructeur dans les visiteurs** : serde n'accepte que son propre type d'erreur. L'erreur du constructeur est donc rangée dans le constructeur, puis le visiteur renvoie `de::Error::custom` pour tout arrêter. Au retour, `read_json` rend l'erreur rangée s'il y en a une. Sinon, il convertit l'erreur de serde_json en `Syntax`, avec sa ligne et sa colonne (`Error::line()`, `Error::column()`).
- **Clé inconnue** : erreur immédiate. Le second changement, qui rassemblera les erreurs, sautera la valeur avec `IgnoredAny` et continuera.
- **Flux ou tampon** : `read_json` enveloppe le lecteur dans un `BufReader` de 64 Kio, car serde_json ne bufferise pas lui-même. D'après sa documentation, `from_reader` est en général plus lent que `from_slice` sur un fichier lu en entier (issue serde-json #160). En revanche, le flux évite de garder les 500 Mo du fichier en mémoire, et permettra la progression et l'annulation (un `Read` qui compte les octets). L'essai mesure les deux.

### 9. `_type_` : la classe d'un objet

- Si le type déclaré de la référence n'a pas de sous-classe, la classe est connue d'avance. Un `_type_` éventuel doit nommer cette classe.
- Sinon, si la première clé est `_type_`, on crée l'objet aussitôt avec cette classe. C'est le cas des fichiers qu'Ortelia écrira.
- Sinon, l'objet est lu en entier dans un `serde_json::Map`, puis **rejoué** par le même `ObjectSeed`. `serde_json::Value` sait lui-même se désérialiser (il implémente `Deserializer`), donc un seul code lit les deux cas. Seuls ces objets passent par un arbre intermédiaire. Leur surcoût est mesuré, grâce au réglage `--late-type` du générateur.
- La classe trouvée doit être concrète, et être le type déclaré ou l'une de ses sous-classes (contrôle en temps constant, Decision 2).

### 10. Le modèle d'essai et le générateur

**Modèle d'essai** (`examples/reseau-meteo/reseau.model.json`, package `reseau`). Il prend la forme de données réelles qui mêlent un arbre profond de séries temporelles à des milliers de zones géographiques découpées en mailles :

```
Reseau
 +-- stations [*] : Station          code, nom, latitude, longitude, altitude (Int), mise_en_service (Date),
 |    |                              active (Boolean), categorie (CategorieStation), description (texte long)
 |    +-- equipements [*] : Equipement (abstraite)      numero_serie, modele, installe_le (Date)
 |    |     Capteur (abstraite)      frequence_s (Int)
 |    |       +-- mesures [*] : Mesure (abstraite)      instant (DateTime)
 |    |       |     Releve           valeur (Float), qualite (Qualite)
 |    |       |     Lacune           fin (DateTime), cause
 |    |       |       +-- annotations [*] : Annotation  auteur, texte (texte long), creee_le (DateTime)
 |    |     Thermometre <- Capteur   plage_min, plage_max (Float)
 |    |     Anemometre  <- Capteur   hauteur_mat (Float)
 |    |     Pluviometre <- Capteur   surface_cm2 (Float)
 |    |     Camera      <- Equipement   resolution, orientation_deg (Float)
 |    +-- interventions [*] : Intervention (abstraite)  date (Date), technicien, compte_rendu (texte long)
 |          Maintenance (duree_h)   Panne (gravite : Gravite, resolue)   Etalonnage (ecart)
 +-- zones [*] : ZoneAlerte          code, niveau (NiveauAlerte), debut, fin (DateTime)
      +-- emprise [1] : Emprise (abstraite)
      |     Polygone                 sommets [*] : Point
      |     Cercle                   rayon_km (Float), centre [1] : Point
      +-- mailles [*] : Maille       numero (Int), statut (StatutMaille), acquise_le (DateTime), coins [*] : Point
Point                                lat, lon (Float)
```

- La référence `annotations` est déclarée dans `Mesure`, donc portée par `Releve` et `Lacune`.
- On obtient une profondeur 4 sous la racine (`Station` → `Thermometre` → `Releve` → `Annotation`), des listes polymorphes (`equipements`, `mesures`, `interventions`), un héritage sur trois niveaux (`Thermometre` → `Capteur` → `Equipement`) et une référence simple de type abstrait (`emprise`).
- Énumérés : `CategorieStation`, `Qualite`, `Gravite`, `NiveauAlerte`, `StatutMaille`.
- `reseau-exemple.json` est un petit fichier écrit à la main : deux stations, un équipement de chaque classe, des relevés, une lacune annotée, une intervention de chaque classe, une zone `Polygone` et une zone `Cercle` avec leurs mailles, et un objet dont `_type_` n'est pas la première clé.

**Générateur** (`cargo xtask gen-data`) :

```
cargo xtask gen-data --size 500M --out ~/s1/reseau-texte.json [--seed 1] [--text 1.0] [--late-type 0.0]
```

- `--size` accepte les suffixes `K`, `M` et `G`. `--text` multiplie la longueur des textes longs ; à 0, ils sont omis. `--late-type` est la part d'objets des listes polymorphes dont `_type_` vient après le premier attribut.
- **Écriture en flux** (`BufWriter`), à travers un `Write` qui compte les octets. Environ 85 % de la taille vont aux stations, 15 % aux zones. Le budget est vérifié avant chaque objet de la liste en cours, et quand il est atteint, le générateur ferme proprement les tableaux et les objets ouverts. La taille dépasse donc la cible de moins d'un objet.
- **Répartitions** :
  - stations : 3 à 8 équipements, dont environ 80 % de capteurs ; 50 à 400 mesures par capteur (97 % de relevés, 3 % de lacunes) ; une annotation sur 1 % des mesures ; 0 à 6 interventions ;
  - zones : 80 % de polygones, de 50 à 5 000 sommets, et 20 % de cercles ; de 1 à 1 000 mailles, environ 5,5 en moyenne (loi géométrique, plus 1 % de zones à grand nombre de mailles) ; 4 coins par maille.
- **Valeurs** : noms de villes et vocabulaire pris dans des listes fixes, dates et dates-heures construites à partir de composantes tirées au hasard (jours de 1 à 28 pour rester valides).
- **Textes** : écrits par `serde_json::to_writer`, qui échappe correctement les caractères spéciaux. Les nombres utilisent `Display` de Rust, qui donne la représentation la plus courte relue à l'identique.
- **Reproductible** : `fastrand::Rng::with_seed(graine)`, dont la suite est fixée par la graine sur toutes les plateformes (documentation de fastrand). Aucune itération sur un `HashMap`, dont l'ordre change d'une exécution à l'autre.
- **Tests de `xtask`** :
  - deux générations identiques ;
  - taille atteinte ;
  - fichier généré chargé avec le modèle d'essai, grâce aux dépendances de développement `ortelia-model`, `ortelia-service` et `tempfile` ;
  - absence des textes longs avec `--text 0`.

### 11. Le programme de mesure (exemple `s1` de `ortelia-service`)

```
cargo run --release -p ortelia-service --example s1 -- all ~/s1/reseau-texte.json
```

- **Un processus par cas** : `all` relance le programme lui-même (`std::env::current_exe()`) avec `case <implémentation> <mode> <fichier>`. Chaque cas écrit ses résultats sur une ligne, et `all` les rassemble dans un tableau Markdown. Un processus neuf par cas donne un pic de mémoire propre à ce cas.
- **Cas** :

  | Implémentation | Mode de lecture | Mesures |
  |---|---|---|
  | plancher (`IgnoredAny`) | flux, tampon | chargement, mémoire |
  | V0 (`serde_json::Value`) | flux, tampon | chargement, mémoire |
  | V1, V2 | flux | chargement, mémoire, accès, modification |
  | V1, V2 | tampon | chargement, mémoire |
  | V1, V2 avec comptage des allocations | flux | allocations |

- **Mesures** :

  | Code | Mesure |
  |---|---|
  | C1 | temps de chargement |
  | M1 | pic de mémoire du processus (`VmHWM` de `/proc/self/status`) |
  | M2 | mémoire après chargement (`VmRSS`), rapportée à la taille du fichier |
  | M3 | nombre d'allocations et octets alloués pendant le chargement |
  | A1 | lecture au hasard d'un décimal (`valeur` d'un relevé), en ns par accès |
  | A2 | parcours en profondeur de tout l'arbre, un attribut lu par objet, en ns par objet |
  | A3 | `numero_serie` lu sur toutes les listes `equipements` (liste polymorphe, attribut hérité) |
  | A4 | extraction de tous les sommets (polygones et coins des mailles) dans un tampon plat de `f64`, en ms par million de sommets |
  | A5 | remontée jusqu'à la racine depuis des `Point` pris au hasard, en ns par niveau |
  | A6 | 50 stations × 8 attributs mis en forme en texte (fenêtre visible d'une table), en µs |
  | A7 | modifier un coin, remonter à sa zone, réextraire la zone (invalidation ciblée), en µs |
  | E1 | 1 million de `set` au hasard (décimal, entier, texte), avec la croissance de `VmRSS` |
  | E2 | insérer puis supprimer 1 000 objets au milieu d'une liste de 100 000 `Point` construite pour la mesure |
  | E3 | suppression d'une station et de tout son sous-arbre |
  | E4 | création de 1 000 mailles de 4 coins sous une zone (comme un découpage en mailles) |

- **Méthode** :
  - compilation release ;
  - pour A et E : 5 passes, médiane retenue, `std::hint::black_box` pour que le compilateur ne supprime pas les calculs ;
  - tirages au hasard par fastrand avec une graine fixe, dans l'ordre du fichier pour A2, au hasard pour A1 et A5.
- **Mémoire** : lue dans `/proc/self/status` sous `#[cfg(target_os = "linux")]`. Ailleurs, elle est indiquée « non disponible », et l'exemple compile pour Windows (Clippy de la CI).
- **Comptage des allocations** (M3) : un allocateur global qui enveloppe `std::alloc::System` et compte par des atomiques. Il n'est actif que dans les cas « comptage » (un `AtomicBool`), pour ne pas fausser les temps.
  - `GlobalAlloc` est un trait `unsafe`. Ce module de l'exemple porte donc `#![allow(unsafe_code)]`, avec un commentaire qui explique pourquoi : il ne fait que relayer à `System`.
  - Alternatives écartées : `dhat` enregistre une pile d'appels par allocation, beaucoup trop lent pour des millions d'objets ; aucune crate de simple comptage n'est documentée dans Context7.
- **Tests de l'exemple** : `[[example]] name = "s1"`, `test = true` dans `Cargo.toml`. `cargo test` exécute alors les tests de l'exemple, qui lancent chaque mesure sur `reseau-exemple.json` pour V1 et V2, et vérifient le rapport quand la mémoire n'est pas disponible (spécification `storage-benchmark`).

### 12. Décision et nettoyage

- **Fichiers mesurés** : deux profils de 500 Mo, « texte » (`--text 1`) et « numérique » (`--text 0`), plus un fichier à `--late-type 0.1` pour le surcoût de `_type_` tardif. V0 est mesuré sur un fichier de 100 Mo : sur 500 Mo, il dépasserait la mémoire de WSL2 (environ 9 fois la taille du fichier, d'après l'estimation).
- **Critères**, dans l'ordre : accès et navigation (A1 à A7) ; mémoire (M1 à M3) ; chargement (C1) ; à résultats proches, la variante la plus simple. Les seuils sont fixés au vu des résultats, et ils sont écrits dans le document de résultats avec leur justification.
- **`docs/essais/s1-stockage.md`** :
  - conditions (processeur, mémoire, version de WSL2 et de Rust, commit) ;
  - fichiers (taille, nombre d'objets par classe) ;
  - tableau des mesures pour chaque profil ;
  - analyse, choix, et ce qu'il implique pour la suite.
- **Nettoyage**, une fois le choix fait :
  - suppression du module écarté ;
  - suppression du trait `ObjectStore` : le Store retenu devient le type concret `Store`, et le constructeur et le lecteur cessent d'être génériques ;
  - la suite de tests commune devient celle du Store ;
  - le programme de mesure garde le plancher, V0 et le Store retenu.

### 13. Versions des bibliothèques

API vérifiées dans Context7 et dans les sources des crates téléchargées dans WSL2 :

| Bibliothèque | Version | Crate(s) | API utilisée |
|---|---|---|---|
| serde | 1.0.229, feature `derive` | `ortelia-model`, `ortelia-service` | `Deserialize` (dérivé), `DeserializeSeed`, `Visitor`, `MapAccess`, `SeqAccess`, `de::Error::custom`, `IgnoredAny` |
| serde_json | 1.0.154 | `ortelia-model`, `ortelia-service`, `xtask` | `from_reader`, `from_slice`, `Deserializer`, `Value` et `Map` (rejeu), `Error::line` / `column`, `to_writer` |
| jiff | 0.2.43, sans les features par défaut, feature `std` | `ortelia-model` | `civil::Date` (`FromStr`, `new`, `year`, `month`, `day`), `Timestamp` (`FromStr`, `from_microsecond`, `as_microsecond`, `subsec_nanosecond`) |
| fastrand | 2.5.0 | `xtask` ; `ortelia-service` (dev, exemple) | `Rng::with_seed`, `u32`, `f64`, `bool` |
| tempfile | 3.27 (dev) | `xtask` | dossier temporaire du test de chargement |

- jiff 0.2 plutôt que 1.0 : `egui_extras` 0.37, dont le sélecteur de date servira au formulaire, dépend de jiff 0.2.35 ou plus. Deux versions de jiff seraient compilées, et leurs types de date ne seraient pas compatibles.
- Sans les features par défaut, jiff n'embarque ni la base des fuseaux horaires ni l'accès au fuseau du système : seuls des dates civiles et des instants UTC servent ici. Si une API utilisée l'exigeait, la compilation le dirait (tâche 1.1).
- serde, serde_json et fastrand figurent déjà dans `Cargo.lock` comme dépendances indirectes : les versions retenues sont celles-là.

### 14. Documentation

- `CLAUDE.md` : arborescence (`examples/reseau-meteo/`, `docs/essais/`), commandes `cargo xtask gen-data` et du programme de mesure, état du projet.
- `docs/feuille-de-route-annuaire.md` : résultat de S1 et lien vers `docs/essais/s1-stockage.md`.

## Risks / Trade-offs

- [Sur 500 Mo, V0 dépasse la mémoire de WSL2] → V0 mesuré sur 100 Mo et extrapolé, ce que le document de résultats indique.
- [Les mesures dans WSL2 diffèrent de celles d'un Windows natif] → les rapports entre variantes comptent plus que les valeurs absolues ; les conditions sont consignées. Un passage dans le bac à sable Windows reste possible si une mesure est douteuse.
- [Le générateur dérive du modèle d'essai, par exemple si un attribut est renommé d'un côté seulement] → un test de `xtask` charge un fichier généré avec le modèle.
- [Le contrôle d'appartenance de l'attribut à chaque accès coûte un peu] → il est le même dans les deux variantes, donc l'essai reste équitable ; son coût apparaît dans A1. On le gardera de toute façon : sans lui, une erreur de programmation lirait la case d'un autre type.
- [Les objets supprimés ne libèrent pas leur place dans V2, alors que V1 libère ses blocs] → E3 ne mesure que le temps. La mémoire après suppressions n'est pas un critère de l'essai, et la récupération de place sera décidée avec la liste des changements.
- [Le rejeu de `_type_` tardif passe par `serde_json::Map`, trié par clé] → l'ordre des attributs ne compte pas, et l'ordre des enfants est celui des tableaux, qui est conservé.
- [Un trait et des fonctions génériques compliquent la lecture pendant l'essai] → ils sont temporaires, et leur suppression fait partie des tâches.
- [La première erreur arrête le chargement, alors que le second changement doit toutes les rassembler] → le chemin d'erreur et la pile du constructeur sont conçus pour continuer après une erreur ; seul le lecteur devra sauter la valeur fautive (`IgnoredAny`).

## Migration Plan

Pas de migration : `ortelia-model` et `ortelia-service` sont vides. Le changement est livré par commits successifs sur une branche, chacun compilable et testé dans WSL2. Retour arrière : annuler les commits.

## Open Questions

- Taille exacte des blocs (1 Mio pour l'arène, 8 Mio pour le tas de V2) : réglable sans effet sur l'API, à ajuster si les mesures le suggèrent.
- Faut-il garder dans le dépôt un fichier généré de quelques centaines de kilo-octets, pour des tests de performance futurs ? À décider avec ces tests.
