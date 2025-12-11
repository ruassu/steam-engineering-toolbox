#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! eframe/egui 기반 데스크톱 GUI 진입점.

use eframe::{egui, App, Frame};
use std::{fs, path::Path};
use steam_engineering_toolbox::{
    config, conversion,
    cooling::{condenser, cooling_tower, drain_cooler, pump_npsh},
    quantity::QuantityKind,
    steam,
    steam::steam_piping::PipeSizingByVelocityInput,
    steam::steam_valves,
    units::{PressureUnit, TemperatureUnit},
};

fn main() -> Result<(), eframe::Error> {
    let cfg = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_always_on_top(),
        ..Default::default()
    };
    let app_cfg = config::load_or_default().unwrap_or_default();
    eframe::run_native(
        "Steam Engineering Toolbox",
        cfg,
        Box::new(move |cc| {
            if let Err(e) = setup_fonts(&cc.egui_ctx) {
                eprintln!("폰트 설정 실패: {e}");
            }
            Box::new(GuiApp::new(app_cfg.clone()))
        }),
    )
}

fn stroke_based_kv_available(strokes: &[f64], cvs: &[f64]) -> bool {
    let mut count = 0;
    for i in 0..strokes.len().min(cvs.len()) {
        if cvs[i] > 0.0 {
            count += 1;
        }
    }
    count >= 2
}

fn interpolate_stroke_cv(strokes: &[f64], cvs: &[f64], target: f64) -> f64 {
    let mut best_cv = 0.0;
    let mut prev_s = None;
    let mut prev_cv = None;
    let t = target.clamp(0.0, 100.0);
    let mut pairs: Vec<(f64, f64)> = strokes
        .iter()
        .zip(cvs.iter())
        .filter(|(_, cv)| **cv > 0.0)
        .map(|(s, cv)| (*s, *cv))
        .collect();
    if pairs.is_empty() {
        return 0.0;
    }
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (s, cv) in pairs {
        if let (Some(ps), Some(pcv)) = (prev_s, prev_cv) {
            if t >= ps && t <= s {
                let ratio = if (s - ps).abs() < f64::EPSILON {
                    0.0
                } else {
                    (t - ps) / (s - ps)
                };
                return pcv + (cv - pcv) * ratio;
            }
        }
        prev_s = Some(s);
        prev_cv = Some(cv);
        best_cv = cv;
    }
    best_cv
}

fn label_with_tip(ui: &mut egui::Ui, text: &str, tip: &str) -> egui::Response {
    ui.label(text).on_hover_text(tip)
}

fn heading_with_tip(ui: &mut egui::Ui, text: &str, tip: &str) -> egui::Response {
    ui.heading(text).on_hover_text(tip)
}

struct GuiApp {
    config: config::Config,
    tab: Tab,
    // 단위 변환
    conv_value: f64,
    conv_from: String,
    conv_to: String,
    conv_kind: QuantityKind,
    conv_result: Option<String>,
    // 증기표
    steam_value: f64,
    steam_mode: SteamMode,
    steam_p_unit: String,
    steam_t_unit: String,
    steam_p_mode: conversion::PressureMode,
    steam_p_unit_out: String,
    steam_p_mode_out: conversion::PressureMode,
    steam_t_unit_out: String,
    steam_temp_input: f64,
    steam_result: Option<String>,
    show_vacuum_table_window: bool,
    show_vacuum_table_viewport: bool,
    apply_initial_view_size: bool,
    // 배관
    pipe_mass_flow: f64,
    pipe_mass_unit: String,
    pipe_pressure: f64,
    pipe_pressure_unit: String,
    pipe_pressure_mode: conversion::PressureMode,
    pipe_temp: f64,
    pipe_temp_unit: String,
    pipe_velocity: f64,
    pipe_velocity_unit: String,
    pipe_diam_out_unit: String,
    pipe_vel_out_unit: String,
    pipe_result: Option<String>,
    pipe_loss_density: f64,
    pipe_loss_diameter: f64,
    pipe_loss_length: f64,
    pipe_loss_eq_length: f64,
    pipe_loss_fittings_k: f64,
    pipe_loss_roughness: f64,
    pipe_loss_visc: f64,
    pipe_loss_sound_speed: f64,
    pipe_loss_dp_out_unit: String,
    pipe_loss_dp_out_mode: conversion::PressureMode,
    pipe_loss_result: Option<String>,
    // 밸브
    valve_mode: ValveMode,
    valve_flow: f64,
    valve_flow_unit: String,
    valve_upstream_p: f64,
    valve_upstream_unit: String,
    valve_upstream_mode: conversion::PressureMode,
    valve_dp: f64,
    valve_dp_unit: String,
    valve_dp_mode: conversion::PressureMode,
    valve_rho: f64,
    valve_rho_unit: String,
    valve_cv_kv: f64,
    valve_result: Option<String>,
    // ST Bypass Valve
    bypass_up_p: f64,
    bypass_up_unit: String,
    bypass_up_mode: conversion::PressureMode,
    bypass_up_t: f64,
    bypass_t_unit: String,
    bypass_down_p: f64,
    bypass_down_unit: String,
    bypass_down_mode: conversion::PressureMode,
    bypass_cv: f64,
    bypass_cv_kind: String,
    bypass_open_pct: f64,
    bypass_h_override_kj_per_kg: f64,
    bypass_spray_kg_h: f64,
    bypass_spray_temp: f64,
    bypass_spray_temp_unit: String,
    bypass_stroke_points: Vec<f64>,
    bypass_cv_points: Vec<f64>,
    bypass_result: Option<String>,
    // Spray water valve (optional, can feed bypass)
    spray_up_p: f64,
    spray_up_unit: String,
    spray_up_mode: conversion::PressureMode,
    spray_down_p: f64,
    spray_down_unit: String,
    spray_down_mode: conversion::PressureMode,
    spray_temp: f64,
    spray_temp_unit: String,
    spray_density: f64,
    spray_cv: f64,
    spray_cv_kind: String,
    spray_open_pct: f64,
    spray_h_override_kj_per_kg: f64,
    spray_stroke_points: Vec<f64>,
    spray_cv_points: Vec<f64>,
    spray_calc_result: Option<String>,
    // 플랜트 배관/오리피스/열팽창
    plant_dp: f64,
    plant_dp_unit: String,
    plant_dp_mode: conversion::PressureMode,
    plant_up_p: f64,
    plant_up_unit: String,
    plant_up_mode: conversion::PressureMode,
    plant_rho: f64,
    plant_cd: f64,
    plant_shape: String,
    plant_diam_unit: String,
    plant_diameter_m: f64,
    plant_beta: f64,
    plant_gamma: f64,
    plant_compressible: bool,
    plant_result: Option<String>,
    plant_mat: String,
    plant_length_m: f64,
    plant_delta_t: f64,
    plant_alpha_override: f64,
    plant_expansion_result: Option<String>,
    plant_pipe_od_m: f64,
    plant_wall_thk_m: f64,
    plant_dim_unit: String,
    plant_service_temp_c: f64,
    plant_allow_stress_mpa: f64,
    plant_corrosion_allow_m: f64,
    plant_weld_eff: f64,
    plant_design_factor: f64,
    plant_mill_tol_frac: f64,
    plant_safety_factor: f64,
    plant_pressure_result: Option<String>,
    // 보일러
    boiler_fuel_flow: f64,
    boiler_fuel_unit: String,
    boiler_lhv: f64,
    boiler_lhv_unit: String,
    boiler_steam_flow: f64,
    boiler_steam_unit: String,
    boiler_h_steam: f64,
    boiler_h_steam_unit: String,
    boiler_h_fw: f64,
    boiler_h_fw_unit: String,
    boiler_fg_flow: f64,
    boiler_fg_flow_unit: String,
    boiler_fg_cp: f64,
    boiler_stack_temp: f64,
    boiler_ambient_temp: f64,
    boiler_excess_air: f64,
    boiler_rad_loss: f64,
    boiler_blowdown_rate: f64,
    boiler_blowdown_h: f64,
    boiler_blowdown_h_unit: String,
    boiler_temp_unit: String,
    boiler_result: Option<String>,
    // 냉각/복수/열교환/펌프
    condenser_pressure: f64,
    condenser_pressure_unit: String,
    condenser_pressure_mode: conversion::PressureMode,
    condenser_temp_c: f64,
    condenser_use_manual_temp: bool,
    condenser_cw_in: f64,
    condenser_cw_out: f64,
    condenser_cw_temp_unit: String,
    condenser_cw_flow: f64,
    condenser_cw_flow_unit: String,
    condenser_ua: f64,
    condenser_area: f64,
    condenser_u: f64,
    condenser_backpressure: f64,
    condenser_backpressure_unit: String,
    condenser_backpressure_mode: conversion::PressureMode,
    condenser_result: Option<String>,
    condenser_auto_condensing_from_pressure: bool,
    condenser_auto_backpressure_from_temp: bool,
    condenser_auto_cw_out_from_range: bool,
    condenser_auto_ua_from_area_u: bool,
    condenser_auto_area_required: bool,

    ct_in: f64,
    ct_out: f64,
    ct_wb: f64,
    ct_db: f64,
    ct_temp_unit: String,
    ct_flow: f64,
    ct_flow_unit: String,
    ct_range_target: f64,
    ct_approach_target: f64,
    ct_result: Option<String>,

    npsh_suction_p: f64,
    npsh_suction_unit: String,
    npsh_suction_mode: conversion::PressureMode,
    npsh_temp: f64,
    npsh_temp_unit: String,
    npsh_static_head: f64,
    npsh_friction: f64,
    npsh_rho: f64,
    npsh_rho_unit: String,
    npsh_required: f64,
    npsh_result: Option<String>,

    drain_shell_in: f64,
    drain_shell_out: f64,
    drain_shell_flow: f64,
    drain_tube_in: f64,
    drain_tube_out: f64,
    drain_tube_flow: f64,
    drain_temp_unit: String,
    drain_flow_unit: String,
    drain_ua: f64,
    drain_area: f64,
    drain_u: f64,
    drain_result: Option<String>,
    // 설정
    font_size: f32,
    ui_scale: f32,
    always_on_top: bool,
    show_settings_modal: bool,
    show_help_modal: bool,
    theme: ThemeChoice,
    custom_font_path: String,
    font_load_error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    UnitConv,
    SteamTables,
    SteamPiping,
    SteamValves,
    Boiler,
    Cooling,
    PlantPiping,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ThemeChoice {
    System,
    Light,
    Dark,
    SoftBlue,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SteamMode {
    ByPressure,
    ByTemperature,
    Superheated,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValveMode {
    RequiredCvKv,
    FlowFromCvKv,
}

fn kv_from_cv_with_kind(cv: f64, kind: &str) -> f64 {
    // 기본 Cv(US) → Kv 환산: 0.865
    match kind.to_lowercase().as_str() {
        "kv" => cv,
        "cv(uk)" => cv * 0.865, // UK Cv를 US와 동일하게 가정
        "cv(us)" | _ => cv * 0.865,
    }
}

/// 공통: 바이너리 폰트 바이트를 egui에 등록.
fn apply_font_bytes(ctx: &egui::Context, bytes: Vec<u8>, name: &str) {
    let mut fonts = egui::FontDefinitions::default();
    let font_name = name.to_string();
    fonts
        .font_data
        .insert(font_name.clone(), egui::FontData::from_owned(bytes));
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, font_name.clone());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, font_name);
    ctx.set_fonts(fonts);
}

/// 한글을 표시하기 위해 기본 폰트를 우선 적용한다.
/// 0) (가능하면) 바이너리에 내장된 malgun.ttf
/// 1) assets/fonts/malgun.ttf
/// 2) Windows 시스템 폰트(맑은 고딕/굴림/바탕 등)
/// 3) 모두 실패 시 Err를 반환해 사용자 지정 폰트 로드를 유도한다.
#[allow(unreachable_code)]
fn setup_fonts(ctx: &egui::Context) -> Result<(), String> {
    // 0) 내장 폰트 시도 (빌드 시 assets/fonts/malgun.ttf 존재 필요)
    const EMBED_MALGUN: &[u8] = include_bytes!("../../assets/fonts/malgun.ttf");
    apply_font_bytes(ctx, EMBED_MALGUN.to_vec(), "embedded_malgun");
    return Ok(());

    // 1) 프로젝트 내 폰트
    let asset_path = Path::new("assets/fonts/malgun.ttf");
    if asset_path.exists() {
        let bytes = fs::read(asset_path).map_err(|e| format!("폰트 파일 읽기 실패: {e}"))?;
        apply_font_bytes(ctx, bytes, "korean_font");
        return Ok(());
    }

    // 2) 시스템 폰트 탐색 (Windows 기준)
    if let Some(windir) = std::env::var_os("WINDIR") {
        let fonts = Path::new(&windir).join("Fonts");
        let candidates = [
            "malgun.ttf",
            "malgunsl.ttf",
            "malgunbd.ttf",
            "gulim.ttc",
            "batang.ttc",
            "gungsuh.ttc",
        ];
        for cand in candidates {
            let p = fonts.join(cand);
            if p.exists() {
                let bytes = fs::read(&p)
                    .map_err(|e| format!("시스템 폰트 읽기 실패({}): {e}", p.display()))?;
                apply_font_bytes(ctx, bytes, "korean_font");
                return Ok(());
            }
        }
    }

    // 3) 실패: 기본 폰트 유지, 사용자 지정 안내
    Err("폰트를 찾지 못했습니다. 설정에서 사용자 폰트(.ttf/.ttc)를 지정해주세요.".into())
}

/// 사용자가 선택한 경로의 폰트를 egui에 등록한다.
fn load_custom_font(ctx: &egui::Context, path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("폰트 파일을 찾을 수 없습니다: {path}"));
    }
    let bytes = fs::read(p).map_err(|e| format!("폰트 파일 읽기 실패: {e}"))?;
    apply_font_bytes(ctx, bytes, "user_font");
    Ok(())
}

fn vacuum_table_ui(ui: &mut egui::Ui) {
    ui.small("압력을 mmHg(g)로 놓고, IF97 포화온도 계산 결과를 표로 표시합니다.");
    egui::Grid::new(ui.next_auto_id())
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.strong("mmHg(g)");
            ui.strong("P(bar a)");
            ui.strong("Tsat(°C)");
            ui.end_row();
            let rows = [
                0.0, -100.0, -200.0, -300.0, -400.0, // 100단계
                -420.0, -440.0, -460.0, -480.0, -500.0, -520.0, -540.0, -560.0, -580.0, -600.0, // 20단계
                -610.0, -620.0, -630.0, -640.0, -650.0, -660.0, -670.0, -680.0, // 10단계
                -685.0, -690.0, -695.0, -700.0, -705.0, -710.0, -715.0, -720.0, -725.0, -730.0, -735.0, -740.0, // 5단계
                -760.0, // -740~-760은 20 단위(끝값만 표시)
            ];
            for mmhg_g in rows {
                let p_abs_bar = ((760.0 + mmhg_g) / 760.0) * 1.01325;
                let t_res = if p_abs_bar > 0.0 {
                    steam::if97::saturation_temp_c_from_pressure_bar_abs(p_abs_bar).ok()
                } else {
                    None
                };
                ui.label(format!("{mmhg_g:.0}"));
                ui.label(format!("{p_abs_bar:.4}"));
                if let Some(t) = t_res {
                    ui.label(format!("{t:.2}"));
                } else {
                    ui.label("N/A");
                }
                ui.end_row();
            }
        });
}

