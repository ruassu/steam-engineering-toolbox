/// 단순 파이프 압력손실(약압축성) 계산 입력.
#[derive(Debug, Clone)]
pub struct GasPressureLossInput {
    pub flow_m3_per_h: f64,
    pub density_kg_per_m3: f64,
    pub dynamic_viscosity_pa_s: f64,
    pub diameter_m: f64,
    pub length_m: f64,
    pub roughness_m: f64,
}

/// 가스 압력손실 결과.
#[derive(Debug, Clone)]
pub struct GasPressureLossResult {
    pub velocity_m_per_s: f64,
    pub pressure_drop_bar: f64,
    pub reynolds: f64,
    pub friction_factor: f64,
}

/// Darcy-Weisbach 기반 가스 배관 압력손실 계산(저압/약압축성 근사).
pub fn gas_pressure_loss(input: GasPressureLossInput) -> GasPressureLossResult {
    let flow_m3_s = input.flow_m3_per_h / 3600.0;
    let area = std::f64::consts::PI * input.diameter_m * input.diameter_m / 4.0;
    let velocity = flow_m3_s / area;
    let reynolds =
        input.density_kg_per_m3 * velocity * input.diameter_m / input.dynamic_viscosity_pa_s;
    let friction_factor = if reynolds < 2300.0 {
        64.0 / reynolds.max(1.0)
    } else {
        let roughness_ratio = input.roughness_m / input.diameter_m;
        let log_term = (roughness_ratio / 3.7).powf(1.11) + 6.9 / reynolds;
        let inv_sqrt_f = -1.8 * log_term.log10();
        1.0 / inv_sqrt_f.powi(2)
    };
    let delta_p_pa = friction_factor
        * (input.length_m / input.diameter_m)
        * input.density_kg_per_m3
        * velocity
        * velocity
        / 2.0;
    GasPressureLossResult {
        velocity_m_per_s: velocity,
        pressure_drop_bar: delta_p_pa / 100_000.0,
        reynolds,
        friction_factor,
    }
}
