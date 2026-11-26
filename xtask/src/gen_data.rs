//! `cargo xtask gen-data`: writes a JSON file that conforms to the test model of the weather
//! network (`examples/reseau-meteo/reseau.model.json`), of a requested size, for the S1 trial.
//!
//! ```text
//! cargo xtask gen-data --size 500M --out ~/s1/reseau-texte.json [--seed 1] [--text 1.0] [--late-type 0.0]
//! ```
//!
//! - The file is written as a stream, through a writer that counts the bytes. About 85 % of the
//!   size goes to the stations, 15 % to the zones. The budget is checked before each object of
//!   the current list; once it is reached, the open arrays and objects are closed properly. The
//!   file therefore exceeds the target by less than one object.
//! - Reproducible: `fastrand::Rng::with_seed`, whose sequence is fixed by the seed on every
//!   platform, and no iteration over a `HashMap`.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

pub const USAGE: &str = "usage: cargo xtask gen-data --size <N[K|M|G]> --out <file> [--seed <n>] [--text <factor>] [--late-type <share>]

  --size       target size; K, M and G are 1024, 1024^2 and 1024^3 bytes
  --out        file to write (use the Linux file system, e.g. ~/s1/, not /mnt/c)
  --seed       seed of the random generator (default 1)
  --text       factor on the length of long texts (default 1.0; 0 omits them)
  --late-type  share of objects of polymorphic lists whose `_type_` comes after their first
               attribute (default 0.0)";

/// The settings of a generation.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub size: u64,
    pub out: PathBuf,
    pub seed: u64,
    pub text: f64,
    pub late_type: f64,
}

/// Parses the arguments that follow `gen-data`.
pub fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut size = None;
    let mut out = None;
    let mut seed = 1;
    let mut text = 1.0;
    let mut late_type = 0.0;
    let mut iter = args.iter();
    while let Some(option) = iter.next() {
        let value = iter
            .next()
            .ok_or_else(|| format!("{option}: missing value"))?;
        match option.as_str() {
            "--size" => size = Some(parse_size(value)?),
            "--out" => out = Some(PathBuf::from(value)),
            "--seed" => {
                seed = value
                    .parse()
                    .map_err(|_| format!("--seed: invalid number `{value}`"))?
            }
            "--text" => text = parse_non_negative("--text", value)?,
            "--late-type" => {
                late_type = parse_non_negative("--late-type", value)?;
                if late_type > 1.0 {
                    return Err("--late-type: a share between 0 and 1".to_owned());
                }
            }
            _ => return Err(format!("unknown option `{option}`")),
        }
    }
    Ok(Options {
        size: size.ok_or("--size is required")?,
        out: out.ok_or("--out is required")?,
        seed,
        text,
        late_type,
    })
}

/// Largest size accepted by `--size`: a typo such as `1e30G` must not fill the disk.
const MAX_SIZE: u64 = 64 << 30;

/// Parses a size such as `200K`, `10M` or `1G` (powers of 1024), or a number of bytes.
fn parse_size(text: &str) -> Result<u64, String> {
    let (number, unit) = match text.char_indices().last() {
        Some((i, 'K' | 'k')) => (&text[..i], 1 << 10),
        Some((i, 'M' | 'm')) => (&text[..i], 1 << 20),
        Some((i, 'G' | 'g')) => (&text[..i], 1 << 30),
        _ => (text, 1),
    };
    let number: f64 = number
        .parse()
        .map_err(|_| format!("--size: invalid size `{text}`"))?;
    let bytes = number * unit as f64;
    // `is_finite` refuses NaN and infinity; the upper bound keeps the cast to `u64` exact.
    if !bytes.is_finite() || bytes <= 0.0 || bytes > MAX_SIZE as f64 {
        return Err(format!("--size: invalid size `{text}` (at most 64G)"));
    }
    Ok(bytes as u64)
}

