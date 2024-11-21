     tablename
--------------------
 bus
 bus_stops
 buses
 departments
 detours
 direction_routes
 fav_routes
 planet_osm_line
 planet_osm_nodes
 planet_osm_point
 planet_osm_polygon
 planet_osm_rels
 planet_osm_roads
 planet_osm_ways
 roles
 route_bus_dto
 routes
 spatial_ref_sys
 stops
 subtype_bus
 subtype_buses
 terminals
 type_bus
 type_buses
 users


                                            Table "public.buses"
       Column       |            Type             | Collation | Nullable |              Default
--------------------+-----------------------------+-----------+----------+-----------------------------------
 id                 | integer                     |           | not null | nextval('buses_id_seq'::regclass)
 number_route       | character varying(255)      |           |          |
 code_route         | character varying(255)      |           |          |
 has_special        | boolean                     |           |          |
 fees               | double precision            |           |          |
 special_fees       | double precision            |           |          |
 first_trip         | timestamp without time zone |           |          |
 last_trip          | timestamp without time zone |           |          |
 frequency          | interval                    |           |          |
 approx_travel_time | interval                    |           |          |
 photo_url          | character varying(255)      |           |          |
 type_id            | integer                     |           |          |
 subtype_id         | integer                     |           |          |
 terminal_id        | integer                     |           |          |
 department_id      | integer                     |           |          |