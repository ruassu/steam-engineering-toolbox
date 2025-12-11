use serde::{Deserialize, Serialize};

/// 에너지 단위. 내부 기준은 줄(J)이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnergyUnit {
    Joule,
    Kilojoule,
    KiloCalorie,
    Btu,
}

fn to_joule(value: f64, unit: EnergyUnit) -> f64 {
    match unit {
        EnergyUnit::Joule => value,
        EnergyUnit::Kilojoule => value * 1000.0,
        EnergyUnit::KiloCalorie => value * 4184.0,
        EnergyUnit::Btu => value * 1055.06,
    }
}

fn from_joule(value: f64, unit: EnergyUnit) -> f64 {
    match unit {
        EnergyUnit::Joule => value,
        EnergyUnit::Kilojoule => value / 1000.0,
        EnergyUnit::KiloCalorie => value / 4184.0,
        EnergyUnit::Btu => value / 1055.06,
    }
}

/// 에너지를 변환한다.
pub fn convert_energy(value: f64, from: EnergyUnit, to: EnergyUnit) -> f64 {
    let j = to_joule(value, from);
    from_joule(j, to)
}
