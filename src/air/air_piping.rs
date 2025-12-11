/// 공기 배관 압력손실 입력(약압축성 근사).
#[derive(Debug, Clone)]
pub struct AirPressureLossInput {
    /// 체적 유량(실제 조건) [m3/h]
    pub flow_m3_per_h: f64,
    /// 밀도 [kg/m3]
    pub density_kg_per_m3: f64,
    /// 동점도 [Pa·s]
    pub dynamic_viscosity_pa_s: f64,
    /// 내경 [m]
    pub diameter_m: f64,
    /// 길이 [m]
    pub length_m: f64,
    /// 거칠기 [m]
    pub roughness_m: f64,
}

/// 공기 압력손실 결과.
#[derive(Debug, Clone)]
pub struct AirPressureLossResult {
    pub velocity_m_per_s: f64,
    pub pressure_drop_bar: f64,
    pub reynolds: f64,
    pub friction_factor: f64,
}

/// Darcy-Weisbach를 약압축성으로 적용해 공기 배관 압력손실을 구한다.
pub fn air_pressure_loss(input: AirPressureLossInput) -> AirPressureLossResult {
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
        0.25 / (log_term.log10().powi(2))
    };
    let delta_p_pa = friction_factor
        * (input.length_m / input.diameter_m)
        * input.density_kg_per_m3
        * velocity
        * velocity
        / 2.0;
    AirPressureLossResult {
        velocity_m_per_s: velocity,
        pressure_drop_bar: delta_p_pa / 100_000.0,
        reynolds,
        friction_factor,
    }
}

/// 오리피스 유량 근사(비압축성)로 공기 유량을 계산한다. 간단 참고용.
pub fn air_orifice_flow_cv(cv: f64, delta_p_bar: f64, density_kg_per_m3: f64) -> f64 {
    let rho_ref = 1000.0;
    let kv = cv * 0.865;
    kv * (delta_p_bar * density_kg_per_m3 / rho_ref).sqrt()
}
