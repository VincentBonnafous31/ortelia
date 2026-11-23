# CLAUDE.md

Ce fichier guide Claude Code (claude.ai/code) dans ce dépôt.

@docs/cadrage-prive/CLAUDE-prive.md

## Projet

**Ortelia** : outil desktop (Windows, Linux) pour construire **sans code** des applications de visualisation et d'édition de données structurées. Un ingénieur métier décrit, dans un mode design, un modèle de données (métamodèle inspiré d'Ecore), les sources, la disposition des fenêtres et les vues ; le mode run exécute l'application. Le code spécifique (imports de fichiers particuliers, calculs) passe par des plugins natifs.

**Priorités** : rapidité et faible consommation mémoire. Fichiers jusqu'à environ 500 Mo, tables jusqu'à 100 000 lignes.

**État** : phase d'affinage, avant l'incrément 0. Il n'y a pas encore de code ni de workspace Cargo.

## Documents

| Fichier | Contenu |
|---|---|
| `docs/feuille-de-route.md` | Incrément 0, puis trois applications jalons : annuaire, éditeur de modèles, mode design |
| `docs/architecture-couches.md` | Couches, couche de présentation sans dépendance UI, interfaces egui / GPUI interchangeables (D25–D27) |
| `examples/annuaire/` | Application 1 : `app.json` (format 0.2), `annuaire.model.json`, `personnes.json` |
| `examples/model-editor/metamodel.model.json` | Ébauche du métamodèle décrit avec lui-même (application 2) |

`docs/cadrage-prive/` contient des notes de conception privées : ne jamais le versionner, ne jamais citer son contenu dans un fichier versionné.

## Règles

### Langues

- Code, identifiants et commentaires du code : **anglais**.
- Spécifications et documents de conception : **français**.
- Interface d'Ortelia : français et anglais, au choix de l'utilisateur.

### Architecture

- Règle des couches (`docs/architecture-couches.md`) : `ortelia-model` ← `ortelia-config` / `ortelia-expr` ← `ortelia-service` ← `ortelia-presentation` ← `ortelia-ui-*` ← `ortelia`. Aucune crate sous `ortelia-ui-*` ne dépend d'egui ou de GPUI.
- Toute la logique d'affichage (tri, filtre, formulaires générés, broker, intentions, formats, état de session, mode design) va dans `ortelia-presentation`, testée sans interface.
- Les formats de configuration doivent être descriptibles par le métamodèle : contenance, références par nom (`refersTo`), polymorphisme par `_type_`, pas de dictionnaires.

### Style Rust

Le porteur du projet débute en Rust et connaît bien C, C++ et Java.
- Écrire un Rust idiomatique mais lisible : pas de généricité ni de macros superflues, et des commentaires sur ce qui n'est pas évident.
- Expliquer les choix par analogie avec C++ ou Java quand c'est utile.

### Documentation des bibliothèques

Toujours consulter **Context7** (serveur MCP déclaré dans `.mcp.json`) avant d'écrire du code qui utilise une bibliothèque. En particulier, egui 0.37 est plus récent que les connaissances du modèle et son API change à chaque version mineure.

## Méthode : OpenSpec

- Développement piloté par les spécifications avec **OpenSpec**, à installer à l'incrément 0 : `npm install -g @fission-ai/openspec`, puis `openspec init`.
- `openspec/config.yaml` déclarera le français pour les artefacts, en gardant en anglais les titres structurels (`### Requirement:`, `#### Scenario:`) et les mots-clés `SHALL` / `MUST`.
- Cycle d'un incrément : `/opsx:explore` (réfléchir), `/opsx:propose <nom>` (proposition, spécifications, conception, tâches, à faire valider), `/opsx:apply` (réaliser), `/opsx:archive` (fusionner dans `openspec/specs/`).
- Après une modification non triviale, et avant de la considérer terminée : `/code-review` sur le diff, puis `/simplify`, puis recompiler.

## Git

- Branche `main`. Pas encore de dépôt distant : GitHub est prévu, avec CI GitHub Actions (Linux et Windows).
- Ne committer ou pousser que sur demande.

## Conventions egui 0.37

L'API d'egui change à chaque version mineure : vérifier dans Context7 ou dans les sources des crates (`~/.cargo/registry/src/*/egui-0.37.*`) plutôt que de supposer.

- Mode immédiat : l'interface est redessinée à chaque image par `eframe::App::ui(&mut self, ui, frame)` (et non plus `update(ctx)`). L'état vit dans l'application, pas dans les widgets.
- Panneaux : `egui::Panel::left(..)` / `Panel::bottom(..)` puis `CentralPanel`, tous affichés avec `.show(ui, ..)`. `SidePanel`, `TopBottomPanel` et `show_inside` sont obsolètes.
- Table : `egui_extras::TableBuilder`. `body.rows(..)` ne dessine que les lignes visibles (testé avec 50 000 lignes). egui mémorise les largeurs sous l'`id_salt` de la table et repart de `Column::initial` quand le nombre de colonnes change.
- `DatePickerButton` travaille sur des `jiff::civil::Date` (pas chrono).
- Les polices emoji et symboles (⏷, 📆…) ne sont plus incluses par défaut : activer la feature `monochrome_emoji_fonts` de la crate `egui`, sinon elles s'affichent en carrés vides sous Linux/WSL.
- Décorations de fenêtre natives ; titre via `ViewportBuilder::with_title`.
