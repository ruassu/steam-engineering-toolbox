use crate::quantity::QuantityKind;
use crate::units::*;

/// 단위 변환 시 발생 가능한 오류.
#[derive(Debug)]
pub enum ConversionError {
    /// 알 수 없는 단위 문자열
    UnknownUnit(String),
    /// 지원하지 않는 물리량
    UnsupportedQuantity(&'static str),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::UnknownUnit(u) => write!(f, "알 수 없는 단위: {u}"),
            ConversionError::UnsupportedQuantity(q) => write!(f, "지원하지 않는 물리량: {q}"),
        }
    }
}

impl std::error::Error for ConversionError {}

/// 게이지/절대 모드를 표현한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureMode {
    Gauge,
    Absolute,
}

/// 압력 변환 (모드 포함). 내부 기준은 bar(abs)로 처리한 뒤 요청 모드로 반환한다.
pub fn convert_pressure_mode(
    value: f64,
    from_unit: PressureUnit,
    from_mode: PressureMode,
    to_unit: PressureUnit,
    to_mode: PressureMode,
) -> f64 {
    const ATM_BAR: f64 = 1.01325;
    const PA_PER_BAR: f64 = 100_000.0;
    const MMHG_PER_BAR: f64 = 750.062;

    // 입력을 bar(abs)로 환산
    let base = match from_unit {
        PressureUnit::Bar | PressureUnit::BarA => value,
        PressureUnit::MilliBar => value / 1000.0,
        PressureUnit::Pascal => value / PA_PER_BAR,
        PressureUnit::KiloPascal => value / 100.0,
        PressureUnit::MegaPascal => value * 10.0,
        PressureUnit::KgPerCm2 => value * 0.980665,
        PressureUnit::Psi => value * 0.0689476,
        PressureUnit::Atm => value * ATM_BAR,
        PressureUnit::MmHg => value / MMHG_PER_BAR,
    };
    let bar_abs = match from_mode {
        PressureMode::Gauge => base + ATM_BAR,
        PressureMode::Absolute => base,
    };

    // 모드에 따라 게이지/절대 환산한 뒤 목표 단위로 변환
    let bar_target = match to_mode {
        PressureMode::Absolute => bar_abs,
        PressureMode::Gauge => bar_abs - ATM_BAR,
    };

    match to_unit {
        PressureUnit::Bar | PressureUnit::BarA => bar_target,
        PressureUnit::MilliBar => bar_target * 1000.0,
        PressureUnit::Pascal => bar_target * PA_PER_BAR,
        PressureUnit::KiloPascal => bar_target * 100.0,
        PressureUnit::MegaPascal => bar_target / 10.0,
        PressureUnit::KgPerCm2 => bar_target / 0.980665,
        PressureUnit::Psi => bar_target / 0.0689476,
        PressureUnit::Atm => bar_target / ATM_BAR,
        PressureUnit::MmHg => bar_target * MMHG_PER_BAR,
    }
}

/// 문자열로 전달된 단위명을 enum으로 변환한 뒤 지정된 단위로 환산한다.
///
/// 단위 문자열 예시는 `C`, `bar`, `m`, `kPa`, `Btu`, `ft/s` 등을 사용할 수 있다.
pub fn convert(
    kind: QuantityKind,
    value: f64,
    from_unit_str: &str,
    to_unit_str: &str,
) -> Result<f64, ConversionError> {
    match kind {
        QuantityKind::Temperature => {
            let from = parse_temperature_unit(from_unit_str)?;
            let to = parse_temperature_unit(to_unit_str)?;
            Ok(convert_temperature(value, from, to))
        }
        QuantityKind::TemperatureDifference => {
            let from = parse_temperature_diff_unit(from_unit_str)?;
            let to = parse_temperature_diff_unit(to_unit_str)?;
            Ok(convert_temperature_diff(value, from, to))
        }
        QuantityKind::Pressure => {
            let from = parse_pressure_unit(from_unit_str)?;
            let to = parse_pressure_unit(to_unit_str)?;
            Ok(convert_pressure(value, from, to))
        }
        QuantityKind::Length => {
            let from = parse_length_unit(from_unit_str)?;
            let to = parse_length_unit(to_unit_str)?;
            Ok(convert_length(value, from, to))
        }
        QuantityKind::Area => {
            let from = parse_area_unit(from_unit_str)?;
            let to = parse_area_unit(to_unit_str)?;
            Ok(convert_area(value, from, to))
        }
        QuantityKind::Volume => {
            let from = parse_volume_unit(from_unit_str)?;
            let to = parse_volume_unit(to_unit_str)?;
            Ok(convert_volume(value, from, to))
        }
        QuantityKind::Velocity => {
            let from = parse_velocity_unit(from_unit_str)?;
            let to = parse_velocity_unit(to_unit_str)?;
            Ok(convert_velocity(value, from, to))
        }
        QuantityKind::Mass => {
            let from = parse_mass_unit(from_unit_str)?;
            let to = parse_mass_unit(to_unit_str)?;
            Ok(convert_mass(value, from, to))
        }
        QuantityKind::Viscosity => {
            let from = parse_viscosity_unit(from_unit_str)?;
            let to = parse_viscosity_unit(to_unit_str)?;
            Ok(convert_viscosity(value, from, to))
        }
        QuantityKind::Energy => {
            let from = parse_energy_unit(from_unit_str)?;
            let to = parse_energy_unit(to_unit_str)?;
            Ok(convert_energy(value, from, to))
        }
        QuantityKind::HeatTransferCoeff => {
            let from = parse_heat_transfer_unit(from_unit_str)?;
            let to = parse_heat_transfer_unit(to_unit_str)?;
            Ok(convert_heat_transfer(value, from, to))
        }
        QuantityKind::ThermalConductivity => {
            let from = parse_conductivity_unit(from_unit_str)?;
            let to = parse_conductivity_unit(to_unit_str)?;
            Ok(convert_conductivity(value, from, to))
        }
        QuantityKind::SpecificEnthalpy => {
            let from = parse_specific_enthalpy_unit(from_unit_str)?;
            let to = parse_specific_enthalpy_unit(to_unit_str)?;
            Ok(convert_specific_enthalpy(value, from, to))
        }
    }
}