impl GuiApp {
    fn new(config: config::Config) -> Self {
        let (conv_from, conv_to) = default_units_for_kind(QuantityKind::Temperature);
        let mut s = Self {
            config,
            tab: Tab::UnitConv,
            conv_value: 100.0,
            conv_from: conv_from.into(),
            conv_to: conv_to.into(),
            conv_kind: QuantityKind::Temperature,
            conv_result: None,
            steam_value: 1.0,
            steam_mode: SteamMode::ByPressure,
            steam_p_unit: "bar".into(),
            steam_t_unit: "C".into(),
            steam_p_mode: conversion::PressureMode::Gauge,
            steam_p_unit_out: "bar".into(),
            steam_p_mode_out: conversion::PressureMode::Absolute,
            steam_t_unit_out: "C".into(),
            steam_temp_input: 200.0,
            steam_result: None,
            show_vacuum_table_window: false,
            show_vacuum_table_viewport: false,
            apply_initial_view_size: true,
            pipe_mass_flow: 500.0,
            pipe_mass_unit: "kg/h".into(),
            pipe_pressure: 5.0,
            pipe_pressure_unit: "bar".into(),
            pipe_pressure_mode: conversion::PressureMode::Gauge,
            pipe_temp: 180.0,
            pipe_temp_unit: "C".into(),
            pipe_velocity: 25.0,
            pipe_velocity_unit: "m/s".into(),
            pipe_diam_out_unit: "m".into(),
            pipe_vel_out_unit: "m/s".into(),
            pipe_result: None,
            pipe_loss_density: 2.5,
            pipe_loss_diameter: 0.1,
            pipe_loss_length: 50.0,
            pipe_loss_eq_length: 0.0,
            pipe_loss_fittings_k: 0.0,
            pipe_loss_roughness: 0.000045,
            pipe_loss_visc: 1.2e-5,
            pipe_loss_sound_speed: 450.0,
            pipe_loss_dp_out_unit: "bar".into(),
            pipe_loss_dp_out_mode: conversion::PressureMode::Absolute,
            pipe_loss_result: None,
            valve_mode: ValveMode::RequiredCvKv,
            valve_flow: 10.0,
            valve_flow_unit: "m3/h".into(),
            valve_upstream_p: 5.0,
            valve_upstream_unit: "bar".into(),
            valve_upstream_mode: conversion::PressureMode::Gauge,
            valve_dp: 1.0,
            valve_dp_unit: "bar".into(),
            valve_dp_mode: conversion::PressureMode::Gauge,
            valve_rho: 1.2,
            valve_rho_unit: "kg/m3".into(),
            valve_cv_kv: 10.0,
            valve_result: None,
            bypass_up_p: 60.0,
            bypass_up_unit: "bar".into(),
            bypass_up_mode: conversion::PressureMode::Gauge,
            bypass_up_t: 520.0,
            bypass_t_unit: "C".into(),
            bypass_down_p: 10.0,
            bypass_down_unit: "bar".into(),
            bypass_down_mode: conversion::PressureMode::Gauge,
            bypass_cv: 200.0,
            bypass_cv_kind: "Cv(US)".into(),
            bypass_open_pct: 50.0,
            bypass_h_override_kj_per_kg: 0.0,
            bypass_spray_kg_h: 0.0,
            bypass_spray_temp: 40.0,
            bypass_spray_temp_unit: "C".into(),
            bypass_stroke_points: vec![0.0, 25.0, 50.0, 75.0, 100.0],
            bypass_cv_points: vec![0.0, 0.0, 0.0, 0.0, 0.0],
            bypass_result: None,
            spray_up_p: 15.0,
            spray_up_unit: "bar".into(),
            spray_up_mode: conversion::PressureMode::Gauge,
            spray_down_p: 10.0,
            spray_down_unit: "bar".into(),
            spray_down_mode: conversion::PressureMode::Gauge,
            spray_temp: 30.0,
            spray_temp_unit: "C".into(),
            spray_density: 1000.0,
            spray_cv: 20.0,
            spray_cv_kind: "Cv(US)".into(),
            spray_open_pct: 100.0,
            spray_h_override_kj_per_kg: 0.0,
            spray_stroke_points: vec![0.0, 25.0, 50.0, 75.0, 100.0],
            spray_cv_points: vec![0.0, 0.0, 0.0, 0.0, 0.0],
            spray_calc_result: None,
            plant_dp: 1.0,
            plant_dp_unit: "bar".into(),
            plant_dp_mode: conversion::PressureMode::Gauge,
            plant_up_p: 2.0,
            plant_up_unit: "bar".into(),
            plant_up_mode: conversion::PressureMode::Gauge,
            plant_rho: 1000.0,
            plant_cd: 0.62,
            plant_shape: "Orifice (sharp)".into(),
            plant_diam_unit: "m".into(),
            plant_diameter_m: 0.05,
            plant_beta: 0.3,
            plant_gamma: 1.3,
            plant_compressible: false,
            plant_result: None,
            plant_mat: "ASTM A106 Gr.B".into(),
            plant_length_m: 10.0,
            plant_delta_t: 50.0,
            plant_alpha_override: 0.0,
            plant_expansion_result: None,
            plant_pipe_od_m: 0.114,   // NPS 4" OD 약 114mm
            plant_wall_thk_m: 0.006,  // 6mm
            plant_dim_unit: "mm".into(),
            plant_service_temp_c: 20.0,
            plant_allow_stress_mpa: 138.0, // A106B room temp 허용응력 근사
            plant_corrosion_allow_m: 0.001, // 1 mm
            plant_weld_eff: 1.0,
            plant_design_factor: 1.0,
            plant_mill_tol_frac: 0.125, // 12.5% 밀 톨
            plant_safety_factor: 1.5,
            plant_pressure_result: None,
            boiler_fuel_flow: 100.0,
            boiler_fuel_unit: "kg/h".into(),
            boiler_lhv: 42000.0,
            boiler_lhv_unit: "kJ/kg".into(),
            boiler_steam_flow: 800.0,
            boiler_steam_unit: "kg/h".into(),
            boiler_h_steam: 2770.0,
            boiler_h_steam_unit: "kJ/kg".into(),
            boiler_h_fw: 500.0,
            boiler_h_fw_unit: "kJ/kg".into(),
            boiler_fg_flow: 1500.0,
            boiler_fg_flow_unit: "kg/h".into(),
            boiler_fg_cp: 1.05,
            boiler_stack_temp: 220.0,
            boiler_ambient_temp: 30.0,
            boiler_excess_air: 0.15,
            boiler_rad_loss: 0.02,
            boiler_blowdown_rate: 0.05,
            boiler_blowdown_h: 600.0,
            boiler_blowdown_h_unit: "kJ/kg".into(),
            boiler_temp_unit: "C".into(),
            boiler_result: None,
            condenser_pressure: 0.2,
            condenser_pressure_unit: "bar".into(),
            condenser_pressure_mode: conversion::PressureMode::Absolute,
            condenser_temp_c: 45.0,
            condenser_use_manual_temp: false,
            condenser_cw_in: 30.0,
            condenser_cw_out: 38.0,
            condenser_cw_temp_unit: "C".into(),
            condenser_cw_flow: 500.0,
            condenser_cw_flow_unit: "m3/h".into(),
            condenser_ua: 0.0,
            condenser_area: 0.0,
            condenser_u: 0.0,
            condenser_backpressure: 0.2,
            condenser_backpressure_unit: "bar".into(),
            condenser_backpressure_mode: conversion::PressureMode::Absolute,
            condenser_result: None,
            condenser_auto_condensing_from_pressure: true,
            condenser_auto_backpressure_from_temp: true,
            condenser_auto_cw_out_from_range: false,
            condenser_auto_ua_from_area_u: true,
            condenser_auto_area_required: false,
            ct_in: 40.0,
            ct_out: 32.0,
            ct_wb: 28.0,
            ct_db: 32.0,
            ct_temp_unit: "C".into(),
            ct_flow: 500.0,
            ct_flow_unit: "m3/h".into(),
            ct_range_target: 8.0,
            ct_approach_target: 4.0,
            ct_result: None,
            npsh_suction_p: 0.5,
            npsh_suction_unit: "bar".into(),
            npsh_suction_mode: conversion::PressureMode::Gauge,
            npsh_temp: 30.0,
            npsh_temp_unit: "C".into(),
            npsh_static_head: 2.0,
            npsh_friction: 0.5,
            npsh_rho: 998.0,
            npsh_rho_unit: "kg/m3".into(),
            npsh_required: 3.0,
            npsh_result: None,
            drain_shell_in: 120.0,
            drain_shell_out: 90.0,
            drain_shell_flow: 10.0,
            drain_tube_in: 30.0,
            drain_tube_out: 60.0,
            drain_tube_flow: 20.0,
            drain_temp_unit: "C".into(),
            drain_flow_unit: "m3/h".into(),
            drain_ua: 0.0,
            drain_area: 0.0,
            drain_u: 0.0,
            drain_result: None,
            font_size: 16.0,
            ui_scale: 1.0,
            always_on_top: true,
            show_settings_modal: false,
            show_help_modal: false,
            theme: ThemeChoice::SoftBlue,
            custom_font_path: String::new(),
            font_load_error: None,
        };
        s.apply_unit_preset(s.config.unit_system);
        s
    }

    /// 단위 시스템 프리셋을 UI 기본 단위에 적용한다.
    pub(crate) fn apply_unit_preset(&mut self, system: config::UnitSystem) {
        match system {
            config::UnitSystem::SIBar => {
                self.steam_p_unit = "bar".into();
                self.steam_p_mode = conversion::PressureMode::Gauge;
                self.steam_p_unit_out = "bar".into();
                self.steam_p_mode_out = conversion::PressureMode::Absolute;
                self.steam_t_unit = "C".into();
                self.steam_t_unit_out = "C".into();
                self.pipe_pressure_unit = "bar".into();
                self.pipe_pressure_mode = conversion::PressureMode::Gauge;
                self.pipe_temp_unit = "C".into();
                self.pipe_velocity_unit = "m/s".into();
                self.pipe_diam_out_unit = "m".into();
                self.pipe_vel_out_unit = "m/s".into();
                self.pipe_loss_dp_out_unit = "bar".into();
                self.pipe_loss_dp_out_mode = conversion::PressureMode::Absolute;
                self.pipe_mass_unit = "kg/h".into();
                self.valve_flow_unit = "m3/h".into();
                self.valve_dp_unit = "bar".into();
                self.valve_dp_mode = conversion::PressureMode::Gauge;
                self.valve_rho_unit = "kg/m3".into();
                self.condenser_pressure_unit = "bar".into();
                self.condenser_pressure_mode = conversion::PressureMode::Gauge;
                self.condenser_cw_temp_unit = "C".into();
                self.condenser_cw_flow_unit = "m3/h".into();
                self.condenser_backpressure_unit = "bar".into();
                self.condenser_backpressure_mode = conversion::PressureMode::Absolute;
                self.ct_temp_unit = "C".into();
                self.ct_flow_unit = "m3/h".into();
                self.npsh_suction_unit = "bar".into();
                self.npsh_suction_mode = conversion::PressureMode::Gauge;
                self.npsh_temp_unit = "C".into();
                self.npsh_rho_unit = "kg/m3".into();
                self.drain_temp_unit = "C".into();
                self.drain_flow_unit = "m3/h".into();
                self.plant_dp_unit = "bar".into();
                self.plant_dp_mode = conversion::PressureMode::Gauge;
            }
            config::UnitSystem::SI => {
                self.steam_p_unit = "kPa".into();
                self.steam_p_mode = conversion::PressureMode::Gauge;
                self.steam_p_unit_out = "kPa".into();
                self.steam_p_mode_out = conversion::PressureMode::Absolute;
                self.steam_t_unit = "C".into();
                self.steam_t_unit_out = "C".into();
                self.pipe_pressure_unit = "kPa".into();
                self.pipe_pressure_mode = conversion::PressureMode::Gauge;
                self.pipe_temp_unit = "C".into();
                self.pipe_velocity_unit = "m/s".into();
                self.pipe_diam_out_unit = "m".into();
                self.pipe_vel_out_unit = "m/s".into();
                self.pipe_loss_dp_out_unit = "kPa".into();
                self.pipe_loss_dp_out_mode = conversion::PressureMode::Absolute;
                self.pipe_mass_unit = "kg/h".into();
                self.valve_flow_unit = "m3/h".into();
                self.valve_dp_unit = "kPa".into();
                self.valve_dp_mode = conversion::PressureMode::Gauge;
                self.valve_rho_unit = "kg/m3".into();
                self.condenser_pressure_unit = "kPa".into();
                self.condenser_pressure_mode = conversion::PressureMode::Gauge;
                self.condenser_cw_temp_unit = "C".into();
                self.condenser_cw_flow_unit = "m3/h".into();
                self.condenser_backpressure_unit = "kPa".into();
                self.condenser_backpressure_mode = conversion::PressureMode::Absolute;
                self.ct_temp_unit = "C".into();
                self.ct_flow_unit = "m3/h".into();
                self.npsh_suction_unit = "kPa".into();
                self.npsh_suction_mode = conversion::PressureMode::Gauge;
                self.npsh_temp_unit = "C".into();
                self.npsh_rho_unit = "kg/m3".into();
                self.drain_temp_unit = "C".into();
                self.drain_flow_unit = "m3/h".into();
                self.plant_dp_unit = "kPa".into();
                self.plant_dp_mode = conversion::PressureMode::Gauge;
            }
            config::UnitSystem::MKS => {
                self.steam_p_unit = "bar".into();
                self.steam_p_mode = conversion::PressureMode::Absolute;
                self.steam_p_unit_out = "bar".into();
                self.steam_p_mode_out = conversion::PressureMode::Absolute;
                self.steam_t_unit = "C".into();
                self.steam_t_unit_out = "C".into();
                self.pipe_pressure_unit = "bar".into();
                self.pipe_pressure_mode = conversion::PressureMode::Absolute;
                self.pipe_temp_unit = "C".into();
                self.pipe_velocity_unit = "m/s".into();
                self.pipe_diam_out_unit = "m".into();
                self.pipe_vel_out_unit = "m/s".into();
                self.pipe_loss_dp_out_unit = "bar".into();
                self.pipe_loss_dp_out_mode = conversion::PressureMode::Absolute;
                self.pipe_mass_unit = "kg/h".into();
                self.valve_flow_unit = "m3/h".into();
                self.valve_dp_unit = "bar".into();
                self.valve_dp_mode = conversion::PressureMode::Absolute;
                self.valve_rho_unit = "kg/m3".into();
                self.condenser_pressure_unit = "bar".into();
                self.condenser_pressure_mode = conversion::PressureMode::Absolute;
                self.condenser_cw_temp_unit = "C".into();
                self.condenser_cw_flow_unit = "m3/h".into();
                self.condenser_backpressure_unit = "bar".into();
                self.condenser_backpressure_mode = conversion::PressureMode::Absolute;
                self.ct_temp_unit = "C".into();
                self.ct_flow_unit = "m3/h".into();
                self.npsh_suction_unit = "bar".into();
                self.npsh_suction_mode = conversion::PressureMode::Absolute;
                self.npsh_temp_unit = "C".into();
                self.npsh_rho_unit = "kg/m3".into();
                self.drain_temp_unit = "C".into();
                self.drain_flow_unit = "m3/h".into();
                self.plant_dp_unit = "bar".into();
                self.plant_dp_mode = conversion::PressureMode::Absolute;
            }
            config::UnitSystem::Imperial => {
                self.steam_p_unit = "psi".into();
                self.steam_p_mode = conversion::PressureMode::Gauge;
                self.steam_p_unit_out = "psi".into();
                self.steam_p_mode_out = conversion::PressureMode::Gauge;
                self.steam_t_unit = "F".into();
                self.steam_t_unit_out = "F".into();
                self.pipe_pressure_unit = "psi".into();
                self.pipe_pressure_mode = conversion::PressureMode::Gauge;
                self.pipe_temp_unit = "F".into();
                self.pipe_velocity_unit = "ft/s".into();
                self.pipe_diam_out_unit = "in".into();
                self.pipe_vel_out_unit = "ft/s".into();
                self.pipe_loss_dp_out_unit = "psi".into();
                self.pipe_loss_dp_out_mode = conversion::PressureMode::Gauge;
                self.pipe_mass_unit = "lb/h".into();
                self.valve_flow_unit = "gpm".into();
                self.valve_dp_unit = "psi".into();
                self.valve_dp_mode = conversion::PressureMode::Gauge;
                self.valve_rho_unit = "lb/ft3".into();
                self.bypass_up_unit = "psi".into();
                self.bypass_down_unit = "psi".into();
                self.bypass_up_mode = conversion::PressureMode::Gauge;
                self.bypass_down_mode = conversion::PressureMode::Gauge;
                self.bypass_t_unit = "F".into();
                self.bypass_spray_temp_unit = "F".into();
                self.bypass_cv_kind = "Cv(US)".into();
                self.spray_up_unit = "psi".into();
                self.spray_down_unit = "psi".into();
                self.spray_up_mode = conversion::PressureMode::Gauge;
                self.spray_down_mode = conversion::PressureMode::Gauge;
                self.spray_temp_unit = "F".into();
                self.spray_cv_kind = "Cv(US)".into();
                // 보일러/에너지 단위
                self.boiler_fuel_unit = "lb/h".into();
                self.boiler_lhv_unit = "Btu/lb".into();
                self.boiler_steam_unit = "lb/h".into();
                self.boiler_h_steam_unit = "Btu/lb".into();
                self.boiler_h_fw_unit = "Btu/lb".into();
                self.boiler_fg_flow_unit = "lb/h".into();
                self.boiler_temp_unit = "F".into();
                self.condenser_pressure_unit = "psi".into();
                self.condenser_pressure_mode = conversion::PressureMode::Gauge;
                self.condenser_cw_temp_unit = "F".into();
                self.condenser_cw_flow_unit = "gpm".into();
                self.condenser_backpressure_unit = "psi".into();
                self.condenser_backpressure_mode = conversion::PressureMode::Absolute;
                self.ct_temp_unit = "F".into();
                self.ct_flow_unit = "gpm".into();
                self.npsh_suction_unit = "psi".into();
                self.npsh_suction_mode = conversion::PressureMode::Gauge;
                self.npsh_temp_unit = "F".into();
                self.npsh_rho_unit = "lb/ft3".into();
                self.drain_temp_unit = "F".into();
                self.drain_flow_unit = "gpm".into();
                self.plant_dp_unit = "psi".into();
                self.plant_dp_mode = conversion::PressureMode::Gauge;
            }
        }
    }
    /// 사이드 메뉴를 제공한다.
    fn ui_nav(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().wrap = Some(false);
        ui.vertical_centered(|ui| {
            ui.heading("메뉴");
            ui.add_space(8.0);
        });
        for (tab, label) in [
            (Tab::SteamTables, "Steam Tables / 증기표"),
            (Tab::UnitConv, "Unit Converter / 단위변환"),
            (Tab::SteamPiping, "Steam Piping / 증기 배관"),
            (Tab::SteamValves, "Steam Valves / 밸브·오리피스"),
            (Tab::Boiler, "Boiler Efficiency / 보일러 효율"),
            (Tab::Cooling, "Cooling/Condensing / 냉각·복수"),
            (Tab::PlantPiping, "Plant Piping / 플랜트 배관"),
        ] {
            let selected = self.tab == tab;
            let button = egui::Button::new(label)
                .fill(if selected {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().extreme_bg_color
                })
                .min_size(egui::vec2(ui.available_width(), 32.0));
            let resp = ui.add(button).on_hover_text("메뉴 전환");
            if resp.clicked() {
                    self.tab = tab;
            }
            ui.add_space(4.0);
        }
    }

