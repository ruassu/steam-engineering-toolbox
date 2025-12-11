use serde::{Deserialize, Serialize};

/// 점도 단위. 내부 기준은 Pa·s이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViscosityUnit {
    PascalSecond,
    Centipoise,
}

fn to_pas(value: f64, unit: ViscosityUnit) -> f64 {
    match unit {
        ViscosityUnit::PascalSecond => value,
        ViscosityUnit::Centipoise => value / 1000.0,
    }
}

fn from_pas(value: f64, unit: ViscosityUnit) -> f64 {
    match unit {
        ViscosityUnit::PascalSecond => value,
        ViscosityUnit::Centipoise => value * 1000.0,
    }
}

/// 점도를 변환한다.
pub fn convert_viscosity(value: f64, from: ViscosityUnit, to: ViscosityUnit) -> f64 {
    let base = to_pas(value, from);
    from_pas(base, to)
}
