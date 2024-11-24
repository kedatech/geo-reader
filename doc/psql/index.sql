CREATE INDEX idx_routes_geometry ON routes USING GIST(geometry);
CREATE INDEX idx_routes_geometry_geography ON routes USING GIST(geometry::geography);