    fn ui_unit_conv(&mut self, ui: &mut egui::Ui) {
        heading_with_tip(ui, "Unit Converter", "여러 물리량을 서로 다른 단위로 변환하는 도구");
        label_with_tip(ui, "카드형 입력 UI", "좌측에서 값을 입력하고 단위를 선택한 뒤 변환 실행을 누릅니다.");
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
                egui::Grid::new("conv_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        label_with_tip(ui, "물리량", "온도·압력·길이 등 변환하려는 물리량 종류");
                        let before = self.conv_kind;
                        egui::ComboBox::from_id_source("conv_kind")
                            .selected_text(kind_label(self.conv_kind))
                            .show_ui(ui, |ui| {
                                for (k, label) in quantity_options() {
                                    ui.selectable_value(&mut self.conv_kind, k, label);
                                }
                            });
                        if before != self.conv_kind {
                            let (f, t) = default_units_for_kind(self.conv_kind);
                            self.conv_from = f.to_string();
                            self.conv_to = t.to_string();
                        }
                        ui.end_row();

                        label_with_tip(ui, "값", "변환 대상 수치를 입력");
                        ui.add(egui::DragValue::new(&mut self.conv_value).speed(1.0));
                        ui.end_row();

                        label_with_tip(ui, "입력 단위", "현재 값의 단위를 선택");
                        egui::ComboBox::from_id_source("conv_from")
                            .selected_text(unit_label(&self.conv_from, self.conv_kind))
                            .show_ui(ui, |ui| {
                                for (label, code) in unit_options(self.conv_kind) {
                                    ui.selectable_value(
                                        &mut self.conv_from,
                                        code.to_string(),
                                        *label,
                                    );
                                }
                            });
                        ui.end_row();

                        label_with_tip(ui, "출력 단위", "변환 후 받고 싶은 단위");
                        egui::ComboBox::from_id_source("conv_to")
                            .selected_text(unit_label(&self.conv_to, self.conv_kind))
                            .show_ui(ui, |ui| {
                                for (label, code) in unit_options(self.conv_kind) {
                                    ui.selectable_value(
                                        &mut self.conv_to,
                                        code.to_string(),
                                        *label,
                                    );
                                }
                            });
                        ui.end_row();
                    });
                ui.add_space(8.0);
                if ui.button("변환 실행").clicked() {
                    self.conv_result = match conversion::convert(
                        self.conv_kind,
                        self.conv_value,
                        self.conv_from.trim(),
                        self.conv_to.trim(),
                    ) {
                        Ok(v) => Some(format!("{v:.6} {}", self.conv_to.trim())),
                        Err(e) => Some(format!("오류: {e}")),
                    };
                }
                if let Some(res) = &self.conv_result {
                    ui.label(res);
                }
            });
        });
    }

    fn ui_steam_tables(&mut self, ui: &mut egui::Ui) {
        heading_with_tip(ui, "Steam Tables", "IAPWS-IF97 기반 증기/수 상태량 계산 (포화/과열)");
        label_with_tip(
            ui,
            "포화/과열 입력 카드 (h/s/v 포함)",
            "압력·온도로 포화/과열 상태를 지정해 Psat, Tsat, h, s, v를 조회합니다.",
        );
        ui.add_space(8.0);
        if ui
            .button("진공 포화온도 표 열기")
            .on_hover_text("mmHg 게이지 기준 포화온도 표를 내장 창으로 표시")
            .clicked()
        {
            self.show_vacuum_table_window = true;
        }
        ui.horizontal(|ui| {
            if ui
                .button("진공 포화온도 표 새 창")
                .on_hover_text("별도 윈도우로 띄워놓고 다른 메뉴를 사용할 수 있습니다.")
                .clicked()
            {
                self.show_vacuum_table_viewport = true;
            }
            ui.small("※ 외부 창으로 띄워두고 다른 메뉴 탐색 가능");
        });
        if self.show_vacuum_table_window {
            egui::Window::new("진공 포화온도 표 (mmHg 게이지 기준 0=대기, -760=진공)")
                .open(&mut self.show_vacuum_table_window)
                .scroll2([true, true])
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    vacuum_table_ui(ui);
                });
        }

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.steam_mode, SteamMode::ByPressure, "압력 기준")
                    .on_hover_text("압력을 입력해 포화온도/엔탈피/엔트로피를 계산");
                ui.selectable_value(&mut self.steam_mode, SteamMode::ByTemperature, "온도 기준")
                    .on_hover_text("온도를 입력해 포화압력/엔탈피/엔트로피를 계산");
                ui.selectable_value(&mut self.steam_mode, SteamMode::Superheated, "과열")
                    .on_hover_text("압력+과열온도를 입력해 과열 증기 상태량 계산");
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                label_with_tip(ui, "값", "현재 모드에서 입력하는 압력 또는 온도");
                ui.add(egui::DragValue::new(&mut self.steam_value).speed(0.5));
                if matches!(self.steam_mode, SteamMode::ByPressure | SteamMode::Superheated) {
                    unit_combo(ui, &mut self.steam_p_unit, pressure_unit_options());
                    ui.selectable_value(&mut self.steam_p_mode, conversion::PressureMode::Gauge, "Gauge (G)");
                    ui.selectable_value(&mut self.steam_p_mode, conversion::PressureMode::Absolute, "Absolute (A)");
                } else {
                    unit_combo(ui, &mut self.steam_t_unit, temperature_unit_options());
                }
            });
            if self.steam_mode == SteamMode::Superheated {
                ui.horizontal(|ui| {
                    label_with_tip(ui, "과열 온도 [°C]", "포화점 대비 과열 온도 (절대 온도가 아님)");
                    ui.add(egui::DragValue::new(&mut self.steam_temp_input).speed(1.0));
                    unit_combo(ui, &mut self.steam_t_unit, temperature_unit_options());
                });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                label_with_tip(ui, "출력 압력 단위", "계산 결과를 표시할 압력 단위");
                unit_combo(ui, &mut self.steam_p_unit_out, pressure_unit_options());
                ui.selectable_value(&mut self.steam_p_mode_out, conversion::PressureMode::Gauge, "Gauge (G)");
                ui.selectable_value(&mut self.steam_p_mode_out, conversion::PressureMode::Absolute, "Absolute (A)");
                label_with_tip(ui, "출력 온도 단위", "계산 결과를 표시할 온도 단위");
                unit_combo(ui, &mut self.steam_t_unit_out, temperature_unit_options());
            });
            ui.small("Tip: mmHg는 게이지 기준(0=대기, -760mmHg=완전진공)으로 처리됩니다.");
            ui.add_space(6.0);
            if ui.button("계산").clicked() {
                self.steam_result = Some(match self.steam_mode {
                    SteamMode::ByPressure => match steam::saturation_by_pressure_mode(
                        convert_pressure_mode_gui(
                            self.steam_value,
                            &self.steam_p_unit,
                            self.steam_p_mode,
                            "bar",
                            conversion::PressureMode::Absolute,
                        ),
                        PressureUnit::BarA,
                        conversion::PressureMode::Absolute,
                    ) {
                        Ok(s) => {
                            let p_out = convert_pressure_mode_gui(
                                s.pressure_bar,
                                "bar",
                                conversion::PressureMode::Absolute,
                                &self.steam_p_unit_out,
                                self.steam_p_mode_out,
                            );
                            let t_out = convert_temperature_gui(s.saturation_temperature_c, "C", &self.steam_t_unit_out);
                            format!(
                                "Psat={:.3} {}, Tsat={:.2} {}, hs(v)={:.1} kJ/kg, vs={:.3} m3/kg, ss={:.3} kJ/kgK | hf={:.1} kJ/kg, vf={:.4} m3/kg, sf={:.3} kJ/kgK",
                                p_out,
                                self.steam_p_unit_out,
                                t_out,
                                self.steam_t_unit_out,
                                s.saturation_enthalpy_kj_per_kg,
                                s.saturation_specific_volume,
                                s.saturation_entropy_kj_per_kgk,
                                s.sat_liquid_enthalpy_kj_per_kg,
                                s.sat_liquid_specific_volume,
                                s.sat_liquid_entropy_kj_per_kgk
                            )
                        }
                        Err(e) => format!(
                            "오류(P={:.3} {}{}): {e}",
                            self.steam_value,
                            self.steam_p_unit,
                            if self.steam_p_mode == conversion::PressureMode::Gauge {
                                "g"
                            } else {
                                "a"
                            }
                        ),
                    },
                    SteamMode::ByTemperature => match steam::saturation_by_temperature(
                        convert_temperature_gui(self.steam_value, &self.steam_t_unit, "C"),
                        TemperatureUnit::Celsius,
                    ) {
                        Ok(s) => {
                            let p_out = convert_pressure_mode_gui(
                                s.pressure_bar,
                                "bar",
                                conversion::PressureMode::Absolute,
                                &self.steam_p_unit_out,
                                self.steam_p_mode_out,
                            );
                            format!(
                                "Psat={:.3} {}, hs={:.1} kJ/kg, v={:.3} m3/kg",
                                p_out,
                                self.steam_p_unit_out,
                                s.saturation_enthalpy_kj_per_kg,
                                s.saturation_specific_volume
                            )
                        }
                        Err(e) => format!(
                            "오류(T={:.2} {}): {e}",
                            self.steam_value, self.steam_t_unit
                        ),
                    },
                    SteamMode::Superheated => match steam::superheated_at(
                        convert_pressure_mode_gui(
                            self.steam_value,
                            &self.steam_p_unit,
                            self.steam_p_mode,
                            "bar",
                            conversion::PressureMode::Absolute,
                        ),
                        PressureUnit::BarA,
                        convert_temperature_gui(self.steam_temp_input, &self.steam_t_unit, "C"),
                        TemperatureUnit::Celsius,
                    ) {
                        Ok(s) => {
                            let p_out = convert_pressure_mode_gui(
                                s.pressure_bar,
                                "bar",
                                conversion::PressureMode::Absolute,
                                &self.steam_p_unit_out,
                                self.steam_p_mode_out,
                            );
                            let t_out = convert_temperature_gui(s.temperature_c, "C", &self.steam_t_unit_out);
                            format!(
                                "P={:.2} {}, T={:.1} {}, h={:.1} kJ/kg",
                                p_out,
                                self.steam_p_unit_out,
                                t_out,
                                self.steam_t_unit_out,
                                s.superheated_enthalpy_kj_per_kg.unwrap_or(0.0)
                            )
                        }
                        Err(e) => format!(
                            "오류(P={:.3} {}{}, T={:.1} {}): {e}",
                            self.steam_value,
                            self.steam_p_unit,
                            if self.steam_p_mode == conversion::PressureMode::Gauge {
                                "g"
                            } else {
                                "a"
                            },
                            self.steam_temp_input,
                            self.steam_t_unit
                        ),
                    },
                });
            }
            if let Some(res) = &self.steam_result {
                ui.separator();
                ui.label(res);
                ui.label("Psat=포화압력, Tsat=포화온도, hs/vs/ss=포화증기, hf/vf/sf=포화수");
            }
        });
    }

    fn ui_steam_piping(&mut self, ui: &mut egui::Ui) {
        heading_with_tip(ui, "Steam Piping", "증기 배관 내경/유속/압력강하를 계산하는 도구");
        label_with_tip(
            ui,
            "Pipe sizing 카드형 UI",
            "질량유량·압력·온도·허용 유속을 입력해 적정 내경/유속/레이놀즈수를 제안합니다.",
        );
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::Grid::new("pipe_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(ui, "질량 유량", "통과하는 증기 질량유량(kg/h 등)");
                    ui.add(egui::DragValue::new(&mut self.pipe_mass_flow).speed(10.0));
                    unit_combo(
                        ui,
                        &mut self.pipe_mass_unit,
                        &[
                            ("kg/h", "kg/h"),
                            ("t/h", "t/h"),
                            ("kg/s", "kg/s"),
                            ("lb/h", "lb/h"),
                        ],
                    );
                    ui.end_row();
                    label_with_tip(ui, "압력 [bar]", "배관 조건 압력 (게이지/절대 선택)");
                    ui.add(egui::DragValue::new(&mut self.pipe_pressure).speed(0.1));
                    unit_combo(ui, &mut self.pipe_pressure_unit, pressure_unit_options());
                    ui.selectable_value(
                        &mut self.pipe_pressure_mode,
                        conversion::PressureMode::Gauge,
                        "Gauge (G)",
                    );
                    ui.selectable_value(
                        &mut self.pipe_pressure_mode,
                        conversion::PressureMode::Absolute,
                        "Absolute (A)",
                    );
                    ui.end_row();
                    label_with_tip(ui, "온도 [°C]", "배관 조건의 증기 온도");
                    ui.add(egui::DragValue::new(&mut self.pipe_temp).speed(1.0));
                    unit_combo(ui, &mut self.pipe_temp_unit, temperature_unit_options());
                    ui.end_row();
                    label_with_tip(
                        ui,
                        "허용 유속 [m/s]",
                        "설계 목표 유속(높이면 배관이 작아지나 소음/침식 위험 증가)",
                    );
                    ui.add(egui::DragValue::new(&mut self.pipe_velocity).speed(1.0));
                    unit_combo(
                        ui,
                        &mut self.pipe_velocity_unit,
                        &[("m/s", "m/s"), ("ft/s", "ft/s")],
                    );
                    ui.end_row();
                });
            ui.small("Tip: mmHg는 게이지 기준(0=대기, -760mmHg=완전진공)으로 처리됩니다.");
            ui.add_space(8.0);
            if ui.button("사이징 계산").clicked() {
                let density = steam::estimate_density(
                    convert_pressure_mode_gui(
                        self.pipe_pressure,
                        &self.pipe_pressure_unit,
                        self.pipe_pressure_mode,
                        "bar",
                        conversion::PressureMode::Absolute,
                    ),
                    PressureUnit::BarA,
                    convert_temperature_gui(self.pipe_temp, &self.pipe_temp_unit, "C"),
                    TemperatureUnit::Celsius,
                );
                let input = PipeSizingByVelocityInput {
                    mass_flow_kg_per_h: convert_massflow_gui(
                        self.pipe_mass_flow,
                        &self.pipe_mass_unit,
                        "kg/h",
                    ),
                    steam_density_kg_per_m3: density,
                    target_velocity_m_per_s: convert_velocity_gui(
                        self.pipe_velocity,
                        &self.pipe_velocity_unit,
                        "m/s",
                    ),
                };
                self.pipe_result = Some(match steam::size_by_velocity(input) {
                    Ok(r) => {
                        let d_out =
                            convert_length_gui(r.inner_diameter_m, "m", &self.pipe_diam_out_unit);
                        let v_out = convert_velocity_gui(
                            r.velocity_m_per_s,
                            "m/s",
                            &self.pipe_vel_out_unit,
                        );
                        format!(
                            "Pipe ID = {:.4} {}, Velocity = {:.2} {}, Reynolds (Re) = {:.2e}",
                            d_out,
                            self.pipe_diam_out_unit,
                            v_out,
                            self.pipe_vel_out_unit,
                            r.reynolds_number
                        )
                    }
                    Err(e) => format!(
                        "오류(ṁ={:.2} {}, P={:.2} {}{}, T={:.1} {}): {e}",
                        self.pipe_mass_flow,
                        self.pipe_mass_unit,
                        self.pipe_pressure,
                        self.pipe_pressure_unit,
                        if self.pipe_pressure_mode == conversion::PressureMode::Gauge {
                            "g"
                        } else {
                            "a"
                        },
                        self.pipe_temp,
                        self.pipe_temp_unit
                    ),
                });
            }
            if let Some(res) = &self.pipe_result {
                ui.separator();
                ui.label(res);
                ui.label("ID=내경, Velocity=유속, Re=레이놀즈수");
            }
        });
        ui.add_space(6.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label("Pressure Loss (Darcy-Weisbach)");
            egui::Grid::new("pipe_loss_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("질량 유량 [kg/h]");
                    ui.add(egui::DragValue::new(&mut self.pipe_mass_flow).speed(10.0));
                    unit_combo(
                        ui,
                        &mut self.pipe_mass_unit,
                        &[("kg/h", "kg/h"), ("lb/h", "lb/h")],
                    );
                    ui.end_row();
                    ui.label("밀도 [kg/m3]");
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_density).speed(0.1));
                    ui.end_row();
                    ui.label("내경 [m]");
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_diameter).speed(0.001));
                    ui.end_row();
                    ui.label("길이 [m]");
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_length).speed(1.0));
                    ui.end_row();
                    ui.label("등가 길이 [m]");
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_eq_length).speed(1.0));
                    ui.end_row();
                    ui.label("피팅 K 합");
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_fittings_k).speed(0.1));
                    ui.end_row();
                    ui.label("거칠기 ε [m]");
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_roughness).speed(0.00001));
                    ui.end_row();
                    ui.label("점도 [Pa·s]");
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_visc).speed(1e-6));
                    ui.end_row();
                    ui.label("음속 [m/s]");
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_sound_speed).speed(5.0));
                    ui.end_row();
                    ui.label("출력 ΔP 단위");
                    unit_combo(ui, &mut self.pipe_loss_dp_out_unit, pressure_unit_options());
                    ui.selectable_value(
                        &mut self.pipe_loss_dp_out_mode,
                        conversion::PressureMode::Gauge,
                        "Gauge (G)",
                    );
                    ui.selectable_value(
                        &mut self.pipe_loss_dp_out_mode,
                        conversion::PressureMode::Absolute,
                        "Absolute (A)",
                    );
                    ui.end_row();
                });
            if ui.button("압력손실 계산").clicked() {
                let input = steam::steam_piping::PressureLossInput {
                    mass_flow_kg_per_h: convert_massflow_gui(
                        self.pipe_mass_flow,
                        &self.pipe_mass_unit,
                        "kg/h",
                    ),
                    steam_density_kg_per_m3: self.pipe_loss_density,
                    diameter_m: self.pipe_loss_diameter,
                    length_m: self.pipe_loss_length,
                    equivalent_length_m: self.pipe_loss_eq_length,
                    fittings_k_sum: self.pipe_loss_fittings_k,
                    roughness_m: self.pipe_loss_roughness,
                    dynamic_viscosity_pa_s: self.pipe_loss_visc,
                    sound_speed_m_per_s: self.pipe_loss_sound_speed,
                };
                self.pipe_loss_result = Some(match steam::steam_piping::pressure_loss(input) {
                    Ok(r) => {
                        let dp_out = convert_pressure_mode_gui(
                            r.pressure_drop_bar,
                            "bar",
                            conversion::PressureMode::Absolute,
                            &self.pipe_loss_dp_out_unit,
                            self.pipe_loss_dp_out_mode,
                        );
                        format!(
                            "ΔP={:.4} {}, v={:.2} m/s, Re={:.2e}, f={:.4}, Mach={:.3}",
                            dp_out,
                            self.pipe_loss_dp_out_unit,
                            r.velocity_m_per_s,
                            r.reynolds_number,
                            r.friction_factor,
                            r.mach
                        )
                    }
                    Err(e) => format!(
                        "오류(ṁ={:.2} {}, ρ={:.2} kg/m3, D={:.4} m, L={:.1} m): {e}",
                        self.pipe_mass_flow,
                        self.pipe_mass_unit,
                        self.pipe_loss_density,
                        self.pipe_loss_diameter,
                        self.pipe_loss_length
                    ),
                });
            }
            if let Some(res) = &self.pipe_loss_result {
                ui.separator();
                ui.label(res);
                ui.label("ΔP=압력강하, v=유속, Re=레이놀즈수, f=마찰계수, Mach=음속비");
            }
        });
    }

    fn ui_steam_valves(&mut self, ui: &mut egui::Ui) {
        heading_with_tip(ui, "Steam Valves & Orifices", "Cv/Kv 산정 또는 주어진 Cv/Kv로 유량 계산");
        label_with_tip(ui, "Cv/Kv 계산 UI", "차압·상류압·유량·밀도 등을 입력하여 밸브 성능을 확인");
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.valve_mode, ValveMode::RequiredCvKv, "필요 Cv/Kv")
                    .on_hover_text("목표 유량을 내기 위한 Cv/Kv 산정");
                ui.selectable_value(&mut self.valve_mode, ValveMode::FlowFromCvKv, "Cv/Kv로 유량")
                    .on_hover_text("Cv/Kv가 주어졌을 때 통과 유량 계산");
            });
            egui::Grid::new("valve_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(
                        ui,
                        match self.valve_mode {
                            ValveMode::RequiredCvKv => "볼류메트릭 유량",
                            ValveMode::FlowFromCvKv => "Cv/Kv 입력",
                        },
                        "유량을 입력하거나 Cv/Kv를 입력",
                    );
                    ui.add(egui::DragValue::new(&mut self.valve_flow).speed(1.0));
                    if matches!(self.valve_mode, ValveMode::RequiredCvKv) {
                        unit_combo(
                            ui,
                            &mut self.valve_flow_unit,
                            &[
                                ("m3/h", "m3/h"),
                                ("kg/h", "kg/h"),
                                ("t/h", "t/h"),
                                ("kg/s", "kg/s"),
                                ("lb/h", "lb/h"),
                                ("gpm", "gpm"),
                            ],
                        );
                    }
                    ui.end_row();
                    label_with_tip(
                        ui,
                        "차압 [bar]",
                        "밸브 양단의 압력차 ΔP (게이지/절대 선택) — 증기/가스는 초크 여부 확인",
                    );
                    ui.add(egui::DragValue::new(&mut self.valve_dp).speed(0.1));
                    unit_combo(ui, &mut self.valve_dp_unit, pressure_unit_options());
                    ui.selectable_value(&mut self.valve_dp_mode, conversion::PressureMode::Gauge, "Gauge (G)");
                    ui.selectable_value(&mut self.valve_dp_mode, conversion::PressureMode::Absolute, "Absolute (A)");
                    ui.end_row();
                    label_with_tip(
                        ui,
                        "상류 압력",
                        "Cv/Kv로 유량 계산 시 상류 절대압 입력 (초크 판정용)",
                    );
                    ui.add(egui::DragValue::new(&mut self.valve_upstream_p).speed(0.1));
                    unit_combo(ui, &mut self.valve_upstream_unit, pressure_unit_options());
                    ui.selectable_value(&mut self.valve_upstream_mode, conversion::PressureMode::Gauge, "Gauge (G)");
                    ui.selectable_value(&mut self.valve_upstream_mode, conversion::PressureMode::Absolute, "Absolute (A)");
                    ui.end_row();
                    label_with_tip(
                        ui,
                        "밀도 [kg/m3]",
                        "유체 밀도(증기/가스면 조건에 맞춰 입력, 증기면 IF97 결과 사용 권장)",
                    );
                    ui.add(egui::DragValue::new(&mut self.valve_rho).speed(0.1));
                    unit_combo(ui, &mut self.valve_rho_unit, &[("kg/m3", "kg/m3"), ("lb/ft3", "lb/ft3")]);
                    ui.end_row();
                    if let ValveMode::FlowFromCvKv = self.valve_mode {
                        label_with_tip(ui, "Cv/Kv 값", "제조사 제공 Cv 또는 Kv");
                        ui.add(egui::DragValue::new(&mut self.valve_cv_kv).speed(0.5));
                        ui.end_row();
                    }
                });
            ui.small("Tip: mmHg는 게이지 기준(0=대기, -760mmHg=완전진공)으로 처리됩니다.");
            ui.add_space(8.0);
            if ui.button("계산").clicked() {
                self.valve_result = Some(match self.valve_mode {
                    ValveMode::RequiredCvKv => match steam_valves::required_kv(
                        convert_flow_gui(self.valve_flow, &self.valve_flow_unit, &self.valve_rho_unit, self.valve_rho),
                        convert_pressure_mode_gui(
                            self.valve_dp,
                            &self.valve_dp_unit,
                            self.valve_dp_mode,
                            "bar",
                            conversion::PressureMode::Gauge,
                        ),
                        convert_density_gui(self.valve_rho, &self.valve_rho_unit, "kg/m3"),
                    ) {
                        Ok(kv) => format!("Kv={:.3}, Cv={:.3}", kv, steam_valves::cv_from_kv(kv)),
                        Err(e) => format!(
                            "오류(Q={:.2} {}, ΔP={:.2} {}{}, ρ={:.2} {}): {e}",
                            self.valve_flow,
                            self.valve_flow_unit,
                            self.valve_dp,
                            self.valve_dp_unit,
                            if self.valve_dp_mode == conversion::PressureMode::Gauge {
                                "g"
                            } else {
                                "a"
                            },
                            self.valve_rho,
                            self.valve_rho_unit
                        ),
                    },
                    ValveMode::FlowFromCvKv => {
                        let upstream_bar_abs = convert_pressure_mode_gui(
                            self.valve_upstream_p,
                            &self.valve_upstream_unit,
                            self.valve_upstream_mode,
                            "bar",
                            conversion::PressureMode::Absolute,
                        );
                        let dp_abs = convert_pressure_mode_gui(
                            self.valve_dp,
                            &self.valve_dp_unit,
                            self.valve_dp_mode,
                            "bar",
                            conversion::PressureMode::Absolute,
                        );
                        let kv = self.valve_cv_kv;
                        match steam_valves::flow_from_kv(
                            kv,
                            convert_pressure_mode_gui(
                                self.valve_dp,
                                &self.valve_dp_unit,
                                self.valve_dp_mode,
                                "bar",
                                conversion::PressureMode::Gauge,
                            ),
                            convert_density_gui(self.valve_rho, &self.valve_rho_unit, "kg/m3"),
                            Some(upstream_bar_abs),
                        ) {
                            Ok(q_m3h) => {
                                let q_out = convert_flow_from_m3h(
                                    q_m3h,
                                    &self.valve_flow_unit,
                                    &self.valve_rho_unit,
                                    self.valve_rho,
                                );
                                let mass_kg_h =
                                    q_m3h * convert_density_gui(self.valve_rho, &self.valve_rho_unit, "kg/m3");
                                let downstream_abs = (upstream_bar_abs - dp_abs).max(0.0);
                                let crit_ratio = 0.55; // 대략 증기 임계비
                                let choked = if upstream_bar_abs > 0.0 {
                                    downstream_abs / upstream_bar_abs < crit_ratio
                                } else {
                                    false
                                };
                                let warn = if choked {
                                    " [주의: 음속 임계(Choked) 가능]"
                                } else {
                                    ""
                                };
                                format!(
                                    "유량 {:.3} {}{warn}, 질량 {:.3} kg/h (Pu={:.2} bar(a), Pd={:.2} bar(a))",
                                    q_out,
                                    self.valve_flow_unit,
                                    mass_kg_h,
                                    upstream_bar_abs,
                                    downstream_abs
                                )
                            }
                            Err(e) => format!(
                                "오류(Cv/Kv={:.2}, ΔP={:.2} {}{}, ρ={:.2} {}): {e}",
                                kv,
                                self.valve_dp,
                                self.valve_dp_unit,
                                if self.valve_dp_mode == conversion::PressureMode::Gauge {
                                    "g"
                                } else {
                                    "a"
                                },
                                self.valve_rho,
                                self.valve_rho_unit
                            ),
                        }
                    }
                });
            }
            if let Some(res) = &self.valve_result {
                ui.separator();
                ui.label(res);
                ui.label("Cv/Kv: 유량 계수, ΔP: 차압, 밀도/임계 유량 여부에 유의");
            }
        });
        ui.add_space(10.0);
        self.ui_bypass_panels(ui);
    }

    /// ST 바이패스 및 TCV 계산 패널.
    /// - Bypass Valve(증기): Cv/Kv 혹은 Stroke-Cv 테이블로 증기 유량을 계산하고, 필요 시 TCV(물) 결과를 합산해 엔탈피를 본다.
    /// - TCV(물): 별도 물 밸브 유량 계산을 제공하며, 결과가 바이패스 스프레이 값으로 자동 반영된다.
    fn ui_bypass_panels(&mut self, ui: &mut egui::Ui) {
        ui.heading("Bypass Valve(증기) / TCV(물)");
        ui.label("Stroke별 Cv 테이블이 있으면 보간, 없으면 Cv/Kv 단일 값 사용");
        ui.add_space(6.0);

        // ---------- ST Bypass Valve (증기) ----------
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading("Bypass Valve (증기)");
            egui::Grid::new("bypass_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("상류 압력");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.bypass_up_p).speed(0.5));
                        unit_combo(ui, &mut self.bypass_up_unit, pressure_unit_options());
                        ui.selectable_value(
                            &mut self.bypass_up_mode,
                            conversion::PressureMode::Gauge,
                            "Gauge (G)",
                        );
                        ui.selectable_value(
                            &mut self.bypass_up_mode,
                            conversion::PressureMode::Absolute,
                            "Absolute (A)",
                        );
                    });
                    ui.end_row();

                    ui.label("상류 온도");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.bypass_up_t).speed(1.0));
                        unit_combo(ui, &mut self.bypass_t_unit, temperature_unit_options());
                    });
                    ui.end_row();

                    ui.label("하류 압력");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.bypass_down_p).speed(0.5));
                        unit_combo(ui, &mut self.bypass_down_unit, pressure_unit_options());
                        ui.selectable_value(
                            &mut self.bypass_down_mode,
                            conversion::PressureMode::Gauge,
                            "Gauge (G)",
                        );
                        ui.selectable_value(
                            &mut self.bypass_down_mode,
                            conversion::PressureMode::Absolute,
                            "Absolute (A)",
                        );
                    });
                    ui.end_row();

                    ui.label("Cv/Kv");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.bypass_cv).speed(1.0));
                        egui::ComboBox::from_id_source("bypass_cv_kind")
                            .selected_text(&self.bypass_cv_kind)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.bypass_cv_kind, "Cv(US)".into(), "Cv(US)");
                                ui.selectable_value(&mut self.bypass_cv_kind, "Cv(UK)".into(), "Cv(UK)");
                                ui.selectable_value(&mut self.bypass_cv_kind, "Kv".into(), "Kv");
                            });
                        ui.label("개도(%)");
                        ui.add(
                            egui::DragValue::new(&mut self.bypass_open_pct)
                                .speed(1.0)
                                .clamp_range(0.0..=100.0),
                        );
                    });
                    ui.end_row();
                    ui.label("증기 엔탈피 입력(kJ/kg, 0=자동 IF97)");
                    ui.add(egui::DragValue::new(&mut self.bypass_h_override_kj_per_kg).speed(10.0));
                    ui.end_row();
                    if stroke_based_kv_available(&self.bypass_stroke_points, &self.bypass_cv_points) {
                        let cv_interp = interpolate_stroke_cv(
                            &self.bypass_stroke_points,
                            &self.bypass_cv_points,
                            self.bypass_open_pct,
                        );
                        ui.label(format!("보간 Cv/Kv~{:.3} (개도 {:.1}%)", cv_interp, self.bypass_open_pct));
                        ui.end_row();
                    }
                });

            ui.label("Stroke-Cv/Kv 테이블(바이패스)");
            let bypass_suffix = if self
                .bypass_cv_kind
                .to_lowercase()
                .starts_with("kv")
            {
                "Kv"
            } else {
                "Cv"
            };
            let mut remove_idx: Option<usize> = None;
            for i in 0..self.bypass_stroke_points.len() {
                ui.horizontal(|ui| {
                    ui.label(format!("Stroke {}:", i + 1));
                    ui.add(
                        egui::DragValue::new(&mut self.bypass_stroke_points[i])
                            .speed(1.0)
                            .clamp_range(0.0..=100.0)
                            .suffix("%"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.bypass_cv_points[i])
                            .speed(1.0)
                            .suffix(bypass_suffix),
                    );
                    if ui.small_button("-").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.small_button("+ 행 추가").clicked() {
                    self.bypass_stroke_points.push(100.0);
                    self.bypass_cv_points.push(0.0);
                }
                ui.label("보간: 개도%에 해당 Cv 사용");
            });
            if let Some(idx) = remove_idx {
                if self.bypass_stroke_points.len() > 1 {
                    self.bypass_stroke_points.remove(idx);
                    self.bypass_cv_points.remove(idx);
                }
            }

            ui.add_space(6.0);
            if ui.button("Bypass 계산").clicked() {
                let up_abs = convert_pressure_mode_gui(
                    self.bypass_up_p,
                    &self.bypass_up_unit,
                    self.bypass_up_mode,
                    "bar",
                    conversion::PressureMode::Absolute,
                );
                let down_abs = convert_pressure_mode_gui(
                    self.bypass_down_p,
                    &self.bypass_down_unit,
                    self.bypass_down_mode,
                    "bar",
                    conversion::PressureMode::Absolute,
                );
                let dp = (up_abs - down_abs).max(0.0);
                let t_c = convert_temperature_gui(self.bypass_up_t, &self.bypass_t_unit, "C");
                let kv = {
                    let mut cv_use = self.bypass_cv;
                    if stroke_based_kv_available(&self.bypass_stroke_points, &self.bypass_cv_points)
                    {
                        cv_use = interpolate_stroke_cv(
                            &self.bypass_stroke_points,
                            &self.bypass_cv_points,
                            self.bypass_open_pct,
                        );
                    }
                    kv_from_cv_with_kind(cv_use, &self.bypass_cv_kind)
                };

                self.bypass_result = if dp <= 0.0 {
                    Some("오류: ΔP가 0 이하입니다.".into())
                } else {
                    // 증기 밀도/엔탈피 계산
                    let props = steam::if97::region_props(up_abs, t_c);
                    match props {
                        Ok((h_j_per_kg, v_m3_per_kg, _s)) => {
                            let rho = if v_m3_per_kg > 0.0 { 1.0 / v_m3_per_kg } else { 0.0 };
                            let h_steam_kj = if self.bypass_h_override_kj_per_kg > 0.0 {
                                self.bypass_h_override_kj_per_kg
                            } else {
                                h_j_per_kg / 1000.0
                            };
                            match steam_valves::flow_from_kv(kv, dp, rho, Some(up_abs)) {
                                Ok(q_m3h) => {
                                    let m_steam = q_m3h * rho;
                                    // 스프레이 엔탈피(물): 하류 압력 기준으로 계산 시도
                                    let spray_t_c =
                                        convert_temperature_gui(self.bypass_spray_temp, &self.bypass_spray_temp_unit, "C");
                                    let spray_h = if self.spray_h_override_kj_per_kg > 0.0 {
                                        self.spray_h_override_kj_per_kg * 1000.0
                                    } else {
                                        steam::if97::region_props(down_abs.max(0.05), spray_t_c)
                                            .map(|(h, _, _)| h)
                                            .unwrap_or(0.0)
                                    };
                                    let m_spray = self.bypass_spray_kg_h.max(0.0);
                                    let m_total = m_steam + m_spray;
                                    let h_mix = if m_total > 0.0 {
                                        (m_steam * h_steam_kj * 1000.0 + m_spray * spray_h) / m_total
                                    } else {
                                        0.0
                                    };
                                    let total_heat_kw = (m_total * h_mix) / 3_600_000.0; // (kg/h)*(J/kg) -> W
                                    let choked = if up_abs > 0.0 {
                                        let crit_ratio = 0.55;
                                        (up_abs - dp).max(0.0) / up_abs < crit_ratio
                                    } else {
                                        false
                                    };
                                    let warn = if choked {
                                        " [주의: 임계(Choked) 가능]"
                                    } else {
                                        ""
                                    };
                                    Some(format!(
                                        "증기 Q={:.3} m³/h, m={:.2} kg/h{warn}; 스프레이={:.1} kg/h → 혼합 엔탈피~{:.1} kJ/kg, 총 열량~{:.1} kW (Pu={:.2} bar(a), Pd={:.2} bar(a), Kv={:.2})",
                                        q_m3h,
                                        m_steam,
                                        m_spray,
                                        h_mix / 1000.0,
                                        total_heat_kw,
                                        up_abs,
                                        down_abs,
                                        kv
                                    ))
                                }
                                Err(e) => Some(format!(
                                    "오류(Kv={:.2}, ΔP={:.2} bar, ρ={:.2} kg/m3): {e}",
                                    kv, dp, rho
                                )),
                            }
                        }
                        Err(e) => Some(format!("IF97 계산 실패: {e}")),
                    }
                };
            }
            if let Some(res) = &self.bypass_result {
                ui.label(res);
            }
        });

        ui.add_space(12.0);

        // ---------- Bypass TCV (물) ----------
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading("Bypass TCV (물)");
            egui::Grid::new("spray_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("상류 압력");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.spray_up_p).speed(0.2));
                        unit_combo(ui, &mut self.spray_up_unit, pressure_unit_options());
                        ui.selectable_value(
                            &mut self.spray_up_mode,
                            conversion::PressureMode::Gauge,
                            "Gauge (G)",
                        );
                        ui.selectable_value(
                            &mut self.spray_up_mode,
                            conversion::PressureMode::Absolute,
                            "Absolute (A)",
                        );
                    });
                    ui.end_row();

                    ui.label("하류 압력");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.spray_down_p).speed(0.2));
                        unit_combo(ui, &mut self.spray_down_unit, pressure_unit_options());
                        ui.selectable_value(
                            &mut self.spray_down_mode,
                            conversion::PressureMode::Gauge,
                            "Gauge (G)",
                        );
                        ui.selectable_value(
                            &mut self.spray_down_mode,
                            conversion::PressureMode::Absolute,
                            "Absolute (A)",
                        );
                    });
                    ui.end_row();

                    ui.label("물 온도");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.spray_temp).speed(0.5));
                        unit_combo(ui, &mut self.spray_temp_unit, temperature_unit_options());
                    });
                    ui.end_row();

                    ui.label("밀도 [kg/m3]");
                    ui.add(egui::DragValue::new(&mut self.spray_density).speed(1.0));
                    ui.end_row();

                    ui.label("Cv/Kv");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.spray_cv).speed(1.0));
                        egui::ComboBox::from_id_source("spray_cv_kind")
                            .selected_text(&self.spray_cv_kind)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.spray_cv_kind, "Cv(US)".into(), "Cv(US)");
                                ui.selectable_value(&mut self.spray_cv_kind, "Cv(UK)".into(), "Cv(UK)");
                                ui.selectable_value(&mut self.spray_cv_kind, "Kv".into(), "Kv");
                            });
                        ui.label("개도(%)");
                        ui.add(
                            egui::DragValue::new(&mut self.spray_open_pct)
                                .speed(1.0)
                                .clamp_range(0.0..=100.0),
                        );
                    });
                    ui.end_row();
                    ui.label("물 엔탈피 입력(kJ/kg, 0=자동)");
                    ui.add(egui::DragValue::new(&mut self.spray_h_override_kj_per_kg).speed(10.0));
                    ui.end_row();
                    if stroke_based_kv_available(&self.spray_stroke_points, &self.spray_cv_points) {
                        let cv_interp = interpolate_stroke_cv(
                            &self.spray_stroke_points,
                            &self.spray_cv_points,
                            self.spray_open_pct,
                        );
                        ui.label(format!("보간 Cv/Kv~{:.3} (개도 {:.1}%)", cv_interp, self.spray_open_pct));
                        ui.end_row();
                    }
                });

            ui.label("Stroke-Cv/Kv 테이블(빈 값은 무시, + / - 로 행 추가/삭제)");
            let spray_suffix = if self.spray_cv_kind.to_lowercase().starts_with("kv") {
                "Kv"
            } else {
                "Cv"
            };
            let mut remove_idx: Option<usize> = None;
            for i in 0..self.spray_stroke_points.len() {
                ui.horizontal(|ui| {
                    ui.label(format!("Stroke {}:", i + 1));
                    ui.add(
                        egui::DragValue::new(&mut self.spray_stroke_points[i])
                            .speed(1.0)
                            .clamp_range(0.0..=100.0)
                            .suffix("%"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.spray_cv_points[i])
                            .speed(1.0)
                            .suffix(spray_suffix),
                    );
                    if ui.small_button("-").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }
            ui.horizontal(|ui| {
                if ui.small_button("+ 행 추가").clicked() {
                    self.spray_stroke_points.push(100.0);
                    self.spray_cv_points.push(0.0);
                }
                ui.label("보간: 개도%에 해당 Cv 사용");
            });
            if let Some(idx) = remove_idx {
                if self.spray_stroke_points.len() > 1 {
                    self.spray_stroke_points.remove(idx);
                    self.spray_cv_points.remove(idx);
                }
            }

            ui.add_space(6.0);
            if ui.button("TCV 유량 계산").clicked() {
                let up_abs = convert_pressure_mode_gui(
                    self.spray_up_p,
                    &self.spray_up_unit,
                    self.spray_up_mode,
                    "bar",
                    conversion::PressureMode::Absolute,
                );
                let down_abs = convert_pressure_mode_gui(
                    self.spray_down_p,
                    &self.spray_down_unit,
                    self.spray_down_mode,
                    "bar",
                    conversion::PressureMode::Absolute,
                );
                let dp = (up_abs - down_abs).max(0.0);
                let rho = self.spray_density;
                let mut cv_use = self.spray_cv;
                if stroke_based_kv_available(&self.spray_stroke_points, &self.spray_cv_points) {
                    cv_use = interpolate_stroke_cv(
                        &self.spray_stroke_points,
                        &self.spray_cv_points,
                        self.spray_open_pct,
                    );
                }
                let kv = kv_from_cv_with_kind(cv_use, &self.spray_cv_kind);
                self.spray_calc_result = if dp <= 0.0 || rho <= 0.0 {
                    Some("오류: ΔP와 밀도는 0보다 커야 합니다.".into())
                } else {
                    match steam_valves::flow_from_kv(kv, dp, rho, Some(up_abs)) {
                        Ok(q_m3h) => {
                            let mass = q_m3h * rho;
                            self.bypass_spray_kg_h = mass;
                            self.bypass_spray_temp = convert_temperature_gui(
                                self.spray_temp,
                                &self.spray_temp_unit,
                                &self.bypass_spray_temp_unit,
                            );
                            Some(format!(
                                "TCV 유량 Q={:.3} m³/h, m={:.2} kg/h (ΔP={:.2} bar, Kv={:.2}) - 바이패스 스프레이 입력에 반영됨",
                                q_m3h, mass, dp, kv
                            ))
                        }
                        Err(e) => Some(format!("오류: {e}")),
                    }
                };
            }
            if let Some(res) = &self.spray_calc_result {
                ui.label(res);
            }
        });
    }
    fn ui_boiler(&mut self, ui: &mut egui::Ui) {
        heading_with_tip(
            ui,
            "Boiler Efficiency",
            "연료 투입량과 증기/급수 엔탈피로 보일러 기본 열효율(PTC) 계산",
        );
        label_with_tip(
            ui,
            "연료 투입량과 증기/급수 엔탈피로 기본 열효율 계산",
            "LHV, 증기 발생량, 급수 엔탈피, 손실 항목 등을 입력해 효율을 추산합니다.",
        );
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::Grid::new("boiler_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(ui, "연료 소비량 [unit/h]", "연료 질량 또는 체적 유량 (kg/h, Nm3/h 등)");
                    ui.add(egui::DragValue::new(&mut self.boiler_fuel_flow).speed(1.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_fuel_unit,
                        &[
                            ("kg/h", "kg/h"),
                            ("t/h", "t/h"),
                            ("kg/s", "kg/s"),
                            ("lb/h", "lb/h"),
                            ("Nm3/h", "Nm3/h"),
                        ],
                    );
                    ui.end_row();
                    label_with_tip(ui, "연료 LHV [kJ/unit]", "저위발열량 (연료 단위당 발열량)");
                    ui.add(egui::DragValue::new(&mut self.boiler_lhv).speed(100.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_lhv_unit,
                        &[
                            ("kJ/kg", "kJ/kg"),
                            ("kcal/kg", "kcal/kg"),
                            ("kJ/Nm3", "kJ/Nm3"),
                            ("Btu/lb", "Btu/lb"),
                        ],
                    );
                    ui.end_row();
                    label_with_tip(ui, "증기 발생량 [kg/h]", "보일러에서 생산되는 증기 질량유량");
                    ui.add(egui::DragValue::new(&mut self.boiler_steam_flow).speed(10.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_steam_unit,
                        &[
                            ("kg/h", "kg/h"),
                            ("t/h", "t/h"),
                            ("kg/s", "kg/s"),
                            ("lb/h", "lb/h"),
                        ],
                    );
                    ui.end_row();
                    label_with_tip(ui, "증기 엔탈피 [kJ/kg]", "생산 증기의 엔탈피 (IF97 결과를 입력해도 됨)");
                    ui.add(egui::DragValue::new(&mut self.boiler_h_steam).speed(10.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_h_steam_unit,
                        &[("kJ/kg", "kJ/kg"), ("kcal/kg", "kcal/kg"), ("Btu/lb", "Btu/lb")],
                    );
                    ui.end_row();
                    label_with_tip(ui, "급수 엔탈피 [kJ/kg]", "급수(보급수) 엔탈피");
                    ui.add(egui::DragValue::new(&mut self.boiler_h_fw).speed(5.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_h_fw_unit,
                        &[("kJ/kg", "kJ/kg"), ("kcal/kg", "kcal/kg"), ("Btu/lb", "Btu/lb")],
                    );
                    ui.end_row();
                });
            if ui.button("효율 계산").clicked() {
                let input = steam::boiler_efficiency::BoilerEfficiencyInput {
                    fuel_flow_per_h: self.boiler_fuel_flow, // 단위 변환 필요 시 확장
                    fuel_lhv_kj_per_unit: convert_energy_gui(
                        self.boiler_lhv,
                        &self.boiler_lhv_unit,
                        "kJ/kg",
                    ),
                    steam_flow_kg_per_h: convert_massflow_gui(
                        self.boiler_steam_flow,
                        &self.boiler_steam_unit,
                        "kg/h",
                    ),
                    steam_enthalpy_kj_per_kg: convert_specific_enthalpy_gui(
                        self.boiler_h_steam,
                        &self.boiler_h_steam_unit,
                        "kJ/kg",
                    ),
                    feedwater_enthalpy_kj_per_kg: convert_specific_enthalpy_gui(
                        self.boiler_h_fw,
                        &self.boiler_h_fw_unit,
                        "kJ/kg",
                    ),
                };
                let res = steam::boiler_efficiency::boiler_efficiency(input);
                self.boiler_result = Some(format!(
                    "효율={:.2} %, 유효열={:.1} kW, 연료열={:.1} kW",
                    res.efficiency * 100.0,
                    res.useful_heat_kw,
                    res.fuel_heat_kw
                ));
            }
            if let Some(res) = &self.boiler_result {
                ui.separator();
                ui.label(res);
            }
        });
        ui.add_space(10.0);
        heading_with_tip(
            ui,
            "PTC 4.0 확장 (스택/복사/블로다운 손실 포함)",
            "배가스 손실, 과잉공기, 블로다운을 포함한 확장 손실 계산",
        );
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::Grid::new("boiler_ptc_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(ui, "배가스 유량", "배기가스 질량유량");
                    ui.add(egui::DragValue::new(&mut self.boiler_fg_flow).speed(10.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_fg_flow_unit,
                        &[
                            ("kg/h", "kg/h"),
                            ("t/h", "t/h"),
                            ("kg/s", "kg/s"),
                            ("lb/h", "lb/h"),
                        ],
                    );
                    ui.end_row();

                    label_with_tip(ui, "배가스 cp [kJ/kgK]", "배기가스 평균 비열 cp");
                    ui.add(egui::DragValue::new(&mut self.boiler_fg_cp).speed(0.01));
                    ui.end_row();

                    label_with_tip(ui, "굴뚝 온도", "스택(굴뚝) 배출 온도");
                    ui.add(egui::DragValue::new(&mut self.boiler_stack_temp).speed(1.0));
                    unit_combo(ui, &mut self.boiler_temp_unit, temperature_unit_options());
                    ui.end_row();

                    label_with_tip(ui, "주변 온도", "기준/연소 공기 온도");
                    ui.add(egui::DragValue::new(&mut self.boiler_ambient_temp).speed(1.0));
                    unit_combo(ui, &mut self.boiler_temp_unit, temperature_unit_options());
                    ui.end_row();

                    ui.small("Tip: mmHg는 게이지 기준(0=대기, -760mmHg=완전진공)으로 처리됩니다.");

                    label_with_tip(ui, "과잉 공기율", "이론 공기량 대비 실제 공기량 비율");
                    ui.add(egui::DragValue::new(&mut self.boiler_excess_air).speed(0.01));
                    ui.end_row();

                    label_with_tip(ui, "복사/표면 손실 [%]", "표면 복사/대류 손실 비율");
                    ui.add(egui::DragValue::new(&mut self.boiler_rad_loss).speed(0.005));
                    ui.end_row();

                    label_with_tip(ui, "블로다운 비율", "보일러 블로다운(배수) 비율");
                    ui.add(egui::DragValue::new(&mut self.boiler_blowdown_rate).speed(0.005));
                    ui.end_row();

                    label_with_tip(ui, "블로다운 엔탈피", "블로다운 배출수 엔탈피");
                    ui.add(egui::DragValue::new(&mut self.boiler_blowdown_h).speed(5.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_blowdown_h_unit,
                        &[("kJ/kg", "kJ/kg"), ("Btu/lb", "Btu/lb")],
                    );
                    ui.end_row();
                });

            if ui.button("PTC 4.0 효율 계산").clicked() {
                let input = steam::boiler_efficiency::BoilerEfficiencyPtcInput {
                    fuel_flow_per_h: self.boiler_fuel_flow,
                    fuel_lhv_kj_per_unit: convert_energy_gui(
                        self.boiler_lhv,
                        &self.boiler_lhv_unit,
                        "kJ/kg",
                    ),
                    steam_flow_kg_per_h: convert_massflow_gui(
                        self.boiler_steam_flow,
                        &self.boiler_steam_unit,
                        "kg/h",
                    ),
                    steam_enthalpy_kj_per_kg: convert_specific_enthalpy_gui(
                        self.boiler_h_steam,
                        &self.boiler_h_steam_unit,
                        "kJ/kg",
                    ),
                    feedwater_enthalpy_kj_per_kg: convert_specific_enthalpy_gui(
                        self.boiler_h_fw,
                        &self.boiler_h_fw_unit,
                        "kJ/kg",
                    ),
                    flue_gas_flow_kg_per_h: convert_massflow_gui(
                        self.boiler_fg_flow,
                        &self.boiler_fg_flow_unit,
                        "kg/h",
                    ),
                    flue_gas_cp_kj_per_kgk: self.boiler_fg_cp,
                    stack_temp_c: convert_temperature_gui(
                        self.boiler_stack_temp,
                        &self.boiler_temp_unit,
                        "C",
                    ),
                    ambient_temp_c: convert_temperature_gui(
                        self.boiler_ambient_temp,
                        &self.boiler_temp_unit,
                        "C",
                    ),
                    excess_air_frac: self.boiler_excess_air,
                    radiation_loss_frac: self.boiler_rad_loss,
                    blowdown_rate_frac: self.boiler_blowdown_rate,
                    blowdown_enthalpy_kj_per_kg: convert_specific_enthalpy_gui(
                        self.boiler_blowdown_h,
                        &self.boiler_blowdown_h_unit,
                        "kJ/kg",
                    ),
                };
                let res = steam::boiler_efficiency::boiler_efficiency_ptc(input);
                self.boiler_result = Some(format!(
                    "PTC 효율={:.2} %, 유효열={:.1} kW, 연료열={:.1} kW",
                    res.efficiency * 100.0,
                    res.useful_heat_kw,
                    res.fuel_heat_kw
                ));
            }
            if let Some(res) = &self.boiler_result {
                ui.separator();
                ui.label(res);
            }
        });
    }

    /// 콘덴서/냉각탑/펌프 NPSH/드레인 쿨러 계산을 묶은 화면.
    fn ui_cooling(&mut self, ui: &mut egui::Ui) {
        heading_with_tip(
            ui,
            "Cooling / Condenser / NPSH / Drain Cooler",
            "복수기 열수지, 냉각탑 Range/Approach, 펌프 NPSH, 드레인/재열기 LMTD 계산",
        );
        label_with_tip(
            ui,
            "복수기 열수지, 냉각탑 Range/Approach, 펌프 NPSH, 드레인/재열기 LMTD 계산",
            "각 카드별로 필요한 값을 입력하면 즉시 계산됩니다.",
        );
        ui.add_space(8.0);

        // 콘덴서
        egui::Frame::group(ui.style()).show(ui, |ui| {
            heading_with_tip(
                ui,
                "Condenser Heat Balance / Vacuum",
                "증기 포화온도·진공·LMTD를 동시에 계산하는 복수기 카드",
            );
            ui.small("증기 포화온도/LMTD 자동 계산, mmHg는 게이지(0=대기) 해석");
            egui::Grid::new("condenser_grid")
                .num_columns(4)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.checkbox(
                        &mut self.condenser_auto_condensing_from_pressure,
                        "auto Tsat",
                    )
                    .on_hover_text("체크 시 압력으로부터 포화온도/압력을 자동 계산합니다.");
                    label_with_tip(ui, "증기 압력", "복수기 상부의 증기/불응축 가스 압력");
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_pressure).speed(0.05))
                        .changed()
                    {
                        self.condenser_auto_condensing_from_pressure = true;
                    }
                    unit_combo(ui, &mut self.condenser_pressure_unit, pressure_unit_options());
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.condenser_pressure_mode,
                            conversion::PressureMode::Gauge,
                            "Gauge (G)",
                        );
                        ui.selectable_value(
                            &mut self.condenser_pressure_mode,
                            conversion::PressureMode::Absolute,
                            "Absolute (A)",
                        );
                    });
                    ui.end_row();

                    ui.checkbox(
                        &mut self.condenser_auto_backpressure_from_temp,
                        "auto Psat",
                    )
                    .on_hover_text("체크 시 포화온도로부터 포화압을 자동 산출합니다.");
                    label_with_tip(ui, "증기 온도", "복수기 증기 온도 (포화온도 자동 계산 가능)");
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_temp_c).speed(0.5))
                        .changed()
                    {
                        self.condenser_auto_condensing_from_pressure = false;
                        self.condenser_auto_backpressure_from_temp = false;
                        self.condenser_use_manual_temp = true;
                    }
                    unit_combo(ui, &mut self.condenser_cw_temp_unit, temperature_unit_options());
                    ui.checkbox(&mut self.condenser_use_manual_temp, "직접 입력");
                    ui.end_row();

                    ui.checkbox(
                        &mut self.condenser_auto_cw_out_from_range,
                        "auto Tout",
                    )
                    .on_hover_text("체크 시 Range(입구-출구 목표)로 출구 온도를 자동 계산합니다.");
                    label_with_tip(ui, "냉각수 입구/출구", "순환 냉각수의 입구/출구 온도 (auto Range 계산 가능)");
                    ui.add(egui::DragValue::new(&mut self.condenser_cw_in).speed(0.5));
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_cw_out).speed(0.5))
                        .changed()
                    {
                        self.condenser_auto_cw_out_from_range = false;
                    }
                    unit_combo(ui, &mut self.condenser_cw_temp_unit, temperature_unit_options());
                    ui.end_row();

                    label_with_tip(ui, "Range 목표(입구-출구)", "냉각수 입구-출구 온도 차 목표치");
                    ui.add(egui::DragValue::new(&mut self.ct_range_target).speed(0.2));
                    ui.label("°C");
                    ui.end_row();

                    ui.label("");
                    label_with_tip(ui, "냉각수 유량", "순환 냉각수 유량");
                    ui.add(egui::DragValue::new(&mut self.condenser_cw_flow).speed(5.0));
                    unit_combo(
                        ui,
                        &mut self.condenser_cw_flow_unit,
                        &[
                            ("m3/h", "m3/h"),
                            ("kg/h", "kg/h"),
                            ("t/h", "t/h"),
                            ("kg/s", "kg/s"),
                            ("lb/h", "lb/h"),
                            ("gpm", "gpm"),
                        ],
                    );
                    ui.end_row();

                    ui.checkbox(&mut self.condenser_auto_ua_from_area_u, "auto UA")
                        .on_hover_text("체크 시 면적×U로 UA를 자동 계산합니다.");
                    label_with_tip(ui, "UA [kW/K]", "전열면적×전달계수");
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_ua).speed(1.0))
                        .changed()
                    {
                        self.condenser_auto_ua_from_area_u = false;
                    }
                    ui.end_row();

                    ui.checkbox(
                        &mut self.condenser_auto_area_required,
                        "auto 면적(요구)",
                    )
                    .on_hover_text("체크 시 필요한 면적을 계산합니다. 해제하면 입력한 면적을 검증에 사용합니다.");
                    label_with_tip(ui, "면적/ U", "전열면적, U값을 직접 입력하여 검증");
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_area).speed(0.5))
                        .changed()
                    {
                        self.condenser_auto_area_required = false;
                    }
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_u).speed(5.0))
                        .changed()
                    {
                        // keep flag as-is; user may still want auto area from new U
                    }
                    ui.label("area[m²], U[W/m²K]");
                    ui.end_row();

                    ui.checkbox(&mut self.condenser_auto_backpressure_from_temp, "auto 배압");
                    label_with_tip(ui, "목표 배압", "압축기/터빈 배압 목표를 입력하거나 Tsat로부터 자동 계산");
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_backpressure).speed(0.05))
                        .changed()
                    {
                        self.condenser_auto_backpressure_from_temp = false;
                    }
                    unit_combo(
                        ui,
                        &mut self.condenser_backpressure_unit,
                        pressure_unit_options(),
                    );
                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.condenser_backpressure_mode,
                            conversion::PressureMode::Absolute,
                            "Absolute (A)",
                        );
                        ui.selectable_value(
                            &mut self.condenser_backpressure_mode,
                            conversion::PressureMode::Gauge,
                            "Gauge (G)",
                        );
                    });
                    ui.end_row();
                });
            ui.collapsing("입력 도움말", |ui| {
                ui.label("배압/포화압: 포화압력 = 응축기 진공. Gauge는 대기 기준.");
                ui.label("UA: U[W/m²K] × Area[m²] / 1000 = kW/K.");
                ui.label("Range: 냉각수 입구-출구 온도차. auto 체크 시 출구온도 자동 산출.");
                ui.label("mmHg는 게이지(0=대기, -760=진공) 해석.");
            });
            if ui.button("콘덴서 계산").clicked() {
                // 입력값 보정/자동산출
                let mut steam_temp_c = if self.condenser_use_manual_temp {
                    Some(convert_temperature_gui(
                        self.condenser_temp_c,
                        &self.condenser_cw_temp_unit,
                        "C",
                    ))
                } else {
                    None
                };
                let cw_flow_m3h = convert_flow_gui(
                    self.condenser_cw_flow,
                    &self.condenser_cw_flow_unit,
                    "kg/m3",
                    1000.0,
                );
                // 증기 압력 절대값
                let steam_p_abs = convert_pressure_mode_gui(
                    self.condenser_pressure,
                    &self.condenser_pressure_unit,
                    self.condenser_pressure_mode,
                    "bar",
                    conversion::PressureMode::Absolute,
                );
                if self.condenser_auto_condensing_from_pressure {
                    if let Ok(tsat) = steam::if97::saturation_temp_c_from_pressure_bar_abs(steam_p_abs) {
                        steam_temp_c = Some(tsat);
                        // UI 표시 업데이트
                        self.condenser_temp_c =
                            convert_temperature_gui(tsat, "C", &self.condenser_cw_temp_unit);
                    }
                }
                // 배압 자동: 포화온도에서 포화압력 계산
                let mut backpressure_abs = if self.condenser_backpressure > 0.0 {
                    Some(convert_pressure_mode_gui(
                        self.condenser_backpressure,
                        &self.condenser_backpressure_unit,
                        self.condenser_backpressure_mode,
                        "bar",
                        conversion::PressureMode::Absolute,
                    ))
                } else {
                    None
                };
                if self.condenser_auto_backpressure_from_temp {
                    if let Some(t) = steam_temp_c {
                        if let Ok(psat) = steam::if97::saturation_pressure_bar_abs_from_temp_c(t) {
                            backpressure_abs = Some(psat);
                            // UI 업데이트
                            let p_disp = convert_pressure_mode_gui(
                                psat,
                                "bar",
                                conversion::PressureMode::Absolute,
                                &self.condenser_backpressure_unit,
                                self.condenser_backpressure_mode,
                            );
                            self.condenser_backpressure = p_disp;
                        }
                    }
                }

                // Range 기반 냉각수 출구 자동
                let cw_in_c =
                    convert_temperature_gui(self.condenser_cw_in, &self.condenser_cw_temp_unit, "C");
                let mut cw_out_c = convert_temperature_gui(
                    self.condenser_cw_out,
                    &self.condenser_cw_temp_unit,
                    "C",
                );
                if self.condenser_auto_cw_out_from_range {
                    cw_out_c = cw_in_c - self.ct_range_target;
                    self.condenser_cw_out =
                        convert_temperature_gui(cw_out_c, "C", &self.condenser_cw_temp_unit);
                }

                // UA 자동
                let mut ua = if self.condenser_ua > 0.0 {
                    Some(self.condenser_ua)
                } else {
                    None
                };
                if self.condenser_auto_ua_from_area_u && self.condenser_area > 0.0 && self.condenser_u > 0.0 {
                    ua = Some(self.condenser_area * self.condenser_u / 1000.0);
                    self.condenser_ua = ua.unwrap();
                }
                let area = if self.condenser_area > 0.0 {
                    Some(self.condenser_area)
                } else {
                    None
                };
                let u = if self.condenser_u > 0.0 {
                    Some(self.condenser_u)
                } else {
                    None
                };

                let result = condenser::compute_condenser(condenser::CondenserInput {
                    steam_pressure: self.condenser_pressure,
                    steam_pressure_unit: parse_pressure_unit_gui(&self.condenser_pressure_unit),
                    steam_pressure_mode: self.condenser_pressure_mode,
                    steam_temp_c,
                    cw_inlet_temp_c: cw_in_c,
                    cw_outlet_temp_c: cw_out_c,
                    cw_flow_m3_per_h: cw_flow_m3h,
                    ua_kw_per_k: ua,
                    area_m2: area,
                    overall_u_w_m2k: u,
                    target_back_pressure_bar_abs: backpressure_abs,
                });
                self.condenser_result = Some(match result {
                    Ok(res) => {
                        let cond_temp_out =
                            convert_temperature_gui(res.condensing_temp_c, "C", &self.condenser_cw_temp_unit);
                        let cond_press_out = convert_pressure_mode_gui(
                            res.condensing_pressure_bar_abs,
                            "bar",
                            conversion::PressureMode::Absolute,
                            &self.condenser_pressure_unit,
                            self.condenser_pressure_mode,
                        );
                        let mut text = format!(
                            "Tsat={:.2} {}, Psat={:.4} {}{}, LMTD={:.2} K, Q~{:.1} kW",
                            cond_temp_out,
                            self.condenser_cw_temp_unit,
                            cond_press_out,
                            self.condenser_pressure_unit,
                            if self.condenser_pressure_mode == conversion::PressureMode::Gauge {
                                "g"
                            } else {
                                "a"
                            },
                            res.lmtd_k,
                            res.heat_duty_kw
                        );
                        if !res.warnings.is_empty() {
                            text.push_str("\n경고: ");
                            text.push_str(&res.warnings.join(" / "));
                        }
                        // 면적/UA 관련 추가 정보
                        if self.condenser_auto_area_required && self.condenser_u > 0.0 {
                            let area_req =
                                (res.heat_duty_kw * 1000.0) / (self.condenser_u * res.lmtd_k.max(1e-6));
                            self.condenser_area = area_req;
                            text.push_str(&format!("\n요구 면적~{:.2} m² (U={:.1} W/m²K)", area_req, self.condenser_u));
                        } else if !self.condenser_auto_area_required
                            && self.condenser_area > 0.0
                            && self.condenser_u > 0.0
                        {
                            let q_cap = self.condenser_area * self.condenser_u * res.lmtd_k / 1000.0;
                            let load_ratio = if q_cap.abs() > 1e-6 {
                                res.heat_duty_kw / q_cap
                            } else {
                                0.0
                            };
                            let capable_pct = if res.heat_duty_kw.abs() > 1e-6 {
                                (q_cap / res.heat_duty_kw).clamp(0.0, 10.0) * 100.0
                            } else {
                                0.0
                            };
                            text.push_str(&format!(
                                "\n입력 면적={:.2} m², U={:.1} W/m²K 기준 Qcap~{:.1} kW, 부하비~{:.2}x",
                                self.condenser_area, self.condenser_u, q_cap, load_ratio
                            ));
                            if load_ratio > 1.0 {
                                text.push_str(&format!(
                                    "\n⚠ 현재 부하가 설계 용량을 초과합니다. 현 조건에서 약 {:.0}% 수준까지 운전 가능(Qcap 기준). 냉각수 온도/유량 개선 또는 면적/U 증대 필요.",
                                    capable_pct
                                ));
                            } else {
                                text.push_str("\n정상 운전 예상(부하 ≤ 설계 용량).");
                            }
                        }
                        text
                    }
                    Err(e) => match e {
                        condenser::CoolingError::NegativeDeltaT => {
                            "오류: 냉각수 온도와 포화온도가 역전되었습니다.".into()
                        }
                        condenser::CoolingError::If97(msg) => format!("포화 계산 오류: {msg}"),
                    },
                });
            }
            if let Some(res) = &self.condenser_result {
                ui.separator();
                for line in res.lines() {
                    if line.starts_with("경고:") {
                        ui.colored_label(ui.visuals().warn_fg_color, line);
                    } else {
                        ui.label(line);
                    }
                }
            }
        });

        ui.add_space(8.0);
        // 냉각탑
        egui::Frame::group(ui.style()).show(ui, |ui| {
            heading_with_tip(
                ui,
                "Cooling Tower (Range / Approach)",
                "순환수 Range/Approach, 열량, 경고를 산출하는 간단 냉각탑 카드",
            );
            egui::Grid::new("ct_grid")
                .num_columns(3)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    label_with_tip(ui, "순환수 입구/출구", "Cooling tower 입구/출구 순환수 온도");
                    ui.add(egui::DragValue::new(&mut self.ct_in).speed(0.5));
                    ui.add(egui::DragValue::new(&mut self.ct_out).speed(0.5));
                    unit_combo(ui, &mut self.ct_temp_unit, temperature_unit_options());
                    ui.end_row();
                    label_with_tip(ui, "대기 DB/WB", "건구/습구 온도");
                    ui.add(egui::DragValue::new(&mut self.ct_db).speed(0.5));
                    ui.add(egui::DragValue::new(&mut self.ct_wb).speed(0.5));
                    unit_combo(ui, &mut self.ct_temp_unit, temperature_unit_options());
                    ui.end_row();
                    label_with_tip(ui, "순환수 유량", "순환수 유량 (m3/h 또는 gpm)");
                    ui.add(egui::DragValue::new(&mut self.ct_flow).speed(5.0));
                    unit_combo(
                        ui,
                        &mut self.ct_flow_unit,
                        &[("m3/h", "m3/h"), ("gpm", "gpm")],
                    );
                    ui.end_row();
                    label_with_tip(ui, "Range/Approach 목표", "목표 Range(입구-출구)와 Approach(출구-습구)");
                    ui.add(egui::DragValue::new(&mut self.ct_range_target).speed(0.2));
                    ui.add(egui::DragValue::new(&mut self.ct_approach_target).speed(0.2));
                    ui.label("°C");
                    ui.end_row();
                });
            if ui.button("냉각탑 계산").clicked() {
                let t_in = convert_temperature_gui(self.ct_in, &self.ct_temp_unit, "C");
                let t_out = convert_temperature_gui(self.ct_out, &self.ct_temp_unit, "C");
                let wb = convert_temperature_gui(self.ct_wb, &self.ct_temp_unit, "C");
                let db = convert_temperature_gui(self.ct_db, &self.ct_temp_unit, "C");
                let flow_m3h =
                    if self.ct_flow_unit.eq_ignore_ascii_case("gpm") { self.ct_flow * 0.2271247 } else { self.ct_flow };
                let res = cooling_tower::compute_cooling_tower(cooling_tower::CoolingTowerInput {
                    water_in_c: t_in,
                    water_out_c: t_out,
                    dry_bulb_c: db,
                    wet_bulb_c: wb,
                    water_flow_m3_per_h: flow_m3h,
                    target_range_c: Some(self.ct_range_target),
                    target_approach_c: Some(self.ct_approach_target),
                });
                let mut msg = format!(
                    "Range={:.2} K, Approach={:.2} K, 열량~{:.1} kW",
                    res.range_c, res.approach_c, res.heat_rejected_kw
                );
                if !res.warnings.is_empty() {
                    msg.push_str("\n경고: ");
                    msg.push_str(&res.warnings.join(" / "));
                }
                self.ct_result = Some(msg);
            }
            if let Some(res) = &self.ct_result {
                ui.separator();
                for line in res.lines() {
                    if line.starts_with("경고:") {
                        ui.colored_label(ui.visuals().warn_fg_color, line);
                    } else {
                        ui.label(line);
                    }
                }
                ui.small("참고: Range=입구-출구, Approach=출구-습구. Approach<2°C는 비현실적일 수 있습니다.");
            }
        });

        ui.add_space(8.0);
        // 펌프 NPSH
        egui::Frame::group(ui.style()).show(ui, |ui| {
            heading_with_tip(
                ui,
                "Circulating Pump NPSH / Margin",
                "흡입 조건에서 NPSHa를 계산하고 NPSHr 대비 여유를 확인",
            );
            egui::Grid::new("npsh_grid")
                .num_columns(3)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    label_with_tip(ui, "흡입 압력", "펌프 흡입 측 압력 (게이지/절대)");
                    ui.add(egui::DragValue::new(&mut self.npsh_suction_p).speed(0.1));
                    unit_combo(ui, &mut self.npsh_suction_unit, pressure_unit_options());
                    ui.selectable_value(
                        &mut self.npsh_suction_mode,
                        conversion::PressureMode::Gauge,
                        "Gauge (G)",
                    );
                    ui.selectable_value(
                        &mut self.npsh_suction_mode,
                        conversion::PressureMode::Absolute,
                        "Absolute (A)",
                    );
                    ui.end_row();

                    label_with_tip(ui, "수온", "흡입수 온도 (증기압 계산)");
                    ui.add(egui::DragValue::new(&mut self.npsh_temp).speed(0.5));
                    unit_combo(ui, &mut self.npsh_temp_unit, temperature_unit_options());
                    ui.end_row();

                    label_with_tip(ui, "정수두 / 마찰손실 [m]", "흡입면에서 펌프까지 정수두 / 손실수두");
                    ui.add(egui::DragValue::new(&mut self.npsh_static_head).speed(0.2));
                    ui.add(egui::DragValue::new(&mut self.npsh_friction).speed(0.2));
                    ui.end_row();

                    label_with_tip(ui, "밀도 / NPSHr", "흡입수 밀도와 제조사 제시 NPSHr");
                    ui.add(egui::DragValue::new(&mut self.npsh_rho).speed(1.0));
                    unit_combo(
                        ui,
                        &mut self.npsh_rho_unit,
                        &[("kg/m3", "kg/m3"), ("lb/ft3", "lb/ft3")],
                    );
                    ui.add(egui::DragValue::new(&mut self.npsh_required).speed(0.2));
                    ui.end_row();
                });
            if ui.button("NPSH 계산").clicked() {
                let rho = convert_density_gui(self.npsh_rho, &self.npsh_rho_unit, "kg/m3");
                let p_bar = convert_pressure_mode_gui(
                    self.npsh_suction_p,
                    &self.npsh_suction_unit,
                    self.npsh_suction_mode,
                    "bar",
                    self.npsh_suction_mode,
                );
                let t_c = convert_temperature_gui(self.npsh_temp, &self.npsh_temp_unit, "C");
                let res = pump_npsh::compute_pump_npsh(pump_npsh::PumpNpshInput {
                    suction_pressure_bar: p_bar,
                    suction_is_abs: self.npsh_suction_mode == conversion::PressureMode::Absolute,
                    liquid_temp_c: t_c,
                    static_head_m: self.npsh_static_head,
                    friction_loss_m: self.npsh_friction,
                    npshr_m: self.npsh_required,
                    rho_kg_m3: rho,
                });
                let mut msg = format!(
                    "NPSHa={:.2} m, Margin={:.2}",
                    res.npsha_m, res.margin_ratio
                );
                if !res.warnings.is_empty() {
                    msg.push_str("\n경고: ");
                    msg.push_str(&res.warnings.join(" / "));
                }
                self.npsh_result = Some(msg);
            }
            if let Some(res) = &self.npsh_result {
                ui.separator();
                for line in res.lines() {
                    if line.starts_with("경고:") {
                        ui.colored_label(ui.visuals().warn_fg_color, line);
                    } else {
                        ui.label(line);
                    }
                }
                ui.small("참고: Margin<1.1이면 공동현상 위험이 큽니다. 흡입압 상승/온도 저하/마찰손실 감소를 검토하십시오.");
            }
        });

        ui.add_space(8.0);
        // 드레인/재열기
        egui::Frame::group(ui.style()).show(ui, |ui| {
            heading_with_tip(
                ui,
                "Drain Cooler / Reheater Heat Balance",
                "쉘/튜브 입출구 온도·유량으로 LMTD와 열수지를 계산",
            );
            egui::Grid::new("drain_grid")
                .num_columns(3)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    label_with_tip(ui, "쉘 IN/OUT", "쉘측 입구/출구 온도");
                    ui.add(egui::DragValue::new(&mut self.drain_shell_in).speed(0.5));
                    ui.add(egui::DragValue::new(&mut self.drain_shell_out).speed(0.5));
                    unit_combo(ui, &mut self.drain_temp_unit, temperature_unit_options());
                    ui.end_row();
                    label_with_tip(ui, "튜브 IN/OUT", "튜브측 입구/출구 온도");
                    ui.add(egui::DragValue::new(&mut self.drain_tube_in).speed(0.5));
                    ui.add(egui::DragValue::new(&mut self.drain_tube_out).speed(0.5));
                    unit_combo(ui, &mut self.drain_temp_unit, temperature_unit_options());
                    ui.end_row();
                    label_with_tip(ui, "쉘/튜브 유량", "쉘측/튜브측 유량");
                    ui.add(egui::DragValue::new(&mut self.drain_shell_flow).speed(1.0));
                    ui.add(egui::DragValue::new(&mut self.drain_tube_flow).speed(1.0));
                    unit_combo(
                        ui,
                        &mut self.drain_flow_unit,
                        &[("m3/h", "m3/h"), ("gpm", "gpm")],
                    );
                    ui.end_row();
                    label_with_tip(ui, "UA 또는 면적/U", "UA 직접 입력 또는 면적/U를 입력해 UA 산출");
                    ui.add(egui::DragValue::new(&mut self.drain_ua).speed(1.0));
                    ui.add(egui::DragValue::new(&mut self.drain_area).speed(0.5));
                    ui.add(egui::DragValue::new(&mut self.drain_u).speed(5.0));
                    ui.end_row();
                });
            if ui.button("열수지 계산").clicked() {
                let flow_shell_m3h = if self.drain_flow_unit.eq_ignore_ascii_case("gpm") {
                    self.drain_shell_flow * 0.2271247
                } else {
                    self.drain_shell_flow
                };
                let flow_tube_m3h = if self.drain_flow_unit.eq_ignore_ascii_case("gpm") {
                    self.drain_tube_flow * 0.2271247
                } else {
                    self.drain_tube_flow
                };
                let t_in_shell =
                    convert_temperature_gui(self.drain_shell_in, &self.drain_temp_unit, "C");
                let t_out_shell =
                    convert_temperature_gui(self.drain_shell_out, &self.drain_temp_unit, "C");
                let t_in_tube =
                    convert_temperature_gui(self.drain_tube_in, &self.drain_temp_unit, "C");
                let t_out_tube =
                    convert_temperature_gui(self.drain_tube_out, &self.drain_temp_unit, "C");
                let res = drain_cooler::compute_drain_cooler(drain_cooler::DrainCoolerInput {
                    shell_in_c: t_in_shell,
                    shell_out_c: t_out_shell,
                    shell_flow_m3_per_h: flow_shell_m3h,
                    tube_in_c: t_in_tube,
                    tube_out_c: t_out_tube,
                    tube_flow_m3_per_h: flow_tube_m3h,
                    ua_kw_per_k: if self.drain_ua > 0.0 {
                        Some(self.drain_ua)
                    } else {
                        None
                    },
                    area_m2: if self.drain_area > 0.0 {
                        Some(self.drain_area)
                    } else {
                        None
                    },
                    overall_u_w_m2k: if self.drain_u > 0.0 { Some(self.drain_u) } else { None },
                });
                let mut msg = format!(
                    "LMTD={:.2} K, 쉘 Q={:.1} kW, 튜브 Q={:.1} kW, 불균형={:.1} kW",
                    res.lmtd_k, res.shell_heat_kw, res.tube_heat_kw, res.imbalance_kw
                );
                if !res.warnings.is_empty() {
                    msg.push_str("\n경고: ");
                    msg.push_str(&res.warnings.join(" / "));
                }
                self.drain_result = Some(msg);
            }
            if let Some(res) = &self.drain_result {
                ui.separator();
                for line in res.lines() {
                    if line.starts_with("경고:") {
                        ui.colored_label(ui.visuals().warn_fg_color, line);
                    } else {
                        ui.label(line);
                    }
                }
            }
        });
    }

    /// 플랜트 배관: 오리피스/노즐 유량 점검 + 재질별 열팽창 계산
    fn ui_plant_piping(&mut self, ui: &mut egui::Ui) {
        heading_with_tip(ui, "Plant Piping", "오리피스/노즐 유량, 열팽창, 내압 계산");
        label_with_tip(
            ui,
            "오리피스·노즐 유량 점검, 재질별 열팽창 계산",
            "압축성 보정(Y), 열팽창량, 내압까지 한 화면에서 계산",
        );
        ui.add_space(8.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            heading_with_tip(ui, "Orifice / Nozzle 유량 점검", "차압식 유량계 또는 노즐 유량 검증");
            egui::Grid::new("plant_orifice")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(ui, "상류 압력", "노즐/오리피스 상류 압력 (게이지/절대)");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.plant_up_p).speed(0.1));
                        unit_combo(ui, &mut self.plant_up_unit, pressure_unit_options());
                        ui.selectable_value(
                            &mut self.plant_up_mode,
                            conversion::PressureMode::Gauge,
                            "Gauge (G)",
                        );
                        ui.selectable_value(
                            &mut self.plant_up_mode,
                            conversion::PressureMode::Absolute,
                            "Absolute (A)",
                        );
                    });
                    ui.end_row();

                    label_with_tip(ui, "차압", "오리피스 양단의 압력차 ΔP");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.plant_dp).speed(0.1));
                        unit_combo(ui, &mut self.plant_dp_unit, pressure_unit_options());
                        ui.selectable_value(
                            &mut self.plant_dp_mode,
                            conversion::PressureMode::Gauge,
                            "Gauge (G)",
                        );
                        ui.selectable_value(
                            &mut self.plant_dp_mode,
                            conversion::PressureMode::Absolute,
                            "Absolute (A)",
                        );
                    });
                    ui.end_row();

                    label_with_tip(ui, "유체 밀도 [kg/m3]", "운전 조건에서의 밀도");
                    ui.add(egui::DragValue::new(&mut self.plant_rho).speed(1.0));
                    ui.end_row();

                    label_with_tip(ui, "지름", "오리피스/노즐 유효 지름 (m 또는 mm)");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.plant_diameter_m)
                                .speed(0.001)
                                .clamp_range(0.0..=5.0),
                        );
                        unit_combo(ui, &mut self.plant_diam_unit, &[("m", "m"), ("mm", "mm")]);
                    });
                    ui.end_row();

                    label_with_tip(ui, "형상 / Cd", "형상별 Cd 기본값 선택 후 필요시 미세 조정");
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_source("plant_shape")
                            .selected_text(&self.plant_shape)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_value(&mut self.plant_shape, "Orifice (sharp)".into(), "Orifice (sharp)")
                                    .clicked()
                                {
                                    self.plant_cd = 0.62;
                                }
                                if ui
                                    .selectable_value(
                                        &mut self.plant_shape,
                                        "Nozzle (ISA)".into(),
                                        "Nozzle (ISA)",
                                    )
                                    .clicked()
                                {
                                    self.plant_cd = 0.98;
                                }
                                if ui
                                    .selectable_value(
                                        &mut self.plant_shape,
                                        "Venturi".into(),
                                        "Venturi",
                                    )
                                    .clicked()
                                {
                                    self.plant_cd = 0.97;
                                }
                            });
                        ui.add(
                            egui::DragValue::new(&mut self.plant_cd)
                                .speed(0.01)
                                .clamp_range(0.1..=1.5),
                        );
                    });
                    ui.end_row();

                    label_with_tip(ui, "Beta(지름비) / k(비열비)", "beta=오리피스/관 지름비, k=비열비(γ)");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.plant_beta)
                                .speed(0.01)
                                .clamp_range(0.1..=0.99),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.plant_gamma)
                                .speed(0.05)
                                .clamp_range(1.0..=1.7),
                        );
                    });
                    ui.end_row();

                    label_with_tip(ui, "압축성 보정 사용", "증기/가스 유량 시 Y 계수 보정 적용");
                    ui.checkbox(&mut self.plant_compressible, "Compressible (Y 적용)");
                    ui.end_row();
                });
            if ui.button("유량 계산").clicked() {
                let dp_bar = convert_pressure_mode_gui(
                    self.plant_dp,
                    &self.plant_dp_unit,
                    self.plant_dp_mode,
                    "bar",
                    conversion::PressureMode::Gauge,
                );
                let pu_bar_abs = convert_pressure_mode_gui(
                    self.plant_up_p,
                    &self.plant_up_unit,
                    self.plant_up_mode,
                    "bar",
                    conversion::PressureMode::Absolute,
                );
                let d_m = convert_length_gui(self.plant_diameter_m, &self.plant_diam_unit, "m");
                if dp_bar <= 0.0 || self.plant_rho <= 0.0 || d_m <= 0.0 {
                    self.plant_result = Some("입력 오류: ΔP, 밀도, 지름은 0보다 커야 합니다.".into());
                } else {
                    let dp_pa = dp_bar * 1.0e5;
                    let area = std::f64::consts::PI * (d_m.powi(2)) / 4.0;
                    if self.plant_compressible {
                        if pu_bar_abs <= dp_bar {
                            self.plant_result =
                                Some("입력 오류: 상류 압력이 차압보다 커야 합니다 (압축성 계산)".into());
                        } else {
                            let beta = self.plant_beta.clamp(0.1, 0.99);
                            let k = self.plant_gamma.clamp(1.0, 1.7);
                            let ratio = (dp_bar / pu_bar_abs).min(0.9);
                            let y = (1.0 - (0.41 + 0.35 * beta * beta) * ratio).clamp(0.1, 1.0);
                            let c = self.plant_cd / (1.0 - beta.powi(4)).sqrt();
                            let m_kg_s = c * y * area * (2.0 * self.plant_rho * dp_pa).sqrt();
                            let m_kg_h = m_kg_s * 3600.0;
                            let q_m3_h = m_kg_h / self.plant_rho;
                            self.plant_result = Some(format!(
                                "압축성: Q ~ {:.3} m³/h, m ~ {:.2} kg/h (Cd={:.2}, Y={:.3}, beta={:.2}, k={:.2}, dp={:.3} bar)",
                                q_m3_h, m_kg_h, self.plant_cd, y, beta, k, dp_bar
                            ));
                        }
                    } else {
                        let q_m3_s = self.plant_cd * area * (2.0 * dp_pa / self.plant_rho).sqrt();
                        let q_m3_h = q_m3_s * 3600.0;
                        let m_kg_h = q_m3_h * self.plant_rho;
                        self.plant_result = Some(format!(
                            "비압축성: Q ~ {:.3} m³/h, m ~ {:.2} kg/h (Cd={:.2}, dp={:.3} bar)",
                            q_m3_h, m_kg_h, self.plant_cd, dp_bar
                        ));
                    }
                }
            }
            if let Some(res) = &self.plant_result {
                ui.label(res);
                ui.label("식: 비압축성 Q = Cd·A·√(2·ΔP/ρ), 압축성은 Y·C(1-β⁴)^-0.5 보정 적용");
            }
        });

        ui.add_space(10.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            heading_with_tip(
                ui,
                "열팽창/수축 (ASTM Power Piping)",
                "배관 길이와 ΔT로 열팽창/수축량을 산출",
            );
            egui::Grid::new("plant_expansion")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(ui, "재질", "선팽창계수 기본값을 자동 적용할 배관 재질");
                    egui::ComboBox::from_id_source("plant_mat")
                        .selected_text(&self.plant_mat)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.plant_mat, "ASTM A106 Gr.B".into(), "ASTM A106 Gr.B");
                            ui.selectable_value(&mut self.plant_mat, "ASTM A53 Gr.B".into(), "ASTM A53 Gr.B");
                            ui.selectable_value(&mut self.plant_mat, "ASTM A312 TP304".into(), "ASTM A312 TP304");
                            ui.selectable_value(&mut self.plant_mat, "ASTM A312 TP316".into(), "ASTM A312 TP316");
                            ui.selectable_value(&mut self.plant_mat, "ASTM A312 TP304L".into(), "ASTM A312 TP304L");
                            ui.selectable_value(&mut self.plant_mat, "ASTM A312 TP316L".into(), "ASTM A312 TP316L");
                            ui.selectable_value(&mut self.plant_mat, "ASTM A335 P11".into(), "ASTM A335 P11");
                            ui.selectable_value(&mut self.plant_mat, "ASTM A335 P12".into(), "ASTM A335 P12");
                            ui.selectable_value(&mut self.plant_mat, "ASTM A335 P91".into(), "ASTM A335 P91");
                            ui.selectable_value(&mut self.plant_mat, "ASTM A335 P92".into(), "ASTM A335 P92");
                        });
                    ui.end_row();

                    label_with_tip(ui, "길이 [m]", "온도 변화가 적용되는 직선 구간 길이");
                    ui.add(egui::DragValue::new(&mut self.plant_length_m).speed(0.1));
                    ui.end_row();

                    label_with_tip(ui, "온도 변화 ΔT [K]", "배관이 겪는 온도 변화량");
                    ui.add(egui::DragValue::new(&mut self.plant_delta_t).speed(1.0));
                    ui.end_row();

                    label_with_tip(
                        ui,
                        "선팽창계수 α [1/K] (0=재질 기본)",
                        "0이면 재질 기본, 입력 시 강제 적용",
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.plant_alpha_override)
                            .speed(1e-6)
                            .clamp_range(0.0..=5e-5),
                    );
                    ui.end_row();
                });
            if ui.button("팽창/수축 계산").clicked() {
                let alpha_default = match self.plant_mat.as_str() {
                    "ASTM A106 Gr.B" | "ASTM A53 Gr.B" => 12.0e-6,
                    "ASTM A335 P11" => 11.8e-6,
                    "ASTM A335 P12" => 11.7e-6,
                    "ASTM A335 P91" => 11.0e-6,
                    "ASTM A335 P92" => 11.0e-6,
                    "ASTM A312 TP304" => 17.3e-6,
                    "ASTM A312 TP304L" => 17.2e-6,
                    "ASTM A312 TP316" => 16.0e-6,
                    "ASTM A312 TP316L" => 16.0e-6,
                    _ => 12.0e-6,
                };
                let alpha = if self.plant_alpha_override > 0.0 {
                    self.plant_alpha_override
                } else {
                    alpha_default
                };
                if self.plant_length_m <= 0.0 {
                    self.plant_expansion_result = Some("입력 오류: 길이는 0보다 커야 합니다.".into());
                } else {
                    let delta_l_m = alpha * self.plant_length_m * self.plant_delta_t;
                    let delta_l_mm = delta_l_m * 1000.0;
                    self.plant_expansion_result = Some(format!(
                        "ΔL ~ {:.4} m (~ {:.2} mm) @ α={:.2}e-6 1/K, ΔT={:.1} K",
                        delta_l_m,
                        delta_l_mm,
                        alpha * 1e6,
                        self.plant_delta_t
                    ));
                }
            }
            if let Some(res) = &self.plant_expansion_result {
                ui.label(res);
                ui.label("참고: ASTM Power Piping 탄소강 ~12e-6/K, 스테인리스 ~16-17e-6/K");
            }
        });

        ui.add_space(10.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading("재질 강도 기반 내압 추정 (얇은/두꺼운 관 자동 판정)");
            egui::Grid::new("plant_pressure_rating")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(ui, "재질 선택", "허용응력 S가 이미 반영된 재질을 선택");
                    egui::ComboBox::from_id_source("plant_strength_mat")
                        .selected_text(&self.plant_mat)
                        .show_ui(ui, |ui| {
                            for (label, s_allow) in [
                                ("ASTM A106 Gr.B", 138.0),
                                ("ASTM A53 Gr.B", 138.0),
                                ("ASTM A335 P11", 120.0),
                                ("ASTM A335 P12", 110.0),
                                ("ASTM A335 P91", 165.0),
                                ("ASTM A335 P92", 170.0),
                                ("ASTM A312 TP304", 138.0),
                                ("ASTM A312 TP304L", 110.0),
                                ("ASTM A312 TP316", 138.0),
                                ("ASTM A312 TP316L", 110.0),
                            ] {
                                if ui.selectable_label(false, label).clicked() {
                                    self.plant_mat = label.to_string();
                                    self.plant_allow_stress_mpa = s_allow;
                                }
                            }
                        });
                    ui.end_row();

                    label_with_tip(ui, "허용응력 S [MPa]", "코드에 명시된 온도별 허용응력 값을 입력/수정");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.plant_allow_stress_mpa).speed(2.0));
                    });
                    ui.end_row();

                    label_with_tip(ui, "파이프 외경 / 두께", "배관 외경과 두께(설계 기준값)");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.plant_pipe_od_m).speed(0.001));
                        ui.add(egui::DragValue::new(&mut self.plant_wall_thk_m).speed(0.0005));
                        egui::ComboBox::from_id_source("plant_dim_unit")
                            .selected_text(&self.plant_dim_unit)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.plant_dim_unit, "mm".into(), "mm");
                                ui.selectable_value(&mut self.plant_dim_unit, "in".into(), "inch");
                            });
                    });
                    ui.end_row();

                    label_with_tip(
                        ui,
                        "부식여유 / 밀 톨 / 용접 효율 E / 설계계수 F",
                        "CA: 부식여유, Mill tol: 제조 공차, E: 용접효율, F: 설계계수(지역코드/설계 여부 반영)",
                    );
                    ui.horizontal(|ui| {
                        let mut ca_disp = if self.plant_dim_unit.eq_ignore_ascii_case("in") {
                            self.plant_corrosion_allow_m / 0.0254
                        } else {
                            self.plant_corrosion_allow_m * 1000.0
                        };
                        let mut mill_pct = self.plant_mill_tol_frac * 100.0;
                        ui.label("CA");
                        ui.add(egui::DragValue::new(&mut ca_disp).speed(0.1).clamp_range(0.0..=20.0));
                        ui.label(if self.plant_dim_unit.eq_ignore_ascii_case("in") {
                            "inch"
                        } else {
                            "mm"
                        });
                        ui.separator();
                        ui.label("밀 톨[%]");
                        ui.add(
                            egui::DragValue::new(&mut mill_pct)
                                .speed(0.5)
                                .clamp_range(0.0..=30.0),
                        );
                        ui.separator();
                        ui.label("E");
                        ui.add(
                            egui::DragValue::new(&mut self.plant_weld_eff)
                                .speed(0.01)
                                .clamp_range(0.1..=1.0),
                        );
                        ui.label("F");
                        ui.add(
                            egui::DragValue::new(&mut self.plant_design_factor)
                                .speed(0.01)
                                .clamp_range(0.1..=1.0),
                        );

                        self.plant_corrosion_allow_m = if self.plant_dim_unit.eq_ignore_ascii_case("in") {
                            ca_disp * 0.0254
                        } else {
                            ca_disp / 1000.0
                        };
                        self.plant_mill_tol_frac = mill_pct / 100.0;
                    });
                    ui.end_row();

                    label_with_tip(ui, "유체 온도 [°C]", "배관 설계 온도(허용응력 선택 시 참고)");
                    ui.add(egui::DragValue::new(&mut self.plant_service_temp_c).speed(1.0));
                    ui.end_row();

                    label_with_tip(ui, "안전율 SF (추가 여유)", "추가 보수적 여유율을 곱해 허용압을 낮춥니다.");
                    ui.add(
                        egui::DragValue::new(&mut self.plant_safety_factor)
                            .speed(0.05)
                            .clamp_range(1.0..=5.0),
                    );
                    ui.end_row();
                });
            if ui.button("내압 계산").clicked() {
                if self.plant_pipe_od_m <= 0.0
                    || self.plant_wall_thk_m <= 0.0
                    || self.plant_allow_stress_mpa <= 0.0
                {
                    self.plant_pressure_result =
                        Some("입력 오류: 외경/두께/강도는 0보다 커야 합니다.".into());
                } else {
                    // 단위 변환: mm/inch -> m
                    let (od_m_raw, thk_m_raw) =
                        if self.plant_dim_unit.eq_ignore_ascii_case("in") {
                            (self.plant_pipe_od_m * 0.0254, self.plant_wall_thk_m * 0.0254)
                        } else {
                            (
                                self.plant_pipe_od_m / 1000.0,
                                self.plant_wall_thk_m / 1000.0,
                            )
                        };
                    let t_net =
                        thk_m_raw * (1.0 - self.plant_mill_tol_frac) - self.plant_corrosion_allow_m;
                    if od_m_raw <= 0.0 || t_net <= 0.0 {
                        self.plant_pressure_result =
                            Some("입력 오류: 순두께가 0 이하입니다. CA/밀톨/두께를 확인하세요.".into());
                    } else {
                        let r_o = od_m_raw / 2.0;
                        let r_i = r_o - t_net;
                        if r_i <= 0.0 {
                            self.plant_pressure_result = Some(
                                "입력 오류: 내경이 0 이하입니다. OD/두께/CA 입력을 확인하세요.".into(),
                            );
                        } else {
                            let d_over_t = od_m_raw / t_net;
                            let s_eff_pa = self.plant_allow_stress_mpa
                                * 1.0e6
                                * self.plant_weld_eff
                                * self.plant_design_factor
                                / self.plant_safety_factor;

                            let (p_hoop_pa, p_axial_pa, model) = if d_over_t > 20.0 {
                                (
                                    2.0 * t_net * s_eff_pa / od_m_raw,
                                    4.0 * t_net * s_eff_pa / od_m_raw,
                                    "얇은 관(Barlow)",
                                )
                            } else {
                                let ro2 = r_o * r_o;
                                let ri2 = r_i * r_i;
                                (
                                    s_eff_pa * (ro2 - ri2) / (ro2 + ri2),
                                    s_eff_pa * (ro2 - ri2) / ri2,
                                    "두꺼운 관(Lamé)",
                                )
                            };
                            let p_allow_pa = p_hoop_pa.min(p_axial_pa);
                            let p_allow_bar = p_allow_pa / 1.0e5;
                            self.plant_pressure_result = Some(format!(
                                "허용압력 ~ {:.2} bar ({} 기준, Hoop {:.2} bar, Axial {:.2} bar, D/t={:.1}, t_eff={:.2} mm, S={:.1} MPa, E={:.2}, F={:.2}, SF={:.2}, CA={:.2} mm, 밀톨={:.1}%)",
                                p_allow_bar,
                                model,
                                p_hoop_pa / 1.0e5,
                                p_axial_pa / 1.0e5,
                                d_over_t,
                                t_net * 1000.0,
                                self.plant_allow_stress_mpa,
                                self.plant_weld_eff,
                                self.plant_design_factor,
                                self.plant_safety_factor,
                                self.plant_corrosion_allow_m * 1000.0,
                                self.plant_mill_tol_frac * 100.0
                            ));
                        }
                    }
                }
            }
            if let Some(res) = &self.plant_pressure_result {
                ui.label(res);
                ui.small("참고: 입력 S는 온도별 허용응력을 직접 사용. 얇은/두꺼운 판정 자동, 코드 검증은 별도로 수행하십시오. D/t>20는 얇은 관, 그 이하는 Lamé 두꺼운 관 식을 사용.");
            }
        });
    }
}

