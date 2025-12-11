/// 간단한 열수지 기반 보일러 효율 계산 입력.
#[derive(Debug, Clone)]
pub struct BoilerEfficiencyInput {
    /// 연료 소비량 [연료단위/h]
    pub fuel_flow_per_h: f64,
    /// 연료 발열량 LHV [kJ/연료단위]
    pub fuel_lhv_kj_per_unit: f64,
    /// 증기 생산량 [kg/h]
    pub steam_flow_kg_per_h: f64,
    /// 증기 엔탈피 [kJ/kg]
    pub steam_enthalpy_kj_per_kg: f64,
    /// 공급수 엔탈피 [kJ/kg]
    pub feedwater_enthalpy_kj_per_kg: f64,
}

/// 보일러 효율 계산 결과.
#[derive(Debug, Clone)]
pub struct BoilerEfficiencyResult {
    /// 열효율 (0~1)
    pub efficiency: f64,
    /// 연료 열량 투입 [kW]
    pub fuel_heat_kw: f64,
    /// 증기 생성 유효 열량 [kW]
    pub useful_heat_kw: f64,
}

/// 기본 열수지(증기엔탈피-급수엔탈피) 기반 보일러 효율을 계산한다.
pub fn boiler_efficiency(input: BoilerEfficiencyInput) -> BoilerEfficiencyResult {
    let fuel_heat_kj_per_h = input.fuel_flow_per_h * input.fuel_lhv_kj_per_unit;
    let useful_kj_per_h = input.steam_flow_kg_per_h
        * (input.steam_enthalpy_kj_per_kg - input.feedwater_enthalpy_kj_per_kg);
    let efficiency = if fuel_heat_kj_per_h > 0.0 {
        (useful_kj_per_h / fuel_heat_kj_per_h).clamp(0.0, 1.2)
    } else {
        0.0
    };
    BoilerEfficiencyResult {
        efficiency,
        fuel_heat_kw: fuel_heat_kj_per_h / 3600.0,
        useful_heat_kw: useful_kj_per_h / 3600.0,
    }
}

/// PTC 4.0 계산에 맞춰 스택 손실/복사손실/블로다운을 고려한 확장 입력.
#[derive(Debug, Clone)]
pub struct BoilerEfficiencyPtcInput {
    /// 연료 소비량 [연료단위/h]
    pub fuel_flow_per_h: f64,
    /// 연료 발열량 LHV [kJ/연료단위]
    pub fuel_lhv_kj_per_unit: f64,
    /// 증기 발생량 [kg/h]
    pub steam_flow_kg_per_h: f64,
    /// 증기 엔탈피 [kJ/kg]
    pub steam_enthalpy_kj_per_kg: f64,
    /// 공급수 엔탈피 [kJ/kg]
    pub feedwater_enthalpy_kj_per_kg: f64,
    /// 배가스 유량 [kg/h]
    pub flue_gas_flow_kg_per_h: f64,
    /// 배가스 정압비열 [kJ/kgK]
    pub flue_gas_cp_kj_per_kgk: f64,
    /// 배가스 온도 [°C]
    pub stack_temp_c: f64,
    /// 주변 공기 온도 [°C]
    pub ambient_temp_c: f64,
    /// 과잉 공기율 (예: 0.15 = 15%)
    pub excess_air_frac: f64,
    /// 복사/표면 손실 [% of fuel heat]
    pub radiation_loss_frac: f64,
    /// 블로다운 비율(급수 대비)
    pub blowdown_rate_frac: f64,
    /// 블로다운 배출 엔탈피 [kJ/kg]
    pub blowdown_enthalpy_kj_per_kg: f64,
}

/// PTC 4.0에 준해 스택 손실, 복사 손실, 블로다운 손실을 고려한 효율을 계산한다.
pub fn boiler_efficiency_ptc(input: BoilerEfficiencyPtcInput) -> BoilerEfficiencyResult {
    let fuel_heat_kj_per_h = input.fuel_flow_per_h * input.fuel_lhv_kj_per_unit;

    // 유효증기열
    let useful_kj_per_h = input.steam_flow_kg_per_h
        * (input.steam_enthalpy_kj_per_kg - input.feedwater_enthalpy_kj_per_kg);

    // 스택 손실: m_fg * cp * ΔT * (1 + 과잉공기 효과 계수)
    let delta_t = (input.stack_temp_c - input.ambient_temp_c).max(0.0);
    let excess_factor = 1.0 + input.excess_air_frac.max(0.0);
    let stack_loss_kj_per_h =
        input.flue_gas_flow_kg_per_h * input.flue_gas_cp_kj_per_kgk * delta_t * excess_factor;

    // 복사/표면 손실
    let radiation_loss_kj_per_h = fuel_heat_kj_per_h * input.radiation_loss_frac.max(0.0);

    // 블로다운 손실
    let blowdown_mass = input.steam_flow_kg_per_h * input.blowdown_rate_frac.max(0.0);
    let blowdown_loss_kj_per_h =
        blowdown_mass * (input.blowdown_enthalpy_kj_per_kg - input.feedwater_enthalpy_kj_per_kg);

    let total_losses = stack_loss_kj_per_h + radiation_loss_kj_per_h + blowdown_loss_kj_per_h;

    let efficiency = if fuel_heat_kj_per_h > 0.0 {
        ((fuel_heat_kj_per_h - total_losses) / fuel_heat_kj_per_h).clamp(0.0, 1.2)
    } else {
        0.0
    };

    BoilerEfficiencyResult {
        efficiency,
        fuel_heat_kw: fuel_heat_kj_per_h / 3600.0,
        useful_heat_kw: useful_kj_per_h / 3600.0,
    }
}
