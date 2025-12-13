use std::io::{self, Write};

use crate::app::AppError;
use crate::config::{Config, UnitSystem};
use crate::conversion;
use crate::i18n::{self, Translator};
use crate::quantity::QuantityKind;
use crate::steam::{
    self, steam_piping::PipeSizingByVelocityInput, steam_piping::PressureLossInput,
};
use crate::units::{self, PressureUnit, TemperatureUnit};

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
pub fn main_menu(tr: &Translator) -> Result<MenuChoice, AppError> {
    println!("{}", tr.t(i18n::keys::MAIN_MENU_TITLE));
    println!("{}", tr.t(i18n::keys::MAIN_MENU_UNIT_CONVERSION));
    println!("{}", tr.t(i18n::keys::MAIN_MENU_STEAM_TABLES));
    println!("{}", tr.t(i18n::keys::MAIN_MENU_STEAM_PIPING));
    println!("{}", tr.t(i18n::keys::MAIN_MENU_STEAM_VALVES));
    println!("{}", tr.t(i18n::keys::MAIN_MENU_SETTINGS));
    println!("{}", tr.t(i18n::keys::MAIN_MENU_EXIT));
    loop {
        let sel = read_line(tr.t(i18n::keys::PROMPT_MENU_SELECT))?;
        match sel.trim() {
            "1" => return Ok(MenuChoice::UnitConversion),
            "2" => return Ok(MenuChoice::SteamTables),
            "3" => return Ok(MenuChoice::SteamPiping),
            "4" => return Ok(MenuChoice::SteamValves),
            "5" => return Ok(MenuChoice::Settings),
            "0" => return Ok(MenuChoice::Exit),
            _ => println!("{}", tr.t(i18n::keys::INVALID_SELECTION_RETRY)),
        }
    }
}

