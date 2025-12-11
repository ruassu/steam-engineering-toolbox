use serde::{Deserialize, Serialize};

/// 온도 단위를 정의한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureUnit {
    Kelvin,
    Celsius,
    Fahrenheit,
    Rankine,
}

/// 온도차 단위를 정의한다. 스케일만 고려한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemperatureDiffUnit {
    Kelvin,
    Celsius,
    Fahrenheit,
    Rankine,
}

/// 주어진 값을 켈빈으로 변환한다.
pub fn to_kelvin(value: f64, unit: TemperatureUnit) -> f64 {
    match unit {
        TemperatureUnit::Kelvin => value,
        TemperatureUnit::Celsius => value + 273.15,
        TemperatureUnit::Fahrenheit => (value + 459.67) * 5.0 / 9.0,
        TemperatureUnit::Rankine => value * 5.0 / 9.0,
    }
}

/// 켈빈 값을 원하는 단위로 변환한다.
pub fn from_kelvin(value_k: f64, unit: TemperatureUnit) -> f64 {
    match unit {
        TemperatureUnit::Kelvin => value_k,
        TemperatureUnit::Celsius => value_k - 273.15,
        TemperatureUnit::Fahrenheit => value_k * 9.0 / 5.0 - 459.67,
        TemperatureUnit::Rankine => value_k * 9.0 / 5.0,
    }
}

/// 온도를 서로 다른 단위로 변환한다.
pub fn convert_temperature(value: f64, from: TemperatureUnit, to: TemperatureUnit) -> f64 {
    let k = to_kelvin(value, from);
    from_kelvin(k, to)
}

/// 온도차를 서로 다른 단위로 변환한다. 절대 기준점 없이 배율만 고려한다.
pub fn convert_temperature_diff(
    value: f64,
    from: TemperatureDiffUnit,
    to: TemperatureDiffUnit,
) -> f64 {
    // 섭씨/켈빈은 1:1, 화씨/랭킨은 1.8:1 배율
    let base_k = match from {
        TemperatureDiffUnit::Kelvin | TemperatureDiffUnit::Celsius => value,
        TemperatureDiffUnit::Fahrenheit | TemperatureDiffUnit::Rankine => value * 5.0 / 9.0,
    };
    match to {
        TemperatureDiffUnit::Kelvin | TemperatureDiffUnit::Celsius => base_k,
        TemperatureDiffUnit::Fahrenheit | TemperatureDiffUnit::Rankine => base_k * 9.0 / 5.0,
    }
}
