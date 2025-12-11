/// Cv/Kv 계산 및 밸브 유량 추정을 위한 모듈.
#[derive(Debug)]
pub enum ValveCalcError {
    /// 입력값 오류
    InvalidInput(&'static str),
    /// 음속 임계 조건으로 유량 제한
    ChokedFlow(&'static str),
}

impl std::fmt::Display for ValveCalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValveCalcError::InvalidInput(msg) => write!(f, "입력 오류: {msg}"),
            ValveCalcError::ChokedFlow(msg) => write!(f, "Choked flow: {msg}"),
        }
    }
}

impl std::error::Error for ValveCalcError {}

/// Cv→Kv 변환 (Cv는 미국식, Kv는 SI 기반)
pub fn kv_from_cv(cv: f64) -> f64 {
    cv * 0.865
}

/// Kv→Cv 변환
pub fn cv_from_kv(kv: f64) -> f64 {
    kv / 0.865
}

/// 요구 Kv 값을 계산한다. 비압축성 근사식: Kv = Q * sqrt(ρ_ref / (ρ * ΔP))
/// - Q: m³/h, ΔP: bar, ρ_ref = 1000 kg/m³
pub fn required_kv(
    volumetric_flow_m3_per_h: f64,
    delta_p_bar: f64,
    fluid_density_kg_m3: f64,
) -> Result<f64, ValveCalcError> {
    if volumetric_flow_m3_per_h <= 0.0 || delta_p_bar <= 0.0 || fluid_density_kg_m3 <= 0.0 {
        return Err(ValveCalcError::InvalidInput(
            "유량, 차압, 밀도는 0보다 커야 합니다.",
        ));
    }
    let rho_ref = 1000.0;
    let kv = volumetric_flow_m3_per_h * (rho_ref / (fluid_density_kg_m3 * delta_p_bar)).sqrt();
    Ok(kv)
}

/// 요구 Cv 값을 계산한다.
pub fn required_cv(
    volumetric_flow_m3_per_h: f64,
    delta_p_bar: f64,
    fluid_density_kg_m3: f64,
) -> Result<f64, ValveCalcError> {
    let kv = required_kv(volumetric_flow_m3_per_h, delta_p_bar, fluid_density_kg_m3)?;
    Ok(cv_from_kv(kv))
}

/// 주어진 Kv로 통과 가능한 유량을 계산한다. 비압축성 근사.
pub fn flow_from_kv(
    kv: f64,
    delta_p_bar: f64,
    fluid_density_kg_m3: f64,
    upstream_bar_abs: Option<f64>,
) -> Result<f64, ValveCalcError> {
    if kv <= 0.0 || delta_p_bar <= 0.0 || fluid_density_kg_m3 <= 0.0 {
        return Err(ValveCalcError::InvalidInput(
            "Kv, 차압, 밀도는 0보다 커야 합니다.",
        ));
    }
    // 간략한 임계 유동 판정: 증기 가정 시 임계비 약 0.55 (gamma ~1.3)
    if let Some(p_up_abs) = upstream_bar_abs {
        let p_down_abs = (p_up_abs - delta_p_bar).max(0.0);
        if p_up_abs > 0.0 && p_down_abs / p_up_abs < 0.55 {
            return Err(ValveCalcError::ChokedFlow(
                "임계(음속) 영역 가능성이 높아 단순 Kv 식이 부정확할 수 있습니다.",
            ));
        }
    }
    let rho_ref = 1000.0;
    let flow = kv * (delta_p_bar * fluid_density_kg_m3 / rho_ref).sqrt();
    Ok(flow)
}

/// Cv 값을 기반으로 SI 기준 유량(m³/h)을 계산한다.
pub fn flow_from_cv(
    cv: f64,
    delta_p_bar: f64,
    fluid_density_kg_m3: f64,
) -> Result<f64, ValveCalcError> {
    flow_from_kv(kv_from_cv(cv), delta_p_bar, fluid_density_kg_m3, None)
}

/// Kv와 밀도를 사용해 질량유량(kg/h)을 반환한다.
pub fn mass_flow_from_kv(
    kv: f64,
    delta_p_bar: f64,
    fluid_density_kg_m3: f64,
) -> Result<f64, ValveCalcError> {
    let q_m3_h = flow_from_kv(kv, delta_p_bar, fluid_density_kg_m3, None)?;
    Ok(q_m3_h * fluid_density_kg_m3)
}