impl App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // 실행 시 첫 프레임에 현재 화면 해상도의 25%(가로) x 20%(세로)로 리사이즈
        if self.apply_initial_view_size {
            if let Some(screen) = ctx.input(|i| {
                let r = i.screen_rect();
                if r.is_positive() {
                    Some(r.size())
                } else {
                    None
                }
            }) {
                // 메뉴가 충분히 보이도록 60% 스케일, 최소 1000x700 보장
                let target = egui::vec2(
                    (screen.x * 0.60).max(1000.0),
                    (screen.y * 0.60).max(700.0),
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target));
                self.apply_initial_view_size = false;
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            if self.always_on_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            },
        ));
        // 진공 포화온도 외부 창
        if self.show_vacuum_table_viewport {
            use std::cell::Cell;
            let close_flag = Cell::new(false);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("vacuum_table_detached"),
                egui::ViewportBuilder::default()
                    .with_title("진공 포화온도 표")
                    .with_inner_size(egui::vec2(420.0, 640.0)),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("닫기").clicked() {
                                close_flag.set(true);
                            }
                        });
                        ui.separator();
                        vacuum_table_ui(ui);
                    });
                },
            );
            if close_flag.get() {
                self.show_vacuum_table_viewport = false;
            }
        }
        // 출력 라벨 선택/드래그 복사 방지 (입력 필드는 기존대로 사용 가능)
        let mut style = (*ctx.style()).clone();
        style.interaction.selectable_labels = false;
        ctx.set_style(style);
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Steam Engineering Toolbox");
                ui.label(" | Desktop GUI");
                ui.separator();
                if ui.button("설정 / Settings").clicked() {
                    self.show_settings_modal = true;
                }
                if ui.button("도움말 / Help").clicked() {
                    self.show_help_modal = true;
                }
            });
        });
        if self.show_settings_modal {
            let mut new_unit_system = self.config.unit_system;
            egui::Window::new("프로그램 설정 / Settings")
                .collapsible(false)
                .resizable(true)
                .open(&mut self.show_settings_modal)
                .show(ctx, |ui| {
                    ui.heading("기본 설정");
            ui.separator();
            ui.label("단위 시스템 프리셋");
            ui.horizontal(|ui| {
                for (label, us) in [
                    ("SI(Bar)", config::UnitSystem::SIBar),
                    ("SI(Pa)", config::UnitSystem::SI),
                    ("MKS", config::UnitSystem::MKS),
                    ("Imperial", config::UnitSystem::Imperial),
                ] {
                    ui.selectable_value(&mut new_unit_system, us, label);
                }
            });
            ui.label("프리셋 선택 시 현재 입력/출력 단위가 변경됩니다.");
            ui.separator();
            ui.label("테마");
            ui.horizontal(|ui| {
                if ui
                    .selectable_value(&mut self.theme, ThemeChoice::System, "시스템")
                    .clicked()
                {
                    ctx.set_visuals(egui::Visuals::default());
                }
                if ui
                    .selectable_value(&mut self.theme, ThemeChoice::Light, "라이트")
                    .clicked()
                {
                    ctx.set_visuals(egui::Visuals::light());
                }
                if ui
                    .selectable_value(&mut self.theme, ThemeChoice::Dark, "다크")
                    .clicked()
                {
                    ctx.set_visuals(egui::Visuals::dark());
                }
                if ui
                    .selectable_value(&mut self.theme, ThemeChoice::SoftBlue, "옅은 블루")
                    .clicked()
                {
                    let mut vis = egui::Visuals::light();
                    vis.extreme_bg_color = egui::Color32::from_rgb(225, 237, 247);
                    vis.panel_fill = egui::Color32::from_rgb(235, 243, 250);
                    vis.window_fill = egui::Color32::from_rgb(240, 246, 252);
                    vis.selection.bg_fill = egui::Color32::from_rgb(140, 180, 220);
                    vis.hyperlink_color = egui::Color32::from_rgb(50, 100, 180);
                    ctx.set_visuals(vis);
                }
            });
            ui.separator();
                    ui.label("기본 폰트 크기");
                    let slider = egui::Slider::new(&mut self.font_size, 12.0..=24.0).suffix(" px");
                    if ui.add(slider).changed() {
                        let mut style = (*ctx.style()).clone();
                        style.text_styles.iter_mut().for_each(|(_, v)| {
                            v.size = self.font_size;
                        });
                        ctx.set_style(style);
                    }
                    ui.separator();
                    ui.label("UI 배율");
                    let scale_slider = egui::Slider::new(&mut self.ui_scale, 0.8..=1.6).suffix(" x");
                    if ui.add(scale_slider).changed() {
                        ctx.set_pixels_per_point(self.ui_scale);
                    }
                    ui.separator();
                    ui.checkbox(&mut self.always_on_top, "창 항상 위에 두기");
                    ui.label("필요 시 체크 해제하여 다른 창 위에 올라오지 않게 설정");
                    ui.separator();
                    ui.label("폰트 설정 / Font");
                    ui.horizontal(|ui| {
                        ui.label("사용자 폰트 경로 / Font path");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.custom_font_path)
                                .hint_text("예: C:\\Windows\\Fonts\\malgun.ttf"),
                        );
                    });
                    if ui.button("폰트 로드 / Load Font").clicked() {
                        match load_custom_font(ctx, self.custom_font_path.trim()) {
                            Ok(_) => self.font_load_error = None,
                            Err(e) => self.font_load_error = Some(e),
                        }
                    }
                    if let Some(err) = &self.font_load_error {
                        ui.colored_label(ui.visuals().error_fg_color, format!("폰트 오류: {err}"));
                    } else {
                        ui.small("assets/fonts/malgun.ttf가 없으면 경로를 수동 지정하여 한글 폰트를 적용하세요.");
                    }
                    ui.separator();
                    ui.label("※ 단위 저장/불러오기, 테마 고정 등은 추후 config.toml과 연계 예정");
                });
            if new_unit_system != self.config.unit_system {
                self.config.unit_system = new_unit_system;
                self.apply_unit_preset(new_unit_system);
            }
        }
        if self.show_help_modal {
            egui::Window::new("도움말 / Help / About")
                .collapsible(false)
                .resizable(true)
                .open(&mut self.show_help_modal)
                .show(ctx, |ui| {
                    ui.heading("Steam Engineering Toolbox");
                    ui.label("증기/수/배관/밸브 계산 오프라인 도구");
                    ui.label("버전: 0.1a");
                    ui.label("제작자: 김민석");
                    ui.separator();
                    ui.label("단위 가이드");
                    ui.label("- 압력 mmHg: 게이지 기준(0=대기, -760mmHg=진공)");
                    ui.label("- g=게이지, a=절대");
                    ui.separator();
                    ui.label(
                        "오류나 개선 제안이 있으면 설정에서 단위/폰트를 조정하거나 문의하세요.",
                    );
                });
        }
        egui::SidePanel::left("nav")
            .resizable(true)
            .min_width(140.0)
            .default_width(200.0)
            .max_width(400.0)
            .show(ctx, |ui| {
                self.ui_nav(ui);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    match self.tab {
                        Tab::UnitConv => self.ui_unit_conv(ui),
                        Tab::SteamTables => self.ui_steam_tables(ui),
                        Tab::SteamPiping => self.ui_steam_piping(ui),
                        Tab::SteamValves => self.ui_steam_valves(ui),
                        Tab::Boiler => self.ui_boiler(ui),
                        Tab::Cooling => self.ui_cooling(ui),
                        Tab::PlantPiping => self.ui_plant_piping(ui),
                    };
                });
        });
    }
}

