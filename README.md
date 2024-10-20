# Geo Reader 

This is a simple project to read and process geospatial data from OpenStreetMap. The project is written in Rust and uses a database PostgreSQL with PostGIS extension.

tree --prune -I 'target'
.
├── Cargo.lock
├── Cargo.toml
├── doc
│   ├── 1-migrate-osm.md
│   ├── 2-osmdb-user.md
│   ├── 3-osmdb-tables.md
│   └── psql
│       └── nodes.sql
└── src
    ├── algorithms
    │   ├── astar.rs
    │   └── mod.rs
    ├── db.rs
    ├── functions
    │   ├── find_by_coordinates.rs
    │   ├── find_nearby.rs
    │   ├── find_places.rs
    │   ├── find_route.rs
    │   └── mod.rs
    ├── lib.rs
    ├── main.rs
    └── utils
        ├── load_graph.rs
        └── mod.rs

7 directories, 18 files

