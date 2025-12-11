use serde::{Deserialize, Serialize};

/// 속도 단위. 내부 기준은 m/s이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VelocityUnit {
    MeterPerSecond,
    FootPerSecond,
    KilometerPerHour,
}

fn to_mps(value: f64, unit: VelocityUnit) -> f64 {
    match unit {
        VelocityUnit::MeterPerSecond => value,
        VelocityUnit::FootPerSecond => value * 0.3048,
        VelocityUnit::KilometerPerHour => value / 3.6,
    }
}

fn from_mps(value: f64, unit: VelocityUnit) -> f64 {
    match unit {
        VelocityUnit::MeterPerSecond => value,
        VelocityUnit::FootPerSecond => value / 0.3048,
        VelocityUnit::KilometerPerHour => value * 3.6,
    }
}

/// 속도를 변환한다.
pub fn convert_velocity(value: f64, from: VelocityUnit, to: VelocityUnit) -> f64 {
    let base = to_mps(value, from);
    from_mps(base, to)
}
