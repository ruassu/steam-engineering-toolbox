use std::io::{self, Write};

use crate::app::AppError;
use crate::config::{Config, UnitSystem};
use crate::conversion;
use crate::quantity::QuantityKind;
use crate::steam::{
    self, steam_piping::PipeSizingByVelocityInput, steam_piping::PressureLossInput,
};
use crate::units::{PressureUnit, TemperatureUnit};

/// 메인 메뉴 선택지를 표현한다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuChoice {
    UnitConversion,
    SteamTables,
    SteamPiping,
    SteamValves,
    Settings,
    Exit,
}

/// 메인 메뉴를 표시하고 선택값을 반환한다.
pub fn main_menu() -> Result<MenuChoice, AppError> {
    println!("\n=== Steam Engineering Toolbox ===");
    println!("1) 단위 변환기");
    println!("2) Steam Tables");
    println!("3) Steam Piping");
    println!("4) Steam Valves & Orifices");
    println!("5) 설정");
    println!("0) 종료");
    loop {
        let sel = read_line("메뉴 선택: ")?;
        match sel.trim() {
            "1" => return Ok(MenuChoice::UnitConversion),
            "2" => return Ok(MenuChoice::SteamTables),
            "3" => return Ok(MenuChoice::SteamPiping),
            "4" => return Ok(MenuChoice::SteamValves),
            "5" => return Ok(MenuChoice::Settings),
            "0" => return Ok(MenuChoice::Exit),
            _ => println!("잘못된 입력입니다. 다시 선택하세요."),
        }
    }
}

/// 단위 변환 메뉴를 처리한다.
pub fn handle_unit_conversion(_cfg: &Config) -> Result<(), AppError> {
    println!("\n-- 단위 변환 --");
    println!("1) 온도  2) 온도차  3) 압력  4) 길이  5) 면적  6) 체적");
    println!("7) 속도  8) 질량  9) 점도 10) 에너지 11) 열전달율 12) 열전도율 13) 비엔탈피");
    println!("참고: mmHg는 게이지 기준(0=대기, -760mmHg=완전진공)으로 처리됩니다.");
    let kind = loop {
        let sel = read_line("항목 번호를 입력: ")?;
        if let Ok(n) = sel.trim().parse::<u32>() {
            if let Some(kind) = map_quantity(n) {
                break kind;
            }
        }
        println!("지원하지 않는 번호입니다.");
    };
    let value = read_f64("값 입력: ")?;
    let from_unit = read_line("입력 단위(ex: C, bar, m): ")?;
    let to_unit = read_line("변환 단위(ex: K, psi, ft): ")?;
    let result = conversion::convert(kind, value, from_unit.trim(), to_unit.trim())?;
    println!("변환 결과: {result} {}", to_unit.trim());
    Ok(())
}

fn map_quantity(n: u32) -> Option<QuantityKind> {
    match n {
        1 => Some(QuantityKind::Temperature),
        2 => Some(QuantityKind::TemperatureDifference),
        3 => Some(QuantityKind::Pressure),
        4 => Some(QuantityKind::Length),
        5 => Some(QuantityKind::Area),
        6 => Some(QuantityKind::Volume),
        7 => Some(QuantityKind::Velocity),
        8 => Some(QuantityKind::Mass),
        9 => Some(QuantityKind::Viscosity),
        10 => Some(QuantityKind::Energy),
        11 => Some(QuantityKind::HeatTransferCoeff),
        12 => Some(QuantityKind::ThermalConductivity),
        13 => Some(QuantityKind::SpecificEnthalpy),
        _ => None,
    }
}

/// Steam Tables 메뉴를 처리한다.
pub fn handle_steam_tables(_cfg: &Config) -> Result<(), AppError> {
    println!("\n-- Steam Tables --");
    println!("참고: 압력 mmHg 입력 시 0=대기, -760mmHg=완전진공으로 해석합니다.");
    println!("1) By Pressure  2) By Temperature  3) Superheated (압력+온도)");
    let choice = read_line("선택: ")?;
    match choice.trim() {
        "1" => {
            let p = read_f64("압력 값: ")?;
            let unit = read_pressure_unit()?;
            let state = steam::saturation_by_pressure(p, unit)?;
            print_state(&state);
        }
        "2" => {
            let t = read_f64("온도 값: ")?;
            let unit = read_temperature_unit()?;
            let state = steam::saturation_by_temperature(t, unit)?;
            print_state(&state);
        }
        "3" => {
            let p = read_f64("압력 값: ")?;
            let p_unit = read_pressure_unit()?;
            let t = read_f64("온도 값: ")?;
            let t_unit = read_temperature_unit()?;
            let state = steam::superheated_at(p, p_unit, t, t_unit)?;
            print_state(&state);
        }
        _ => println!("잘못된 선택입니다."),
    }
    Ok(())
}