fn parse_non_negative(option: &str, text: &str) -> Result<f64, String> {
    match text.parse::<f64>() {
        Ok(value) if value >= 0.0 && value.is_finite() => Ok(value),
        _ => Err(format!("{option}: invalid number `{text}`")),
    }
}

/// Writes the file described by `options`, and returns its size.
pub fn run(options: &Options) -> io::Result<u64> {
    let file = BufWriter::with_capacity(1 << 20, File::create(&options.out)?);
    generate(file, options)
}

/// Writes a generated file to `out`, and returns its size in bytes.
pub fn generate<W: Write>(out: W, options: &Options) -> io::Result<u64> {
    let mut generator = Generator {
        json: Json::new(out),
        rng: fastrand::Rng::with_seed(options.seed),
        text: options.text,
        late_type: options.late_type,
        budget: 0,
        buffer: String::new(),
    };
    generator.network(options.size)?;
    let mut writer = generator.json.out;
    writer.flush()?;
    Ok(writer.count)
}

/// A writer that counts the bytes written through it.
pub(crate) struct CountingWriter<W> {
    inner: W,
    pub(crate) count: u64,
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.count += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// An open JSON array or object.
struct Frame {
    /// No element or key written yet: the next one needs no comma.
    empty: bool,
    /// The `_type_` of an object, to write after its first attribute (`--late-type`).
    late_type: Option<&'static str>,
}

/// A minimal compact JSON writer, which knows where commas go.
pub(crate) struct Json<W> {
    pub(crate) out: CountingWriter<W>,
    frames: Vec<Frame>,
}

impl<W: Write> Json<W> {
    /// A writer of JSON to `out`.
    pub(crate) fn new(out: W) -> Json<W> {
        Json {
            out: CountingWriter {
                inner: out,
                count: 0,
            },
            frames: Vec::new(),
        }
    }

    pub(crate) fn open(&mut self, bracket: &[u8]) -> io::Result<()> {
        self.out.write_all(bracket)?;
        self.frames.push(Frame {
            empty: true,
            late_type: None,
        });
        Ok(())
    }

    pub(crate) fn close(&mut self, bracket: &[u8]) -> io::Result<()> {
        let frame = self.frames.pop().expect("open frame");
        if let Some(class) = frame.late_type {
            // An object with a single attribute: its `_type_` comes last.
            self.out.write_all(if frame.empty { b"" } else { b"," })?;
            write!(self.out, "\"_type_\":\"{class}\"")?;
        }
        self.out.write_all(bracket)
    }

    /// Writes the comma that separates this item from the previous one, if needed.
    pub(crate) fn separator(&mut self) -> io::Result<()> {
        let frame = self.frames.last_mut().expect("open frame");
        if frame.empty {
            frame.empty = false;
            Ok(())
        } else {
            let pending = frame.late_type.take();
            self.out.write_all(b",")?;
            if let Some(class) = pending {
                write!(self.out, "\"_type_\":\"{class}\",")?;
            }
            Ok(())
        }
    }

    /// Writes `"key":`, before its value.
    pub(crate) fn key(&mut self, key: &str) -> io::Result<()> {
        self.separator()?;
        write!(self.out, "\"{key}\":")
    }

    /// Opens an object, with its `_type_` first or after its first attribute.
    fn open_typed(&mut self, class: &'static str, late: bool) -> io::Result<()> {
        self.open(b"{")?;
        if late {
            self.frames.last_mut().expect("open frame").late_type = Some(class);
            Ok(())
        } else {
            self.key("_type_")?;
            self.string(class)
        }
    }

    /// Writes a string, escaped by serde_json.
    fn string(&mut self, text: &str) -> io::Result<()> {
        serde_json::to_writer(&mut self.out, text).map_err(io::Error::other)
    }

