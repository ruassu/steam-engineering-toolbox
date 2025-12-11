/// 배관 가열 시 발생하는 응축수량 계산 입력.
#[derive(Debug, Clone)]
pub struct StartupCondensateInput {
    /// 배관 금속 질량 [kg]
    pub pipe_metal_mass_kg: f64,
    /// 금속 비열 [kJ/kgK]
    pub pipe_specific_heat_kj_per_kgk: f64,
    /// 초기 온도 [°C]
    pub initial_temp_c: f64,
    /// 목표 온도(증기 온도 근사) [°C]
    pub target_temp_c: f64,
    /// 증기 잠열 [kJ/kg] (포화 증기에서 응축 시)
    pub steam_latent_heat_kj_per_kg: f64,
}

/// 배관 가열 시 발생 응축수량 결과.
#[derive(Debug, Clone)]
pub struct StartupCondensateResult {
    /// 필요한 열량 [kJ]
    pub required_energy_kj: f64,
    /// 발생 응축수량 [kg]
    pub condensate_mass_kg: f64,
}

/// 배관이 냉간에서 증기 온도로 가열될 때 발생하는 응축수량을 계산한다.
///
/// 단순히 금속 열용량만 고려하며, 외부 손실/단열 효과는 무시한다.
pub fn condensate_load_startup(input: StartupCondensateInput) -> StartupCondensateResult {
    let delta_t = (input.target_temp_c - input.initial_temp_c).max(0.0);
    let required_energy_kj =
        input.pipe_metal_mass_kg * input.pipe_specific_heat_kj_per_kgk * delta_t;
    let condensate_mass_kg = if input.steam_latent_heat_kj_per_kg > 0.0 {
        required_energy_kj / input.steam_latent_heat_kj_per_kg
    } else {
        0.0
    };
    StartupCondensateResult {
        required_energy_kj,
        condensate_mass_kg,
    }
}

/// 공정 가열(연속) 시 열부하와 응축수량 계산 입력.
#[derive(Debug, Clone)]
pub struct ContinuousHeatingInput {
    /// 유체 질량 유량 [kg/h]
    pub mass_flow_kg_per_h: f64,
    /// 유체 비열 [kJ/kgK]
    pub specific_heat_kj_per_kgk: f64,
    /// 입구 온도 [°C]
    pub inlet_temp_c: f64,
    /// 출구 온도 [°C]
    pub outlet_temp_c: f64,
    /// 증기 잠열 [kJ/kg]
    pub steam_latent_heat_kj_per_kg: f64,
}

/// 연속 가열 열부하 결과.
#[derive(Debug, Clone)]
pub struct ContinuousHeatingResult {
    /// 필요 열량 [kW]
    pub heat_load_kw: f64,
    /// 응축수량 [kg/h]
    pub condensate_kg_per_h: f64,
}

/// 연속 공정 가열에 필요한 열량과 응축수량을 계산한다.
pub fn condensate_load_continuous(input: ContinuousHeatingInput) -> ContinuousHeatingResult {
    let delta_t = (input.outlet_temp_c - input.inlet_temp_c).max(0.0);
    let heat_kj_per_h = input.mass_flow_kg_per_h * input.specific_heat_kj_per_kgk * delta_t;
    let heat_kw = heat_kj_per_h / 3600.0;
    let condensate = if input.steam_latent_heat_kj_per_kg > 0.0 {
        heat_kj_per_h / input.steam_latent_heat_kj_per_kg
    } else {
        0.0
    };
    ContinuousHeatingResult {
        heat_load_kw: heat_kw,
        condensate_kg_per_h: condensate,
    }
}

/// 배치 가열 시 필요한 열량과 응축수량을 계산한다.
pub fn condensate_load_batch(
    fluid_mass_kg: f64,
    specific_heat_kj_per_kgk: f64,
    initial_temp_c: f64,
    target_temp_c: f64,
    steam_latent_heat_kj_per_kg: f64,
) -> ContinuousHeatingResult {
    let delta_t = (target_temp_c - initial_temp_c).max(0.0);
    let heat_kj = fluid_mass_kg * specific_heat_kj_per_kgk * delta_t;
    let condensate = if steam_latent_heat_kj_per_kg > 0.0 {
        heat_kj / steam_latent_heat_kj_per_kg
    } else {
        0.0
    };
    ContinuousHeatingResult {
        heat_load_kw: heat_kj / 3600.0,
        condensate_kg_per_h: condensate,
    }
}

/// 단열이 없는 배관의 복사/대류 열손실을 추정하여 응축수량을 환산한다.
pub fn radiant_heat_loss_condensate(
    heat_loss_w: f64,
    steam_latent_heat_kj_per_kg: f64,
) -> ContinuousHeatingResult {
    let heat_kw = heat_loss_w / 1000.0;
    let heat_kj_per_h = heat_kw * 3600.0;
    let condensate = if steam_latent_heat_kj_per_kg > 0.0 {
        heat_kj_per_h / steam_latent_heat_kj_per_kg
    } else {
        0.0
    };
    ContinuousHeatingResult {
        heat_load_kw: heat_kw,
        condensate_kg_per_h: condensate,
    }
}

/// 스톨 포인트 계산 입력.
#[derive(Debug, Clone)]
pub struct StallPointInput {
    /// 코일/히터 내부 차압 [bar]
    pub coil_dp_bar: f64,
    /// 트랩이 필요한 최소 차압 [bar]
    pub trap_required_dp_bar: f64,
}

/// 스톨 포인트 결과.
#[derive(Debug, Clone)]
pub struct StallPointResult {
    /// 스톨 발생 여부
    pub is_stall: bool,
    /// 확보된 유효 차압 [bar]
    pub available_dp_bar: f64,
}

/// 코일 차압이 트랩 필요 차압보다 작은지 판단해 스톨 여부를 반환한다.
pub fn stall_point(input: StallPointInput) -> StallPointResult {
    let available = input.coil_dp_bar;
    StallPointResult {
        is_stall: available < input.trap_required_dp_bar,
        available_dp_bar: available,
    }
}
