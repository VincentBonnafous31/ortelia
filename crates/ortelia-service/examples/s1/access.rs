//! Measures of attribute access and navigation (A1 to A7) of the store.
//!
//! Each measure runs several passes and keeps the median; `black_box` keeps the compiler from
//! removing computations whose result is unused; random draws use a fixed seed.

use std::fmt::Write;
use std::hint::black_box;
use std::time::Instant;

use ortelia_model::metamodel::{AttrId, AttrType, ClassId, Model, RefId};
use ortelia_model::store::{ObjectId, Store};
use ortelia_model::value::Value;

use crate::report::Measure;

/// Seed of all random draws.
pub const SEED: u64 = 42;

/// How much work each measure does.
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    pub passes: usize,
    pub a1_reads: usize,
    pub a5_walks: usize,
    pub a6_windows: usize,
    pub a7_edits: usize,
    pub e1_sets: usize,
    pub e2_list: usize,
    pub e2_objects: usize,
    pub e4_cells: usize,
}

impl Settings {
    /// The settings of the trial.
    pub fn full() -> Settings {
        Settings {
            passes: 5,
            a1_reads: 1_000_000,
            a5_walks: 100_000,
            a6_windows: 1_000,
            a7_edits: 1_000,
            e1_sets: 1_000_000,
            e2_list: 100_000,
            e2_objects: 1_000,
            e4_cells: 1_000,
        }
    }
}

/// The names of the test model, resolved once.
pub struct Network {
    pub reading: ClassId,
    pub point: ClassId,
    pub polygon: ClassId,
    pub cell: ClassId,
    pub zone: ClassId,
    pub stations: RefId,
    pub zones: RefId,
    pub equipments: RefId,
    pub footprint: RefId,
    pub vertices: RefId,
    pub cells: RefId,
    pub corners: RefId,
    pub value: AttrId,
    pub serial_number: AttrId,
    pub lat: AttrId,
    pub lon: AttrId,
    pub number: AttrId,
    pub status: AttrId,
    pub name: AttrId,
    pub altitude: AttrId,
    /// The 8 columns of the station table of A6.
    pub columns: [AttrId; 8],
}

impl Network {
    pub fn new(model: &Model) -> Network {
        let class = |name: &str| {
            model
                .class_id(name)
                .unwrap_or_else(|| panic!("class {name}"))
        };
        let attr = |c: &str, name: &str| {
            model
                .class(class(c))
                .attribute(name)
                .unwrap_or_else(|| panic!("{c}.{name}"))
        };
        let reference = |c: &str, name: &str| {
            model
                .class(class(c))
                .reference(name)
                .unwrap_or_else(|| panic!("{c}.{name}"))
        };
        let column = |name| attr("Station", name);
        Network {
            reading: class("Releve"),
            point: class("Point"),
            polygon: class("Polygone"),
            cell: class("Maille"),
            zone: class("ZoneAlerte"),
            stations: reference("Reseau", "stations"),
            zones: reference("Reseau", "zones"),
            equipments: reference("Station", "equipements"),
            footprint: reference("ZoneAlerte", "emprise"),
            vertices: reference("Polygone", "sommets"),
            cells: reference("ZoneAlerte", "mailles"),
            corners: reference("Maille", "coins"),
            value: attr("Releve", "valeur"),
            serial_number: attr("Equipement", "numero_serie"),
            lat: attr("Point", "lat"),
            lon: attr("Point", "lon"),
            number: attr("Maille", "numero"),
            status: attr("Maille", "statut"),
            name: attr("Station", "nom"),
            altitude: attr("Station", "altitude"),
            columns: [
                column("code"),
                column("nom"),
                column("latitude"),
                column("longitude"),
                column("altitude"),
                column("mise_en_service"),
                column("active"),
                column("categorie"),
            ],
        }
    }
}

/// Objects found once after loading, to draw from.
pub struct Inventory {
    pub stations: Vec<ObjectId>,
    pub readings: Vec<ObjectId>,
    pub points: Vec<ObjectId>,
    /// Points that are corners of a cell.
    pub corners: Vec<ObjectId>,
    pub zones: Vec<ObjectId>,
}