/// Steam Piping 메뉴를 처리한다.
pub fn handle_steam_piping(_cfg: &Config) -> Result<(), AppError> {
    println!("\n-- Steam Piping --");
    println!("1) 목표 유속 기준 사이징");
    println!("2) 압력손실 계산");
    let sel = read_line("선택: ")?;
    match sel.trim() {
        "1" => {
            let mflow = read_f64("질량 유량 [kg/h]: ")?;
            let pressure = read_f64("운전 압력 값: ")?;
            let p_unit = read_pressure_unit()?;
            let temp = read_f64("운전 온도 값: ")?;
            let t_unit = read_temperature_unit()?;
            let density = steam::estimate_density(pressure, p_unit, temp, t_unit);
            let target_v = read_f64("허용 유속 [m/s]: ")?;
            let input = PipeSizingByVelocityInput {
                mass_flow_kg_per_h: mflow,
                steam_density_kg_per_m3: density,
                target_velocity_m_per_s: target_v,
            };
            let result = steam::size_by_velocity(input)?;
            println!("추천 내경: {:.4} m", result.inner_diameter_m);
            println!(
                "예상 유속: {:.2} m/s, Re={:.2e}",
                result.velocity_m_per_s, result.reynolds_number
            );
        }
        "2" => {
            let mflow = read_f64("질량 유량 [kg/h]: ")?;
            let density = read_f64("증기 밀도 [kg/m3] (알 수 없으면 0 입력 시 근사 사용): ")?;
            let density = if density <= 0.0 {
                let p = read_f64("운전 압력 값: ")?;
                let p_unit = read_pressure_unit()?;
                let t = read_f64("운전 온도 값: ")?;
                let t_unit = read_temperature_unit()?;
                steam::estimate_density(p, p_unit, t, t_unit)
            } else {
                density
            };
            let diameter = read_f64("배관 내경 [m]: ")?;
            let length = read_f64("배관 길이 [m]: ")?;
            let eq_len = read_f64("등가 길이 [m] (없으면 0): ")?;
            let k_sum = read_f64("피팅 K 합계 (없으면 0): ")?;
            let roughness = read_f64("거칠기 ε [m] (탄소강 배관 약 0.000045): ")?;
            let visc = read_f64("동점도 [Pa·s] (증기 기본값 1.2e-5 추천): ")?;
            let sound_speed = read_f64("음속 [m/s] (기본 450 정도): ")?;
            let input = PressureLossInput {
                mass_flow_kg_per_h: mflow,
                steam_density_kg_per_m3: density,
                diameter_m: diameter,
                length_m: length,
                equivalent_length_m: eq_len,
                fittings_k_sum: k_sum,
                roughness_m: roughness,
                dynamic_viscosity_pa_s: visc,
                sound_speed_m_per_s: sound_speed,
            };
            let result = steam::pressure_loss(input)?;
            println!(
                "유속: {:.2} m/s, ΔP: {:.4} bar, f={:.4}, Re={:.2e}, Mach={:.3}",
                result.velocity_m_per_s,
                result.pressure_drop_bar,
                result.friction_factor,
                result.reynolds_number,
                result.mach
            );
        }
        _ => println!("잘못된 선택입니다."),
    }
    Ok(())
}