fn quantity_options() -> Vec<(QuantityKind, &'static str)> {
    vec![
        (QuantityKind::Temperature, "온도"),
        (QuantityKind::TemperatureDifference, "온도차"),
        (QuantityKind::Pressure, "압력"),
        (QuantityKind::Length, "길이"),
        (QuantityKind::Area, "면적"),
        (QuantityKind::Volume, "체적"),
        (QuantityKind::Velocity, "속도"),
        (QuantityKind::Mass, "질량"),
        (QuantityKind::Viscosity, "점도"),
        (QuantityKind::Energy, "에너지"),
        (QuantityKind::HeatTransferCoeff, "열전달율"),
        (QuantityKind::ThermalConductivity, "열전도율"),
        (QuantityKind::SpecificEnthalpy, "비엔탈피"),
    ]
}

fn kind_label(kind: QuantityKind) -> &'static str {
    quantity_options()
        .into_iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, l)| l)
        .unwrap_or("미정")
}

fn default_units_for_kind(kind: QuantityKind) -> (&'static str, &'static str) {
    match kind {
        QuantityKind::Temperature => ("C", "K"),
        QuantityKind::TemperatureDifference => ("C", "K"),
        QuantityKind::Pressure => ("bar", "kPa"),
        QuantityKind::Length => ("m", "ft"),
        QuantityKind::Area => ("m2", "ft2"),
        QuantityKind::Volume => ("m3", "l"),
        QuantityKind::Velocity => ("m/s", "km/h"),
        QuantityKind::Mass => ("kg", "lb"),
        QuantityKind::Viscosity => ("Pa·s", "cps"),
        QuantityKind::Energy => ("J", "kJ"),
        QuantityKind::HeatTransferCoeff => ("W/m2K", "Btu/h-ft2-F"),
        QuantityKind::ThermalConductivity => ("W/mK", "Btu/h-ft-F"),
        QuantityKind::SpecificEnthalpy => ("kJ/kg", "kcal/kg"),
    }
}

