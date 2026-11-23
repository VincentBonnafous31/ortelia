# Ortelia

Ortelia is a lightweight, fast desktop tool (Windows, Linux) for building data applications **without code**.
A domain engineer describes a data model (inspired by Ecore), the data sources, the window layout and the views
(table, form, tree, chart, 2D/3D map, Gantt); Ortelia runs the result. Specific features, such as custom file
importers or computation algorithms, are added as plugins.

Ortelia is written in Rust.

> **Status:** design phase. No code yet.

The design documents are written in French and live in [`docs/`](docs/):

- [`docs/feuille-de-route.md`](docs/feuille-de-route.md): roadmap (three milestone applications);
- [`docs/architecture-couches.md`](docs/architecture-couches.md): layered architecture with an interchangeable UI.

Example applications and data are in [`examples/`](examples/).

The name pays tribute to Abraham Ortelius, who published the first modern atlas in 1570: many maps brought
together into one coherent work, as Ortelia brings many views together around one model.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
