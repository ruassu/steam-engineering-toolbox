use crate::units::{convert_pressure, convert_temperature, PressureUnit, TemperatureUnit};

/// 배관 계산 오류를 표현한다.
#[derive(Debug)]
pub enum PipeCalcError {
    /// 입력값이 잘못된 경우
    InvalidInput(&'static str),
}

impl std::fmt::Display for PipeCalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipeCalcError::InvalidInput(msg) => write!(f, "입력 오류: {msg}"),
        }
    }
}

impl std::error::Error for PipeCalcError {}

/// 속도 기준 사이징 입력값.
#[derive(Debug, Clone)]
pub struct PipeSizingByVelocityInput {
    pub mass_flow_kg_per_h: f64,
    pub steam_density_kg_per_m3: f64,
    pub target_velocity_m_per_s: f64,
}

/// 속도 기준 사이징 결과.
#[derive(Debug, Clone)]
pub struct PipeSizingByVelocityResult {
    pub inner_diameter_m: f64,
    pub velocity_m_per_s: f64,
    pub reynolds_number: f64,
}

/// Darcy-Weisbach 기반 압력손실 입력값.
#[derive(Debug, Clone)]
pub struct PressureLossInput {
    pub mass_flow_kg_per_h: f64,
    pub steam_density_kg_per_m3: f64,
    pub diameter_m: f64,
    pub length_m: f64,
    /// 피팅 계수 총합 (K 값 합)
    pub fittings_k_sum: f64,
    /// 별도로 주어진 등가 길이 [m] (직접 입력)
    pub equivalent_length_m: f64,
    pub roughness_m: f64,
    pub dynamic_viscosity_pa_s: f64,
    /// 음속 [m/s] (증기 400~500 m/s 정도). Mach 계산용.
    pub sound_speed_m_per_s: f64,
    /// 압력손실 계산용 상태 압력(절대 bar). 제공 시 IF97로 밀도/점도 계산에 사용.
    pub state_pressure_bar_abs: Option<f64>,
    /// 압력손실 계산용 상태 온도(°C). 제공 시 IF97로 밀도/점도 계산에 사용.
    pub state_temperature_c: Option<f64>,
}

/// 압력손실 계산 결과.
#[derive(Debug, Clone)]
pub struct PressureLossResult {
    pub velocity_m_per_s: f64,
    pub pressure_drop_bar: f64,
    pub reynolds_number: f64,
    pub friction_factor: f64,
    pub mach: f64,
}

/// 목표 유속을 만족하는 배관 내경을 계산한다.
pub fn size_by_velocity(
    input: PipeSizingByVelocityInput,
) -> Result<PipeSizingByVelocityResult, PipeCalcError> {
    if input.mass_flow_kg_per_h <= 0.0 {
        return Err(PipeCalcError::InvalidInput(
            "질량 유량은 0보다 커야 합니다.",
        ));
    }
    if input.steam_density_kg_per_m3 <= 0.0 || input.target_velocity_m_per_s <= 0.0 {
        return Err(PipeCalcError::InvalidInput(
            "밀도와 목표 유속은 0보다 커야 합니다.",
        ));
    }

    let mass_flow_kg_s = input.mass_flow_kg_per_h / 3600.0;
    let volumetric_flow_m3_s = mass_flow_kg_s / input.steam_density_kg_per_m3;
    let area = volumetric_flow_m3_s / input.target_velocity_m_per_s;
    let diameter = (4.0 * area / std::f64::consts::PI).sqrt();

    // 유속 재계산 및 레이놀즈수 추정
    let velocity = volumetric_flow_m3_s / (std::f64::consts::PI * diameter * diameter / 4.0);
    let dyn_visc = 1.2e-5; // 대략적인 증기 점도 [Pa·s], 향후 실제 값으로 치환
    let reynolds = input.steam_density_kg_per_m3 * velocity * diameter / dyn_visc;

    Ok(PipeSizingByVelocityResult {
        inner_diameter_m: diameter,
        velocity_m_per_s: velocity,
        reynolds_number: reynolds,
    })
}