    /// Writes a number (or any value whose `Display` is valid JSON).
    pub(crate) fn raw(&mut self, value: impl std::fmt::Display) -> io::Result<()> {
        write!(self.out, "{value}")
    }

    pub(crate) fn field_str(&mut self, key: &str, value: &str) -> io::Result<()> {
        self.key(key)?;
        self.string(value)
    }

    pub(crate) fn field_raw(&mut self, key: &str, value: impl std::fmt::Display) -> io::Result<()> {
        self.key(key)?;
        self.raw(value)
    }
}

const CITIES: &[&str] = &[
    "Brest",
    "Quimper",
    "Rennes",
    "Nantes",
    "Lorient",
    "Vannes",
    "Saint-Brieuc",
    "Morlaix",
    "Caen",
    "Rouen",
    "Lille",
    "Amiens",
    "Reims",
    "Metz",
    "Nancy",
    "Strasbourg",
    "Colmar",
    "Dijon",
    "Besançon",
    "Lyon",
    "Grenoble",
    "Annecy",
    "Chambéry",
    "Clermont-Ferrand",
    "Limoges",
    "Poitiers",
    "La Rochelle",
    "Bordeaux",
    "Biarritz",
    "Pau",
    "Toulouse",
    "Montpellier",
    "Nîmes",
    "Marseille",
    "Toulon",
    "Nice",
    "Ajaccio",
    "Bastia",
    "Perpignan",
    "Tours",
    "Orléans",
    "Le Mans",
    "Angers",
    "Cherbourg",
    "Dunkerque",
    "Troyes",
    "Bourges",
    "Aurillac",
];
const WORDS: &[&str] = &[
    "capteur",
    "relevé",
    "vent",
    "pluie",
    "abri",
    "étalonnage",
    "sonde",
    "câble",
    "mât",
    "nettoyage",
    "contrôle",
    "écart",
    "température",
    "humidité",
    "pression",
    "rafale",
    "givre",
    "orage",
    "entretien",
    "remplacement",
    "batterie",
    "panneau",
    "solaire",
    "transmission",
    "données",
    "station",
    "mesure",
    "dérive",
    "vérification",
    "réglage",
    "fixation",
    "corrosion",
    "étanchéité",
    "boîtier",
    "alimentation",
    "coupure",
    "reprise",
    "le",
    "la",
    "les",
    "du",
    "de",
    "sur",
    "avec",
    "après",
    "avant",
    "sans",
    "très",
    "légère",
    "forte",
    "constatée",
    "effectué",
];
const TECHNICIANS: &[&str] = &[
    "A. Martin",
    "B. Le Gall",
    "C. Bernard",
    "D. Moreau",
    "E. Kerjean",
    "F. Dubois",
    "G. Laurent",
    "H. Simon",
    "I. Michel",
    "J. Garnier",
];
const MODELS: &[&str] = &[
    "PT100",
    "PT1000",
    "WMT700",
    "Young 05103",
    "ARG100",
    "Hellmann",
    "Campbell 107",
    "Thies 4.3351",
];
const CAUSES: &[&str] = &[
    "Coupure d'alimentation",
    "Panne de transmission",
    "Givre sur la sonde",
    "Maintenance programmée",
    "Défaut du capteur",
];
const RESOLUTIONS: &[&str] = &["640x480", "1280x720", "1920x1080", "3840x2160"];
const CATEGORIES: &[&str] = &["SYNOPTIQUE", "CLIMATOLOGIQUE", "AUTOMATIQUE"];
const QUALITIES: &[&str] = &["BONNE", "DOUTEUSE", "FAUSSE"];
const SEVERITIES: &[&str] = &["MINEURE", "MAJEURE", "CRITIQUE"];
const LEVELS: &[&str] = &["VERT", "JAUNE", "ORANGE", "ROUGE"];
const CELL_STATUSES: &[&str] = &["A_ACQUERIR", "PROGRAMMEE", "ACQUISE", "REJETEE"];

/// Share of the size that goes to the stations; the rest goes to the zones.
const STATION_SHARE: f64 = 0.85;

struct Generator<W> {
    json: Json<W>,
    rng: fastrand::Rng,
    text: f64,
    late_type: f64,
    /// Size at which the current part (stations, then zones) stops.
    budget: u64,
    /// Reused for long texts.
    buffer: String,
}

impl<W: Write> Generator<W> {
    fn full(&self) -> bool {
        self.json.out.count >= self.budget
    }

