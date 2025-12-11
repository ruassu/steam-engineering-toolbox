use serde::{Deserialize, Serialize};

/// 열전도율(k) 단위. 내부 기준은 W/m·K이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConductivityUnit {
    WPerMeterK,
    BtuPerHourFootF,
}

fn to_base(value: f64, unit: ConductivityUnit) -> f64 {
    match unit {
        ConductivityUnit::WPerMeterK => value,
        ConductivityUnit::BtuPerHourFootF => value * 1.730735,
    }
}

fn from_base(value: f64, unit: ConductivityUnit) -> f64 {
    match unit {
        ConductivityUnit::WPerMeterK => value,
        ConductivityUnit::BtuPerHourFootF => value / 1.730735,
    }
}

/// 열전도율을 변환한다.
pub fn convert_conductivity(value: f64, from: ConductivityUnit, to: ConductivityUnit) -> f64 {
    let base = to_base(value, from);
    from_base(base, to)
}