fn unit_options(kind: QuantityKind) -> &'static [(&'static str, &'static str)] {
    match kind {
        QuantityKind::Temperature => &[
            ("Celsius (°C)", "C"),
            ("Kelvin (K)", "K"),
            ("Fahrenheit (°F)", "F"),
            ("Rankine (R)", "R"),
        ],
        QuantityKind::TemperatureDifference => {
            &[("Δ°C", "C"), ("ΔK", "K"), ("Δ°F", "F"), ("ΔR", "R")]
        }
        QuantityKind::Pressure => &[
            ("bar(g)", "bar"),
            ("bar(a)", "bara"),
            ("kPa", "kPa"),
            ("MPa", "MPa"),
            ("psi", "psi"),
            ("atm", "atm"),
            ("Pa", "Pa"),
            ("mmHg", "mmHg"),
        ],
        QuantityKind::Length => &[
            ("m", "m"),
            ("mm", "mm"),
            ("cm", "cm"),
            ("inch", "in"),
            ("ft", "ft"),
            ("yd", "yd"),
            ("km", "km"),
        ],
        QuantityKind::Area => &[("m²", "m2"), ("ft²", "ft2")],
        QuantityKind::Volume => &[("m³", "m3"), ("L", "l"), ("mL", "ml"), ("ft³", "ft3")],
        QuantityKind::Velocity => &[("m/s", "m/s"), ("km/h", "km/h"), ("ft/s", "ft/s")],
        QuantityKind::Mass => &[("kg", "kg"), ("g", "g"), ("lb", "lb")],
        QuantityKind::Viscosity => &[("Pa·s", "Pa·s"), ("cP", "cps")],
        QuantityKind::Energy => &[("J", "J"), ("kJ", "kJ"), ("kcal", "kcal"), ("Btu", "Btu")],
        QuantityKind::HeatTransferCoeff => &[("W/m²·K", "W/m2K"), ("Btu/(h·ft²·F)", "Btu/h-ft2-F")],
        QuantityKind::ThermalConductivity => &[("W/m·K", "W/mK"), ("Btu/(h·ft·F)", "Btu/h-ft-F")],
        QuantityKind::SpecificEnthalpy => &[("kJ/kg", "kJ/kg"), ("kcal/kg", "kcal/kg"), ("Btu/lb", "Btu/lb")],
    }
}

