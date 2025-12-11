use serde::{Deserialize, Serialize};

/// 질량 단위. 내부 기준은 kg이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MassUnit {
    Kilogram,
    Gram,
    Pound,
}

fn to_kg(value: f64, unit: MassUnit) -> f64 {
    match unit {
        MassUnit::Kilogram => value,
        MassUnit::Gram => value / 1000.0,
        MassUnit::Pound => value * 0.453592,
    }
}

fn from_kg(value: f64, unit: MassUnit) -> f64 {
    match unit {
        MassUnit::Kilogram => value,
        MassUnit::Gram => value * 1000.0,
        MassUnit::Pound => value / 0.453592,
    }
}

/// 질량을 변환한다.
pub fn convert_mass(value: f64, from: MassUnit, to: MassUnit) -> f64 {
    let base = to_kg(value, from);
    from_kg(base, to)
}