    fn network(&mut self, size: u64) -> io::Result<()> {
        self.json.open(b"{")?;
        self.json.field_str("nom", "Réseau généré")?;
        self.budget = (size as f64 * STATION_SHARE) as u64;
        self.json.key("stations")?;
        self.json.open(b"[")?;
        let mut number = 0;
        while !self.full() {
            number += 1;
            self.station(number)?;
        }
        self.json.close(b"]")?;

        self.budget = size;
        self.json.key("zones")?;
        self.json.open(b"[")?;
        let mut number = 0;
        while !self.full() {
            number += 1;
            self.zone(number)?;
        }
        self.json.close(b"]")?;
        self.json.close(b"}")?;
        self.json.out.write_all(b"\n")
    }

    fn station(&mut self, number: u32) -> io::Result<()> {
        self.json.separator()?;
        self.json.open(b"{")?;
        self.json.field_str("code", &format!("ST-{number:06}"))?;
        let city = self.pick(CITIES);
        self.json.field_str("nom", &format!("{city} {number}"))?;
        let latitude = self.decimal(41.0, 51.0, 6);
        self.json.field_raw("latitude", latitude)?;
        let longitude = self.decimal(-5.0, 9.0, 6);
        self.json.field_raw("longitude", longitude)?;
        let altitude = self.rng.i64(0..=2500);
        self.json.field_raw("altitude", altitude)?;
        let date = self.date(1950, 2025);
        self.json.field_str("mise_en_service", &date)?;
        let active = self.rng.f64() < 0.9;
        self.json.field_raw("active", active)?;
        let category = self.pick(CATEGORIES);
        self.json.field_str("categorie", category)?;
        self.long_text("description", 500, 5000)?;

        self.json.key("equipements")?;
        self.json.open(b"[")?;
        for i in 0..self.rng.u32(3..=8) {
            if self.full() {
                break;
            }
            self.equipment(number, i)?;
        }
        self.json.close(b"]")?;

        self.json.key("interventions")?;
        self.json.open(b"[")?;
        for _ in 0..self.rng.u32(0..=6) {
            if self.full() {
                break;
            }
            self.intervention()?;
        }
        self.json.close(b"]")?;
        self.json.close(b"}")
    }

    fn equipment(&mut self, station: u32, index: u32) -> io::Result<()> {
        let sensor = self.rng.f64() < 0.8;
        let class = if sensor {
            self.pick(&["Thermometre", "Anemometre", "Pluviometre"])
        } else {
            "Camera"
        };
        self.json.separator()?;
        let late = self.late();
        self.json.open_typed(class, late)?;
        self.json.field_str(
            "numero_serie",
            &format!("{}-{station:06}-{index}", &class[..1]),
        )?;
        let model = self.pick(MODELS);
        self.json.field_str("modele", model)?;
        let date = self.date(2000, 2025);
        self.json.field_str("installe_le", &date)?;
        match class {
            "Thermometre" => {
                let min = self.rng.i64(-50..=-10);
                self.json.field_raw("plage_min", min)?;
                let max = self.decimal(40.0, 70.0, 1);
                self.json.field_raw("plage_max", max)?;
            }
            "Anemometre" => {
                let height = self.decimal(2.0, 30.0, 1);
                self.json.field_raw("hauteur_mat", height)?;
            }
            "Pluviometre" => {
                let surface = self.pick(&[200, 400, 1000]);
                self.json.field_raw("surface_cm2", surface)?;
            }
            _ => {
                let resolution = self.pick(RESOLUTIONS);
                self.json.field_str("resolution", resolution)?;
                let orientation = self.decimal(0.0, 360.0, 1);
                self.json.field_raw("orientation_deg", orientation)?;
            }
        }
        if sensor {
            let frequency = self.pick(&[60, 600, 3600]);
            self.json.field_raw("frequence_s", frequency)?;
            self.json.key("mesures")?;
            self.json.open(b"[")?;
            for _ in 0..self.rng.u32(50..=400) {
                if self.full() {
                    break;
                }
                self.measure()?;
            }
            self.json.close(b"]")?;
        }
        self.json.close(b"}")
    }