fn unit_label(code: &str, kind: QuantityKind) -> String {
    for (label, c) in unit_options(kind) {
        if code.eq_ignore_ascii_case(c) {
            return label.to_string();
        }
    }
    code.to_string()
}

fn unit_combo(ui: &mut egui::Ui, value: &mut String, options: &[(&str, &str)]) {
    let current = options
        .iter()
        .find(|(_, c)| value.eq_ignore_ascii_case(c))
        .map(|(l, _)| *l)
        .unwrap_or(value.as_str());
    egui::ComboBox::from_id_source(ui.next_auto_id())
        .selected_text(current)
        .show_ui(ui, |ui| {
            for (label, code) in options {
                ui.selectable_value(value, code.to_string(), *label);
            }
        });
}

fn pressure_unit_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("bar(g)", "bar"),
        ("bar(a)", "bara"),
        ("kPa", "kPa"),
        ("MPa", "MPa"),
        ("psi", "psi"),
        ("atm", "atm"),
        ("Pa", "Pa"),
        ("mmHg", "mmHg"),
    ]
}

fn temperature_unit_options() -> &'static [(&'static str, &'static str)] {
    &[("°C", "C"), ("K", "K"), ("°F", "F"), ("R", "R")]
}

fn convert_pressure_mode_gui(
    v: f64,
    from_unit: &str,
    from_mode: conversion::PressureMode,
    to_unit: &str,
    to_mode: conversion::PressureMode,
) -> f64 {
    let from = parse_pressure_unit_gui(from_unit);
    let to = parse_pressure_unit_gui(to_unit);
    conversion::convert_pressure_mode(v, from, from_mode, to, to_mode)
}

