use serde::{Deserialize, Serialize};

/// 길이 단위. 내부 기준은 미터이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LengthUnit {
    Meter,
    Millimeter,
    Centimeter,
    Kilometer,
    Inch,
    Foot,
    Yard,
}

fn to_meter(value: f64, unit: LengthUnit) -> f64 {
    match unit {
        LengthUnit::Meter => value,
        LengthUnit::Millimeter => value / 1000.0,
        LengthUnit::Centimeter => value / 100.0,
        LengthUnit::Kilometer => value * 1000.0,
        LengthUnit::Inch => value * 0.0254,
        LengthUnit::Foot => value * 0.3048,
        LengthUnit::Yard => value * 0.9144,
    }
}

fn from_meter(value_m: f64, unit: LengthUnit) -> f64 {
    match unit {
        LengthUnit::Meter => value_m,
        LengthUnit::Millimeter => value_m * 1000.0,
        LengthUnit::Centimeter => value_m * 100.0,
        LengthUnit::Kilometer => value_m / 1000.0,
        LengthUnit::Inch => value_m / 0.0254,
        LengthUnit::Foot => value_m / 0.3048,
        LengthUnit::Yard => value_m / 0.9144,
    }
}

/// 길이를 다른 단위로 변환한다.
pub fn convert_length(value: f64, from: LengthUnit, to: LengthUnit) -> f64 {
    let m = to_meter(value, from);
    from_meter(m, to)
}