fn parse_temperature_unit(s: &str) -> Result<TemperatureUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "k" | "kelvin" => Ok(TemperatureUnit::Kelvin),
        "c" | "celsius" | "°c" => Ok(TemperatureUnit::Celsius),
        "f" | "fahrenheit" | "°f" => Ok(TemperatureUnit::Fahrenheit),
        "r" | "rankine" => Ok(TemperatureUnit::Rankine),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_temperature_diff_unit(s: &str) -> Result<TemperatureDiffUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "k" | "kelvin" => Ok(TemperatureDiffUnit::Kelvin),
        "c" | "celsius" | "°c" => Ok(TemperatureDiffUnit::Celsius),
        "f" | "fahrenheit" | "°f" => Ok(TemperatureDiffUnit::Fahrenheit),
        "r" | "rankine" => Ok(TemperatureDiffUnit::Rankine),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_pressure_unit(s: &str) -> Result<PressureUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "bar" => Ok(PressureUnit::Bar),
        "bara" => Ok(PressureUnit::BarA),
        "mbar" | "millibar" => Ok(PressureUnit::MilliBar),
        "pa" | "pascal" => Ok(PressureUnit::Pascal),
        "kpa" | "kilopascal" => Ok(PressureUnit::KiloPascal),
        "mpa" | "megapascal" => Ok(PressureUnit::MegaPascal),
        "kg/cm2" | "kgf/cm2" => Ok(PressureUnit::KgPerCm2),
        "psi" => Ok(PressureUnit::Psi),
        "atm" => Ok(PressureUnit::Atm),
        "mmhg" | "torr" => Ok(PressureUnit::MmHg),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_length_unit(s: &str) -> Result<LengthUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "m" | "meter" | "metre" => Ok(LengthUnit::Meter),
        "mm" => Ok(LengthUnit::Millimeter),
        "cm" => Ok(LengthUnit::Centimeter),
        "km" => Ok(LengthUnit::Kilometer),
        "in" | "inch" => Ok(LengthUnit::Inch),
        "ft" | "foot" => Ok(LengthUnit::Foot),
        "yd" | "yard" => Ok(LengthUnit::Yard),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_area_unit(s: &str) -> Result<AreaUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "m2" | "m^2" | "sqm" => Ok(AreaUnit::SquareMeter),
        "ft2" | "ft^2" | "sqft" => Ok(AreaUnit::SquareFoot),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_volume_unit(s: &str) -> Result<VolumeUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "m3" | "m^3" => Ok(VolumeUnit::CubicMeter),
        "l" | "liter" | "litre" => Ok(VolumeUnit::Liter),
        "ml" | "milliliter" => Ok(VolumeUnit::Milliliter),
        "ft3" | "ft^3" | "cuft" => Ok(VolumeUnit::CubicFoot),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_velocity_unit(s: &str) -> Result<VelocityUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "m/s" | "mps" => Ok(VelocityUnit::MeterPerSecond),
        "ft/s" | "fps" => Ok(VelocityUnit::FootPerSecond),
        "km/h" | "kph" => Ok(VelocityUnit::KilometerPerHour),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_mass_unit(s: &str) -> Result<MassUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "kg" => Ok(MassUnit::Kilogram),
        "g" => Ok(MassUnit::Gram),
        "lb" | "lbs" | "lbm" => Ok(MassUnit::Pound),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_viscosity_unit(s: &str) -> Result<ViscosityUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "pa·s" | "pa.s" | "pas" => Ok(ViscosityUnit::PascalSecond),
        "cps" | "cp" => Ok(ViscosityUnit::Centipoise),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_energy_unit(s: &str) -> Result<EnergyUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "j" | "joule" => Ok(EnergyUnit::Joule),
        "kj" | "kilojoule" => Ok(EnergyUnit::Kilojoule),
        "kcal" | "kilocalorie" => Ok(EnergyUnit::KiloCalorie),
        "btu" => Ok(EnergyUnit::Btu),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_heat_transfer_unit(s: &str) -> Result<HeatTransferUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "w/m2k" | "w/m^2k" => Ok(HeatTransferUnit::WPerSquareMeterK),
        "btu/h-ft2-f" | "btu/(h·ft2·f)" => Ok(HeatTransferUnit::BtuPerHourSquareFootF),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_conductivity_unit(s: &str) -> Result<ConductivityUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "w/mk" | "w/(m·k)" => Ok(ConductivityUnit::WPerMeterK),
        "btu/h-ft-f" | "btu/(h·ft·f)" => Ok(ConductivityUnit::BtuPerHourFootF),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}

fn parse_specific_enthalpy_unit(s: &str) -> Result<SpecificEnthalpyUnit, ConversionError> {
    match s.to_lowercase().as_str() {
        "kj/kg" => Ok(SpecificEnthalpyUnit::KjPerKg),
        "kcal/kg" => Ok(SpecificEnthalpyUnit::KcalPerKg),
        "btu/lb" | "btu/lbm" => Ok(SpecificEnthalpyUnit::BtuPerPound),
        _ => Err(ConversionError::UnknownUnit(s.to_string())),
    }
}
