/// 다루는 물리량 종류를 나타낸다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityKind {
    Temperature,
    TemperatureDifference,
    Pressure,
    Length,
    Area,
    Volume,
    Velocity,
    Mass,
    Viscosity,
    Energy,
    HeatTransferCoeff,
    ThermalConductivity,
    SpecificEnthalpy,
}

/// 내부 기준 단위로 환산된 값을 담는 컨테이너.
///
/// 각 kind에 따라 `value_base`는 SI(Bar) 기준 내부 단위(압력=bar, 온도=K,
/// 길이=m, 에너지=J, 비엔탈피=kJ/kg 등)로 저장한다.
#[derive(Debug, Clone, Copy)]
pub struct QuantityValue {
    pub kind: QuantityKind,
    pub value_base: f64,
}
