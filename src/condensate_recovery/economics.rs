/// 회수 설비 경제성 입력.
#[derive(Debug, Clone)]
pub struct RecoveryEconomicsInput {
    /// 초기 투자비 [원]
    pub capex: f64,
    /// 연간 운전/유지비 [원/년]
    pub opex_per_year: f64,
    /// 연간 절감액 [원/년]
    pub saving_per_year: f64,
    /// 할인율(%) -> 소수
    pub discount_rate: f64,
    /// 분석 기간 [년]
    pub years: u32,
}

/// 경제성 결과.
#[derive(Debug, Clone)]
pub struct RecoveryEconomicsResult {
    /// 단순 회수기간 [년]
    pub payback_years: f64,
    /// 순현재가치(NPV) [원]
    pub npv: f64,
}

/// 단순 회수기간과 NPV를 계산한다.
pub fn recovery_economics(input: RecoveryEconomicsInput) -> RecoveryEconomicsResult {
    let payback_years = if input.saving_per_year > 0.0 {
        (input.capex + input.opex_per_year) / input.saving_per_year
    } else {
        f64::INFINITY
    };
    let mut npv = -input.capex;
    for year in 1..=input.years {
        let net = input.saving_per_year - input.opex_per_year;
        let df = (1.0 + input.discount_rate).powi(year as i32);
        npv += net / df;
    }
    RecoveryEconomicsResult { payback_years, npv }
}