    fn measure(&mut self) -> io::Result<()> {
        let gap = self.rng.f64() < 0.03;
        self.json.separator()?;
        let late = self.late();
        self.json
            .open_typed(if gap { "Lacune" } else { "Releve" }, late)?;
        let instant = self.instant();
        self.json.field_str("instant", &instant)?;
        if gap {
            let end = self.instant();
            self.json.field_str("fin", &end)?;
            let cause = self.pick(CAUSES);
            self.json.field_str("cause", cause)?;
        } else {
            let value = self.decimal(-20.0, 40.0, 2);
            self.json.field_raw("valeur", value)?;
            let quality = if self.rng.f64() < 0.9 {
                "BONNE"
            } else {
                self.pick(QUALITIES)
            };
            self.json.field_str("qualite", quality)?;
        }
        if self.rng.f64() < 0.01 {
            self.json.key("annotations")?;
            self.json.open(b"[")?;
            if !self.full() {
                self.json.separator()?;
                self.json.open(b"{")?;
                let author = self.pick(TECHNICIANS);
                self.json.field_str("auteur", author)?;
                self.long_text("texte", 100, 2000)?;
                let created = self.instant();
                self.json.field_str("creee_le", &created)?;
                self.json.close(b"}")?;
            }
            self.json.close(b"]")?;
        }
        self.json.close(b"}")
    }

    fn intervention(&mut self) -> io::Result<()> {
        let class = self.pick(&["Maintenance", "Panne", "Etalonnage"]);
        self.json.separator()?;
        let late = self.late();
        self.json.open_typed(class, late)?;
        let date = self.date(2015, 2026);
        self.json.field_str("date", &date)?;
        let technician = self.pick(TECHNICIANS);
        self.json.field_str("technicien", technician)?;
        match class {
            "Maintenance" => {
                let hours = self.decimal(0.5, 8.0, 1);
                self.json.field_raw("duree_h", hours)?;
            }
            "Panne" => {
                let severity = self.pick(SEVERITIES);
                self.json.field_str("gravite", severity)?;
                let solved = self.rng.bool();
                self.json.field_raw("resolue", solved)?;
            }
            _ => {
                let gap = self.decimal(-1.0, 1.0, 3);
                self.json.field_raw("ecart", gap)?;
            }
        }
        self.long_text("compte_rendu", 200, 3000)?;
        self.json.close(b"}")
    }