fn convert_temperature_gui(v: f64, from: &str, to: &str) -> f64 {
    conversion::convert(QuantityKind::Temperature, v, from, to).unwrap_or(v)
}

fn convert_massflow_gui(v: f64, from: &str, to: &str) -> f64 {
    // 지원: kg/h, t/h, kg/s, lb/h
    let to_lower = |s: &str| s.to_ascii_lowercase();
    let from_l = to_lower(from);
    let to_l = to_lower(to);
    if from_l == to_l {
        return v;
    }
    // 먼저 kg/h로 변환
    let kg_per_h = match from_l.as_str() {
        "kg/h" => v,
        "t/h" | "ton/h" | "tonne/h" => v * 1000.0,
        "kg/s" => v * 3600.0,
        "lb/h" => v * 0.45359237,
        _ => v,
    };
    // 대상 변환
    match to_l.as_str() {
        "kg/h" => kg_per_h,
        "t/h" | "ton/h" | "tonne/h" => kg_per_h / 1000.0,
        "kg/s" => kg_per_h / 3600.0,
        "lb/h" => kg_per_h / 0.45359237,
        _ => kg_per_h,
    }
}

fn convert_velocity_gui(v: f64, from: &str, to: &str) -> f64 {
    conversion::convert(QuantityKind::Velocity, v, from, to).unwrap_or(v)
}

fn convert_length_gui(v: f64, from: &str, to: &str) -> f64 {
    conversion::convert(QuantityKind::Length, v, from, to).unwrap_or(v)
}

fn convert_flow_gui(v: f64, from: &str, rho_unit: &str, rho: f64) -> f64 {
    // volumetric m3/h 또는 질량 kg/h 선택
    if from.eq_ignore_ascii_case("kg/h") {
        v / convert_density_gui(rho, rho_unit, "kg/m3")
    } else if from.eq_ignore_ascii_case("t/h") {
        (v * 1000.0) / convert_density_gui(rho, rho_unit, "kg/m3")
    } else if from.eq_ignore_ascii_case("kg/s") {
        (v * 3600.0) / convert_density_gui(rho, rho_unit, "kg/m3")
    } else if from.eq_ignore_ascii_case("lb/h") {
        // lb/h -> kg/h -> m3/h
        let kg_h = v * 0.45359237;
        kg_h / convert_density_gui(rho, rho_unit, "kg/m3")
    } else if from.eq_ignore_ascii_case("gpm") {
        // US gallon per minute -> m3/h
        v * 0.2271247
    } else {
        v
    }
}

fn convert_flow_from_m3h(v_m3h: f64, to: &str, rho_unit: &str, rho: f64) -> f64 {
    if to.eq_ignore_ascii_case("kg/h") {
        v_m3h * convert_density_gui(rho, rho_unit, "kg/m3")
    } else if to.eq_ignore_ascii_case("t/h") {
        v_m3h * convert_density_gui(rho, rho_unit, "kg/m3") / 1000.0
    } else if to.eq_ignore_ascii_case("kg/s") {
        v_m3h * convert_density_gui(rho, rho_unit, "kg/m3") / 3600.0
    } else if to.eq_ignore_ascii_case("lb/h") {
        v_m3h * convert_density_gui(rho, rho_unit, "kg/m3") / 0.45359237
    } else if to.eq_ignore_ascii_case("gpm") {
        v_m3h / 0.2271247
    } else {
        v_m3h
    }
}

fn convert_density_gui(v: f64, from: &str, to: &str) -> f64 {
    if from.eq_ignore_ascii_case(to) {
        v
    } else if from.eq_ignore_ascii_case("lb/ft3") && to.eq_ignore_ascii_case("kg/m3") {
        v * 16.0185
    } else if from.eq_ignore_ascii_case("kg/m3") && to.eq_ignore_ascii_case("lb/ft3") {
        v / 16.0185
    } else {
        v
    }
}

fn convert_energy_gui(v: f64, from: &str, to: &str) -> f64 {
    conversion::convert(QuantityKind::Energy, v, from, to).unwrap_or(v)
}

fn convert_specific_enthalpy_gui(v: f64, from: &str, to: &str) -> f64 {
    conversion::convert(QuantityKind::SpecificEnthalpy, v, from, to).unwrap_or(v)
}

fn parse_pressure_unit_gui(s: &str) -> PressureUnit {
    match s.to_lowercase().as_str() {
        "bar" => PressureUnit::Bar,
        "bara" | "bar(a)" => PressureUnit::BarA,
        "kpa" => PressureUnit::KiloPascal,
        "mpa" => PressureUnit::MegaPascal,
        "psi" => PressureUnit::Psi,
        "atm" => PressureUnit::Atm,
        "pa" => PressureUnit::Pascal,
        "mmhg" => PressureUnit::MmHg,
        _ => PressureUnit::Bar,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steam_engineering_toolbox::steam::steam_piping::{pressure_loss, PressureLossInput};

    #[test]
    fn preset_sibar_applies_si_defaults() {
        let mut app = GuiApp::new(config::Config::default());
        app.apply_unit_preset(config::UnitSystem::SIBar);
        assert_eq!(app.steam_p_unit, "bar");
        assert_eq!(app.steam_p_mode, conversion::PressureMode::Gauge);
        assert_eq!(app.pipe_pressure_unit, "bar");
        assert_eq!(app.pipe_pressure_mode, conversion::PressureMode::Gauge);
        assert_eq!(app.valve_flow_unit, "m3/h");
        assert_eq!(app.boiler_lhv_unit, "kJ/kg");
    }

    #[test]
    fn preset_imperial_applies_imperial_defaults() {
        let mut app = GuiApp::new(config::Config::default());
        app.apply_unit_preset(config::UnitSystem::Imperial);
        assert_eq!(app.steam_p_unit, "psi");
        assert_eq!(app.steam_p_mode, conversion::PressureMode::Gauge);
        assert_eq!(app.pipe_pressure_unit, "psi");
        assert_eq!(app.pipe_diam_out_unit, "in");
        assert_eq!(app.pipe_velocity_unit, "ft/s");
        assert_eq!(app.valve_flow_unit, "gpm");
        assert_eq!(app.boiler_lhv_unit, "Btu/lb");
        assert_eq!(app.boiler_temp_unit, "F");
    }

    #[test]
    fn convert_energy_kcal_to_kj() {
        let out = conversion::convert(QuantityKind::Energy, 1.0, "kcal", "kJ").unwrap();
        assert!((out - 4.184).abs() < 1e-6);
    }

    #[test]
    fn convert_specific_enthalpy_kcal_per_kg() {
        let out =
            conversion::convert(QuantityKind::SpecificEnthalpy, 1.0, "kcal/kg", "kJ/kg").unwrap();
        assert!((out - 4.184).abs() < 1e-6);
    }

    #[test]
    fn convert_flow_gpm_roundtrip() {
        let v_m3h = super::convert_flow_gui(10.0, "gpm", "kg/m3", 1000.0);
        let back = super::convert_flow_from_m3h(v_m3h, "gpm", "kg/m3", 1000.0);
        assert!((back - 10.0).abs() < 1e-6);
    }

    #[test]
    fn pressure_loss_mach_check() {
        let input = PressureLossInput {
            mass_flow_kg_per_h: 3600.0, // 1 kg/s
            steam_density_kg_per_m3: 1.0,
            diameter_m: 0.1,
            length_m: 10.0,
            equivalent_length_m: 0.0,
            fittings_k_sum: 0.0,
            roughness_m: 0.000045,
            dynamic_viscosity_pa_s: 1.2e-5,
            sound_speed_m_per_s: 300.0,
        };
        let res = pressure_loss(input).unwrap();
        assert!((res.mach - 0.424).abs() < 0.01, "mach={}", res.mach);
    }
}
