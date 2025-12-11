/// Darcy-Weisbach 기반 물 배관 압력손실 계산 입력.
#[derive(Debug, Clone)]
pub struct WaterPressureLossInput {
    /// 체적 유량 [m3/h]
    pub flow_m3_per_h: f64,
    /// 물 밀도 [kg/m3]
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

/// 압력손실 결과.
#[derive(Debug, Clone)]
pub struct WaterPressureLossResult {
    /// 유속 [m/s]
    pub velocity_m_per_s: f64,
    /// 압력강하 [bar]
    pub pressure_drop_bar: f64,
    /// 레이놀즈수
    pub reynolds: f64,
    /// 마찰계수
    pub friction_factor: f64,
}

/// Darcy-Weisbach 식으로 물 배관 압력손실을 계산한다.
pub fn water_pressure_loss(input: WaterPressureLossInput) -> WaterPressureLossResult {
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

    WaterPressureLossResult {
        velocity_m_per_s: velocity,
        pressure_drop_bar: delta_p_pa / 100_000.0,
        reynolds,
        friction_factor,
    }
}

/// 목표 유속을 만족하는 물 배관 내경을 계산한다.
pub fn water_pipe_size_for_velocity(flow_m3_per_h: f64, target_velocity_m_per_s: f64) -> f64 {
    let flow_m3_s = flow_m3_per_h / 3600.0;
    let area = flow_m3_s / target_velocity_m_per_s.max(0.1);
    (4.0 * area / std::f64::consts::PI).sqrt()
}
