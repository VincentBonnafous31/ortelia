//! `cargo xtask gen-data --model annuaire`: writes a whole directory application in a folder,
//! with as many persons as requested, for the checks of the table view.
//!
//! ```text
//! cargo xtask gen-data --model annuaire --count 100000 --out ~/annuaire-100k [--seed 1]
//! ```
//!
//! - `app.json` and `annuaire.model.json` are those of `examples/annuaire`, included in the
//!   program at compile time (`include_str!`, like a resource in a jar).
//! - `personnes.json` is written as a stream, by the JSON writer of `gen_data`.
//! - Reproducible: same seed and count, same file (`fastrand::Rng::with_seed`).

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::gen_data::{Json, date, decimal, pick};

pub const USAGE: &str =
    "usage: cargo xtask gen-data --model annuaire --count <n> --out <folder> [--seed <n>]

  --model      annuaire: the application of examples/annuaire (without --model: the network of
               the S1 trial, see above)
  --count      number of persons
  --out        folder to write app.json, annuaire.model.json and personnes.json in (created if
               needed; use the Linux file system, e.g. ~/annuaire-100k, not /mnt/c)
  --seed       seed of the random generator (default 1)";

/// The settings of a generation.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub count: u64,
    pub out: PathBuf,
    pub seed: u64,
}

/// Parses the arguments that follow `gen-data`, `--model annuaire` included.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut model = None;
    let mut count = None;
    let mut out = None;
    let mut seed = 1;
    let mut iter = args.iter();
    while let Some(option) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| format!("{option}: missing value"))?;
        match option.as_str() {
            "--model" => model = Some(value.as_str()),
            "--count" => {
                count = Some(
                    value
                        .parse()
                        .map_err(|_| format!("--count: invalid number `{value}`"))?,
                )
            }
            "--out" => out = Some(PathBuf::from(value)),
            "--seed" => {
                seed = value
                    .parse()
                    .map_err(|_| format!("--seed: invalid number `{value}`"))?
            }
            _ => return Err(format!("unknown option `{option}`")),
        }
    }
    match model {
        Some("annuaire") => {}
        Some(other) => return Err(format!("--model: unknown model `{other}` (annuaire)")),
        None => return Err("--model is required".to_owned()),
    }
    Ok(Options {
        count: count.ok_or("--count is required")?,
        out: out.ok_or("--out is required")?,
        seed,
    })
}

const APP_JSON: &str = include_str!("../../examples/annuaire/app.json");
const MODEL_JSON: &str = include_str!("../../examples/annuaire/annuaire.model.json");

/// Writes the application in the folder of `options`, and returns the size of `personnes.json`.
pub fn run(options: &Options) -> io::Result<u64> {
    fs::create_dir_all(&options.out)?;
    fs::write(options.out.join("app.json"), APP_JSON)?;
    fs::write(options.out.join("annuaire.model.json"), MODEL_JSON)?;
    let persons = persons_file(&options.out);
    let file = BufWriter::with_capacity(1 << 20, File::create(persons)?);
    generate(file, options)
}

/// The data file of the application, in its folder.
pub fn persons_file(folder: &Path) -> PathBuf {
    folder.join("personnes.json")
}

/// Writes the persons to `out`, and returns the size in bytes.
pub fn generate<W: Write>(out: W, options: &Options) -> io::Result<u64> {
    let mut generator = Generator {
        json: Json::new(out),
        rng: fastrand::Rng::with_seed(options.seed),
    };
    generator.json.open(b"[")?;
    for _ in 0..options.count {
        generator.person()?;
    }
    generator.json.close(b"]")?;
    let mut writer = generator.json.out;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(writer.count)
}