/// Darcy-Weisbach 식을 사용해 압력손실을 추정한다.
///
/// ΔP = f * (L/D) * ρ * v² / 2
/// - f: 마찰계수(여기서는 단순 블라지우스/문수근사 혼합으로 계산)
pub fn pressure_loss(input: PressureLossInput) -> Result<PressureLossResult, PipeCalcError> {
    if input.mass_flow_kg_per_h <= 0.0 || input.diameter_m <= 0.0 || input.length_m <= 0.0 {
        return Err(PipeCalcError::InvalidInput(
            "질량유량, 직경, 길이는 0보다 커야 합니다.",
        ));
    }
    let (steam_density_kg_per_m3, dyn_visc) = resolve_steam_props(&input);
    let mass_flow_kg_s = input.mass_flow_kg_per_h / 3600.0;
    let area = std::f64::consts::PI * input.diameter_m * input.diameter_m / 4.0;
    let velocity = mass_flow_kg_s / (steam_density_kg_per_m3 * area);

    let reynolds = steam_density_kg_per_m3 * velocity * input.diameter_m / dyn_visc;

    // 단순 마찰계수 근사: 층류/난류 모두를 감안한 Petukhov 근사
    let friction_factor = if reynolds < 2300.0 {
        64.0 / reynolds
    } else {
        let roughness_ratio = input.roughness_m / input.diameter_m;
        let log_term = (roughness_ratio / 3.7).powf(1.11) + 6.9 / reynolds;
        let inv_sqrt_f = -1.8 * log_term.log10();
        1.0 / inv_sqrt_f.powi(2)
    };

    // 등가 길이: 직접 입력 + K값을 등가 길이로 환산
    let eq_len_from_k = if friction_factor > 0.0 {
        input.fittings_k_sum * input.diameter_m / friction_factor
    } else {
        0.0
    };
    let total_length = input.length_m + input.equivalent_length_m + eq_len_from_k;

    let delta_p_pa = friction_factor
        * (total_length / input.diameter_m)
        * steam_density_kg_per_m3
        * velocity
        * velocity
        / 2.0;
    let delta_p_bar = delta_p_pa / 100_000.0;
    let mach = if input.sound_speed_m_per_s > 0.0 {
        velocity / input.sound_speed_m_per_s
    } else {
        0.0
    };

    Ok(PressureLossResult {
        velocity_m_per_s: velocity,
        pressure_drop_bar: delta_p_bar,
        reynolds_number: reynolds,
        friction_factor,
        mach,
    })
}

fn resolve_steam_props(input: &PressureLossInput) -> (f64, f64) {
    if let (Some(p_bar_abs), Some(t_c)) = (input.state_pressure_bar_abs, input.state_temperature_c)
    {
        if let Ok((_, v, _)) = crate::steam::if97::region_props(p_bar_abs, t_c) {
            if v.is_finite() && v > 0.0 {
                let density = 1.0 / v;
                let mu = steam_dynamic_viscosity_pa_s(t_c, density);
                return (density, mu);
            }
        }
    }
    (input.steam_density_kg_per_m3, input.dynamic_viscosity_pa_s)
}

fn steam_dynamic_viscosity_pa_s(temp_c: f64, density: f64) -> f64 {
    // 증기/과열 영역은 서덜랜드 근사, 액체 영역은 일반적인 물 점도 근사 사용
    let temp_k = temp_c + 273.15;
    if density > 50.0 {
        liquid_water_viscosity(temp_c)
    } else {
        steam_vapor_viscosity(temp_k)
    }
}

fn steam_vapor_viscosity(temp_k: f64) -> f64 {
    // 간단한 서덜랜드 근사 (기본값: 300K에서 약 1.3e-5 Pa·s)
    let t0 = 300.0;
    let mu0 = 1.3e-5;
    let s = 111.0;
    mu0 * (temp_k / t0).powf(1.5) * (t0 + s) / (temp_k + s)
}

fn liquid_water_viscosity(temp_c: f64) -> f64 {
    // 0~370°C 범위에서 흔히 쓰이는 물 점도 근사식 [Pa·s]
    let exponent = 247.8 / (temp_c + 133.15);
    2.414e-5 * 10f64.powf(exponent)
}

/// 이상기체 근사로 증기 밀도를 계산한다. (압력 입력은 게이지 bar로 가정)
pub fn estimate_density(
    pressure_value: f64,
    pressure_unit: PressureUnit,
    temperature_value: f64,
    temperature_unit: TemperatureUnit,
) -> f64 {
    let p_bar = convert_pressure(pressure_value, pressure_unit, PressureUnit::Bar);
    let t_k = convert_temperature(temperature_value, temperature_unit, TemperatureUnit::Kelvin);
    let p_pa = p_bar * 100_000.0;
    let r_specific = 461.5; // 증기 기체상수 [J/(kg·K)]
    p_pa / (r_specific * t_k)
}
