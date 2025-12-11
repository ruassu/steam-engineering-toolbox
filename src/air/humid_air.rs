/// 습도비 계산 결과.
#[derive(Debug, Clone)]
pub struct HumidAirState {
    /// 상대습도 [%]
    pub relative_humidity_pct: f64,
    /// 습도비 [kg수증기/kg건공기]
    pub humidity_ratio: f64,
    /// 포화 수증기 분압 [kPa]
    pub vapor_pressure_kpa: f64,
}

/// 건구온도와 상대습도로 습도비를 근사 계산한다.
///
/// Tetens 식으로 포화수증기압을 추정한 후, W = 0.622 * Pv / (P - Pv) 를 사용한다.
pub fn humidity_ratio_from_rh(
    dry_bulb_c: f64,
    relative_humidity_pct: f64,
    total_pressure_kpa: f64,
) -> HumidAirState {
    let rh = (relative_humidity_pct / 100.0).clamp(0.0, 1.0);
    let p_sat = saturation_pressure_tetens_kpa(dry_bulb_c);
    let pv = rh * p_sat;
    let w = 0.622 * pv / (total_pressure_kpa.max(pv + 1e-6) - pv);
    HumidAirState {
        relative_humidity_pct: rh * 100.0,
        humidity_ratio: w,
        vapor_pressure_kpa: pv,
    }
}

fn saturation_pressure_tetens_kpa(t_c: f64) -> f64 {
    // Tetens: Psat(kPa) = 0.61078 * exp(17.27*T / (T+237.3))
    0.61078 * (17.27 * t_c / (t_c + 237.3)).exp()
}
