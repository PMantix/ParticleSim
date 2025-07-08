#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{Body, Species};
    use ultraviolet::Vec2;
    use crate::body::foil::Foil;

    #[test]
    fn dendrite_rate_counts_transitions() {
        let mut plotting = PlottingSystem::new(1.0);
        let config = PlotConfig {
            plot_type: PlotType::TimeSeries,
            quantity: Quantity::DendriteFormationRate,
            title: "rate".to_string(),
            sampling_mode: SamplingMode::SingleTimestep,
            spatial_bins: 10,
            time_window: 10.0,
            update_frequency: 1.0,
        };
        let window_id = plotting.create_plot_window(config);

        // Initial ion
        let mut bodies = vec![Body::new(Vec2::zero(), Vec2::zero(), 1.0, 1.0, 1.0, Species::LithiumIon)];
        let foils: Vec<Foil> = Vec::new();

        // First update at t=0 to set baseline
        plotting.update_plots(&bodies, &foils, 0.0);

        // Ion becomes metal
        bodies[0].species = Species::LithiumMetal;

        // Second update after 1s
        plotting.update_plots(&bodies, &foils, 1.0);
        let window = plotting.windows.get(&window_id).unwrap();
        assert_eq!(window.data.y_data.len(), 1);
        assert!((window.data.y_data[0] - 1.0).abs() < 1e-6);
    }
}

