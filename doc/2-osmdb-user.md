### Creación de un usuario en PostgreSQL para acceder a datos de OSM

1. **Conectar a PostgreSQL como administrador**  
   Abre la terminal y ejecuta:

   ```bash
   psql -U postgres
   ```

   Ingresa la contraseña si se te solicita.

2. **Crear un nuevo usuario**  
   Ejecuta el siguiente comando:

   ```sql
   CREATE USER geo_user WITH PASSWORD 'geo_password';
   ```

3. **Crear la base de datos (opcional)**  
   Si aún no tienes una base de datos para los datos OSM:

   ```sql
   CREATE DATABASE osmdb;
   ```

   Si ya tienes la base de datos creada, este paso no es necesario.

4. **Conectar a la base de datos**  
   Cambia a la base de datos donde están los datos:

   ```bash
   \c osmdb
   ```

5. **Otorgar permisos de conexión y uso del esquema**  
   Da permisos básicos al nuevo usuario:

   ```sql
   GRANT CONNECT ON DATABASE osmdb TO geo_user;
   GRANT USAGE ON SCHEMA public TO geo_user;
   ```

6. **Otorgar permisos de selección en las tablas**  
   Permite que el usuario pueda leer todas las tablas del esquema:

   ```sql
   GRANT SELECT ON ALL TABLES IN SCHEMA public TO geo_user;
   ```

7. **Otorgar permisos para futuras tablas (opcional)**  
   Para que el usuario tenga permisos automáticamente sobre nuevas tablas:

   ```sql
   ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO geo_user;
   ```

8. **Verificar los permisos otorgados**  
   Usa este comando para revisar los permisos:

   ```sql
   \dp planet_osm_point
   ```

   Debe mostrar algo como esto:

   ```
   Access privileges
    Schema |       Name        | Type  |   Access privileges   
   --------+-------------------+-------+------------------------
    public | planet_osm_point  | table | geo_user=SELECT/...
   ```

9. **Probar la conexión**  
   Asegúrate de que los datos de conexión en tu aplicación son correctos. La URL de conexión se verá así:

   ```text
   postgres://geo_user:geo_password@localhost/osmdb
   ```

Ahora el usuario geo_user tiene permisos para conectarse a la base de datos osmdb y consultar los datos.