    fn zone(&mut self, number: u32) -> io::Result<()> {
        self.json.separator()?;
        self.json.open(b"{")?;
        self.json.field_str("code", &format!("Z-{number:06}"))?;
        let level = self.pick(LEVELS);
        self.json.field_str("niveau", level)?;
        let start = self.instant();
        self.json.field_str("debut", &start)?;
        let end = self.instant();
        self.json.field_str("fin", &end)?;

        // The footprint: a single reference, `_type_` always first.
        let (lat, lon) = (self.decimal(41.0, 51.0, 6), self.decimal(-5.0, 9.0, 6));
        self.json.key("emprise")?;
        if self.rng.f64() < 0.8 {
            self.json.open_typed("Polygone", false)?;
            self.json.key("sommets")?;
            self.json.open(b"[")?;
            // Log-uniform from 50 to 5 000 vertices: many small polygons, a few large ones.
            let vertices = (50.0 * 100f64.powf(self.rng.f64())) as u32;
            for i in 0..vertices {
                if self.full() {
                    break;
                }
                let angle = std::f64::consts::TAU * f64::from(i) / f64::from(vertices);
                let radius = 0.2 + 0.05 * self.rng.f64();
                self.point(lat + radius * angle.sin(), lon + radius * angle.cos())?;
            }
            self.json.close(b"]")?;
        } else {
            self.json.open_typed("Cercle", false)?;
            let radius = self.decimal(1.0, 100.0, 1);
            self.json.field_raw("rayon_km", radius)?;
            self.json.key("centre")?;
            self.point_object(lat, lon)?;
        }
        self.json.close(b"}")?;

        self.json.key("mailles")?;
        self.json.open(b"[")?;
        for i in 0..self.cell_count() {
            if self.full() {
                break;
            }
            self.cell(i + 1, lat, lon)?;
        }
        self.json.close(b"]")?;
        self.json.close(b"}")
    }

    /// Number of cells of a zone, from 1 to 1 000, about 5.5 on average: a geometric law of
    /// mean 3, and 1 % of zones with a log-uniform number from 20 to 1 000.
    fn cell_count(&mut self) -> u32 {
        if self.rng.f64() < 0.01 {
            return (20.0 * 50f64.powf(self.rng.f64())) as u32;
        }
        let mut count = 1;
        while count < 1000 && self.rng.f64() >= 1.0 / 3.0 {
            count += 1;
        }
        count
    }

    fn cell(&mut self, number: u32, lat: f64, lon: f64) -> io::Result<()> {
        self.json.separator()?;
        self.json.open(b"{")?;
        self.json.field_raw("numero", number)?;
        let status = self.pick(CELL_STATUSES);
        self.json.field_str("statut", status)?;
        if status == "ACQUISE" {
            let acquired = self.instant();
            self.json.field_str("acquise_le", &acquired)?;
        }
        self.json.key("coins")?;
        self.json.open(b"[")?;
        let (dlat, dlon) = (0.05 * f64::from(number % 10), 0.05 * f64::from(number / 10));
        for (corner_lat, corner_lon) in [(0.0, 0.0), (0.0, 0.05), (0.05, 0.05), (0.05, 0.0)] {
            if self.full() {
                break;
            }
            self.point(lat + dlat + corner_lat, lon + dlon + corner_lon)?;
        }
        self.json.close(b"]")?;
        self.json.close(b"}")
    }

    /// A point as an element of an array.
    fn point(&mut self, lat: f64, lon: f64) -> io::Result<()> {
        self.json.separator()?;
        self.point_object(lat, lon)
    }

    fn point_object(&mut self, lat: f64, lon: f64) -> io::Result<()> {
        self.json.open(b"{")?;
        self.json.field_raw("lat", round(lat, 6))?;
        self.json.field_raw("lon", round(lon, 6))?;
        self.json.close(b"}")
    }

    /// A long text of `min` to `max` characters times `--text`; omitted when `--text` is 0.
    fn long_text(&mut self, key: &str, min: usize, max: usize) -> io::Result<()> {
        let length = (self.rng.usize(min..=max) as f64 * self.text) as usize;
        if length == 0 {
            return Ok(());
        }
        let mut text = std::mem::take(&mut self.buffer);
        text.clear();
        while text.len() < length {
            if !text.is_empty() {
                text.push_str(if self.rng.u32(0..12) == 0 { ".\n" } else { " " });
            }
            text.push_str(self.pick(WORDS));
        }
        let result = self.json.field_str(key, &text);
        self.buffer = text;
        result
    }

