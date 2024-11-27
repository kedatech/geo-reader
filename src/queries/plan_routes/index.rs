use super::{
    geo_validation::{GeoValidator, ValidationResult},
    spatial_search::{SpatialSearch, SearchError}
};
use crate::plan_routes::_structs::*;
use geo_types::Point;
use tracing::{debug, info, warn, error};

#[derive(Debug, thiserror::Error)]
pub enum PlanningError {
    #[error("Geographic validation error: {0}")]
    ValidationError(#[from] crate::queries::plan_routes::geo_validation::ValidationError),
    #[error("Search error: {0}")]
    SearchError(#[from] SearchError),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("No valid routes found")]
    NoValidRoutes,
}

#[derive(Debug)]
pub struct PlanningConfig {
    pub max_route_distance: f64,    // 5km para encontrar rutas cercanas
    pub max_transfer_distance: f64, // 1km para transbordos próximos
    pub max_transfers: i32,         // máximo 10 transbordos
    pub results_limit: usize,       // máximo 3 planes diferentes
}

impl Default for PlanningConfig {
    fn default() -> Self {
        Self {
            max_route_distance: 0.05,     // ~5km en grados
            max_transfer_distance: 0.01,   // ~1km en grados
            max_transfers: 10,
            results_limit: 3,
        }
    }
}

pub struct RoutePlanner {
    config: PlanningConfig,
    validator: GeoValidator,
    search: SpatialSearch,
}

impl RoutePlanner {
    pub fn new(
        validator: GeoValidator, 
        search: SpatialSearch,
        config: Option<PlanningConfig>
    ) -> Self {
        Self {
            config: config.unwrap_or_default(),
            validator,
            search,
        }
    }

    /// Planifica rutas entre dos puntos
    pub fn plan_route(&self, origin: Point<f64>, destination: Point<f64>) 
        -> Result<Vec<RoutePlan>, PlanningError> 
    {
        // 1. Validar puntos y determinar contexto geográfico
        info!("Validating geographic points");
        let validation = self.validator.validate_route(origin, destination)?;
        
        if !validation.is_valid {
            error!("Invalid points for route planning");
            return Err(PlanningError::ValidationError(
                crate::queries::plan_routes::geo_validation::ValidationError::InvalidCoordinates
            ));
        }

        // 2. Crear request con configuración apropiada
        let request = self.create_route_request(
            origin, 
            destination,
            &validation
        );

        // 3. Buscar rutas posibles
        info!("Searching for possible routes");
        debug!("Search request: {:?}", request);
        
        let mut plans = self.search.find_routes_to_destination(
            request.origin,
            request.destination,
            request.max_transfers,
            request.max_route_distance,
        ).map_err(PlanningError::SearchError)?;

        // 4. Optimizar y filtrar resultados
        self.optimize_results(&validation, &mut plans);

        // 5. Validar y retornar planes finales
        if plans.is_empty() {
            return Err(PlanningError::NoValidRoutes);
        }

        Ok(plans.into_iter().take(self.config.results_limit).collect())
    }

    /// Crea una solicitud de ruta con la configuración apropiada
    fn create_route_request(&self, origin: Point<f64>, destination: Point<f64>, validation: &ValidationResult) 
        -> RouteRequest 
    {
        let mut request = RouteRequest {
            origin,
            destination,
            max_route_distance: self.config.max_route_distance,
            max_transfer_distance: self.config.max_transfer_distance,
            max_transfers: self.config.max_transfers,
        };

        // Ajustar parámetros según el contexto
        if validation.is_interdepartmental {
            // Para rutas interdepartamentales, aumentamos las distancias de búsqueda
            request.max_route_distance *= 1.5;
            request.max_transfer_distance *= 1.5;
        }

        // Si el punto está cerca de un límite departamental, aumentar radio de búsqueda
        if validation.distance_to_boundary < self.config.max_transfer_distance {
            request.max_route_distance *= 1.2;
        }

        request
    }

    /// Optimiza y ordena los resultados según múltiples criterios
    fn optimize_results(&self, validation: &ValidationResult, plans: &mut Vec<RoutePlan>) {
        // Calcular scores para cada plan
        let mut plan_scores: Vec<(usize, f64)> = plans
            .iter()
            .enumerate()
            .map(|(idx, plan)| {
                let score = self.calculate_plan_score(plan, validation);
                (idx, score)
            })
            .collect();

        // Ordenar por score (menor es mejor)
        plan_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Reordenar planes según los scores
        let ordered_plans: Vec<RoutePlan> = plan_scores
            .into_iter()
            .map(|(idx, _)| plans[idx].clone())
            .collect();

        *plans = ordered_plans;
    }

    /// Calcula un score para un plan basado en múltiples factores
    fn calculate_plan_score(&self, plan: &RoutePlan, validation: &ValidationResult) -> f64 {
        let mut score = 0.0;

        // Factor: Número de transbordos (peso alto)
        score += plan.transfers_count as f64 * 10.0;

        // Factor: Distancia total (normalizada)
        score += plan.total_distance / self.config.max_route_distance;

        // Factor: Tipos de transbordo
        for route in &plan.routes {
            match route.transfer_type {
                TransferType::Direct => score += 0.0,   // Mejor caso
                TransferType::Near => score += 2.0,     // Penalización media
                TransferType::Proximate => score += 5.0, // Mayor penalización
            }
        }

        // Bonus: Si es ruta interdepartamental cuando se necesita
        if validation.is_interdepartmental && plan.is_interdepartmental {
            score *= 0.8; // 20% de bonus
        }

        // Penalización: Si es ruta interdepartamental cuando no se necesita
        if !validation.is_interdepartmental && plan.is_interdepartmental {
            score *= 1.2; // 20% de penalización
        }

        score
    }

    // Métodos de utilidad para acceder a componentes internos si es necesario
    pub fn validator(&self) -> &GeoValidator {
        &self.validator
    }

    pub fn search(&self) -> &SpatialSearch {
        &self.search
    }

    pub fn config(&self) -> &PlanningConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo_types::Point;

    // Helper para crear un plan de prueba
    fn create_test_plan(transfers: i32, distance: f64, is_interdept: bool) -> RoutePlan {
        let mut plan = RoutePlan::new();
        plan.total_distance = distance;
        plan.transfers_count = transfers;
        plan.is_interdepartmental = is_interdept;
        plan
    }

    #[test]
    fn test_plan_scoring() {
        // Implementar tests específicos para el scoring
        // TODO: Agregar casos de prueba
    }

    #[test]
    fn test_route_planning() {
        // Implementar tests completos de planificación
        // TODO: Agregar casos de prueba
    }
}