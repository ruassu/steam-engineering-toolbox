use super::if97;
use crate::conversion::PressureMode;
use crate::units::{convert_temperature, PressureUnit, TemperatureUnit};

/// 단순 선형 보간 기반 포화/과열 증기 특성을 제공한다.
#[derive(Debug, Clone)]
pub struct SteamState {
    /// 입력 기준 압력(bar)
    pub pressure_bar: f64,
    /// 입력 기준 온도(°C)
    pub temperature_c: f64,
    /// 포화 증기 온도(°C)
    pub saturation_temperature_c: f64,
    /// 포화 증기 비엔탈피(kJ/kg)
    pub saturation_enthalpy_kj_per_kg: f64,
    /// 포화 증기 비체적(m³/kg)
    pub saturation_specific_volume: f64,
    /// 포화 증기 엔트로피(kJ/kg·K)
    pub saturation_entropy_kj_per_kgk: f64,
    /// 포화수(액) 비엔탈피(kJ/kg)
    pub sat_liquid_enthalpy_kj_per_kg: f64,
    /// 포화수(액) 비체적(m³/kg)
    pub sat_liquid_specific_volume: f64,
    /// 포화수(액) 엔트로피(kJ/kg·K)
    pub sat_liquid_entropy_kj_per_kgk: f64,
    /// 과열 시 계산된 비엔탈피(kJ/kg)
    pub superheated_enthalpy_kj_per_kg: Option<f64>,
}

/// 증기표 계산 시 발생 가능한 오류.
#[derive(Debug)]
pub enum SteamTableError {
    /// 입력 범위를 벗어남
    OutOfRange(&'static str),
    /// 포화 경계에 너무 근접
    NearSaturation(&'static str),
}

impl std::fmt::Display for SteamTableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SteamTableError::OutOfRange(msg) => write!(f, "범위를 벗어남: {msg}"),
            SteamTableError::NearSaturation(msg) => write!(f, "포화 경계 근접: {msg}"),
        }
    }
}

impl std::error::Error for SteamTableError {}

#[derive(Debug, Clone, Copy)]
struct SteamTableRow {
    pressure_bar: f64,
    temperature_c: f64,
    enthalpy_kj_per_kg: f64,
    specific_volume: f64,
}

// 간단한 표. 정확도보다는 안정성을 우선하며, 향후 IAPWS-IF97 구현으로 교체 가능하게 설계.
// 진공 영역(0.01 bar)까지 포함한다.
const SAT_TABLE: [SteamTableRow; 14] = [
    SteamTableRow {
        pressure_bar: 0.01, // 약 1 kPa
        temperature_c: 6.9,
        enthalpy_kj_per_kg: 2500.0,
        specific_volume: 129.0,
    },
    SteamTableRow {
        pressure_bar: 0.02,
        temperature_c: 17.5,
        enthalpy_kj_per_kg: 2535.0,
        specific_volume: 73.6,
    },
    SteamTableRow {
        pressure_bar: 0.05,
        temperature_c: 32.9,
        enthalpy_kj_per_kg: 2565.0,
        specific_volume: 29.3,
    },
    SteamTableRow {
        pressure_bar: 0.1,
        temperature_c: 45.8,
        enthalpy_kj_per_kg: 2584.0,
        specific_volume: 14.7,
    },
    SteamTableRow {
        pressure_bar: 0.2,
        temperature_c: 60.1,
        enthalpy_kj_per_kg: 2600.0,
        specific_volume: 7.65,
    },
    SteamTableRow {
        pressure_bar: 0.3,
        temperature_c: 69.1,
        enthalpy_kj_per_kg: 2611.0,
        specific_volume: 5.08,
    },
    SteamTableRow {
        pressure_bar: 0.5,
        temperature_c: 81.3,
        enthalpy_kj_per_kg: 2645.0,
        specific_volume: 3.24,
    },
    SteamTableRow {
        pressure_bar: 1.0,
        temperature_c: 100.0,
        enthalpy_kj_per_kg: 2676.0,
        specific_volume: 1.694,
    },
    SteamTableRow {
        pressure_bar: 2.0,
        temperature_c: 120.2,
        enthalpy_kj_per_kg: 2706.0,
        specific_volume: 0.885,
    },
    SteamTableRow {
        pressure_bar: 3.0,
        temperature_c: 133.5,
        enthalpy_kj_per_kg: 2725.0,
        specific_volume: 0.595,
    },
    SteamTableRow {
        pressure_bar: 5.0,
        temperature_c: 151.8,
        enthalpy_kj_per_kg: 2749.0,
        specific_volume: 0.375,
    },
    SteamTableRow {
        pressure_bar: 8.0,
        temperature_c: 170.4,
        enthalpy_kj_per_kg: 2771.0,
        specific_volume: 0.248,
    },
    SteamTableRow {
        pressure_bar: 10.0,
        temperature_c: 179.9,
        enthalpy_kj_per_kg: 2783.0,
        specific_volume: 0.201,
    },
    SteamTableRow {
        pressure_bar: 15.0,
        temperature_c: 198.3,
        enthalpy_kj_per_kg: 2803.0,
        specific_volume: 0.132,
    },
];

