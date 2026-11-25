//! Measures of modification (E1 to E4), through the transactions of the document: one
//! transaction per pass of a measure, whose recording (the journal, old values copied) is part
//! of the measure.
//!
//! They run in the order E1, E2, E4, E3: E3 deletes stations, which E1 draws from.

use std::time::Instant;

use ortelia_model::document::Document;
use ortelia_model::store::{ObjectId, Position};
use ortelia_model::value::Value;

use crate::access::{Inventory, Network, SEED, Settings, median_of, nanos, per_unit};
use crate::memory;
use crate::report::Measure;

/// What E1 to E4 did, for the tests to check the document afterwards.
pub struct Outcome {
    pub measures: Vec<Measure>,
    // The fields below are only read by the tests.
    /// The zone that received the cells of E4, and the cells created.
    #[cfg_attr(not(test), allow(dead_code))]
    pub e4_zone: Option<ObjectId>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub e4_cells: Vec<ObjectId>,
    /// The stations deleted by E3.
    #[cfg_attr(not(test), allow(dead_code))]
    pub e3_deleted: Vec<ObjectId>,
}

/// Runs E1, E2, E4 and E3, and returns their measures.
pub fn measure_all(
    document: &mut Document,
    net: &Network,
    inventory: &Inventory,
    settings: &Settings,
) -> Outcome {
    let mut rng = fastrand::Rng::with_seed(SEED + 1);
    let passes = settings.passes;
    let mut measures = Vec::new();

    // E1: random sets of a float, an integer or a text.
    let texts: Vec<String> = (0..1000)
        .map(|i| format!("Station modifiée n° {i}"))
        .collect();
    let operations: Vec<(u8, usize)> = (0..settings.e1_sets)
        .map(|_| match rng.u8(0..3) {
            0 if !inventory.readings.is_empty() => (0, rng.usize(..inventory.readings.len())),
            kind if !inventory.stations.is_empty() => {
                (kind.max(1), rng.usize(..inventory.stations.len()))
            }
            _ => (u8::MAX, 0),
        })
        .collect();
    let before = memory::resident_mib();
    let e1 = median_of(passes, |pass| {
        let start = Instant::now();
        let mut done = 0;
        let mut edit = document.edit();
        for &(kind, index) in &operations {
            // The values depend on the pass, so that a pass does not give the values that the
            // previous one left, which the document would not record.
            let result = match kind {
                0 => edit.set(
                    inventory.readings[index],
                    net.value,
                    Value::Float(index as f64 * 0.5 + pass as f64),
                ),
                1 => edit.set(
                    inventory.stations[index],
                    net.altitude,
                    Value::Int((index + pass) as i64),
                ),
                2 => edit.set(
                    inventory.stations[index],
                    net.name,
                    Value::Str(&texts[(index + pass) % texts.len()]),
                ),
                _ => continue,
            };
            result.expect("set");
            done += 1;
        }
        // Records the transaction: part of the measure.
        drop(edit);
        nanos(start) / f64::from(done.max(1))
    });
    let growth = match (before, memory::resident_mib()) {
        (Some(before), Some(after)) => Some(after - before),
        _ => None,
    };
    let sets = operations.iter().filter(|op| op.0 != u8::MAX).count();
    measures.push(Measure::new(
        "E1",
        "set au hasard (décimal, entier, texte)",
        "ns/set",
        per_unit(e1, sets),
    ));
    measures.push(Measure::new(
        "E1m",
        "croissance de VmRSS pendant les passes de E1",
        "Mio",
        growth,
    ));

    // E2: insert then delete objects in the middle of a long list built for the measure.
    let root = document.root();
    let mut edit = document.edit();
    let zone = edit
        .create_child(root, net.zones, Position::End, net.zone)
        .expect("zone");
    let polygon = edit
        .create_child(zone, net.footprint, Position::End, net.polygon)
        .expect("polygon");
    for _ in 0..settings.e2_list {
        edit.create_child(polygon, net.vertices, Position::End, net.point)
            .expect("point");
    }
    drop(edit);
    let e2 = median_of(passes, |_| {
        let start = Instant::now();
        let mut edit = document.edit();
        let middle = edit.store().children(polygon, net.vertices).len() / 2;
        let inserted: Vec<ObjectId> = (0..settings.e2_objects)
            .map(|_| {
                edit.create_child(polygon, net.vertices, Position::At(middle), net.point)
                    .expect("insert")
            })
            .collect();
        for id in inserted {
            edit.delete(id).expect("delete");
        }
        drop(edit);
        nanos(start) / 1e6
    });
    assert_eq!(
        document.store().children(polygon, net.vertices).len(),
        settings.e2_list
    );
    let label = format!(
        "insérer puis supprimer {} objets au milieu d'une liste de {}",
        settings.e2_objects, settings.e2_list
    );
    measures.push(Measure::new("E2", &label, "ms", Some(e2)));
    document.edit().delete(zone).expect("delete zone");

    // E4: create cells of 4 corners under a zone, as a cutting into cells would.
    let e4_zone = inventory.zones.first().copied();
    let mut e4_cells = Vec::new();
    if let Some(zone) = e4_zone {
        let e4 = median_of(passes, |_| {
            let start = Instant::now();
            let mut edit = document.edit();
            for number in 0..settings.e4_cells {
                let cell = edit
                    .create_child(zone, net.cells, Position::End, net.cell)
                    .expect("cell");
                edit.set(cell, net.number, Value::Int(number as i64))
                    .expect("numero");
                edit.set(cell, net.status, Value::Enum(0)).expect("statut");
                for (lat, lon) in [(0.0, 0.0), (0.0, 0.05), (0.05, 0.05), (0.05, 0.0)] {
                    let corner = edit
                        .create_child(cell, net.corners, Position::End, net.point)
                        .expect("corner");
                    edit.set(corner, net.lat, Value::Float(48.0 + lat))
                        .expect("lat");
                    edit.set(corner, net.lon, Value::Float(-4.0 + lon))
                        .expect("lon");
                }
                e4_cells.push(cell);
            }
            drop(edit);
            nanos(start) / 1e6
        });
        let label = format!(
            "création de {} mailles de 4 coins sous une zone",
            settings.e4_cells
        );
        measures.push(Measure::new("E4", &label, "ms", Some(e4)));
    } else {
        measures.push(Measure::new(
            "E4",
            "création de mailles sous une zone",
            "ms",
            None,
        ));
    }

    // E3: delete a station and its whole subtree; each pass deletes another station.
    // The first stations, in file order. A small file may have fewer stations than passes.
    let count = inventory.stations.len().min(passes);
    let mut e3_deleted = Vec::new();
    let mut e3 = 0.0;
    if count > 0 {
        e3 = median_of(count, |pass| {
            let station = inventory.stations[pass];
            let start = Instant::now();
            document.edit().delete(station).expect("delete station");
            let elapsed = nanos(start) / 1e6;
            e3_deleted.push(station);
            elapsed
        });
    }
    measures.push(Measure::new(
        "E3",
        "suppression d'une station et de tout son sous-arbre",
        "ms",
        per_unit(e3, count),
    ));

    Outcome {
        measures,
        e4_zone,
        e4_cells,
        e3_deleted,
    }
}
