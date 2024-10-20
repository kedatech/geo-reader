SELECT
    l.osm_id AS line_id,
    ST_AsText(ST_StartPoint(l.geom)) AS start_point,
    ST_AsText(ST_EndPoint(l.geom)) AS end_point,
    ST_Length(l.geom::geography) AS distance
FROM
    planet_osm_line l;

--Este query traerá las líneas y sus nodos extremos (intersecciones o puntos) junto con la distancia entre ellos: