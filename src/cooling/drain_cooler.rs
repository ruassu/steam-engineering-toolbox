/// 드레인/재열기 등 2유체 열교환기 열수지 입력.
#[derive(Debug, Clone)]
pub struct DrainCoolerInput {
    /// 쉘측 입구/출구 온도(°C)
    pub shell_in_c: f64,
    pub shell_out_c: f64,
    /// 쉘측 유량(m³/h, 물 가정)
    pub shell_flow_m3_per_h: f64,
    /// 튜브측 입구/출구 온도(°C)
    pub tube_in_c: f64,
    pub tube_out_c: f64,
    /// 튜브측 유량(m³/h, 물 가정)
    pub tube_flow_m3_per_h: f64,
    /// UA(kW/K) 또는
    pub ua_kw_per_k: Option<f64>,
    /// 면적+U로 UA를 구성
    pub area_m2: Option<f64>,
    pub overall_u_w_m2k: Option<f64>,
}

/// 드레인/재열기 열수지 결과.
#[derive(Debug, Clone)]
pub struct DrainCoolerResult {
    /// 로그 평균 온도차
    pub lmtd_k: f64,
    /// 쉘측 열량(kW)
    pub shell_heat_kw: f64,
    /// 튜브측 열량(kW)
    pub tube_heat_kw: f64,
    /// 불균형(kW)
    pub imbalance_kw: f64,
    /// 경고/주의
    pub warnings: Vec<String>,
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

/// 2유체 열수지를 계산한다. cp는 물(4.186 kJ/kgK)로 단순 가정한다.
pub fn compute_drain_cooler(input: DrainCoolerInput) -> DrainCoolerResult {
    let cp = 4.186;
    let rho = 1000.0;
    let shell_m = input.shell_flow_m3_per_h * rho / 3600.0;
    let tube_m = input.tube_flow_m3_per_h * rho / 3600.0;
    let shell_heat_kw = shell_m * cp * (input.shell_out_c - input.shell_in_c);
    let tube_heat_kw = tube_m * cp * (input.tube_out_c - input.tube_in_c);

    let dt1 = (input.shell_in_c - input.tube_out_c).abs();
    let dt2 = (input.shell_out_c - input.tube_in_c).abs();
    let lmtd = log_mean(dt1, dt2).unwrap_or(0.0);

    let mut warnings = Vec::new();
    if lmtd <= 0.0 {
        warnings.push("LMTD가 0 이하입니다. 온도 교차가 잘못되었을 수 있습니다.".into());
    }
    let imbalance = (shell_heat_kw - tube_heat_kw).abs();
    if imbalance > shell_heat_kw.abs().max(tube_heat_kw.abs()) * 0.05 {
        warnings.push("쉘/튜브 열수지 불균형이 5%를 초과합니다.".into());
    }

    // TODO: UA 기반 설계 검증, 핀 효율 등 상세 열전달 모델 추가
    DrainCoolerResult {
        lmtd_k: lmtd,
        shell_heat_kw,
        tube_heat_kw,
        imbalance_kw: imbalance,
        warnings,
    }
}
