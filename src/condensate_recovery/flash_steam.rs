/// 플래시 증기 계산 입력.
#[derive(Debug, Clone)]
pub struct FlashSteamInput {
    /// 고압 응축수 엔탈피 [kJ/kg]
    pub condensate_enthalpy_high_kj_per_kg: f64,
    /// 저압 포화수 엔탈피 [kJ/kg]
    pub saturated_liquid_low_kj_per_kg: f64,
    /// 저압 포화 증기 엔탈피 [kJ/kg]
    pub saturated_vapor_low_kj_per_kg: f64,
}

/// 플래시 증기 결과.
#[derive(Debug, Clone)]
pub struct FlashSteamResult {
    /// 플래시 증기 질량 비율(kg steam / kg condensate)
    pub flash_fraction: f64,
}

/// 엔탈피 보존으로 플래시 증기 발생 비율을 계산한다.
pub fn flash_steam(input: FlashSteamInput) -> FlashSteamResult {
    let denom = input.saturated_vapor_low_kj_per_kg - input.saturated_liquid_low_kj_per_kg;
    let flash_fraction = if denom > 0.0 {
        ((input.condensate_enthalpy_high_kj_per_kg - input.saturated_liquid_low_kj_per_kg) / denom)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    FlashSteamResult { flash_fraction }
}
