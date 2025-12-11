use crate::conversion::PressureMode;
use crate::steam;
use crate::units::PressureUnit;

/// 콘덴서(복수기) 열수지를 계산하기 위한 입력 값.
#[derive(Debug, Clone)]
pub struct CondenserInput {
    /// 증기 압력 값
    pub steam_pressure: f64,
    /// 증기 압력 단위
    pub steam_pressure_unit: PressureUnit,
    /// 게이지/절대 모드
    pub steam_pressure_mode: PressureMode,
    /// 증기 온도(°C). `None`이면 포화온도를 압력으로부터 계산한다.
    pub steam_temp_c: Option<f64>,
    /// 냉각수 유입 온도(°C)
    pub cw_inlet_temp_c: f64,
    /// 냉각수 유출 온도(°C)
    pub cw_outlet_temp_c: f64,
    /// 냉각수 유량(m³/h, 체적기준)
    pub cw_flow_m3_per_h: f64,
    /// UA 값(kW/K). `Some`일 경우 LMTD*UA를 Q 추정에 사용하고, 없으면 냉각수 열수지로 Q를 구한다.
    pub ua_kw_per_k: Option<f64>,
    /// 열교환기 전열면적(m²) - U*Area로 UA를 구성할 때 사용
    pub area_m2: Option<f64>,
    /// 종합전열계수 U(W/m²·K) - 면적과 곱해 UA를 계산
    pub overall_u_w_m2k: Option<f64>,
    /// 목표 배압(절대, bar). 설정 시 목표 대비 경고를 표시한다.
    pub target_back_pressure_bar_abs: Option<f64>,
}

/// 콘덴서 계산 결과.
#[derive(Debug, Clone)]
pub struct CondenserResult {
    /// 포화 기준 증기 온도(°C)
    pub condensing_temp_c: f64,
    /// 포화 기준 증기 압력(bar abs)
    pub condensing_pressure_bar_abs: f64,
    /// 로그 평균 온도차(°C 또는 K)
    pub lmtd_k: f64,
    /// 열량(kW)
    pub heat_duty_kw: f64,
    /// 경고/주의 메시지
    pub warnings: Vec<String>,
}

/// 콘덴서 계산 중 발생 가능한 오류.
#[derive(Debug, Clone)]
pub enum CoolingError {
    /// 온도차가 0 이하라 LMTD를 계산할 수 없음
    NegativeDeltaT,
    /// IF97 포화 계산 실패
    If97(String),
}

fn log_mean(delta1: f64, delta2: f64) -> Option<f64> {
    if delta1 <= 0.0 || delta2 <= 0.0 {
        return None;
    }
    if (delta1 - delta2).abs() < 1e-9 {
        return Some(delta1);
    }
    let lm = (delta1 / delta2).ln();
    Some((delta1 - delta2) / lm)
}

/// 콘덴서 열수지와 진공 수준을 계산한다.
pub fn compute_condenser(input: CondenserInput) -> Result<CondenserResult, CoolingError> {
    // 압력을 bar(abs)로 변환
    let p_bar_abs =
        crate::conversion::convert_pressure_mode(
            input.steam_pressure,
            input.steam_pressure_unit,
            input.steam_pressure_mode,
            PressureUnit::Bar,
            PressureMode::Absolute,
        );

    // 포화 온도/압력 계산
    let (tsat_c, psat_bar_abs) = if let Some(t) = input.steam_temp_c {
        let psat = steam::if97::saturation_pressure_bar_abs_from_temp_c(t)
            .map_err(|e| CoolingError::If97(e.to_string()))?;
        (t, psat)
    } else {
        let tsat = steam::if97::saturation_temp_c_from_pressure_bar_abs(p_bar_abs)
            .map_err(|e| CoolingError::If97(e.to_string()))?;
        (tsat, p_bar_abs)
    };

    // LMTD 계산
    let d1 = tsat_c - input.cw_outlet_temp_c;
    let d2 = tsat_c - input.cw_inlet_temp_c;
    let lmtd = log_mean(d1, d2).ok_or(CoolingError::NegativeDeltaT)?;

    // 냉각수 질량유량(kg/s) 가정: 물, 밀도 1000 kg/m3
    let m_cw = input.cw_flow_m3_per_h * (1000.0 / 3600.0);
    let cp = 4.186; // kJ/kgK
    let q_kw_from_water = m_cw * cp * (input.cw_outlet_temp_c - input.cw_inlet_temp_c);

    // UA로부터의 Q 추정 (선택)
    let ua_kw_per_k = input
        .ua_kw_per_k
        .or_else(|| input.area_m2.zip(input.overall_u_w_m2k).map(|(a, u)| a * u / 1000.0));
    let q_kw = if let Some(ua) = ua_kw_per_k {
        ua * lmtd
    } else {
        q_kw_from_water
    };

    let mut warnings = Vec::new();
    if d1 <= 0.0 || d2 <= 0.0 {
        warnings.push("냉각수 출구/입구 온도가 포화온도 이상입니다. 역류 또는 센서 오류 가능".into());
    }
    if let Some(target) = input.target_back_pressure_bar_abs {
        if psat_bar_abs > target {
            warnings.push(format!(
                "배압 {:.3} bar(abs)가 목표 {:.3} bar(abs)보다 높습니다.",
                psat_bar_abs, target
            ));
        }
    }
    if (q_kw - q_kw_from_water).abs() > 0.05 * q_kw_from_water && input.ua_kw_per_k.is_some() {
        warnings.push("UA 기반 열량과 냉각수 열수지 열량이 크게 다릅니다.".into());
    }

    Ok(CondenserResult {
        condensing_temp_c: tsat_c,
        condensing_pressure_bar_abs: psat_bar_abs,
        lmtd_k: lmtd,
        heat_duty_kw: q_kw,
        warnings,
    })
}
