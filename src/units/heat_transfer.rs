use serde::{Deserialize, Serialize};

/// 열전달계수(U) 단위. 내부 기준은 W/m²·K이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeatTransferUnit {
    WPerSquareMeterK,
    BtuPerHourSquareFootF,
}

fn to_base(value: f64, unit: HeatTransferUnit) -> f64 {
    match unit {
        HeatTransferUnit::WPerSquareMeterK => value,
        HeatTransferUnit::BtuPerHourSquareFootF => value * 5.678263,
    }
}

fn from_base(value: f64, unit: HeatTransferUnit) -> f64 {
    match unit {
        HeatTransferUnit::WPerSquareMeterK => value,
        HeatTransferUnit::BtuPerHourSquareFootF => value / 5.678263,
    }
}

/// 열전달계수를 변환한다.
pub fn convert_heat_transfer(value: f64, from: HeatTransferUnit, to: HeatTransferUnit) -> f64 {
    let base = to_base(value, from);
    from_base(base, to)
}