/// 압력 기준 포화 증기 특성을 선형 보간 또는 IF97 Region4 근사로 계산한다.
pub fn saturation_by_pressure(
    value: f64,
    unit: PressureUnit,
) -> Result<SteamState, SteamTableError> {
    saturation_by_pressure_mode(value, unit, PressureMode::Gauge)
}

/// 압력과 모드(게이지/절대)를 받아 포화 증기 특성을 계산한다.
pub fn saturation_by_pressure_mode(
    value: f64,
    unit: PressureUnit,
    mode: PressureMode,
) -> Result<SteamState, SteamTableError> {
    let pressure_bar_abs = to_bar_absolute_mode(value, unit, mode);
    if pressure_bar_abs < 0.0007 || pressure_bar_abs > 220.0 {
        return Err(SteamTableError::OutOfRange(
            "압력 범위 밖입니다. 0.0007~220 bar(abs)에서 계산 가능합니다.",
        ));
    }
    // 0.01~15 bar는 표 보간, 그 외는 IF97 Region4 사용
    let (temperature_c, h_vap, v_vap, s_vap, h_liq, v_liq, s_liq) = if pressure_bar_abs
        <= SAT_TABLE.last().unwrap().pressure_bar
    {
        let (low, high) = bracket_by_pressure(pressure_bar_abs)?;
        let ratio = (pressure_bar_abs - low.pressure_bar) / (high.pressure_bar - low.pressure_bar);
        let t = low.temperature_c + ratio * (high.temperature_c - low.temperature_c);
        // 포화수/증기 엔탈피/체적/엔트로피를 Region1/2로 계산
        let (h_l, v_l, s_l) = if97::region1_props(pressure_bar_abs, t).unwrap_or((
            low.enthalpy_kj_per_kg - 200.0,
            0.001,
            1.0,
        ));
        let (h_v, v_v, s_v) = if97::region2_props(pressure_bar_abs, t).unwrap_or((
            low.enthalpy_kj_per_kg,
            low.specific_volume,
            7.0,
        ));
        (t, h_v, v_v, s_v, h_l, v_l, s_l)
    } else {
        let t = if97::saturation_temp_c_from_pressure_bar_abs(pressure_bar_abs)
            .map_err(|_| SteamTableError::OutOfRange("IF97 역계산 실패"))?;
        // 포화수/증기 계산
        let (h_l, v_l, s_l) =
            if97::region1_props(pressure_bar_abs, t).unwrap_or((500.0, 0.001, 1.0));
        let (h_v, v_v, s_v) =
            if97::region2_props(pressure_bar_abs, t).unwrap_or((2500.0, 0.1, 7.0));
        (t, h_v, v_v, s_v, h_l, v_l, s_l)
    };

    Ok(SteamState {
        pressure_bar: pressure_bar_abs,
        temperature_c,
        saturation_temperature_c: temperature_c,
        saturation_enthalpy_kj_per_kg: h_vap,
        saturation_specific_volume: v_vap,
        saturation_entropy_kj_per_kgk: s_vap,
        sat_liquid_enthalpy_kj_per_kg: h_liq,
        sat_liquid_specific_volume: v_liq,
        sat_liquid_entropy_kj_per_kgk: s_liq,
        superheated_enthalpy_kj_per_kg: None,
    })
}

/// 온도 기준 포화 증기 특성을 선형 보간 또는 근사식으로 계산한다.
pub fn saturation_by_temperature(
    value: f64,
    unit: TemperatureUnit,
) -> Result<SteamState, SteamTableError> {
    let temperature_c = convert_temperature(value, unit, TemperatureUnit::Celsius);
    if temperature_c < 0.0 || temperature_c > 360.0 {
        return Err(SteamTableError::OutOfRange(
            "온도 범위 밖입니다. 0~360°C에서 계산 가능합니다.",
        ));
    }
    // 표 범위 내: 보간, 그 외: 근사
    let (pressure_bar, h_vap, v_vap, s_vap, h_liq, v_liq, s_liq) = if temperature_c
        >= SAT_TABLE.first().unwrap().temperature_c
        && temperature_c <= SAT_TABLE.last().unwrap().temperature_c
    {
        let (low, high) = bracket_by_temperature(temperature_c)?;
        let ratio = (temperature_c - low.temperature_c) / (high.temperature_c - low.temperature_c);
        let p = low.pressure_bar + ratio * (high.pressure_bar - low.pressure_bar);
        let (h_l, v_l, s_l) = if97::region1_props(p, temperature_c).unwrap_or((500.0, 0.001, 1.0));
        let (h_v, v_v, s_v) = if97::region2_props(p, temperature_c).unwrap_or((2500.0, 0.1, 7.0));
        (p, h_v, v_v, s_v, h_l, v_l, s_l)
    } else {
        let p = if97::saturation_pressure_bar_abs_from_temp_c(temperature_c)
            .map_err(|_| SteamTableError::OutOfRange("IF97 계산 실패"))?;
        let (h_l, v_l, s_l) = if97::region1_props(p, temperature_c).unwrap_or((500.0, 0.001, 1.0));
        let (h_v, v_v, s_v) = if97::region2_props(p, temperature_c).unwrap_or((2500.0, 0.1, 7.0));
        (p, h_v, v_v, s_v, h_l, v_l, s_l)
    };

    Ok(SteamState {
        pressure_bar,
        temperature_c,
        saturation_temperature_c: temperature_c,
        saturation_enthalpy_kj_per_kg: h_vap,
        saturation_specific_volume: v_vap,
        saturation_entropy_kj_per_kgk: s_vap,
        sat_liquid_enthalpy_kj_per_kg: h_liq,
        sat_liquid_specific_volume: v_liq,
        sat_liquid_entropy_kj_per_kgk: s_liq,
        superheated_enthalpy_kj_per_kg: None,
    })
}

