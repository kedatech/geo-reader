Al importar datos de **OpenStreetMap (OSM)** con `**osm2pgsql**` en una base de datos PostgreSQL con **PostGIS**, se generan varias tablas. A continuación, te explicaré las **tablas principales** que suele contener la base de datos OSM y su función específica.

## **Tablas Generadas por osm2pgsql**

1.  **planet\_osm\_point**
2.  **planet\_osm\_line**
3.  **planet\_osm\_polygon**
4.  **planet\_osm\_roads**
5.  **planet\_osm\_nodes**
6.  **planet\_osm\_ways**
7.  **planet\_osm\_rels**
8.  **osm2pgsql\_properties**
9.  **spatial\_ref\_sys**

---

### **1\.** `**planet_osm_point**`

**Descripción**: Contiene elementos **puntuales** como puntos de interés (cafés, restaurantes, estaciones, etc.). Cada entrada representa un **nodo individual** con una ubicación precisa (latitud y longitud).

**Ejemplos**:

*   Estaciones de servicio, paradas de autobús, monumentos, etc.

**Columnas**:

*   `**osm_id**`: ID del objeto en OSM.
*   `**name**`: Nombre del lugar.
*   `**amenity**`**,** `**shop**`**,** `**leisure**`: Etiquetas que describen el tipo de lugar.
*   `**way**`: Geometría del punto (tipo `Point`).

**Ejemplo de Consulta**:

---

### **2\.** `**planet_osm_line**`

**Descripción**: Almacena **elementos lineales**, como carreteras, caminos, vías férreas, o ríos. Cada fila representa un **way** (camino) en OSM.

**Ejemplos**:

*   Carreteras, ríos, vías de tren, ciclovías.

**Columnas**:

*   `**osm_id**`: ID del way en OSM.
*   `**name**`: Nombre de la vía.
*   `**highway**`**,** `**waterway**`**,** `**railway**`: Etiquetas para clasificar el tipo de línea.
*   `**way**`: Geometría del elemento (tipo `LineString`).

**Ejemplo de Consulta**:

---

### **3\.** `**planet_osm_polygon**`

**Descripción**: Contiene **elementos poligonales** como edificios, parques, lagos y áreas residenciales. Estos elementos tienen una geometría que define un **área cerrada**.

**Ejemplos**:

*   Parques, edificios, áreas urbanas o naturales.

**Columnas**:

*   `**osm_id**`: ID del polígono en OSM.
*   `**name**`: Nombre del área.
*   `**leisure**`**,** `**building**`**,** `**natural**`: Etiquetas para definir el tipo de área.
*   `**way**`: Geometría del elemento (tipo `Polygon`).

**Ejemplo de Consulta**:

---

### **4\.** `**planet_osm_roads**`

**Descripción**: Esta tabla es una versión **optimizada** de `planet_osm_line` que contiene solo **carreteras** y rutas relevantes para la navegación.

**Ejemplos**:

*   Autopistas, calles residenciales, carreteras principales.

**Columnas**:

*   `**osm_id**`: ID del way en OSM.
*   `**name**`: Nombre de la carretera.
*   `**highway**`: Tipo de carretera.
*   `**way**`: Geometría del elemento (tipo `LineString`).

**Ejemplo de Consulta**:

---

### **5\.** `**planet_osm_nodes**`

**Descripción**: Contiene todos los **nodos** importados desde OSM. Los nodos son puntos básicos que pueden ser usados para definir **ways** o ser puntos de interés independientes.

**Ejemplos**:

*   Puntos de intersección en una carretera.

**Columnas**:

*   `**osm_id**`: ID del nodo en OSM.
*   `**tags**`: Etiquetas asociadas al nodo.
*   `**way**`: Geometría del nodo (tipo `Point`).

---

### **6\.** `**planet_osm_ways**`

**Descripción**: Almacena **ways** (caminos) importados de OSM. Los ways son secuencias de nodos que pueden representar calles, senderos o bordes de polígonos.

**Ejemplos**:

*   Líneas de costa, caminos, límites administrativos.

**Columnas**:

*   `**osm_id**`: ID del way en OSM.
*   `**tags**`: Etiquetas asociadas al way.
*   `**way**`: Geometría del elemento (tipo `LineString` o `Polygon`).

---

### **7\.** `**planet_osm_rels**`

**Descripción**: Almacena **relaciones** de OSM. Las relaciones agrupan varios nodos, ways o incluso otras relaciones para definir un objeto complejo.

**Ejemplos**:

*   Rutas de transporte público, límites de una ciudad.

**Columnas**:

*   `**osm_id**`: ID de la relación en OSM.
*   `**member_id**`: ID de los miembros de la relación.
*   `**role**`: Rol de cada miembro en la relación.

---

### **8\.** `**osm2pgsql_properties**`

**Descripción**: Esta tabla almacena propiedades adicionales que fueron extraídas durante la importación desde OSM. Puede contener información clave-valor para etiquetas específicas.

**Ejemplo**:

*   Información adicional sobre amenidades o tipos de transporte.

**Columnas**:

*   `**k**`: Clave de la etiqueta.
*   `**v**`: Valor de la etiqueta.

---

### **9\.** `**spatial_ref_sys**`

*   **Descripción**: Esta es una tabla estándar de PostGIS que contiene las definiciones de sistemas de referencia espacial (SRID). Asegura que las geometrías se interpreten correctamente en términos de coordenadas.
*   **Ejemplo**:
    *   SRID 4326, que utiliza latitud y longitud.

---

## **Resumen de las Tablas y Su Uso**

| **Tabla** | **Descripción** | **Tipo de Geometría** | **Uso Principal** |
| --- | --- | --- | --- |
| `planet_osm_point` | Puntos de interés (cafés, paradas, etc.) | `Point` | Consultas sobre lugares específicos. |
| `planet_osm_line` | Elementos lineales (carreteras, ríos) | `LineString` | Análisis de redes de transporte. |
| `planet_osm_polygon` | Áreas cerradas (parques, edificios) | `Polygon` | Análisis de áreas geográficas. |
| `planet_osm_roads` | Carreteras y rutas de navegación | `LineString` | Optimización para navegación. |
| `planet_osm_nodes` | Nodos individuales | `Point` | Estructura básica para ways y relaciones. |
| `planet_osm_ways` | Caminos formados por nodos | `LineString` o `Polygon` | Representación de caminos y polígonos. |
| `planet_osm_rels` | Relaciones complejas (rutas, límites) | Varia según el tipo | Agrupación de elementos relacionados. |
| `osm2pgsql_properties` | Propiedades adicionales | \- | Información extraída de etiquetas. |
| `spatial_ref_sys` | Definiciones de sistemas de referencia espacial | \- | Gestión de SRIDs y coordenadas. |

---

## **Conclusión**

La base de datos **OSM** importada con **osm2pgsql** organiza los datos en varias tablas que facilitan el acceso a diferentes tipos de elementos geoespaciales. Puedes utilizar estas tablas para realizar consultas específicas sobre puntos, líneas y áreas, o para analizar redes complejas como rutas de transporte público.

Si tienes alguna otra pregunta o necesitas ejemplos adicionales de consultas, ¡no dudes en preguntar!

```
SELECT name, highway, way FROM planet_osm_roads WHERE highway = 'motorway';
```

```
SELECT name, leisure, way FROM planet_osm_polygon WHERE leisure = 'park';
```

```
SELECT name, highway, way FROM planet_osm_line WHERE highway IS NOT NULL;
```

```
SELECT name, amenity, way FROM planet_osm_point WHERE amenity = 'cafe';
```