/// Steam Valves 메뉴를 처리한다.
pub fn handle_steam_valves(_cfg: &Config) -> Result<(), AppError> {
    println!("\n-- Steam Valves & Orifices --");
    println!("1) 필요한 Cv/Kv 계산");
    println!("2) Cv/Kv로 가능한 유량 계산");
    let sel = read_line("선택: ")?;
    match sel.trim() {
        "1" => {
            let flow = read_f64("볼류메트릭 유량 [m3/h]: ")?;
            let dp = read_f64("차압 [bar]: ")?;
            let rho = read_f64("밀도 [kg/m3]: ")?;
            let kv = steam::required_kv(flow, dp, rho)?;
            let cv = steam::cv_from_kv(kv);
            println!("필요 Kv: {:.3}, Cv: {:.3}", kv, cv);
        }
        "2" => {
            let mode = read_line("입력 단위 선택 (1=Kv, 2=Cv): ")?;
            let value = read_f64("Kv/Cv 값: ")?;
            let dp = read_f64("차압 [bar]: ")?;
            let rho = read_f64("밀도 [kg/m3]: ")?;
            let p_up = read_f64("상류 압력 [bar(a)]: ")?;
            let flow = if mode.trim() == "2" {
                steam::flow_from_cv(value, dp, rho)?
            } else {
                steam::flow_from_kv(value, dp, rho, Some(p_up))?
            };
            println!(
                "가능한 유량: {:.3} m3/h (질량유량 {:.3} kg/h)",
                flow,
                flow * rho
            );
        }
        _ => println!("잘못된 선택입니다."),
    }
    Ok(())
}

/// 설정 메뉴를 처리한다.
pub fn handle_settings(cfg: &mut Config) -> Result<(), AppError> {
    println!("\n-- 설정 --");
    println!("현재 단위 시스템: {:?}", cfg.unit_system);
    println!("1) SI(Bar)  2) SI  3) MKS  4) Imperial");
    let sel = read_line("변경할 번호(취소하려면 엔터): ")?;
    if sel.trim().is_empty() {
        return Ok(());
    }
    cfg.unit_system = match sel.trim() {
        "1" => UnitSystem::SIBar,
        "2" => UnitSystem::SI,
        "3" => UnitSystem::MKS,
        "4" => UnitSystem::Imperial,
        _ => {
            println!("잘못된 입력이므로 변경하지 않습니다.");
            cfg.unit_system
        }
    };
    println!("단위 시스템이 {:?} 로 설정되었습니다.", cfg.unit_system);
    Ok(())
}

fn read_line(prompt: &str) -> Result<String, AppError> {
    print!("{prompt}");
    io::stdout().flush().map_err(AppError::Io)?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).map_err(AppError::Io)?;
    Ok(buf)
}

fn read_f64(prompt: &str) -> Result<f64, AppError> {
    loop {
        let s = read_line(prompt)?;
        match s.trim().parse::<f64>() {
            Ok(v) => return Ok(v),
            Err(_) => println!("숫자를 입력하세요."),
        }
    }
}

fn read_pressure_unit() -> Result<PressureUnit, AppError> {
    println!("압력 단위: 1=bar 2=kPa 3=MPa 4=psi 5=atm");
    let sel = read_line("선택: ")?;
    let unit = match sel.trim() {
        "1" => PressureUnit::Bar,
        "2" => PressureUnit::KiloPascal,
        "3" => PressureUnit::MegaPascal,
        "4" => PressureUnit::Psi,
        "5" => PressureUnit::Atm,
        _ => PressureUnit::Bar,
    };
    Ok(unit)
}

fn read_temperature_unit() -> Result<TemperatureUnit, AppError> {
    println!("온도 단위: 1=°C 2=K 3=°F 4=R");
    let sel = read_line("선택: ")?;
    let unit = match sel.trim() {
        "1" => TemperatureUnit::Celsius,
        "2" => TemperatureUnit::Kelvin,
        "3" => TemperatureUnit::Fahrenheit,
        "4" => TemperatureUnit::Rankine,
        _ => TemperatureUnit::Celsius,
    };
    Ok(unit)
}

fn print_state(state: &steam::SteamState) {
    println!("포화 온도: {:.2} °C", state.saturation_temperature_c);
    println!("포화 압력: {:.3} bar", state.pressure_bar);
    println!(
        "비엔탈피: {:.1} kJ/kg, 비체적: {:.3} m3/kg",
        state.saturation_enthalpy_kj_per_kg, state.saturation_specific_volume
    );
    if let Some(h) = state.superheated_enthalpy_kj_per_kg {
        println!("과열 비엔탈피: {:.1} kJ/kg", h);
    }
}