impl Inventory {
    pub fn new(store: &Store, net: &Network) -> Inventory {
        let root = store.root().expect("root");
        let mut inventory = Inventory {
            stations: store.children(root, net.stations).to_vec(),
            readings: Vec::new(),
            points: Vec::new(),
            corners: Vec::new(),
            zones: store.children(root, net.zones).to_vec(),
        };
        let model = store.model();
        let mut stack = vec![root];
        while let Some(obj) = stack.pop() {
            let class = store.class_of(obj);
            if class == net.reading {
                inventory.readings.push(obj);
            } else if class == net.point {
                inventory.points.push(obj);
                if store.container(obj).map(|c| c.1) == Some(net.corners) {
                    inventory.corners.push(obj);
                }
            }
            for reference in model.class(class).references() {
                stack.extend_from_slice(store.children(obj, reference.id()));
            }
        }
        inventory
    }
}

/// Runs `passes` passes of `pass` and returns the median of their results.
pub fn median_of(passes: usize, mut pass: impl FnMut(usize) -> f64) -> f64 {
    let mut results: Vec<f64> = (0..passes.max(1)).map(&mut pass).collect();
    results.sort_by(f64::total_cmp);
    results[results.len() / 2]
}

/// Nanoseconds elapsed since `start`.
pub fn nanos(start: Instant) -> f64 {
    start.elapsed().as_nanos() as f64
}

fn float(value: Value<'_>) -> f64 {
    match value {
        Value::Float(v) => v,
        _ => f64::NAN,
    }
}

/// Draws `count` indices below `len` (none if `len` is 0).
pub fn draws(rng: &mut fastrand::Rng, len: usize, count: usize) -> Vec<usize> {
    if len == 0 {
        return Vec::new();
    }
    (0..count).map(|_| rng.usize(..len)).collect()
}

/// Runs A1 to A7 and returns their measures.
pub fn measure_all(
    store: &mut Store,
    net: &Network,
    inventory: &Inventory,
    settings: &Settings,
) -> Vec<Measure> {
    let mut rng = fastrand::Rng::with_seed(SEED);
    let passes = settings.passes;
    let mut measures = Vec::new();

    // A1: random reads of a float.
    let picks = draws(&mut rng, inventory.readings.len(), settings.a1_reads);
    let a1 = median_of(passes, |_| {
        let start = Instant::now();
        let mut sum = 0.0;
        for &i in &picks {
            sum += float(store.get(inventory.readings[i], net.value));
        }
        black_box(sum);
        nanos(start) / picks.len().max(1) as f64
    });
    measures.push(Measure::new(
        "A1",
        "lecture au hasard d'un décimal (valeur d'un relevé)",
        "ns/accès",
        per_unit(a1, picks.len()),
    ));

    // A2: depth-first walk of the whole tree, one attribute read per object.
    let first_attribute: Vec<Option<AttrId>> = store
        .model()
        .classes()
        .iter()
        .map(|c| c.attributes().first().map(|a| a.id()))
        .collect();
    let mut objects = 0;
    let a2 = median_of(passes, |_| {
        let start = Instant::now();
        objects = 0;
        let mut stack = vec![store.root().expect("root")];
        while let Some(obj) = stack.pop() {
            objects += 1;
            let class = store.class_of(obj);
            if let Some(attr) = first_attribute[class.index()] {
                black_box(store.get(obj, attr));
            }
            for reference in store.model().class(class).references() {
                stack.extend_from_slice(store.children(obj, reference.id()));
            }
        }
        nanos(start) / objects as f64
    });
    measures.push(Measure::new(
        "A2",
        "parcours en profondeur de tout l'arbre, un attribut lu par objet",
        "ns/objet",
        Some(a2),
    ));

    // A3: an inherited attribute read on every polymorphic list of equipments.
    let mut equipments = 0;
    let a3 = median_of(passes, |_| {
        let start = Instant::now();
        equipments = 0;
        let mut total = 0;
        for &station in &inventory.stations {
            for &equipment in store.children(station, net.equipments) {
                equipments += 1;
                if let Value::Str(serial) = store.get(equipment, net.serial_number) {
                    total += serial.len();
                }
            }
        }
        black_box(total);
        nanos(start) / equipments.max(1) as f64
    });
    measures.push(Measure::new(
        "A3",
        "numero_serie lu sur toutes les listes equipements",
        "ns/objet",
        per_unit(a3, equipments),
    ));

    // A4: every vertex (polygons and cell corners) into a flat buffer of f64.
    let mut buffer = Vec::new();
    let a4 = median_of(passes, |_| {
        let start = Instant::now();
        buffer.clear();
        for &zone in &inventory.zones {
            extract_zone(store, net, zone, &mut buffer);
        }
        black_box(&buffer);
        nanos(start) / 1e6
    });
    let vertices = buffer.len() / 2;
    let a4 = (vertices > 0).then(|| a4 / (vertices as f64 / 1e6));
    measures.push(Measure::new(
        "A4",
        "extraction de tous les sommets dans un tampon plat de f64",
        "ms/million de sommets",
        a4,
    ));

    // A5: walking up to the root from random points.
    let picks = draws(&mut rng, inventory.points.len(), settings.a5_walks);
    let mut levels = 0;
    let a5 = median_of(passes, |_| {
        let start = Instant::now();
        levels = 0;
        for &i in &picks {
            let mut current = inventory.points[i];
            while let Some((parent, _)) = store.container(current) {
                levels += 1;
                current = parent;
            }
            black_box(current);
        }
        nanos(start) / levels.max(1) as f64
    });
    measures.push(Measure::new(
        "A5",
        "remontée jusqu'à la racine depuis des Point au hasard",
        "ns/niveau",
        per_unit(a5, levels),
    ));

    // A6: the visible window of a table, 50 stations × 8 attributes formatted as text.
    let window = inventory.stations.len().min(50);
    let starts = draws(&mut rng, inventory.stations.len(), settings.a6_windows);
    let mut text = String::new();
    let a6 = median_of(passes, |_| {
        let start = Instant::now();
        for &first in &starts {
            text.clear();
            for row in 0..window {
                let station = inventory.stations[(first + row) % inventory.stations.len()];
                for &column in &net.columns {
                    format_value(store.model(), column, store.get(station, column), &mut text);
                    text.push('\t');
                }
            }
            black_box(&text);
        }
        nanos(start) / 1e3 / starts.len().max(1) as f64
    });
    measures.push(Measure::new(
        "A6",
        "50 stations × 8 attributs mis en forme en texte",
        "µs/fenêtre",
        per_unit(a6, starts.len()),
    ));

    // A7: modify a corner, walk up to its zone, extract the zone again.
    let picks = draws(&mut rng, inventory.corners.len(), settings.a7_edits);
    let a7 = median_of(passes, |_| {
        let start = Instant::now();
        for &i in &picks {
            let corner = inventory.corners[i];
            let lat = float(store.get(corner, net.lat));
            store
                .set(corner, net.lat, Value::Float(lat + 1e-6))
                .expect("set lat");
            let cell = store.container(corner).expect("cell").0;
            let zone = store.container(cell).expect("zone").0;
            buffer.clear();
            extract_zone(store, net, zone, &mut buffer);
            black_box(&buffer);
        }
        nanos(start) / 1e3 / picks.len().max(1) as f64
    });
    measures.push(Measure::new(
        "A7",
        "modifier un coin, remonter à sa zone, réextraire la zone",
        "µs/opération",
        per_unit(a7, picks.len()),
    ));
    measures
}