// Accents, particles, apostrophes and capitals inside the names: what the sort and the filter
// of the table must handle.
const LAST_NAMES: &[&str] = &[
    "Martin",
    "Bernard",
    "Dubois",
    "Thomas",
    "Robert",
    "Richard",
    "Petit",
    "Durand",
    "Leroy",
    "Moreau",
    "Simon",
    "Laurent",
    "Lefèvre",
    "Michel",
    "Garcia",
    "David",
    "Bertrand",
    "Roux",
    "Fournier",
    "Girard",
    "André",
    "Mercier",
    "Dupont",
    "Lambert",
    "Bonnet",
    "François",
    "Légaré",
    "Éluard",
    "Bérenger",
    "Château",
    "Héraud",
    "de La Tour",
    "de la Fontaine",
    "Le Goff",
    "Le Gall",
    "Du Bellay",
    "d'Aubigné",
    "N'Diaye",
    "MacDonald",
    "DuBois",
    "Saint-Exupéry",
    "O'Neill",
    "van der Berg",
];
const FIRST_NAMES: &[&str] = &[
    "Émile",
    "Élodie",
    "Chloé",
    "Inès",
    "Léa",
    "Zoé",
    "Hélène",
    "Jérôme",
    "François",
    "Noël",
    "Anaïs",
    "Gaëlle",
    "Jean-Luc",
    "Marie-Ève",
    "Théo",
    "Lucas",
    "Hugo",
    "Camille",
    "Julien",
    "Antoine",
    "Sophie",
    "Nathalie",
    "Pierre",
    "Louis",
    "Manon",
    "Clément",
    "Maëlys",
    "Loïc",
    "Bérénice",
    "Benoît",
];
const SITUATIONS: &[&str] = &["CELIBATAIRE", "MARIE", "PACSE", "DIVORCE", "VEUF"];
const NOTES: &[&str] = &[
    "A déménagé récemment.",
    "Préfère être contacté par courriel.",
    "Bénévole lors de la fête de quartier.",
    "Nouvelle adresse à vérifier.",
    "Référent de l'association depuis 2019.",
    "Ne souhaite pas recevoir la lettre d'information.",
];

struct Generator<W> {
    json: Json<W>,
    rng: fastrand::Rng,
}

impl<W: Write> Generator<W> {
    /// A person: `nom` always (it is required), the other attributes absent in a fixed share
    /// of the persons.
    fn person(&mut self) -> io::Result<()> {
        // One person per line: the file stays readable with a text editor.
        self.json.separator()?;
        self.json.raw("\n")?;
        self.json.open(b"{")?;
        let nom = pick(&mut self.rng, LAST_NAMES);
        self.json.field_str("nom", nom)?;
        if self.present(95) {
            let prenom = pick(&mut self.rng, FIRST_NAMES);
            self.json.field_str("prenom", prenom)?;
        }
        if self.present(90) {
            let birth = date(&mut self.rng, 1930, 2010);
            self.json.field_str("date_naissance", &birth)?;
        }
        if self.present(85) {
            // Mostly few children.
            let children = [0, 0, 0, 1, 1, 2, 2, 3, 4, 5][self.rng.usize(..10)];
            self.json.field_raw("nombre_enfants", children)?;
        }
        if self.present(85) {
            let height = decimal(&mut self.rng, 1.45, 2.05, 2);
            self.json.field_raw("taille", height)?;
        }
        if self.present(90) {
            let contact = self.rng.bool();
            self.json.field_raw("contact_autorise", contact)?;
        }
        if self.present(80) {
            let situation = pick(&mut self.rng, SITUATIONS);
            self.json.field_str("situation", situation)?;
        }
        if self.present(95) {
            let day = date(&mut self.rng, 2015, 2026);
            let (h, m, s) = (
                self.rng.u32(0..24),
                self.rng.u32(0..60),
                self.rng.u32(0..60),
            );
            self.json
                .field_str("inscription", &format!("{day}T{h:02}:{m:02}:{s:02}Z"))?;
        }
        if self.present(30) {
            let note = pick(&mut self.rng, NOTES);
            self.json.field_str("notes", note)?;
        }
        self.json.close(b"}")
    }

    /// `true` for `percent` % of the calls.
    fn present(&mut self, percent: u32) -> bool {
        self.rng.u32(0..100) < percent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn generate_folder(count: u64, seed: u64) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        let options = Options {
            count,
            out: dir.path().join("annuaire"),
            seed,
        };
        run(&options).expect("generate");
        dir
    }

