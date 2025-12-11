use serde::{Deserialize, Serialize};

/// 게이지/절대압을 구분하기 위한 확장용 enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureKind {
    Gauge,
    Absolute,
}

/// 압력 단위. 내부 기준은 항상 bar(게이지 기준)이다.
/// mmHg의 경우 0을 대기압, -760mmHg를 완전 진공(게이지 기준)으로 취급한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PressureUnit {
    Bar,
    BarA,
    MilliBar,
    Pascal,
    KiloPascal,
    MegaPascal,
    KgPerCm2,
    Psi,
    Atm,
    MmHg,
}

const ATM_BAR: f64 = 1.01325;
const MMHG_PER_BAR: f64 = 750.062;
const PA_PER_BAR: f64 = 100_000.0;

/// 주어진 압력을 bar 로 변환한다.
/// 내부 기준은 게이지 압력(bar g)이다. 절대압 단위는 대기압(1.01325 bar)을 보정하여 게이지로 환산한다.
/// mmHg는 0을 대기압, -760mmHg를 완전 진공으로 간주하는 게이지 척도로 처리한다.
pub fn to_bar(value: f64, unit: PressureUnit) -> f64 {
    match unit {
        PressureUnit::Bar => value,
        PressureUnit::BarA => value - ATM_BAR,
        PressureUnit::MilliBar => value / 1000.0,
        PressureUnit::Pascal => value / PA_PER_BAR,
        PressureUnit::KiloPascal => value / 100.0,
        PressureUnit::MegaPascal => value * 10.0,
        PressureUnit::KgPerCm2 => value * 0.980665,
        PressureUnit::Psi => value * 0.0689476,
        // atm 은 절대압으로 간주
        PressureUnit::Atm => value * ATM_BAR - ATM_BAR,
        // mmHg는 게이지 기준(0=대기)으로 취급
        PressureUnit::MmHg => value / MMHG_PER_BAR,
    }
}

/// bar 값을 원하는 단위로 변환한다.
/// 내부 기준(bar g)을 요청된 단위로 변환하며, 절대압 단위는 대기압을 더해 반환한다.
/// mmHg는 게이지 기준(0=대기)으로 반환한다.
pub fn from_bar(value_bar: f64, unit: PressureUnit) -> f64 {
    match unit {
        PressureUnit::Bar => value_bar,
        PressureUnit::BarA => value_bar + ATM_BAR,
        PressureUnit::MilliBar => value_bar * 1000.0,
        PressureUnit::Pascal => value_bar * PA_PER_BAR,
        PressureUnit::KiloPascal => value_bar * 100.0,
        PressureUnit::MegaPascal => value_bar / 10.0,
        PressureUnit::KgPerCm2 => value_bar / 0.980665,
        PressureUnit::Psi => value_bar / 0.0689476,
        PressureUnit::Atm => (value_bar + ATM_BAR) / ATM_BAR,
        PressureUnit::MmHg => value_bar * MMHG_PER_BAR,
    }
}

/// 압력을 원하는 단위로 변환한다.
pub fn convert_pressure(value: f64, from: PressureUnit, to: PressureUnit) -> f64 {
    let bar = to_bar(value, from);
    from_bar(bar, to)
}