/// `Some(value)` if the measure covered at least one unit.
pub fn per_unit(value: f64, units: usize) -> Option<f64> {
    (units > 0).then_some(value)
}

/// Appends the vertices of a zone (its polygon and the corners of its cells) to `buffer`.
pub fn extract_zone(store: &Store, net: &Network, zone: ObjectId, buffer: &mut Vec<f64>) {
    for &footprint in store.children(zone, net.footprint) {
        if store.class_of(footprint) == net.polygon {
            push_points(store, net, store.children(footprint, net.vertices), buffer);
        }
    }
    for &cell in store.children(zone, net.cells) {
        push_points(store, net, store.children(cell, net.corners), buffer);
    }
}

fn push_points(store: &Store, net: &Network, points: &[ObjectId], buffer: &mut Vec<f64>) {
    for &point in points {
        buffer.push(float(store.get(point, net.lat)));
        buffer.push(float(store.get(point, net.lon)));
    }
}

/// Formats a value as a table cell would.
fn format_value(model: &Model, attr: AttrId, value: Value<'_>, text: &mut String) {
    let _ = match value {
        Value::Unset => Ok(()),
        Value::Str(s) => {
            text.push_str(s);
            Ok(())
        }
        Value::Int(v) => write!(text, "{v}"),
        Value::Float(v) => write!(text, "{v:.4}"),
        Value::Bool(v) => {
            text.push_str(if v { "oui" } else { "non" });
            Ok(())
        }
        Value::Date(d) => write!(text, "{d}"),
        Value::DateTime(t) => write!(text, "{t}"),
        Value::Enum(literal) => {
            if let AttrType::Enum(id) = model.attribute(attr).attr_type() {
                text.push_str(&model.enum_type(id).literals()[literal as usize]);
            }
            Ok(())
        }
    };
}
