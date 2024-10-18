Aquí tienes una **pequeña guía paso a paso** para migrar un archivo **`.osm.pbf`** (como el que mencionaste) a PostgreSQL utilizando **`osm2pgsql`** y PostGIS.

---

## **Guía Rápida para Migrar un Archivo `.osm.pbf` a PostgreSQL**

### **Paso 1: Instalación de PostgreSQL y PostGIS**

1. **Instala PostgreSQL y PostGIS** en tu sistema.

   - **Ubuntu/Debian**:
     ```bash
     sudo apt update
     sudo apt install postgresql postgis
     ```

   - **Windows/MacOS**:
     - Descarga PostgreSQL desde: [https://www.postgresql.org/download](https://www.postgresql.org/download) y habilita **PostGIS** durante la instalación.

2. **Verifica que PostgreSQL y PostGIS están instalados**:
   ```bash
   psql --version
   ```

---

### **Paso 2: Crear una Base de Datos con PostGIS**

1. Cambia al usuario `postgres`:
   ```bash
   sudo -i -u postgres
   ```

2. Inicia la consola de PostgreSQL:
   ```bash
   psql
   ```

3. Crea una nueva base de datos (ejemplo: `osmdb`):
   ```sql
   CREATE DATABASE osmdb;
   ```

4. Conéctate a la base de datos:
   ```sql
   \c osmdb
   ```

5. Habilita la extensión **PostGIS**:
   ```sql
   CREATE EXTENSION postgis;
   CREATE EXTENSION hstore;
   ```

6. Sal de la consola:
   ```sql
   \q
   ```

---

### **Paso 3: Instalar `osm2pgsql`**

1. **Instala osm2pgsql**:

   - **Ubuntu/Debian**:
     ```bash
     sudo apt install osm2pgsql
     ```

   - **MacOS** (usando Homebrew):
     ```bash
     brew install osm2pgsql
     ```

2. **Verifica la instalación**:
   ```bash
   osm2pgsql --version
   ```

---

### **Paso 4: Descargar un Archivo `.osm.pbf`**

1. Si ya tienes el archivo (por ejemplo, `el-salvador-latest.osm.pbf`), asegúrate de tenerlo en tu sistema.

2. Si necesitas descargar uno, visita:
   - [Geofabrik](https://download.geofabrik.de/)
   - [BBBike](https://download.bbbike.org/osm/)

---

### **Paso 5: Importar el Archivo `.osm.pbf` a PostgreSQL**

1. **Ejecuta el siguiente comando** para importar los datos:

   ```bash
   osm2pgsql -d osmdb --create --slim -G --hstore -C 2000 --number-processes 4 ~/Descargas/el-salvador-latest.osm.pbf
   ```

   **Explicación de los parámetros:**
   - **`-d osmdb`**: Nombre de la base de datos.
   - **`--create`**: Crea nuevas tablas para almacenar los datos.
   - **`--slim`**: Utiliza el modo "slim", necesario para grandes archivos.
   - **`-G`**: Crea geometrías utilizando relaciones multiparte.
   - **`--hstore`**: Almacena las etiquetas OSM como un tipo `hstore`.
   - **`-C 2000`**: Asigna 2000 MB de RAM para el proceso.
   - **`--number-processes 4`**: Usa 4 procesos para la importación (ajusta según tu CPU).

---

### **Paso 6: Verificar las Tablas Importadas**

1. Cambia al usuario `postgres` (si no lo estás ya):
   ```bash
   sudo -i -u postgres
   ```

2. Inicia la consola de PostgreSQL y conéctate a la base de datos:
   ```bash
   psql -d osmdb
   ```

3. Lista las tablas importadas:
   ```sql
   \dt
   ```

   Deberías ver tablas como:
   - `planet_osm_point`
   - `planet_osm_line`
   - `planet_osm_polygon`
   - `planet_osm_roads`

4. Verifica que las tablas contienen datos:
   ```sql
   SELECT * FROM planet_osm_point LIMIT 5;
   ```

5. Sal de la consola PostgreSQL:
   ```sql
   \q
   ```

---

### **Paso 7: Optimización (Opcional)**

1. **Crear índices espaciales** para mejorar el rendimiento de las consultas:
   ```sql
   CREATE INDEX idx_planet_osm_point_way ON planet_osm_point USING GIST (way);
   CREATE INDEX idx_planet_osm_line_way ON planet_osm_line USING GIST (way);
   CREATE INDEX idx_planet_osm_polygon_way ON planet_osm_polygon USING GIST (way);
   ```

2. **Vaciar y analizar la base de datos** para optimizar el rendimiento:
   ```sql
   VACUUM ANALYZE;
   ```

---

### **Paso 8: Consultas Básicas**

Aquí tienes algunos ejemplos de consultas que puedes hacer para verificar los datos importados:

- **Buscar cafés en los datos importados**:
  ```sql
  SELECT name, amenity, way FROM planet_osm_point WHERE amenity = 'cafe';
  ```

- **Contar el número de parques**:
  ```sql
  SELECT COUNT(*) FROM planet_osm_polygon WHERE leisure = 'park';
  ```

- **Listar las carreteras principales**:
  ```sql
  SELECT name, highway FROM planet_osm_roads WHERE highway IN ('primary', 'secondary');
  ```

---

### **Paso 9: Visualización de Datos (Opcional)**

Si deseas visualizar los datos en un programa SIG como **QGIS**:

1. Abre **QGIS**.
2. Ve a **Capa > Añadir Capa > Añadir Capa PostGIS**.
3. Conéctate a tu base de datos PostgreSQL (`osmdb`).
4. Carga las capas (por ejemplo, `planet_osm_point`, `planet_osm_line`) y explora los datos visualmente.