    fn persons(folder: &Path) -> Vec<serde_json::Value> {
        let text = fs::read_to_string(persons_file(folder)).expect("read");
        match serde_json::from_str(&text).expect("valid JSON") {
            serde_json::Value::Array(persons) => persons,
            other => panic!("not an array: {other}"),
        }
    }

    #[test]
    fn parses_the_options() {
        let parsed = parse_args(&args(&[
            "--model", "annuaire", "--count", "1000", "--out", "dir", "--seed", "3",
        ]))
        .expect("parse");
        assert_eq!(
            parsed,
            Options {
                count: 1000,
                out: PathBuf::from("dir"),
                seed: 3
            }
        );
        let error = parse_args(&args(&["--model", "annuaire", "--out", "dir"])).unwrap_err();
        assert!(error.contains("--count"), "{error}");
        let error = parse_args(&args(&["--model", "x", "--count", "1", "--out", "d"])).unwrap_err();
        assert!(error.contains("unknown model"), "{error}");
        let error = parse_args(&args(&["--model", "annuaire", "--size", "1M"])).unwrap_err();
        assert!(error.contains("--size"), "{error}");
    }

    /// Scenario "Application ouverte".
    #[test]
    fn generated_application_opens() {
        use ortelia_service::app::{AppContext, SourceStatus};

        let dir = generate_folder(1000, 1);
        let folder = dir.path().join("annuaire");
        let context =
            AppContext::open(&folder.join("app.json")).unwrap_or_else(|error| panic!("{error}"));
        assert!(context.warnings().is_empty(), "{:?}", context.warnings());
        for source in context.sources() {
            match &source.status {
                SourceStatus::Loaded {
                    diagnostic_count, ..
                } => assert_eq!(*diagnostic_count, 0, "{:?}", source.status),
                SourceStatus::Failed(error) => panic!("{error}"),
            }
        }
        let document = context.document();
        let store = document.store();
        let model = store.model();
        let racine = model.class(model.class_id("Racine").expect("Racine"));
        let annuaire = racine.reference("annuaire").expect("annuaire");
        let annuaire_class = model.class(model.class_id("Annuaire").expect("Annuaire"));
        let personnes = annuaire_class.reference("personnes").expect("personnes");
        let annuaire = store.children(document.root(), annuaire)[0];
        assert_eq!(store.children(annuaire, personnes).len(), 1000);
    }

    /// Scenario "Même graine, mêmes fichiers".
    #[test]
    fn same_seed_gives_the_same_file() {
        let first = generate_folder(500, 7);
        let second = generate_folder(500, 7);
        let read = |dir: &tempfile::TempDir| {
            fs::read(persons_file(&dir.path().join("annuaire"))).expect("read")
        };
        assert_eq!(read(&first), read(&second));
        let other = generate_folder(500, 8);
        assert_ne!(read(&first), read(&other));
    }

    /// Scenario "Noms variés".
    #[test]
    fn names_are_varied() {
        let dir = generate_folder(1000, 1);
        let persons = persons(&dir.path().join("annuaire"));
        assert_eq!(persons.len(), 1000);
        let names: Vec<&str> = persons
            .iter()
            .flat_map(|p| [p.get("nom"), p.get("prenom")])
            .flatten()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(names.iter().any(|n| !n.is_ascii()), "no accent");
        // A capital after the first letter (`de La Tour`, `MacDonald`).
        assert!(
            names
                .iter()
                .any(|n| n.chars().skip(1).any(char::is_uppercase)),
            "no capital inside a name"
        );
        assert!(persons.iter().any(|p| p.get("situation").is_none()));
        assert!(persons.iter().all(|p| p.get("nom").is_some()));
    }

    /// The application and the model are those of `examples/annuaire`.
    #[test]
    fn app_and_model_are_copied() {
        let dir = generate_folder(1, 1);
        let folder = dir.path().join("annuaire");
        assert_eq!(
            fs::read_to_string(folder.join("app.json")).unwrap(),
            APP_JSON
        );
        assert_eq!(
            fs::read_to_string(folder.join("annuaire.model.json")).unwrap(),
            MODEL_JSON
        );
    }
}
