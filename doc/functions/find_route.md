### **Flujo de `find_route_as_geojson`**

El objetivo principal de esta función es encontrar la **ruta más corta** entre dos puntos utilizando el **algoritmo A*** y devolver la ruta en formato **GeoJSON**. Veamos paso a paso el flujo de esta función.

---

### **Flujo Detallado**

#### 1. **Carga del grafo desde la base de datos**

```rust
let graph = load_graph_from_db().await?;
```

- **`load_graph_from_db`** se conecta a la base de datos PostgreSQL y carga las rutas disponibles en forma de un grafo.
- El grafo es representado como un **`BTreeMap`** de nodos, donde cada nodo tiene una lista de vecinos y sus distancias.

Ejemplo de grafo:
```text
(13.7466, -89.6759) → [(13.7470, -89.6760): 30.0]
```

---

#### 2. **Buscar el nodo más cercano al punto de inicio y final**

```rust
let start = find_nearest_node(start_lat, start_lon).await?;
let end = find_nearest_node(end_lat, end_lon).await?;
```

- **`find_nearest_node`** busca el nodo más cercano al punto dado (en un radio de 100 metros). Si no se encuentra un nodo, se lanza un error.
- Si hay múltiples nodos en el radio, selecciona el más cercano por distancia.

---

#### 3. **Definición de la heurística**

```rust
let heuristic = |(lat, lon): Coordinate| -> OrderedFloat<f64> {
    OrderedFloat(
        ((lat.into_inner() - end.0.into_inner()).powi(2)
            + (lon.into_inner() - end.1.into_inner()).powi(2))
            .sqrt(),
    )
};
```

- Esta **heurística** calcula la **distancia euclidiana** entre un nodo actual y el nodo objetivo.
- Devuelve un `OrderedFloat<f64>` para que pueda ser comparado correctamente en el algoritmo A*.

---

#### 4. **Ejecutar el Algoritmo A\***

```rust
match astar(&graph, start, end, heuristic) {
    Some((_, path)) => { /* Procesa la ruta */ }
    None => Err("No route found".into()),
}
```

- El algoritmo **A\*** busca la **ruta más corta** desde el nodo de inicio al nodo final utilizando la heurística definida.
- Si encuentra una ruta, devuelve una lista de coordenadas que representan la ruta; si no, devuelve un error.

---

#### 5. **Generar el GeoJSON**

```rust
let geojson = format!(
    "{{\"type\": \"LineString\", \"coordinates\": [{}]}}",
    path.iter()
        .map(|(lat, lon)| format!("[{}, {}]", lon.into_inner(), lat.into_inner()))
        .collect::<Vec<_>>()
        .join(", ")
);
Ok(geojson)
```

- Si se encuentra la ruta, la función convierte las coordenadas en **GeoJSON**.
- Ejemplo de salida:
```json
{
    "type": "LineString",
    "coordinates": [[-89.6759, 13.7466], [-89.6760, 13.7470]]
}
```

---

#### 6. **Manejo de Errores**

- Si no se encuentra ningún nodo cercano en **`find_nearest_node`**, o si no se puede generar una ruta, se lanza un **error** claro.

---

### **Resumen del Flujo Completo**

1. **Carga del grafo** desde la base de datos.
2. **Busca los nodos más cercanos** al punto de inicio y final.
3. Define una **heurística** basada en la distancia euclidiana.
4. **Ejecuta A\*** para encontrar la ruta más corta.
5. **Genera GeoJSON** a partir de la ruta.
6. Si algo falla, lanza un **error** indicando el problema.

---

### **Prueba**

Ahora que entiendes el flujo, puedes verificar:

1. **Proveer coordenadas válidas** dentro del radio esperado.
2. Si encuentras errores, **verifica los nodos disponibles** en la base de datos cerca de los puntos de inicio y fin. Usa la siguiente consulta:

```sql
SELECT ST_AsText(way), ST_SRID(way) FROM planet_osm_point LIMIT 10;
```

Este flujo asegura que, si las coordenadas y los datos son válidos, la función devolverá la ruta más corta en GeoJSON.