/// 단위 변환 메뉴를 처리한다.
pub fn handle_unit_conversion(tr: &Translator, _cfg: &Config) -> Result<(), AppError> {
    println!("{}", tr.t(i18n::keys::UNIT_CONVERSION_HEADING));
    println!("{}", tr.t(i18n::keys::UNIT_CONVERSION_OPTIONS_LINE1));
    println!("{}", tr.t(i18n::keys::UNIT_CONVERSION_OPTIONS_LINE2));
    println!("{}", tr.t(i18n::keys::UNIT_CONVERSION_NOTE_MMHG));
    println!("{}", tr.t(i18n::keys::HELP_UNIT_CONVERSION));
    let kind = loop {
        let sel = read_line(tr.t(i18n::keys::UNIT_CONVERSION_PROMPT_KIND))?;
        if let Ok(n) = sel.trim().parse::<u32>() {
            if let Some(kind) = map_quantity(n) {
                break kind;
            }
        }
        println!("{}", tr.t(i18n::keys::UNIT_CONVERSION_UNSUPPORTED));
    };
    let value = read_f64(tr.t(i18n::keys::UNIT_CONVERSION_PROMPT_VALUE), tr)?;
    let from_unit = read_line(tr.t(i18n::keys::UNIT_CONVERSION_PROMPT_FROM_UNIT))?;
    let to_unit = read_line(tr.t(i18n::keys::UNIT_CONVERSION_PROMPT_TO_UNIT))?;
    let result = conversion::convert(kind, value, from_unit.trim(), to_unit.trim())?;
    println!(
        "{} {} {}",
        tr.t(i18n::keys::UNIT_CONVERSION_RESULT),
        result,
        to_unit.trim()
    );
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
pub fn handle_steam_tables(tr: &Translator, _cfg: &Config) -> Result<(), AppError> {
    println!("{}", tr.t(i18n::keys::STEAM_TABLES_HEADING));
    println!("{}", tr.t(i18n::keys::STEAM_TABLES_NOTE));
    println!("{}", tr.t(i18n::keys::STEAM_TABLES_OPTIONS));
    println!("{}", tr.t(i18n::keys::HELP_STEAM_TABLES));
    let choice = read_line(tr.t(i18n::keys::PROMPT_SELECT))?;
    match choice.trim() {
        "1" => {
            let p = read_f64(tr.t(i18n::keys::PROMPT_PRESSURE_VALUE), tr)?;
            let unit = read_pressure_unit(tr)?;
            let state = steam::saturation_by_pressure(p, unit)?;
            print_state(&state, tr);
        }
        "2" => {
            let t = read_f64(tr.t(i18n::keys::PROMPT_TEMPERATURE_VALUE), tr)?;
            let unit = read_temperature_unit(tr)?;
            let state = steam::saturation_by_temperature(t, unit)?;
            print_state(&state, tr);
        }
        "3" => {
            let p = read_f64(tr.t(i18n::keys::PROMPT_PRESSURE_VALUE), tr)?;
            let p_unit = read_pressure_unit(tr)?;
            let t = read_f64(tr.t(i18n::keys::PROMPT_TEMPERATURE_VALUE), tr)?;
            let t_unit = read_temperature_unit(tr)?;
            let state = steam::superheated_at(p, p_unit, t, t_unit)?;
            print_state(&state, tr);
        }
        _ => println!("{}", tr.t(i18n::keys::INVALID_SELECTION_RETRY)),
    }
    Ok(())
}

/// Steam Piping 메뉴를 처리한다.
pub fn handle_steam_piping(tr: &Translator, _cfg: &Config) -> Result<(), AppError> {
    println!("{}", tr.t(i18n::keys::STEAM_PIPING_HEADING));
    println!("{}", tr.t(i18n::keys::STEAM_PIPING_OPTION_SIZING));
    println!("{}", tr.t(i18n::keys::STEAM_PIPING_OPTION_PRESSURE_DROP));
    let sel = read_line(tr.t(i18n::keys::PROMPT_SELECT))?;
    match sel.trim() {
        "1" => {
            println!("{}", tr.t(i18n::keys::HELP_STEAM_PIPING_SIZING));
            let mflow = read_f64(tr.t(i18n::keys::PROMPT_MASS_FLOW), tr)?;
            let pressure = read_f64(tr.t(i18n::keys::PROMPT_OPERATING_PRESSURE), tr)?;
            let p_unit = read_pressure_unit(tr)?;
            let temp = read_f64(tr.t(i18n::keys::PROMPT_OPERATING_TEMPERATURE), tr)?;
            let t_unit = read_temperature_unit(tr)?;
            let density = steam::estimate_density(pressure, p_unit, temp, t_unit);
            let target_v = read_f64(tr.t(i18n::keys::PROMPT_TARGET_VELOCITY), tr)?;
            let input = PipeSizingByVelocityInput {
                mass_flow_kg_per_h: mflow,
                steam_density_kg_per_m3: density,
                target_velocity_m_per_s: target_v,
            };
            let result = steam::size_by_velocity(input)?;
            println!(
                "{} {:.1} mm ({:.3} in)",
                tr.t(i18n::keys::RESULT_RECOMMENDED_ID),
                result.inner_diameter_m * 1000.0,
                result.inner_diameter_m / 0.0254
            );
            println!(
                "{} {:.2} m/s, Re={:.2e}",
                tr.t(i18n::keys::RESULT_EXPECTED_VELOCITY),
                result.velocity_m_per_s,
                result.reynolds_number
            );
        }
        "2" => {
            println!("{}", tr.t(i18n::keys::HELP_STEAM_PIPING_DROP));
            let mflow = read_f64(tr.t(i18n::keys::PROMPT_MASS_FLOW), tr)?;
            let p = read_f64(tr.t(i18n::keys::PROMPT_OPERATING_PRESSURE_MODE), tr)?;
            let p_unit = read_pressure_unit(tr)?;
            let t = read_f64(tr.t(i18n::keys::PROMPT_OPERATING_TEMPERATURE), tr)?;
            let t_unit = read_temperature_unit(tr)?;
            let state_p_bar_abs =
                units::convert_pressure(p, p_unit, units::PressureUnit::BarA).max(0.0);
            let state_t_c = units::convert_temperature(t, t_unit, units::TemperatureUnit::Celsius);

            let density_input = read_f64(tr.t(i18n::keys::PROMPT_DENSITY_OPTIONAL), tr)?;
            let density = if density_input <= 0.0 {
                steam::if97::region_props(state_p_bar_abs, state_t_c)
                    .ok()
                    .map(|(_, v, _)| 1.0 / v.max(1e-9))
                    .unwrap_or_else(|| steam::estimate_density(p, p_unit, t, t_unit))
            } else {
                density_input
            };
            let diameter = read_diameter_m(tr.t(i18n::keys::PROMPT_DIAMETER), tr)?;
            let length = read_f64(tr.t(i18n::keys::PROMPT_LENGTH), tr)?;
            let eq_len = read_f64(tr.t(i18n::keys::PROMPT_EQ_LENGTH), tr)?;
            let k_sum = read_f64(tr.t(i18n::keys::PROMPT_FITTINGS_K), tr)?;
            let roughness = read_f64(tr.t(i18n::keys::PROMPT_ROUGHNESS), tr)?;
            let visc = read_f64(tr.t(i18n::keys::PROMPT_VISCOSITY), tr)?;
            let sound_speed = read_f64(tr.t(i18n::keys::PROMPT_SOUND_SPEED), tr)?;
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
                state_pressure_bar_abs: Some(state_p_bar_abs),
                state_temperature_c: Some(state_t_c),
            };
            let result = steam::pressure_loss(input)?;
            println!(
                "{} {:.2} m/s, ΔP: {:.4} bar, f={:.4}, Re={:.2e}, Mach={:.3}",
                tr.t(i18n::keys::RESULT_PRESSURE_DROP),
                result.velocity_m_per_s,
                result.pressure_drop_bar,
                result.friction_factor,
                result.reynolds_number,
                result.mach
            );
        }
        _ => println!("{}", tr.t(i18n::keys::INVALID_SELECTION_RETRY)),
    }
    Ok(())
}

