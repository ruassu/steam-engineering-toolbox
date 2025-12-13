use crate::config::Config;
use crate::conversion;
use crate::i18n::{self, Translator};
use crate::steam::{steam_piping, steam_tables, steam_valves};
use crate::ui_cli;
use crate::ui_cli::MenuChoice;

/// 애플리케이션 실행 중 발생 가능한 오류를 표현한다.
#[derive(Debug)]
pub enum AppError {
    /// 파일 입출력 오류
    Io(std::io::Error),
    /// 설정 저장/로드 오류
    Config(crate::config::ConfigError),
    /// 단위 변환 오류
    Conversion(conversion::ConversionError),
    /// 증기표 계산 오류
    SteamTable(steam_tables::SteamTableError),
    /// 배관 계산 오류
    Pipe(steam_piping::PipeCalcError),
    /// 밸브/오리피스 계산 오류
    Valve(steam_valves::ValveCalcError),
    /// 아직 구현되지 않은 기능 호출
    Unimplemented(&'static str),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "입출력 오류: {e}"),
            AppError::Config(e) => write!(f, "설정 오류: {e}"),
            AppError::Conversion(e) => write!(f, "단위 변환 오류: {e}"),
            AppError::SteamTable(e) => write!(f, "증기표 계산 오류: {e}"),
            AppError::Pipe(e) => write!(f, "배관 계산 오류: {e}"),
            AppError::Valve(e) => write!(f, "밸브 계산 오류: {e}"),
            AppError::Unimplemented(msg) => write!(f, "아직 구현되지 않음: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Io(value)
    }
}

impl From<crate::config::ConfigError> for AppError {
    fn from(value: crate::config::ConfigError) -> Self {
        AppError::Config(value)
    }
}

impl From<conversion::ConversionError> for AppError {
    fn from(value: conversion::ConversionError) -> Self {
        AppError::Conversion(value)
    }
}

impl From<steam_tables::SteamTableError> for AppError {
    fn from(value: steam_tables::SteamTableError) -> Self {
        AppError::SteamTable(value)
    }
}

impl From<steam_piping::PipeCalcError> for AppError {
    fn from(value: steam_piping::PipeCalcError) -> Self {
        AppError::Pipe(value)
    }
}

impl From<steam_valves::ValveCalcError> for AppError {
    fn from(value: steam_valves::ValveCalcError) -> Self {
        AppError::Valve(value)
    }
}

/// CLI 애플리케이션의 메인 루프를 실행한다.
pub fn run(config: &mut Config, tr: &Translator) -> Result<(), AppError> {
    loop {
        match ui_cli::main_menu(tr)? {
            MenuChoice::UnitConversion => ui_cli::handle_unit_conversion(tr, config)?,
            MenuChoice::SteamTables => ui_cli::handle_steam_tables(tr, config)?,
            MenuChoice::SteamPiping => ui_cli::handle_steam_piping(tr, config)?,
            MenuChoice::SteamValves => ui_cli::handle_steam_valves(tr, config)?,
            MenuChoice::Settings => {
                ui_cli::handle_settings(tr, config)?;
                config.save()?;
            }
            MenuChoice::Exit => {
                config.save()?;
                println!("{}", tr.t(i18n::keys::APP_EXIT));
                break;
            }
        }
    }
    Ok(())
}