/// 과열 증기 특성을 간단한 정압 비열(cp ≈ 2.08 kJ/kgK) 근사로 계산한다.
/// sat_temp보다 높은 온도에서만 사용한다.
pub fn superheated_at(
    pressure_value: f64,
    pressure_unit: PressureUnit,
    temperature_value: f64,
    temperature_unit: TemperatureUnit,
) -> Result<SteamState, SteamTableError> {
    superheated_at_mode(
        pressure_value,
        pressure_unit,
        PressureMode::Gauge,
        temperature_value,
        temperature_unit,
    )
}

/// 게이지/절대 압력 모드를 지정해 과열 증기를 계산한다.
pub fn superheated_at_mode(
    pressure_value: f64,
    pressure_unit: PressureUnit,
    pressure_mode: PressureMode,
    temperature_value: f64,
    temperature_unit: TemperatureUnit,
) -> Result<SteamState, SteamTableError> {
    let mut state = saturation_by_pressure_mode(pressure_value, pressure_unit, pressure_mode)?;
    let target_c = convert_temperature(
        temperature_value,
        temperature_unit,
        TemperatureUnit::Celsius,
    );
    if target_c <= state.saturation_temperature_c {
        return Err(SteamTableError::OutOfRange(
            "과열 계산은 포화 온도보다 높은 경우에만 유효",
        ));
    }
    if (target_c - state.saturation_temperature_c).abs() < 3.0 {
        return Err(SteamTableError::NearSaturation(
            "포화 온도에 매우 근접한 과열 영역입니다. 결과에 유의하세요.",
        ));
    }
    let cp = 2.08; // kJ/kgK
    let delta_t = target_c - state.saturation_temperature_c;
    let h_super = state.saturation_enthalpy_kj_per_kg + cp * delta_t;
    state.temperature_c = target_c;
    state.superheated_enthalpy_kj_per_kg = Some(h_super);
    Ok(state)
}

fn bracket_by_pressure(p_bar: f64) -> Result<(SteamTableRow, SteamTableRow), SteamTableError> {
    if p_bar < SAT_TABLE.first().unwrap().pressure_bar
        || p_bar > SAT_TABLE.last().unwrap().pressure_bar
    {
        return Err(SteamTableError::OutOfRange(
            "표 범위 밖의 압력입니다. 0.01~15 bar(abs) 보간, 그 이상은 근사식을 사용합니다.",
        ));
    }
    for pair in SAT_TABLE.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if p_bar >= a.pressure_bar && p_bar <= b.pressure_bar {
            return Ok((a, b));
        }
    }
    Err(SteamTableError::OutOfRange("보간 실패"))
}

fn to_bar_absolute_mode(value: f64, unit: PressureUnit, mode: PressureMode) -> f64 {
    const ATM_BAR: f64 = 1.01325;
    let base = match unit {
        PressureUnit::Bar | PressureUnit::BarA => value,
        PressureUnit::MilliBar => value / 1000.0,
        PressureUnit::Pascal => value / 100_000.0,
        PressureUnit::KiloPascal => value / 100.0,
        PressureUnit::MegaPascal => value * 10.0,
        PressureUnit::KgPerCm2 => value * 0.980665,
        PressureUnit::Psi => value * 0.0689476,
        PressureUnit::Atm => value * ATM_BAR,
        // mmHg는 0=대기, -760mmHg=진공인 게이지 척도로 처리한다.
        PressureUnit::MmHg => value / 750.062,
    };
    match mode {
        PressureMode::Gauge => base + ATM_BAR,
        PressureMode::Absolute => base,
    }
}

fn bracket_by_temperature(t_c: f64) -> Result<(SteamTableRow, SteamTableRow), SteamTableError> {
    if t_c < SAT_TABLE.first().unwrap().temperature_c
        || t_c > SAT_TABLE.last().unwrap().temperature_c
    {
        return Err(SteamTableError::OutOfRange(
            "표 범위 밖의 온도입니다. 약 7~200°C 사이로 입력하세요.",
        ));
    }
    for pair in SAT_TABLE.windows(2) {
        let a = pair[0];
        let b = pair[1];
        if t_c >= a.temperature_c && t_c <= b.temperature_c {
            return Ok((a, b));
        }
    }
    Err(SteamTableError::OutOfRange("보간 실패"))
}
