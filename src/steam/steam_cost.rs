/// 에너지 단가 계산 입력.
#[derive(Debug, Clone)]
pub struct EnergyUnitCostInput {
    /// 연료 단가 [원 / 연료단위]
    pub fuel_price_per_unit: f64,
    /// 연료 발열량 [kJ / 연료단위]
    pub fuel_lhv_kj_per_unit: f64,
    /// 보일러 효율(0~1)
    pub boiler_efficiency: f64,
}

/// 에너지 단가 계산 결과.
#[derive(Debug, Clone)]
pub struct EnergyUnitCostResult {
    /// 에너지 단가 [원 / kJ]
    pub cost_per_kj: f64,
    /// 에너지 단가 [원 / MJ]
    pub cost_per_mj: f64,
}

/// 보일러 효율과 연료 단가로 에너지 단가를 계산한다.
pub fn energy_unit_cost(input: EnergyUnitCostInput) -> EnergyUnitCostResult {
    let useful_kj = input.fuel_lhv_kj_per_unit * input.boiler_efficiency.max(0.0);
    let cost_per_kj = if useful_kj > 0.0 {
        input.fuel_price_per_unit / useful_kj
    } else {
        0.0
    };
    EnergyUnitCostResult {
        cost_per_kj,
        cost_per_mj: cost_per_kj * 1000.0,
    }
}

/// 증기 단가 계산 입력.
#[derive(Debug, Clone)]
pub struct SteamUnitCostInput {
    /// 에너지 단가 [원/kJ]
    pub energy_cost_per_kj: f64,
    /// 증기 잠열 [kJ/kg]
    pub steam_latent_heat_kj_per_kg: f64,
    /// 블로다운/복수 손실 계수(0~1, 0.1이면 10% 추가 에너지 필요)
    pub loss_factor: f64,
}

/// 증기 단가 결과.
#[derive(Debug, Clone)]
pub struct SteamUnitCostResult {
    /// 증기 단가 [원/kg]
    pub cost_per_kg: f64,
    /// 증기 단가 [원/ton]
    pub cost_per_ton: f64,
}

/// 증기 생산 단가를 계산한다.
pub fn steam_unit_cost(input: SteamUnitCostInput) -> SteamUnitCostResult {
    let effective_latent = input.steam_latent_heat_kj_per_kg * (1.0 + input.loss_factor.max(0.0));
    let cost_per_kg = if effective_latent > 0.0 {
        input.energy_cost_per_kj * effective_latent
    } else {
        0.0
    };
    SteamUnitCostResult {
        cost_per_kg,
        cost_per_ton: cost_per_kg * 1000.0,
    }
}
