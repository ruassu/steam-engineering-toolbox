/// 감압 시 건도를 계산하기 위한 입력.
#[derive(Debug, Clone)]
pub struct PressureReductionInput {
    /// 기존 건도(0~1)
    pub initial_dryness: f64,
    /// 감압 전 압력 [bar]
    pub pressure_before_bar: f64,
    /// 감압 후 압력 [bar]
    pub pressure_after_bar: f64,
    /// 감압 전 비엔탈피(kJ/kg)
    pub enthalpy_before_kj_per_kg: f64,
    /// 감압 후 포화 증기 비엔탈피(kJ/kg)
    pub enthalpy_sat_vapor_after_kj_per_kg: f64,
    /// 감압 후 포화수 비엔탈피(kJ/kg)
    pub enthalpy_sat_liquid_after_kj_per_kg: f64,
}

/// 건도 계산 결과.
#[derive(Debug, Clone)]
pub struct DrynessResult {
    /// 감압 후 건도
    pub dryness: f64,
}

/// 단순 엔탈피 보존을 이용해 감압 후 건도를 계산한다.
pub fn dryness_after_pressure_reduction(input: PressureReductionInput) -> DrynessResult {
    let h_before = input.enthalpy_before_kj_per_kg;
    let h_l_after = input.enthalpy_sat_liquid_after_kj_per_kg;
    let h_v_after = input.enthalpy_sat_vapor_after_kj_per_kg;
    let dryness = if h_v_after > h_l_after {
        ((h_before - h_l_after) / (h_v_after - h_l_after)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    DrynessResult { dryness }
}

/// 응축수 분리기를 추가로 사용하는 경우 건도 개선 효과를 반영한다.
pub fn dryness_with_separation(
    dryness_after_pr: DrynessResult,
    separator_efficiency: f64,
) -> DrynessResult {
    // 분리 효율만큼 습분을 제거한다고 가정
    let residual_moisture = (1.0 - dryness_after_pr.dryness) * (1.0 - separator_efficiency);
    DrynessResult {
        dryness: (1.0 - residual_moisture).clamp(0.0, 1.0),
    }
}

/// 공기가 혼입된 증기의 혼합 온도를 근사 계산한다.
///
/// 단순 비열 혼합식: T_mix = (m_s*cp_s*T_s + m_a*cp_a*T_a) / (m_s*cp_s + m_a*cp_a)
pub fn mixed_steam_air_temperature(
    steam_mass_flow_kg_s: f64,
    air_mass_flow_kg_s: f64,
    steam_temp_c: f64,
    air_temp_c: f64,
) -> f64 {
    let cp_steam = 2.0; // kJ/kgK 근사
    let cp_air = 1.0; // kJ/kgK 근사
    let numerator =
        steam_mass_flow_kg_s * cp_steam * steam_temp_c + air_mass_flow_kg_s * cp_air * air_temp_c;
    let denominator = steam_mass_flow_kg_s * cp_steam + air_mass_flow_kg_s * cp_air;
    if denominator > 0.0 {
        numerator / denominator
    } else {
        steam_temp_c
    }
}
