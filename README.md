# Geo Reader 

This is a simple project to read and process geospatial data from OpenStreetMap. The project is written in Rust and uses a database PostgreSQL with PostGIS extension.

tree --prune -I 'target'
bloque de codigo


```markdown
.
├── Cargo.lock
├── Cargo.toml
├── doc
│   ├── 1-migrate-osm.md
│   ├── 2-osmdb-user.md
│   ├── 3-osmdb-tables.md
│   ├── functions
│   │   └── find_route.md
│   ├── psql
│   │   ├── nearby.sql
│   │   └── nodes.sql
│   └── Tables.md
├── README.md
├── src
│   ├── algorithms
│   │   ├── astar.rs
│   │   └── mod.rs
│   ├── api
│   │   ├── handlers.rs
│   │   └── mod.rs
│   ├── db.rs
│   ├── main_console.rs
│   ├── main.rs
│   ├── queries
│   │   ├── find_by_coordinates.rs
│   │   ├── find_nearby.rs
│   │   ├── find_places.rs
│   │   ├── find_route.rs
│   │   ├── mod.rs
│   │   └── nearby_route.rs
│   └── utils
│       ├── load_graph.rs
│       └── mod.rs
└── sshl

9 directories, 26 files
```

## Endpoints

### places

Este endpoint devolverá registros que coincidan con el parámetro `name` proporcionado.
Ejemplo de uso:
curl -X GET "http://localhost:8080/api/places?name=izalco"


### nearby
curl -X GET "http://localhost:8080/api/nearby_routes?latitude=13.6894&longitude=-89.1872&max_distance=1000"
