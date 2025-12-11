use serde::{Deserialize, Serialize};

/// 면적 단위. 내부 기준은 제곱미터이다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AreaUnit {
    SquareMeter,
    SquareFoot,
}

fn to_square_meter(value: f64, unit: AreaUnit) -> f64 {
    match unit {
        AreaUnit::SquareMeter => value,
        AreaUnit::SquareFoot => value * 0.092903,
    }
}

fn from_square_meter(value: f64, unit: AreaUnit) -> f64 {
    match unit {
        AreaUnit::SquareMeter => value,
        AreaUnit::SquareFoot => value / 0.092903,
    }
}

/// 면적을 변환한다.
pub fn convert_area(value: f64, from: AreaUnit, to: AreaUnit) -> f64 {
    let m2 = to_square_meter(value, from);
    from_square_meter(m2, to)
}
