use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::units::*;

/// 사용 가능한 단위 시스템 프리셋을 정의한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitSystem {
    /// SI(Bar) 기준. 내부 계산 기본값.
    SIBar,
    /// SI (Pa 기반)
    SI,
    /// MKS 혼합
    MKS,
    /// 영국식/야드파운드법
    Imperial,
}

/// 각 물리량별 기본 단위 설정을 담는다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultUnits {
    pub temperature: TemperatureUnit,
    pub temperature_diff: TemperatureDiffUnit,
    pub pressure: PressureUnit,
    pub length: LengthUnit,
    pub area: AreaUnit,
    pub volume: VolumeUnit,
    pub velocity: VelocityUnit,
    pub mass: MassUnit,
    pub viscosity: ViscosityUnit,
    pub energy: EnergyUnit,
    pub heat_transfer: HeatTransferUnit,
    pub conductivity: ConductivityUnit,
    pub specific_enthalpy: SpecificEnthalpyUnit,
}

impl Default for DefaultUnits {
    fn default() -> Self {
        Self {
            temperature: TemperatureUnit::Celsius,
            temperature_diff: TemperatureDiffUnit::Kelvin,
            pressure: PressureUnit::Bar,
            length: LengthUnit::Meter,
            area: AreaUnit::SquareMeter,
            volume: VolumeUnit::CubicMeter,
            velocity: VelocityUnit::MeterPerSecond,
            mass: MassUnit::Kilogram,
            viscosity: ViscosityUnit::PascalSecond,
            energy: EnergyUnit::Joule,
            heat_transfer: HeatTransferUnit::WPerSquareMeterK,
            conductivity: ConductivityUnit::WPerMeterK,
            specific_enthalpy: SpecificEnthalpyUnit::KjPerKg,
        }
    }
}

/// 애플리케이션 설정을 표현한다.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub unit_system: UnitSystem,
    pub default_units: DefaultUnits,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            unit_system: UnitSystem::SIBar,
            default_units: DefaultUnits::default(),
        }
    }
}

/// 설정 로드/저장 시 발생 가능한 오류를 표현한다.
#[derive(Debug)]
pub enum ConfigError {
    /// 파일 입출력 오류
    Io(std::io::Error),
    /// TOML 직렬화/역직렬화 오류
    Serde(toml::de::Error),
    /// TOML 직렬화 오류
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "파일 입출력 오류: {e}"),
            ConfigError::Serde(e) => write!(f, "설정 파싱 오류: {e}"),
            ConfigError::Serialize(e) => write!(f, "설정 직렬화 오류: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        ConfigError::Io(value)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(value: toml::de::Error) -> Self {
        ConfigError::Serde(value)
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(value: toml::ser::Error) -> Self {
        ConfigError::Serialize(value)
    }
}

/// config.toml을 로드하거나 없으면 기본 설정을 생성한다.
pub fn load_or_default() -> Result<Config, ConfigError> {
    let path = Path::new("config.toml");
    if path.exists() {
        let content = fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    } else {
        let cfg = Config::default();
        save_config(&cfg)?;
        Ok(cfg)
    }
}

fn save_config(cfg: &Config) -> Result<(), ConfigError> {
    let content = toml::to_string_pretty(cfg)?;
    fs::write("config.toml", content)?;
    Ok(())
}

impl Config {
    /// 설정을 config.toml에 저장한다.
    pub fn save(&self) -> Result<(), ConfigError> {
        save_config(self)
    }
}
