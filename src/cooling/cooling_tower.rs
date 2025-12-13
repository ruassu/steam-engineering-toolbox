/// 냉각탑(접촉식) 범위/접근 계산을 위한 입력 값.
#[derive(Debug, Clone)]
pub struct CoolingTowerInput {
    /// 순환수 입구 온도(°C)
    pub water_in_c: f64,
    /// 순환수 출구 온도(°C)
    pub water_out_c: f64,
    /// 대기 건구 온도(°C)
    pub dry_bulb_c: f64,
    /// 대기 습구 온도(°C)
    pub wet_bulb_c: f64,
    /// 순환수 유량(m³/h)
    pub water_flow_m3_per_h: f64,
    /// 목표 범위(°C) - 선택
    pub target_range_c: Option<f64>,
    /// 목표 접근(°C) - 선택
    pub target_approach_c: Option<f64>,
}

/// 냉각탑 범위/접근 계산 결과.
#[derive(Debug, Clone)]
pub struct CoolingTowerResult {
    /// Range = Tin - Tout
    pub range_c: f64,
    /// Approach = Tout - WB
    pub approach_c: f64,
    /// 냉각수 열량(kW) - 단순 cp*ΔT 계산
    pub heat_rejected_kw: f64,
    /// 경고/주의 메시지
    pub warnings: Vec<String>,
}

/// 냉각탑 범위/접근을 계산한다. 복잡한 L/G 추정은 TODO로 남겨둔다.
pub fn compute_cooling_tower(input: CoolingTowerInput) -> CoolingTowerResult {
    let range_c = input.water_in_c - input.water_out_c;
    let approach_c = input.water_out_c - input.wet_bulb_c;

    // 단순 열량 계산 (cp=4.186 kJ/kgK, ρ=1000)
    let m = input.water_flow_m3_per_h * (1000.0 / 3600.0);
    let heat_kw = m * 4.186 * range_c;

    let mut warnings = Vec::new();
    if approach_c < 0.0 {
        warnings
            .push("접근(Approach)이 음수입니다. 습구 온도보다 낮은 냉각은 불가능합니다.".into());
    } else if approach_c < 2.0 {
        warnings.push("접근이 2°C 미만입니다. 실제 운전에서 달성하기 어려울 수 있습니다.".into());
    }
    if let Some(t) = input.target_range_c {
        if range_c < t {
            warnings.push(format!(
                "Range {:.1}°C가 목표 {:.1}°C보다 작습니다.",
                range_c, t
            ));
        }
    }
    if let Some(t) = input.target_approach_c {
        if approach_c > t {
            warnings.push(format!(
                "Approach {:.1}°C가 목표 {:.1}°C보다 큽니다.",
                approach_c, t
            ));
        }
    }

    // TODO: L/G 추정, 팬 곡선/모터 부하 추정 추가
    CoolingTowerResult {
        range_c,
        approach_c,
        heat_rejected_kw: heat_kw,
        warnings,
    }
}
