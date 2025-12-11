const G: f64 = 9.80665;

/// 펌프 NPSH 계산 입력.
#[derive(Debug, Clone)]
pub struct PumpNpshInput {
    /// 흡입 측 압력(bar, 게이지 또는 절대)
    pub suction_pressure_bar: f64,
    /// 압력 모드: true=절대, false=게이지
    pub suction_is_abs: bool,
    /// 유체 온도(°C) - 물로 가정하여 증기압을 계산
    pub liquid_temp_c: f64,
    /// 정수두(m) - 액면에서 펌프 중심까지의 높이(+상승, -낙차)
    pub static_head_m: f64,
    /// 흡입 배관 마찰손실(m 수두)
    pub friction_loss_m: f64,
    /// 요구 NPSH (제조사 값, m)
    pub npshr_m: f64,
    /// 유체 밀도(kg/m³)
    pub rho_kg_m3: f64,
}

/// 펌프 NPSH 계산 결과.
#[derive(Debug, Clone)]
pub struct PumpNpshResult {
    /// 사용 가능 NPSH(m)
    pub npsha_m: f64,
    /// Margin = NPSHa / NPSHr
    pub margin_ratio: f64,
    /// 경고/주의 메시지
    pub warnings: Vec<String>,
}

/// Antoine 식으로 대략적인 물의 포화증기압(bar abs)을 구한다.
fn water_vapor_pressure_bar_abs(t_c: f64) -> f64 {
    // 유효 범위: 1~100°C
    let t = t_c.clamp(1.0, 100.0);
    let log10_p_mmhg = 8.07131 - 1730.63 / (233.426 + t);
    let p_mmhg = 10_f64.powf(log10_p_mmhg);
    // 760 mmHg = 1.01325 bar(abs)
    p_mmhg / 750.062
}

/// 펌프 NPSH를 계산한다.
pub fn compute_pump_npsh(input: PumpNpshInput) -> PumpNpshResult {
    let p_suction_abs_bar = if input.suction_is_abs {
        input.suction_pressure_bar
    } else {
        input.suction_pressure_bar + 1.01325
    };
    let pv_bar = water_vapor_pressure_bar_abs(input.liquid_temp_c);

    // 압력차를 수두(m)로 환산: (ΔP [Pa]) / (ρ*g)
    let delta_p_pa = (p_suction_abs_bar - pv_bar) * 100_000.0;
    let head_from_pressure = delta_p_pa / (input.rho_kg_m3 * G);
    let npsha = head_from_pressure + input.static_head_m - input.friction_loss_m;

    let margin = if input.npshr_m > 0.0 {
        npsha / input.npshr_m
    } else {
        f64::INFINITY
    };
    let mut warnings = Vec::new();
    if margin < 1.1 {
        warnings.push(format!(
            "NPSH Margin {:.2} (<1.1). 공동현상 위험.",
            margin
        ));
    }
    PumpNpshResult {
        npsha_m: npsha,
        margin_ratio: margin,
        warnings,
    }
}
