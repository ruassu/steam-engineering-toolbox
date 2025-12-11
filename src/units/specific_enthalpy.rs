use serde::{Deserialize, Serialize};

/// 비엔탈피 단위. 내부 기준은 kJ/kg이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecificEnthalpyUnit {
    KjPerKg,
    KcalPerKg,
    BtuPerPound,
}

fn to_base(value: f64, unit: SpecificEnthalpyUnit) -> f64 {
    match unit {
        SpecificEnthalpyUnit::KjPerKg => value,
        SpecificEnthalpyUnit::KcalPerKg => value * 4.184,
        SpecificEnthalpyUnit::BtuPerPound => value * 2.326,
    }
}

fn from_base(value: f64, unit: SpecificEnthalpyUnit) -> f64 {
    match unit {
        SpecificEnthalpyUnit::KjPerKg => value,
        SpecificEnthalpyUnit::KcalPerKg => value / 4.184,
        SpecificEnthalpyUnit::BtuPerPound => value / 2.326,
    }
}

/// 비엔탈피를 변환한다.
pub fn convert_specific_enthalpy(
    value: f64,
    from: SpecificEnthalpyUnit,
    to: SpecificEnthalpyUnit,
) -> f64 {
    let base = to_base(value, from);
    from_base(base, to)
}
