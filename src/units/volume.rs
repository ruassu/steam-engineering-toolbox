use serde::{Deserialize, Serialize};

/// 체적 단위. 내부 기준은 입방미터이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolumeUnit {
    CubicMeter,
    Liter,
    Milliliter,
    CubicFoot,
}

fn to_cubic_meter(value: f64, unit: VolumeUnit) -> f64 {
    match unit {
        VolumeUnit::CubicMeter => value,
        VolumeUnit::Liter => value / 1000.0,
        VolumeUnit::Milliliter => value / 1_000_000.0,
        VolumeUnit::CubicFoot => value * 0.0283168,
    }
}

fn from_cubic_meter(value: f64, unit: VolumeUnit) -> f64 {
    match unit {
        VolumeUnit::CubicMeter => value,
        VolumeUnit::Liter => value * 1000.0,
        VolumeUnit::Milliliter => value * 1_000_000.0,
        VolumeUnit::CubicFoot => value / 0.0283168,
    }
}

/// 체적을 변환한다.
pub fn convert_volume(value: f64, from: VolumeUnit, to: VolumeUnit) -> f64 {
    let m3 = to_cubic_meter(value, from);
    from_cubic_meter(m3, to)
}