    /// `true` if the next object of a polymorphic list gets its `_type_` late.
    fn late(&mut self) -> bool {
        self.late_type > 0.0 && self.rng.f64() < self.late_type
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        pick(&mut self.rng, items)
    }

    fn decimal(&mut self, min: f64, max: f64, digits: i32) -> f64 {
        decimal(&mut self.rng, min, max, digits)
    }

    fn date(&mut self, first_year: i32, last_year: i32) -> String {
        date(&mut self.rng, first_year, last_year)
    }

    /// An RFC 3339 instant from random components, with microseconds half of the time.
    fn instant(&mut self) -> String {
        let date = self.date(2020, 2026);
        let (h, m, s) = (
            self.rng.u32(0..24),
            self.rng.u32(0..60),
            self.rng.u32(0..60),
        );
        if self.rng.bool() {
            format!(
                "{date}T{h:02}:{m:02}:{s:02}.{:06}Z",
                self.rng.u32(0..1_000_000)
            )
        } else {
            format!("{date}T{h:02}:{m:02}:{s:02}Z")
        }
    }
}

// Random values, shared with the generator of the directory (`gen_annuaire`).

/// An item of `items`.
pub(crate) fn pick<T: Copy>(rng: &mut fastrand::Rng, items: &[T]) -> T {
    items[rng.usize(..items.len())]
}

/// A number from `min` to `max`, rounded to `digits` decimals.
pub(crate) fn decimal(rng: &mut fastrand::Rng, min: f64, max: f64, digits: i32) -> f64 {
    round(min + (max - min) * rng.f64(), digits)
}

/// A date from random components (days 1 to 28, always valid).
pub(crate) fn date(rng: &mut fastrand::Rng, first_year: i32, last_year: i32) -> String {
    let year = rng.i32(first_year..=last_year);
    format!("{year:04}-{:02}-{:02}", rng.u32(1..=12), rng.u32(1..=28))
}

/// Rounds to `digits` decimals. `Display` of the result is the shortest text that reads back
/// exactly, so the file holds at most `digits` decimals.
fn round(value: f64, digits: i32) -> f64 {
    let scale = 10f64.powi(digits);
    (value * scale).round() / scale
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(size: u64) -> Options {
        Options {
            size,
            out: PathBuf::new(),
            seed: 7,
            text: 1.0,
            late_type: 0.0,
        }
    }

    fn generate_bytes(options: &Options) -> Vec<u8> {
        let mut bytes = Vec::new();
        let size = generate(&mut bytes, options).expect("generate");
        assert_eq!(size, bytes.len() as u64);
        bytes
    }

    #[test]
    fn parses_the_options() {
        let args: Vec<String> = [
            "--size",
            "1.5M",
            "--out",
            "a.json",
            "--seed",
            "3",
            "--text",
            "0",
            "--late-type",
            "0.1",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let parsed = parse_args(&args).expect("parse");
        assert_eq!(
            parsed,
            Options {
                size: 3 << 19,
                out: "a.json".into(),
                seed: 3,
                text: 0.0,
                late_type: 0.1
            }
        );
        assert_eq!(parse_size("200K"), Ok(200 << 10));
        assert_eq!(parse_size("2G"), Ok(2 << 30));
        assert_eq!(parse_size("1000"), Ok(1000));
        assert!(parse_size("dix").is_err());
        assert!(parse_size("inf").is_err());
        assert!(parse_size("1e30G").is_err());
        assert!(parse_size("65G").is_err());
        assert_eq!(parse_size("64G"), Ok(64 << 30));
        assert!(parse_args(&["--size".to_string()]).is_err());
        assert!(parse_args(&["--out".to_string(), "x".to_string()]).is_err());
        let late = ["--size", "1K", "--out", "x", "--late-type", "2"].map(String::from);
        assert!(parse_args(&late).is_err());
    }

    /// Scenario "Même graine, même fichier".
    #[test]
    fn same_seed_gives_the_same_file() {
        let first = generate_bytes(&options(200 << 10));
        assert_eq!(first, generate_bytes(&options(200 << 10)));
        let other = Options {
            seed: 8,
            ..options(200 << 10)
        };
        assert_ne!(first, generate_bytes(&other));
    }

    /// Scenario "Taille demandée atteinte".
    #[test]
    fn requested_size_is_reached() {
        let target = 1 << 20;
        let bytes = generate_bytes(&options(target));
        let size = bytes.len() as u64;
        assert!(size >= target, "{size}");
        assert!(size < target + target / 10, "{size}");
        serde_json::from_slice::<serde_json::Value>(&bytes).expect("valid JSON");
    }

    #[test]
    fn tiny_size_still_gives_valid_json() {
        for size in [1, 100, 5000] {
            let bytes = generate_bytes(&options(size));
            serde_json::from_slice::<serde_json::Value>(&bytes).expect("valid JSON");
        }
    }

    /// Scenario "Sans texte long".
    #[test]
    fn no_long_text_with_text_zero() {
        let bytes = generate_bytes(&Options {
            text: 0.0,
            ..options(1 << 20)
        });
        let text = String::from_utf8(bytes).expect("UTF-8");
        for key in ["\"description\"", "\"compte_rendu\"", "\"texte\""] {
            assert!(!text.contains(key), "{key}");
        }
        let with_text = String::from_utf8(generate_bytes(&options(1 << 20))).expect("UTF-8");
        assert!(with_text.contains("\"description\"") && with_text.contains("\"compte_rendu\""));
    }

    #[test]
    fn late_type_puts_type_after_the_first_attribute() {
        let bytes = generate_bytes(&Options {
            late_type: 1.0,
            ..options(100 << 10)
        });
        let text = String::from_utf8(bytes).expect("UTF-8");
        assert!(
            text.contains(r#"{"numero_serie":"#),
            "equipments start with an attribute"
        );
        assert!(!text.contains(r#"{"_type_":"Releve""#));
        // Footprints, a single reference, keep `_type_` first.
        assert!(text.contains(r#""emprise":{"_type_":"#));
    }

    /// Scenario "Fichier conforme au modèle": a file with 10 % of late `_type_`, written to
    /// disk, loads with the test model.
    #[test]
    fn generated_file_loads_with_the_test_model() {
        use std::sync::Arc;

        use ortelia_model::builder::DataPath;
        use ortelia_model::fragment::Shape;
        use ortelia_model::metamodel::{Model, ModelFile};

        let dir = tempfile::tempdir().expect("temp dir");
        let out = dir.path().join("reseau.json");
        let options = Options {
            out: out.clone(),
            late_type: 0.1,
            ..options(1 << 20)
        };
        run(&options).expect("generate");

        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../examples/reseau-meteo/reseau.model.json"
        );
        let text = std::fs::read_to_string(path).expect("model");
        let model =
            Arc::new(Model::compile(&[ModelFile::parse(&text).expect("parse")]).expect("compile"));
        let root = model.class_id("Reseau").expect("Reseau");
        let file = File::open(&out).expect("open");
        // A fragment for the root of a network.
        let loaded =
            ortelia_service::json::read_json(file, model, Shape::root(root), DataPath::default())
                .unwrap_or_else(|failure| panic!("{failure}"));
        // A generated file conforms to the model: every required attribute has a value.
        assert_eq!(loaded.diagnostic_count, 0, "{:#?}", loaded.diagnostics);
        let fragment = loaded.fragment;
        let store = fragment.store();
        let reseau = store.model().class(root);
        let stations = reseau.reference("stations").expect("stations");
        let zones = reseau.reference("zones").expect("zones");
        assert_eq!(fragment.heads().len(), 1);
        let root = fragment.heads()[0];
        assert!(!store.children(root, stations).is_empty());
        assert!(!store.children(root, zones).is_empty());
    }
}
