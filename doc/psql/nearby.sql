CREATE OR REPLACE FUNCTION get_nearby_routes(
    latitude DOUBLE PRECISION, 
    longitude DOUBLE PRECISION, 
    max_distance DOUBLE PRECISION
)
RETURNS TABLE (
    route_id INT,
    bus_id INT,
    direction_id INT,
    route_geometry GEOMETRY,
    distance DOUBLE PRECISION,
    number_route VARCHAR(255),
    code_route VARCHAR(255),
    fees DOUBLE PRECISION,
    special_fees DOUBLE PRECISION,
    first_trip TIMESTAMP,
    last_trip TIMESTAMP,
    frequency INTERVAL,
    photo_url VARCHAR(255)
) AS $$
BEGIN
    RETURN QUERY
    WITH point AS (
        SELECT ST_SetSRID(ST_MakePoint(longitude, latitude), 4326) AS geom
    )
    SELECT r.id AS route_id,
           r.bus_id,
           r.direction_id,
           r.geometry AS route_geometry,
           ST_Distance(p.geom, r.geometry) AS distance,
           b.number_route,
           b.code_route,
           b.fees,
           b.special_fees,
           b.first_trip,
           b.last_trip,
           b.frequency,
           b.photo_url
    FROM routes r
    JOIN buses b ON r.bus_id = b.id
    JOIN point p ON ST_DWithin(p.geom, r.geometry, max_distance)
    ORDER BY distance
    LIMIT 5;
END;
$$ LANGUAGE plpgsql;