/// Steam Valves 메뉴를 처리한다.
pub fn handle_steam_valves(tr: &Translator, _cfg: &Config) -> Result<(), AppError> {
    println!("{}", tr.t(i18n::keys::STEAM_VALVES_HEADING));
    println!("{}", tr.t(i18n::keys::STEAM_VALVES_OPTION_REQUIRED));
    println!("{}", tr.t(i18n::keys::STEAM_VALVES_OPTION_FLOW));
    let sel = read_line(tr.t(i18n::keys::PROMPT_SELECT))?;
    match sel.trim() {
        "1" => {
            println!("{}", tr.t(i18n::keys::HELP_STEAM_VALVES_REQUIRED));
            let flow = read_f64(tr.t(i18n::keys::PROMPT_VOLUMETRIC_FLOW), tr)?;
            let dp = read_f64(tr.t(i18n::keys::PROMPT_DELTA_P), tr)?;
            let rho = read_f64(tr.t(i18n::keys::PROMPT_DENSITY_GENERIC), tr)?;
            let kv = steam::required_kv(flow, dp, rho)?;
            let cv = steam::cv_from_kv(kv);
            println!(
                "{} Kv={:.3}, Cv={:.3}",
                tr.t(i18n::keys::RESULT_REQUIRED_KV_CV),
                kv,
                cv
            );
        }
        "2" => {
            println!("{}", tr.t(i18n::keys::HELP_STEAM_VALVES_FLOW));
            let mode = read_line(tr.t(i18n::keys::PROMPT_INPUT_MODE_KV_CV))?;
            let value = read_f64(tr.t(i18n::keys::PROMPT_KV_CV_VALUE), tr)?;
            let dp = read_f64(tr.t(i18n::keys::PROMPT_DELTA_P), tr)?;
            let rho = read_f64(tr.t(i18n::keys::PROMPT_DENSITY_GENERIC), tr)?;
            let p_up = read_f64(tr.t(i18n::keys::PROMPT_UPSTREAM_PRESSURE), tr)?;
            let flow = if mode.trim() == "2" {
                steam::flow_from_cv(value, dp, rho)?
            } else {
                steam::flow_from_kv(value, dp, rho, Some(p_up))?
            };
            println!(
                "{} {:.3} m3/h ({:.3} kg/h)",
                tr.t(i18n::keys::RESULT_POSSIBLE_FLOW),
                flow,
                flow * rho
            );
        }
        _ => println!("{}", tr.t(i18n::keys::INVALID_SELECTION_RETRY)),
    }
    Ok(())
}

/// 설정 메뉴를 처리한다.
pub fn handle_settings(tr: &Translator, cfg: &mut Config) -> Result<(), AppError> {
    println!("{}", tr.t(i18n::keys::SETTINGS_HEADING));
    println!(
        "{} {:?}",
        tr.t(i18n::keys::SETTINGS_CURRENT_UNIT_SYSTEM),
        cfg.unit_system
    );
    println!("{}", tr.t(i18n::keys::SETTINGS_OPTIONS));
    println!("{}", tr.t(i18n::keys::HELP_SETTINGS));
    let sel = read_line(tr.t(i18n::keys::SETTINGS_PROMPT_CHANGE))?;
    if sel.trim().is_empty() {
        return Ok(());
    }
    cfg.unit_system = match sel.trim() {
        "1" => UnitSystem::SIBar,
        "2" => UnitSystem::SI,
        "3" => UnitSystem::MKS,
        "4" => UnitSystem::Imperial,
        _ => {
            println!("{}", tr.t(i18n::keys::SETTINGS_INVALID));
            cfg.unit_system
        }
    };
    println!("{} {:?}", tr.t(i18n::keys::SETTINGS_SAVED), cfg.unit_system);
    Ok(())
}

fn read_line(prompt: &str) -> Result<String, AppError> {
    print!("{prompt}");
    io::stdout().flush().map_err(AppError::Io)?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).map_err(AppError::Io)?;
    Ok(buf)
}

fn read_f64(prompt: &str, tr: &Translator) -> Result<f64, AppError> {
    loop {
        let s = read_line(prompt)?;
        match s.trim().parse::<f64>() {
            Ok(v) => return Ok(v),
            Err(_) => println!("{}", tr.t(i18n::keys::ERROR_INVALID_NUMBER)),
        }
    }
}

fn read_diameter_m(prompt: &str, tr: &Translator) -> Result<f64, AppError> {
    loop {
        let raw = read_line(prompt)?;
        if let Some(m) = parse_diameter_to_m(&raw) {
            if m > 0.0 {
                return Ok(m);
            }
        }
        println!("{}", tr.t(i18n::keys::ERROR_INVALID_NUMBER));
    }
}

fn parse_diameter_to_m(input: &str) -> Option<f64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    let (num_part, unit) = if lower.ends_with("mm") {
        (&trimmed[..trimmed.len().saturating_sub(2)], "mm")
    } else if lower.ends_with("in") {
        (&trimmed[..trimmed.len().saturating_sub(2)], "in")
    } else if trimmed.ends_with('"') {
        (&trimmed[..trimmed.len().saturating_sub(1)], "in")
    } else if lower.ends_with('m') {
        (&trimmed[..trimmed.len().saturating_sub(1)], "m")
    } else {
        (trimmed, "mm") // 기본: mm
    };
    let value: f64 = num_part.trim().parse().ok()?;
    match unit {
        "mm" => Some(value / 1000.0),
        "in" => Some(value * 0.0254),
        "m" => Some(value),
        _ => None,
    }
}

fn read_pressure_unit(tr: &Translator) -> Result<PressureUnit, AppError> {
    println!("{}", tr.t(i18n::keys::PRESSURE_UNIT_OPTIONS));
    let sel = read_line(tr.t(i18n::keys::PROMPT_SELECT))?;
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

fn read_temperature_unit(tr: &Translator) -> Result<TemperatureUnit, AppError> {
    println!("{}", tr.t(i18n::keys::TEMPERATURE_UNIT_OPTIONS));
    let sel = read_line(tr.t(i18n::keys::PROMPT_SELECT))?;
    let unit = match sel.trim() {
        "1" => TemperatureUnit::Celsius,
        "2" => TemperatureUnit::Kelvin,
        "3" => TemperatureUnit::Fahrenheit,
        "4" => TemperatureUnit::Rankine,
        _ => TemperatureUnit::Celsius,
    };
    Ok(unit)
}

fn print_state(state: &steam::SteamState, tr: &Translator) {
    println!(
        "{} {:.2} °C",
        tr.t(i18n::keys::STATE_SATURATION_T),
        state.saturation_temperature_c
    );
    println!(
        "{} {:.3} bar",
        tr.t(i18n::keys::STATE_SATURATION_P),
        state.pressure_bar
    );
    println!(
        "{} {:.1} kJ/kg, {:.3} m3/kg",
        tr.t(i18n::keys::STATE_ENTHALPY_VOLUME),
        state.saturation_enthalpy_kj_per_kg,
        state.saturation_specific_volume
    );
    if let Some(h) = state.superheated_enthalpy_kj_per_kg {
        println!(
            "{} {:.1} kJ/kg",
            tr.t(i18n::keys::STATE_SUPERHEATED_ENTHALPY),
            h
        );
    }
}
