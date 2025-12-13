#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! eframe/egui 기반 데스크톱 GUI 진입점.

use eframe::{egui, App, Frame};
use image::GenericImageView;
use rfd::FileDialog;
use std::{env, fs, path::Path};
use steam_engineering_toolbox::{
    config, conversion,
    cooling::{condenser, cooling_tower, drain_cooler, pump_npsh},
    i18n,
    material_db,
    quantity::QuantityKind,
    steam,
    steam::steam_piping::PipeSizingByVelocityInput,
    steam::steam_valves,
    units::{PressureUnit, TemperatureUnit},
};

fn main() -> Result<(), eframe::Error> {
    // CLI 언어 옵션 처리: --lang xx 또는 --lang=xx (xx: auto/en-us/en-uk/ko-kr/ko)
    let mut cli_lang: Option<String> = None;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if let Some(val) = a.strip_prefix("--lang=") {
            cli_lang = Some(val.to_string());
        } else if a == "--lang" || a == "-L" {
            if i + 1 < args.len() {
                cli_lang = Some(args[i + 1].clone());
                i += 1;
            }
        }
        i += 1;
    }

    let icon_data = load_app_icon();
    let mut viewport = egui::ViewportBuilder::default()
        .with_always_on_top()
        .with_transparent(true);
    if let Some(icon) = icon_data.clone() {
        viewport = viewport.with_icon(icon);
    }
    let cfg = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let mut app_cfg = config::load_or_default().unwrap_or_default();
    if let Some(lang_cli) = cli_lang {
        let resolved = i18n::resolve_language(&lang_cli, Some(app_cfg.language.as_str()));
        app_cfg.language = resolved;
    }
    eframe::run_native(
        "Steam Engineering Toolbox",
        cfg,
        Box::new(move |cc| {
            if let Err(e) = setup_fonts(&cc.egui_ctx) {
                eprintln!("Font error: {e}");
            }
            Box::new(GuiApp::new(app_cfg.clone()))
        }),
    )
}

fn load_app_icon() -> Option<egui::IconData> {
    let search = [
        "SE_Cal.png",
        "icon.png",
        "assets/icon.png",
        "../SE_Cal.png",
        "../../SE_Cal.png",
    ];
    let path = search.iter().find(|p| Path::new(*p).exists()).map(|s| s.to_string())?;
    let bytes = fs::read(&path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = img.dimensions();
    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width: w,
        height: h,
    })
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

fn fill_template(template: &str, vars: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

fn legend_toggle(ui: &mut egui::Ui, title: &str, body: &str, state: &mut bool) {
    ui.horizontal(|ui| {
        ui.checkbox(state, title);
    });
    if *state {
        ui.add(
            egui::Label::new(egui::RichText::new(body).small())
                .wrap(true),
        );
    }
}

struct GuiApp {
    config: config::Config,
    tr: i18n::Translator,
    lang_input: String,
    lang_pack_dir_input: String,
    lang_save_status: Option<String>,
    tab: Tab,
    window_alpha: f32,
    show_formula_modal: bool,
    // 해설 토글
    show_legend_steam: bool,
    show_legend_pipe: bool,
    show_legend_pipe_loss: bool,
    show_legend_valve: bool,
    show_legend_plant: bool,
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
    pipe_loss_pressure_bar_abs: f64,
    pipe_loss_temperature_c: f64,
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
        let bytes = fs::read(asset_path).map_err(|e| format!("Failed to read font file: {e}"))?;
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
                    .map_err(|e| format!("Failed to read system font ({}): {e}", p.display()))?;
                apply_font_bytes(ctx, bytes, "korean_font");
                return Ok(());
            }
        }
    }

    // 3) 실패: 기본 폰트 유지, 사용자 지정 안내
    Err("Font not found. Please set a user font (.ttf/.ttc) in settings.".into())
}

/// 사용자가 선택한 경로의 폰트를 egui에 등록한다.
fn load_custom_font(ctx: &egui::Context, path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("Font file not found: {path}"));
    }
    let bytes = fs::read(p).map_err(|e| format!("Failed to read font file: {e}"))?;
    apply_font_bytes(ctx, bytes, "user_font");
    Ok(())
}

fn vacuum_table_ui<F>(ui: &mut egui::Ui, txt: &F)
where
    F: Fn(&str, &str) -> String,
{
    ui.small(txt(
        "gui.steam.vacuum_table.intro",
        "Fix pressure to mmHg (gauge) and show IF97 saturation temps.",
    ));
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
                -420.0, -440.0, -460.0, -480.0, -500.0, -520.0, -540.0, -560.0, -580.0,
                -600.0, // 20단계
                -610.0, -620.0, -630.0, -640.0, -650.0, -660.0, -670.0, -680.0, // 10단계
                -685.0, -690.0, -695.0, -700.0, -705.0, -710.0, -715.0, -720.0, -725.0, -730.0,
                -735.0, -740.0, // 5단계
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
        let lang_code = i18n::resolve_language("auto", Some(config.language.as_str()));
        let tr = i18n::Translator::new_with_pack(&lang_code, config.language_pack_dir.as_deref());
        let has_overrides = tr.lookup("gui.nav.app_title").is_some();
        eprintln!("GUI language resolved: {lang_code}, overrides_loaded={has_overrides}");
        let lang_input = config.language.clone();
        let lang_pack_dir_input = config.language_pack_dir.clone().unwrap_or_default();
        let mut s = Self {
            config: config.clone(),
            tr,
            lang_input,
            lang_pack_dir_input,
            lang_save_status: None,
            tab: Tab::UnitConv,
            window_alpha: config.window_alpha.clamp(0.3, 1.0),
            show_formula_modal: false,
            show_legend_steam: false,
            show_legend_pipe: false,
            show_legend_pipe_loss: false,
            show_legend_valve: false,
            show_legend_plant: false,
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
            pipe_loss_pressure_bar_abs: 6.0,
            pipe_loss_temperature_c: 180.0,
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
            plant_mat: "A106B".into(),
            plant_length_m: 10.0,
            plant_delta_t: 50.0,
            plant_alpha_override: 0.0,
            plant_expansion_result: None,
            plant_pipe_od_m: 0.114,  // NPS 4" OD 약 114mm
            plant_wall_thk_m: 0.006, // 6mm
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
        let tr = self.tr.clone();
        let txt = |key: &str, default: &str| tr.lookup(key).unwrap_or_else(|| default.to_string());
        ui.style_mut().wrap = Some(false);
        ui.vertical_centered(|ui| {
            ui.heading(txt("gui.nav.heading", "Menu"));
            ui.add_space(8.0);
        });
        for (tab, label) in [
            (Tab::SteamTables, txt("gui.tab.steam_tables", "Steam Tables")),
            (Tab::UnitConv, txt("gui.tab.unit_conv", "Unit Converter")),
            (Tab::SteamPiping, txt("gui.tab.steam_piping", "Steam Piping")),
            (Tab::SteamValves, txt("gui.tab.steam_valves", "Steam Valves")),
            (Tab::Boiler, txt("gui.tab.boiler", "Boiler Efficiency")),
            (Tab::Cooling, txt("gui.tab.cooling", "Cooling/Condensing")),
            (Tab::PlantPiping, txt("gui.tab.plant_piping", "Plant Piping")),
        ] {
            let selected = self.tab == tab;
            let button = egui::Button::new(label)
                .fill(if selected {
                    ui.visuals().selection.bg_fill
                } else {
                    ui.visuals().extreme_bg_color
                })
                .min_size(egui::vec2(ui.available_width(), 32.0));
            let resp = ui
                .add(button)
                .on_hover_text(txt("gui.nav.switch_tip", "Switch menu"));
            if resp.clicked() {
                self.tab = tab;
            }
            ui.add_space(4.0);
        }
    }

    fn ui_unit_conv(&mut self, ui: &mut egui::Ui) {
        let tr = self.tr.clone();
        let txt = |key: &str, default: &str| tr.lookup(key).unwrap_or_else(|| default.to_string());
        heading_with_tip(
            ui,
            &txt("gui.unit.heading", "Unit Converter"),
            &txt(
                "gui.unit.tip",
                "Convert various physical quantities between units.",
            ),
        );
        label_with_tip(
            ui,
            &txt("gui.unit.card_label", "Card-style input"),
            &txt(
                "gui.unit.card_tip",
                "Enter value and select units, then run conversion.",
            ),
        );
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.vertical(|ui| {
                egui::Grid::new("conv_grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                    label_with_tip(
                        ui,
                        &txt("gui.unit.quantity.label", "Quantity"),
                        &txt("gui.unit.quantity_tip", "Select the quantity type"),
                    );
                        let before = self.conv_kind;
                        let q_options = vec![
                            (
                                QuantityKind::Temperature,
                                txt("gui.unit.quantity.temperature", "Temperature"),
                            ),
                            (
                                QuantityKind::TemperatureDifference,
                                txt("gui.unit.quantity.temperature_diff", "ΔTemperature"),
                            ),
                            (
                                QuantityKind::Pressure,
                                txt("gui.unit.quantity.pressure", "Pressure"),
                            ),
                            (QuantityKind::Length, txt("gui.unit.quantity.length", "Length")),
                            (QuantityKind::Area, txt("gui.unit.quantity.area", "Area")),
                            (QuantityKind::Volume, txt("gui.unit.quantity.volume", "Volume")),
                            (
                                QuantityKind::Velocity,
                                txt("gui.unit.quantity.velocity", "Velocity"),
                            ),
                            (QuantityKind::Mass, txt("gui.unit.quantity.mass", "Mass")),
                            (
                                QuantityKind::Viscosity,
                                txt("gui.unit.quantity.viscosity", "Viscosity"),
                            ),
                            (QuantityKind::Energy, txt("gui.unit.quantity.energy", "Energy")),
                            (
                                QuantityKind::HeatTransferCoeff,
                                txt(
                                    "gui.unit.quantity.heat_transfer_coeff",
                                    "Heat transfer coeff.",
                                ),
                            ),
                            (
                                QuantityKind::ThermalConductivity,
                                txt(
                                    "gui.unit.quantity.thermal_conductivity",
                                    "Thermal conductivity",
                                ),
                            ),
                            (
                                QuantityKind::SpecificEnthalpy,
                                txt("gui.unit.quantity.specific_enthalpy", "Specific enthalpy"),
                            ),
                        ];
                        let selected_label = q_options
                            .iter()
                            .find(|(k, _)| *k == self.conv_kind)
                            .map(|(_, l)| l.clone())
                            .unwrap_or_else(|| txt("gui.unit.quantity.label", "Quantity"));
                        egui::ComboBox::from_id_source("conv_kind")
                            .selected_text(selected_label)
                            .show_ui(ui, |ui| {
                                for (k, label) in &q_options {
                                    ui.selectable_value(&mut self.conv_kind, *k, label.clone());
                                }
                            });
                        if before != self.conv_kind {
                            let (f, t) = default_units_for_kind(self.conv_kind);
                            self.conv_from = f.to_string();
                            self.conv_to = t.to_string();
                        }
                        ui.end_row();

                        label_with_tip(
                            ui,
                            &txt("gui.unit.value", "Value"),
                            &txt("gui.unit.value_tip", "Enter the value to convert"),
                        );
                        ui.add(egui::DragValue::new(&mut self.conv_value).speed(1.0));
                        ui.end_row();

                        label_with_tip(
                            ui,
                            &txt("gui.unit.from", "From unit"),
                            &txt("gui.unit.from_tip", "Current unit of the value"),
                        );
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

                        label_with_tip(
                            ui,
                            &txt("gui.unit.to", "To unit"),
                            &txt("gui.unit.to_tip", "Desired unit after conversion"),
                        );
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
                if ui.button(txt("gui.unit.run", "Convert")).clicked() {
                    self.conv_result = match conversion::convert(
                        self.conv_kind,
                        self.conv_value,
                        self.conv_from.trim(),
                        self.conv_to.trim(),
                    ) {
                        Ok(v) => Some(format!("{v:.6} {}", self.conv_to.trim())),
                        Err(e) => Some(format!(
                            "{}: {e}",
                            txt("gui.unit.error_prefix", "Error")
                        )),
                    };
                }
                if let Some(res) = &self.conv_result {
                    ui.label(res);
                }
            });
        });
    }

    fn ui_steam_tables(&mut self, ui: &mut egui::Ui) {
        let tr = self.tr.clone();
        let txt = |key: &str, default: &str| tr.lookup(key).unwrap_or_else(|| default.to_string());
        heading_with_tip(
            ui,
            &txt("gui.steam.heading", "Steam Tables"),
            &txt(
                "gui.steam.tip",
                "Steam/water properties (sat/superheated) based on IF97.",
            ),
        );
        label_with_tip(
            ui,
            &txt("gui.steam.card_label", "Saturation/Superheat card"),
            &txt(
                "gui.steam.card_tip",
                "Enter pressure/temperature to get Psat/Tsat/h/s/v.",
            ),
        );
        ui.add_space(8.0);
        if ui
            .button(txt("gui.steam.vacuum_open", "Open vacuum table"))
            .on_hover_text(txt(
                "gui.steam.vacuum_open_tip",
                "Show built-in vacuum saturation table (mmHg gauge).",
            ))
            .clicked()
        {
            self.show_vacuum_table_window = true;
        }
        ui.horizontal(|ui| {
            if ui
                .button(txt("gui.steam.vacuum_window", "Open vacuum table in new window"))
                .on_hover_text(txt(
                    "gui.steam.vacuum_window_tip",
                    "Open vacuum table in a separate window.",
                ))
                .clicked()
            {
                self.show_vacuum_table_viewport = true;
            }
            ui.small(txt(
                "gui.steam.vacuum_note",
                "You can keep the external window open while using other menus.",
            ));
        });
        if self.show_vacuum_table_window {
            egui::Window::new(txt(
                "gui.steam.vacuum_title",
                "Vacuum saturation table (mmHg gauge: 0=atm, -760=vacuum)",
            ))
                .open(&mut self.show_vacuum_table_window)
                .scroll2([true, true])
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    vacuum_table_ui(ui, &txt);
                });
        }

        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.steam_mode,
                    SteamMode::ByPressure,
                    txt("gui.steam.mode.pressure", "By pressure"),
                )
                .on_hover_text(txt(
                    "gui.steam.mode.pressure_tip",
                    "Enter pressure to get Psat/Tsat/h/s/v.",
                ));
                ui.selectable_value(
                    &mut self.steam_mode,
                    SteamMode::ByTemperature,
                    txt("gui.steam.mode.temperature", "By temperature"),
                )
                .on_hover_text(txt(
                    "gui.steam.mode.temperature_tip",
                    "Enter temperature to get Psat/Tsat/h/s/v.",
                ));
                ui.selectable_value(
                    &mut self.steam_mode,
                    SteamMode::Superheated,
                    txt("gui.steam.mode.superheated", "Superheated"),
                )
                .on_hover_text(txt(
                    "gui.steam.mode.superheated_tip",
                    "Enter P+superheat to get superheated properties.",
                ));
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                label_with_tip(
                    ui,
                    &txt("gui.steam.value", "Value"),
                    &txt(
                        "gui.steam.value_tip",
                        "Pressure or temperature depending on mode",
                    ),
                );
                ui.add(egui::DragValue::new(&mut self.steam_value).speed(0.5));
                if matches!(self.steam_mode, SteamMode::ByPressure | SteamMode::Superheated) {
                    unit_combo(ui, &mut self.steam_p_unit, pressure_unit_options());
                    ui.selectable_value(
                        &mut self.steam_p_mode,
                        conversion::PressureMode::Gauge,
                        "Gauge (G)",
                    );
                    ui.selectable_value(
                        &mut self.steam_p_mode,
                        conversion::PressureMode::Absolute,
                        "Absolute (A)",
                    );
                } else {
                    unit_combo(ui, &mut self.steam_t_unit, temperature_unit_options());
                }
            });
            if self.steam_mode == SteamMode::Superheated {
                ui.horizontal(|ui| {
                    label_with_tip(
                        ui,
                        &txt("gui.steam.superheat", "Superheat [°C]"),
                        &txt(
                            "gui.steam.superheat_tip",
                            "Superheat above saturation (not absolute temperature)",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.steam_temp_input).speed(1.0));
                    unit_combo(ui, &mut self.steam_t_unit, temperature_unit_options());
                });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                label_with_tip(
                    ui,
                    &txt("gui.steam.output_pressure", "Output pressure unit"),
                    &txt("gui.steam.output_pressure_tip", "Pressure unit for results"),
                );
                unit_combo(ui, &mut self.steam_p_unit_out, pressure_unit_options());
                ui.selectable_value(
                    &mut self.steam_p_mode_out,
                    conversion::PressureMode::Gauge,
                    "Gauge (G)",
                );
                ui.selectable_value(
                    &mut self.steam_p_mode_out,
                    conversion::PressureMode::Absolute,
                    "Absolute (A)",
                );
                label_with_tip(
                    ui,
                    &txt("gui.steam.output_temperature", "Output temperature unit"),
                    &txt("gui.steam.output_temperature_tip", "Temperature unit for results"),
                );
                unit_combo(ui, &mut self.steam_t_unit_out, temperature_unit_options());
            });
            ui.small(txt(
                "gui.steam.tip_mmhg",
                "Tip: mmHg is treated as gauge (0=atm, -760=vacuum).",
            ));
            ui.add_space(6.0);
            if ui.button(txt("gui.steam.run", "Calculate")).clicked() {
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
                        let t_out =
                            convert_temperature_gui(s.saturation_temperature_c, "C", &self.steam_t_unit_out);
                        let tpl = txt(
                            "gui.steam.result.sat_full",
                            "Psat={psat} {p_unit}, Tsat={tsat} {t_unit}, hs(v)={hs} kJ/kg, vs={vs} m3/kg, ss={ss} kJ/kgK | hf={hf} kJ/kg, vf={vf} m3/kg, sf={sf} kJ/kgK",
                        );
                        fill_template(
                            &tpl,
                            &[
                                ("psat", format!("{:.3}", p_out)),
                                ("p_unit", self.steam_p_unit_out.clone()),
                                ("tsat", format!("{:.2}", t_out)),
                                ("t_unit", self.steam_t_unit_out.clone()),
                                ("hs", format!("{:.1}", s.saturation_enthalpy_kj_per_kg)),
                                ("vs", format!("{:.3}", s.saturation_specific_volume)),
                                ("ss", format!("{:.3}", s.saturation_entropy_kj_per_kgk)),
                                ("hf", format!("{:.1}", s.sat_liquid_enthalpy_kj_per_kg)),
                                ("vf", format!("{:.4}", s.sat_liquid_specific_volume)),
                                ("sf", format!("{:.3}", s.sat_liquid_entropy_kj_per_kgk)),
                            ],
                        )
                    }
                    Err(e) => {
                        let tpl = txt(
                            "gui.steam.error.pressure",
                            "Error(P={p} {p_unit}{mode}): {e}",
                        );
                        let mode = if self.steam_p_mode == conversion::PressureMode::Gauge {
                            "g"
                        } else {
                            "a"
                        };
                        fill_template(
                            &tpl,
                            &[
                                ("p", format!("{:.3}", self.steam_value)),
                                ("p_unit", self.steam_p_unit.clone()),
                                ("mode", mode.to_string()),
                                ("e", e.to_string()),
                            ],
                        )
                    }
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
                        let tpl = txt(
                            "gui.steam.result.sat_temp",
                            "Psat={psat} {p_unit}, hs={hs} kJ/kg, v={v} m3/kg",
                        );
                        fill_template(
                            &tpl,
                            &[
                                ("psat", format!("{:.3}", p_out)),
                                ("p_unit", self.steam_p_unit_out.clone()),
                                ("hs", format!("{:.1}", s.saturation_enthalpy_kj_per_kg)),
                                ("v", format!("{:.3}", s.saturation_specific_volume)),
                            ],
                        )
                    }
                    Err(e) => {
                        let tpl = txt("gui.steam.error.temperature", "Error(T={t} {t_unit}): {e}");
                        fill_template(
                            &tpl,
                            &[
                                ("t", format!("{:.2}", self.steam_value)),
                                ("t_unit", self.steam_t_unit.clone()),
                                ("e", e.to_string()),
                            ],
                        )
                    }
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
                        let tpl = txt(
                            "gui.steam.result.superheat",
                            "P={p} {p_unit}, T={t} {t_unit}, h={h} kJ/kg",
                        );
                        fill_template(
                            &tpl,
                            &[
                                ("p", format!("{:.2}", p_out)),
                                ("p_unit", self.steam_p_unit_out.clone()),
                                ("t", format!("{:.1}", t_out)),
                                ("t_unit", self.steam_t_unit_out.clone()),
                                (
                                    "h",
                                    format!("{:.1}", s.superheated_enthalpy_kj_per_kg.unwrap_or(0.0)),
                                ),
                            ],
                        )
                    }
                    Err(e) => {
                        let tpl = txt(
                            "gui.steam.error.superheat",
                            "Error(P={p} {p_unit}{mode}, T={t} {t_unit}): {e}",
                        );
                        let mode = if self.steam_p_mode == conversion::PressureMode::Gauge {
                            "g"
                        } else {
                            "a"
                        };
                        fill_template(
                            &tpl,
                            &[
                                ("p", format!("{:.3}", self.steam_value)),
                                ("p_unit", self.steam_p_unit.clone()),
                                ("mode", mode.to_string()),
                                ("t", format!("{:.1}", self.steam_temp_input)),
                                ("t_unit", self.steam_t_unit.clone()),
                                ("e", e.to_string()),
                            ],
                        )
                    }
            },
        });
    }
    if let Some(res) = &self.steam_result {
        ui.separator();
        ui.label(res);
        legend_toggle(
            ui,
            &txt("legend.steam.title", "Legend / notes"),
            &txt(
                "legend.steam.body",
                "Psat=sat pressure, Tsat=sat temperature, hs/vs/ss=sat vapor, hf/vf/sf=sat liquid",
            ),
            &mut self.show_legend_steam,
        );
    }
});
    }

    fn ui_steam_piping(&mut self, ui: &mut egui::Ui) {
        let tr = self.tr.clone();
        let txt = move |key: &str, default: &str| {
            tr.lookup(key).unwrap_or_else(|| default.to_string())
        };
        heading_with_tip(
            ui,
            &txt("gui.pipe.heading", "Steam Piping"),
            &txt(
                "gui.pipe.tip",
                "Pipe sizing and pressure-drop calculator for steam/gas.",
            ),
        );
        label_with_tip(
            ui,
            &txt("gui.pipe.card_label", "Pipe sizing card"),
            &txt(
                "gui.pipe.card_tip",
                "Enter mass flow, pressure/temperature, and target velocity to size ID and Reynolds.",
            ),
        );
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::Grid::new("pipe_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(
                        ui,
                        &txt("gui.pipe.mass_flow", "Mass flow"),
                        &txt("gui.pipe.mass_flow_tip", "Steam/gas mass flow (kg/h etc.)"),
                    );
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
                    label_with_tip(
                        ui,
                        &txt("gui.pipe.pressure", "Pressure [bar]"),
                        &txt(
                            "gui.pipe.pressure_tip",
                            "Operating pressure (select gauge/absolute).",
                        ),
                    );
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
                    label_with_tip(
                        ui,
                        &txt("gui.pipe.temperature", "Temperature [°C]"),
                        &txt(
                            "gui.pipe.temperature_tip",
                            "Operating steam temperature.",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.pipe_temp).speed(1.0));
                    unit_combo(ui, &mut self.pipe_temp_unit, temperature_unit_options());
                    ui.end_row();
                    label_with_tip(
                        ui,
                        &txt("gui.pipe.velocity", "Target velocity [m/s]"),
                        &txt(
                            "gui.pipe.velocity_tip",
                            "Design target velocity (higher → smaller ID but more noise/erosion).",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.pipe_velocity).speed(1.0));
                    unit_combo(
                        ui,
                        &mut self.pipe_velocity_unit,
                        &[("m/s", "m/s"), ("ft/s", "ft/s")],
                    );
                    ui.end_row();
                });
            ui.small(txt(
                "gui.pipe.tip_mmhg",
                "Tip: mmHg is treated as gauge (0=atm, -760mmHg=vacuum).",
            ));
            ui.add_space(8.0);
            if ui.button(txt("gui.pipe.run_sizing", "Run sizing")).clicked() {
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
                    Err(e) => {
                        let tpl = txt(
                            "gui.pipe.error.sizing",
                            "Error(mdot={mdot} {m_unit}, P={p} {p_unit}{mode}, T={t} {t_unit}): {e}",
                        );
                        let mode = if self.pipe_pressure_mode == conversion::PressureMode::Gauge {
                            "g"
                        } else {
                            "a"
                        };
                        fill_template(
                            &tpl,
                            &[
                                ("mdot", format!("{:.2}", self.pipe_mass_flow)),
                                ("m_unit", self.pipe_mass_unit.clone()),
                                ("p", format!("{:.2}", self.pipe_pressure)),
                                ("p_unit", self.pipe_pressure_unit.clone()),
                                ("mode", mode.to_string()),
                                ("t", format!("{:.1}", self.pipe_temp)),
                                ("t_unit", self.pipe_temp_unit.clone()),
                                ("e", e.to_string()),
                            ],
                        )
                    }
                });
            }
            if let Some(res) = &self.pipe_result {
                ui.separator();
                ui.label(res);
                legend_toggle(
                    ui,
                    &txt("legend.pipe.title", "Legend / notes"),
                    &txt(
                        "legend.pipe.body",
                        "ID=inner diameter, Velocity=flow velocity, Re=Reynolds number",
                    ),
                    &mut self.show_legend_pipe,
                );
            }
        });
        ui.add_space(6.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(txt(
                "gui.pipe.loss.heading",
                "Pressure Loss (Darcy-Weisbach)",
            ));
            egui::Grid::new("pipe_loss_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(txt(
                        "gui.pipe.loss.mass_flow",
                        "Mass flow [kg/h]",
                    ));
                    ui.add(egui::DragValue::new(&mut self.pipe_mass_flow).speed(10.0));
                    unit_combo(
                        ui,
                        &mut self.pipe_mass_unit,
                        &[("kg/h", "kg/h"), ("lb/h", "lb/h")],
                    );
                    ui.end_row();
                    ui.label(txt(
                        "gui.pipe.loss.pressure",
                        "State pressure [bar(a)] (IF97)",
                    ));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_pressure_bar_abs).speed(0.1));
                    ui.end_row();
                    ui.label(txt(
                        "gui.pipe.loss.temperature",
                        "State temperature [°C] (IF97)",
                    ));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_temperature_c).speed(1.0));
                    ui.end_row();
                    ui.label(txt("gui.pipe.loss.density", "Density [kg/m3]"));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_density).speed(0.1));
                    ui.end_row();
                    ui.label(txt("gui.pipe.loss.diameter", "Inner diameter [m]"));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_diameter).speed(0.001));
                    ui.end_row();
                    ui.label(txt("gui.pipe.loss.length", "Length [m]"));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_length).speed(1.0));
                    ui.end_row();
                    ui.label(txt("gui.pipe.loss.eq_length", "Equivalent length [m]"));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_eq_length).speed(1.0));
                    ui.end_row();
                    ui.label(txt("gui.pipe.loss.fittings", "Fittings K sum"));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_fittings_k).speed(0.1));
                    ui.end_row();
                    ui.label(txt("gui.pipe.loss.roughness", "Roughness ε [m]"));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_roughness).speed(0.00001));
                    ui.end_row();
                    ui.label(txt("gui.pipe.loss.viscosity", "Viscosity [Pa·s]"));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_visc).speed(1e-6));
                    ui.end_row();
                    ui.label(txt("gui.pipe.loss.sound_speed", "Speed of sound [m/s]"));
                    ui.add(egui::DragValue::new(&mut self.pipe_loss_sound_speed).speed(5.0));
                    ui.end_row();
                    ui.label(txt(
                        "gui.pipe.loss.output",
                        "Output ΔP unit",
                    ));
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
            if ui.button(txt("gui.pipe.loss.run", "Calculate ΔP")).clicked() {
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
                    state_pressure_bar_abs: Some(self.pipe_loss_pressure_bar_abs),
                    state_temperature_c: Some(self.pipe_loss_temperature_c),
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
                    Err(e) => {
                        let tpl = txt(
                            "gui.pipe.loss.error",
                            "Error(mdot={mdot} {m_unit}, rho={rho} kg/m3, D={d} m, L={l} m): {e}",
                        );
                        fill_template(
                            &tpl,
                            &[
                                ("mdot", format!("{:.2}", self.pipe_mass_flow)),
                                ("m_unit", self.pipe_mass_unit.clone()),
                                ("rho", format!("{:.2}", self.pipe_loss_density)),
                                ("d", format!("{:.4}", self.pipe_loss_diameter)),
                                ("l", format!("{:.1}", self.pipe_loss_length)),
                                ("e", e.to_string()),
                            ],
                        )
                    }
                });
            }
            if let Some(res) = &self.pipe_loss_result {
                ui.separator();
                ui.label(res);
                legend_toggle(
                    ui,
                    &txt("legend.pipe_loss.title", "Legend / notes"),
                    &txt("legend.pipe_loss.body", "ΔP=pressure drop, v=velocity, Re=Reynolds, f=friction factor, Mach=speed ratio"),
                    &mut self.show_legend_pipe_loss,
                );
            }
        });
    }

    fn ui_steam_valves(&mut self, ui: &mut egui::Ui) {
        let tr = self.tr.clone();
        let txt = move |key: &str, default: &str| {
            tr.lookup(key).unwrap_or_else(|| default.to_string())
        };
        heading_with_tip(
            ui,
            &txt("gui.valve.heading", "Steam Valves & Orifices"),
            &txt(
                "gui.valve.tip",
                "Compute required Cv/Kv or flow for given Cv/Kv.",
            ),
        );
        label_with_tip(
            ui,
            &txt("gui.valve.card_label", "Cv/Kv calculator"),
            &txt(
                "gui.valve.card_tip",
                "Use ΔP/upstream P/flow/density to size or check flow.",
            ),
        );
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.valve_mode,
                    ValveMode::RequiredCvKv,
                    txt("gui.valve.mode.required", "Required Cv/Kv"),
                )
                .on_hover_text(txt(
                    "gui.valve.mode.required_tip",
                    "Compute Cv/Kv to achieve the target flow.",
                ));
                ui.selectable_value(
                    &mut self.valve_mode,
                    ValveMode::FlowFromCvKv,
                    txt("gui.valve.mode.flow", "Flow from Cv/Kv"),
                )
                .on_hover_text(txt(
                    "gui.valve.mode.flow_tip",
                    "Compute flow when Cv/Kv is given.",
                ));
            });
            egui::Grid::new("valve_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(
                        ui,
                        &match self.valve_mode {
                            ValveMode::RequiredCvKv => {
                                txt("gui.valve.input.flow", "Volumetric flow")
                            }
                            ValveMode::FlowFromCvKv => {
                                txt("gui.valve.input.cv", "Cv/Kv input")
                            }
                        },
                        &txt(
                            "gui.valve.input.flow_tip",
                            "Enter flow to size Cv/Kv, or enter Cv/Kv to compute flow.",
                        ),
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
                        &txt("gui.valve.input.dp", "ΔP [bar]"),
                        &txt(
                            "gui.valve.input.dp_tip",
                            "Pressure drop across valve (choose gauge/absolute); check choking for steam/gas.",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.valve_dp).speed(0.1));
                    unit_combo(ui, &mut self.valve_dp_unit, pressure_unit_options());
                    ui.selectable_value(&mut self.valve_dp_mode, conversion::PressureMode::Gauge, "Gauge (G)");
                    ui.selectable_value(&mut self.valve_dp_mode, conversion::PressureMode::Absolute, "Absolute (A)");
                    ui.end_row();
                    label_with_tip(
                        ui,
                        &txt("gui.valve.input.upstream", "Upstream pressure"),
                        &txt(
                            "gui.valve.input.upstream_tip",
                            "Absolute upstream pressure when computing flow (for choking check).",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.valve_upstream_p).speed(0.1));
                    unit_combo(ui, &mut self.valve_upstream_unit, pressure_unit_options());
                    ui.selectable_value(&mut self.valve_upstream_mode, conversion::PressureMode::Gauge, "Gauge (G)");
                    ui.selectable_value(&mut self.valve_upstream_mode, conversion::PressureMode::Absolute, "Absolute (A)");
                    ui.end_row();
                    label_with_tip(
                        ui,
                        &txt("gui.valve.input.density", "Density [kg/m3]"),
                        &txt(
                            "gui.valve.input.density_tip",
                            "Fluid density (use condition-based density; IF97 recommended for steam).",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.valve_rho).speed(0.1));
                    unit_combo(ui, &mut self.valve_rho_unit, &[("kg/m3", "kg/m3"), ("lb/ft3", "lb/ft3")]);
                    ui.end_row();
                    if let ValveMode::FlowFromCvKv = self.valve_mode {
                        label_with_tip(
                            ui,
                            &txt("gui.valve.input.cv_value", "Cv/Kv value"),
                            &txt("gui.valve.input.cv_tip", "Manufacturer Cv or Kv value"),
                        );
                        ui.add(egui::DragValue::new(&mut self.valve_cv_kv).speed(0.5));
                        ui.end_row();
                    }
                });
            ui.small(txt(
                "gui.valve.tip_mmhg",
                "Tip: mmHg is treated as gauge (0=atm, -760mmHg=vacuum).",
            ));
            ui.add_space(8.0);
            if ui.button(txt("gui.valve.run", "Calculate")).clicked() {
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
                        Ok(kv) => {
                            let tpl = txt("gui.valve.result.required", "Kv={kv}, Cv={cv}");
                            fill_template(
                                &tpl,
                                &[
                                    ("kv", format!("{:.3}", kv)),
                                    ("cv", format!("{:.3}", steam_valves::cv_from_kv(kv))),
                                ],
                            )
                        }
                        Err(e) => {
                            let tpl = txt(
                                "gui.valve.error.required",
                                "Error(Q={q} {q_unit}, ΔP={dp} {dp_unit}{mode}, rho={rho} {rho_unit}): {e}",
                            );
                            let mode = if self.valve_dp_mode == conversion::PressureMode::Gauge {
                                "g"
                            } else {
                                "a"
                            };
                            fill_template(
                                &tpl,
                                &[
                                    ("q", format!("{:.2}", self.valve_flow)),
                                    ("q_unit", self.valve_flow_unit.clone()),
                                    ("dp", format!("{:.2}", self.valve_dp)),
                                    ("dp_unit", self.valve_dp_unit.clone()),
                                    ("mode", mode.to_string()),
                                    ("rho", format!("{:.2}", self.valve_rho)),
                                    ("rho_unit", self.valve_rho_unit.clone()),
                                    ("e", e.to_string()),
                                ],
                            )
                        }
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
                                    txt("gui.valve.warn.choked", " [Warning: potential choked flow]").to_string()
                                } else {
                                    String::new()
                                };
                                let tpl = txt(
                                    "gui.valve.result.flow",
                                    "Flow {flow} {flow_unit}{warn}, mass {mass} kg/h (Pu={pu} bar(a), Pd={pd} bar(a))",
                                );
                                fill_template(
                                    &tpl,
                                    &[
                                        ("flow", format!("{:.3}", q_out)),
                                        ("flow_unit", self.valve_flow_unit.clone()),
                                        ("warn", warn),
                                        ("mass", format!("{:.3}", mass_kg_h)),
                                        ("pu", format!("{:.2}", upstream_bar_abs)),
                                        ("pd", format!("{:.2}", downstream_abs)),
                                    ],
                                )
                            }
                            Err(e) => {
                                let tpl = txt(
                                    "gui.valve.error.flow",
                                    "Error(Cv/Kv={cv}, ΔP={dp} {dp_unit}{mode}, rho={rho} {rho_unit}): {e}",
                                );
                                let mode = if self.valve_dp_mode == conversion::PressureMode::Gauge {
                                    "g"
                                } else {
                                    "a"
                                };
                                fill_template(
                                    &tpl,
                                    &[
                                        ("cv", format!("{:.2}", kv)),
                                        ("dp", format!("{:.2}", self.valve_dp)),
                                        ("dp_unit", self.valve_dp_unit.clone()),
                                        ("mode", mode.to_string()),
                                        ("rho", format!("{:.2}", self.valve_rho)),
                                        ("rho_unit", self.valve_rho_unit.clone()),
                                        ("e", e.to_string()),
                                    ],
                                )
                            }
                        }
                    }
                });
            }
            if let Some(res) = &self.valve_result {
                ui.separator();
                ui.label(res);
                legend_toggle(
                    ui,
                    &txt("legend.valve.title", "Legend / notes"),
                    &txt("legend.valve.body", "Cv/Kv: flow coefficient, ΔP: pressure drop; note density and choking limits."),
                    &mut self.show_legend_valve,
                );
            }
        });
        ui.add_space(10.0);
        self.ui_bypass_panels(ui);
    }

    /// ST 바이패스 및 TCV 계산 패널.
    /// - Bypass Valve(증기): Cv/Kv 혹은 Stroke-Cv 테이블로 증기 유량을 계산하고, 필요 시 TCV(물) 결과를 합산해 엔탈피를 본다.
    /// - TCV(물): 별도 물 밸브 유량 계산을 제공하며, 결과가 바이패스 스프레이 값으로 자동 반영된다.
    fn ui_bypass_panels(&mut self, ui: &mut egui::Ui) {
        let tr = self.tr.clone();
        let txt = move |key: &str, default: &str| {
            tr.lookup(key).unwrap_or_else(|| default.to_string())
        };
        ui.heading(txt(
            "gui.bypass.heading",
            "Bypass Valve (steam) / TCV (water)",
        ));
        ui.label(txt(
            "gui.bypass.tip",
            "Use stroke-Cv table if available, otherwise single Cv/Kv.",
        ));
        ui.add_space(6.0);

        // ---------- ST Bypass Valve (증기) ----------
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading(txt("gui.bypass.steam.heading", "Bypass Valve (steam)"));
            egui::Grid::new("bypass_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(txt("gui.bypass.steam.up_p", "Upstream pressure"));
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

                    ui.label(txt("gui.bypass.steam.up_t", "Upstream temperature"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.bypass_up_t).speed(1.0));
                        unit_combo(ui, &mut self.bypass_t_unit, temperature_unit_options());
                    });
                    ui.end_row();

                    ui.label(txt("gui.bypass.steam.down_p", "Downstream pressure"));
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

                    ui.label(txt("gui.bypass.steam.cv", "Cv/Kv"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.bypass_cv).speed(1.0));
                        egui::ComboBox::from_id_source("bypass_cv_kind")
                            .selected_text(&self.bypass_cv_kind)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.bypass_cv_kind, "Cv(US)".into(), "Cv(US)");
                                ui.selectable_value(&mut self.bypass_cv_kind, "Cv(UK)".into(), "Cv(UK)");
                                ui.selectable_value(&mut self.bypass_cv_kind, "Kv".into(), "Kv");
                            });
                        ui.label(txt("gui.bypass.steam.stroke", "Stroke (%)"));
                        ui.add(
                            egui::DragValue::new(&mut self.bypass_open_pct)
                                .speed(1.0)
                                .clamp_range(0.0..=100.0),
                        );
                    });
                    ui.end_row();
                    ui.label(txt(
                        "gui.bypass.steam.h_override",
                        "Steam enthalpy input (kJ/kg, 0=auto IF97)",
                    ));
                    ui.add(egui::DragValue::new(&mut self.bypass_h_override_kj_per_kg).speed(10.0));
                    ui.end_row();
                    if stroke_based_kv_available(&self.bypass_stroke_points, &self.bypass_cv_points) {
                        let cv_interp = interpolate_stroke_cv(
                            &self.bypass_stroke_points,
                            &self.bypass_cv_points,
                            self.bypass_open_pct,
                        );
                        ui.label(format!(
                            "{}",
                            fill_template(
                                &txt(
                                    "gui.bypass.steam.cv_interp",
                                    "Interpolated Cv/Kv≈{cv:.3} (stroke {stroke:.1}%)",
                                ),
                                &[
                                    ("cv", format!("{:.3}", cv_interp)),
                                    ("stroke", format!("{:.1}", self.bypass_open_pct)),
                                ],
                            )
                        ));
                        ui.end_row();
                    }
                });

            ui.label(txt(
                "gui.bypass.steam.table",
                "Stroke-Cv/Kv table (bypass)",
            ));
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
                if ui
                    .small_button(txt("gui.bypass.table.add_row", "+ Add row"))
                    .clicked()
                {
                    self.bypass_stroke_points.push(100.0);
                    self.bypass_cv_points.push(0.0);
                }
                ui.label(txt(
                    "gui.bypass.table.note",
                    "Interpolation uses Cv for the matching stroke percent.",
                ));
            });
            if let Some(idx) = remove_idx {
                if self.bypass_stroke_points.len() > 1 {
                    self.bypass_stroke_points.remove(idx);
                    self.bypass_cv_points.remove(idx);
                }
            }

            ui.add_space(6.0);
            if ui
                .button(txt("gui.bypass.run", "Calculate bypass"))
                .clicked()
            {
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
                    Some(txt("gui.bypass.error.dp_nonpos", "Error: ΔP must be > 0").to_string())
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
                                        txt(
                                            "gui.bypass.steam.warn.choked",
                                            " [Warning: potential choked flow]",
                                        )
                                    } else {
                                        String::new()
                                    };
                                    Some(fill_template(
                                        &txt(
                                            "gui.bypass.steam.result",
                                            "Steam Q={q:.3} m³/h, m={m:.2} kg/h{warn}; spray={spray:.1} kg/h → mixed h≈{h_mix:.1} kJ/kg, total heat≈{heat:.1} kW (Pu={pu:.2} bar(a), Pd={pd:.2} bar(a), Kv={kv:.2})",
                                        ),
                                        &[
                                            ("q", format!("{:.3}", q_m3h)),
                                            ("m", format!("{:.2}", m_steam)),
                                            ("spray", format!("{:.1}", m_spray)),
                                            ("h_mix", format!("{:.1}", h_mix / 1000.0)),
                                            ("heat", format!("{:.1}", total_heat_kw)),
                                            ("pu", format!("{:.2}", up_abs)),
                                            ("pd", format!("{:.2}", down_abs)),
                                            ("kv", format!("{:.2}", kv)),
                                            ("warn", warn),
                                        ],
                                    ))
                                }
                                Err(e) => Some(fill_template(
                                    &txt(
                                        "gui.bypass.steam.error.flow",
                                        "Error(Kv={kv:.2}, ΔP={dp:.2} bar, ρ={rho:.2} kg/m3): {e}",
                                    ),
                                    &[
                                        ("kv", format!("{:.2}", kv)),
                                        ("dp", format!("{:.2}", dp)),
                                        ("rho", format!("{:.2}", rho)),
                                        ("e", e.to_string()),
                                    ],
                                )),
                            }
                        }
                        Err(e) => Some(fill_template(
                            &txt("gui.bypass.steam.error.if97", "IF97 calculation failed: {e}"),
                            &[("e", e.to_string())],
                        )),
                    }
                };
            }
            if let Some(res) = &self.bypass_result {
                ui.label(res);
            }
        });

        ui.add_space(12.0);

        // ---------- {t_head} ----------
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.heading(txt("gui.bypass.water.heading", "Bypass TCV (water)"));
            egui::Grid::new("spray_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(txt("gui.bypass.water.up_p", "Upstream pressure"));
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

                    ui.label(txt("gui.bypass.water.down_p", "Downstream pressure"));
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

                    ui.label(txt("gui.bypass.water.temp", "Water temperature"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.spray_temp).speed(0.5));
                        unit_combo(ui, &mut self.spray_temp_unit, temperature_unit_options());
                    });
                    ui.end_row();

                    ui.label(txt("gui.bypass.water.density", "Density [kg/m3]"));
                    ui.add(egui::DragValue::new(&mut self.spray_density).speed(1.0));
                    ui.end_row();

                    ui.label(txt("gui.bypass.steam.cv", "Cv/Kv"));
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.spray_cv).speed(1.0));
                        egui::ComboBox::from_id_source("spray_cv_kind")
                            .selected_text(&self.spray_cv_kind)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.spray_cv_kind, "Cv(US)".into(), "Cv(US)");
                                ui.selectable_value(&mut self.spray_cv_kind, "Cv(UK)".into(), "Cv(UK)");
                                ui.selectable_value(&mut self.spray_cv_kind, "Kv".into(), "Kv");
                            });
                        ui.label(txt("gui.bypass.water.stroke", "Stroke (%)"));
                        ui.add(
                            egui::DragValue::new(&mut self.spray_open_pct)
                                .speed(1.0)
                                .clamp_range(0.0..=100.0),
                        );
                    });
                    ui.end_row();
                    ui.label(txt("gui.bypass.water.h_override", "Water enthalpy input (kJ/kg, 0=auto)"));
                    ui.add(egui::DragValue::new(&mut self.spray_h_override_kj_per_kg).speed(10.0));
                    ui.end_row();
                    if stroke_based_kv_available(&self.spray_stroke_points, &self.spray_cv_points) {
                        let cv_interp = interpolate_stroke_cv(
                            &self.spray_stroke_points,
                            &self.spray_cv_points,
                            self.spray_open_pct,
                        );
                        ui.label(fill_template(&txt("gui.bypass.water.cv_interp", "Interpolated Cv/Kv≈{cv:.3} (stroke {stroke:.1}%)"), &[("cv", format!("{:.3}", cv_interp)), ("stroke", format!("{:.1}", self.spray_open_pct))]));
                        ui.end_row();
                    }
                });

            ui.label(txt("gui.bypass.water.table", "Stroke-Cv/Kv table (water)"));
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
                if ui.small_button(txt("gui.bypass.table.add_row", "+ Add row")).clicked() {
                    self.spray_stroke_points.push(100.0);
                    self.spray_cv_points.push(0.0);
                }
                ui.label(txt("gui.bypass.water.tip_table", "Empty rows are ignored; use +/- to add/remove."));
            });
            if let Some(idx) = remove_idx {
                if self.spray_stroke_points.len() > 1 {
                    self.spray_stroke_points.remove(idx);
                    self.spray_cv_points.remove(idx);
                }
            }

            ui.add_space(6.0);
            if ui.button(txt("gui.bypass.water.run", "Calculate TCV flow")).clicked() {
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
                    Some(
                        txt(
                            "gui.bypass.water.error.input",
                            "Error: ΔP and density must be > 0",
                        )
                        .to_string(),
                    )
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
                            Some(fill_template(
                                &txt(
                                    "gui.bypass.water.result",
                                    "TCV flow Q={q:.3} m³/h, m={m:.2} kg/h (ΔP={dp:.2} bar, Kv={kv:.2}) - used for bypass spray input",
                                ),
                                &[
                                    ("q", format!("{:.3}", q_m3h)),
                                    ("m", format!("{:.2}", mass)),
                                    ("dp", format!("{:.2}", dp)),
                                    ("kv", format!("{:.2}", kv)),
                                ],
                            ))
                        }
                        Err(e) => Some(fill_template(
                            &txt("gui.bypass.water.error.generic", "Error: {e}"),
                            &[("e", e.to_string())],
                        )),
                    }
                };
            }
            if let Some(res) = &self.spray_calc_result {
                ui.label(res);
            }
        });
    }
    fn ui_boiler(&mut self, ui: &mut egui::Ui) {
        let tr = self.tr.clone();
        let txt = move |key: &str, default: &str| {
            tr.lookup(key).unwrap_or_else(|| default.to_string())
        };
        heading_with_tip(
            ui,
            &txt("gui.boiler.heading", "Boiler Efficiency"),
            &txt(
                "gui.boiler.tip",
                "Compute basic boiler efficiency (PTC) from fuel input and steam/feedwater enthalpy.",
            ),
        );
        label_with_tip(
            ui,
            &txt(
                "gui.boiler.subheading",
                "Basic efficiency from fuel LHV, steam/feedwater enthalpy, losses.",
            ),
            &txt(
                "gui.boiler.subhint",
                "Enter LHV, steam/feedwater flows/enthalpy and losses to estimate efficiency.",
            ),
        );
        ui.add_space(8.0);
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::Grid::new("boiler_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(
                        ui,
                        &txt("gui.boiler.fuel_flow", "Fuel flow [unit/h]"),
                        &txt(
                            "gui.boiler.fuel_flow_tip",
                            "Fuel mass or volume flow (kg/h, Nm3/h, etc.)",
                        ),
                    );
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
                    label_with_tip(
                        ui,
                        &txt("gui.boiler.lhv", "Fuel LHV [kJ/unit]"),
                        &txt("gui.boiler.lhv_tip", "Lower heating value per fuel unit"),
                    );
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
                    label_with_tip(
                        ui,
                        &txt("gui.boiler.steam_flow", "Steam production [kg/h]"),
                        &txt("gui.boiler.steam_flow_tip", "Produced steam mass flow"),
                    );
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
                    label_with_tip(
                        ui,
                        &txt("gui.boiler.h_steam", "Steam enthalpy [kJ/kg]"),
                        &txt(
                            "gui.boiler.h_steam_tip",
                            "Enthalpy of produced steam (IF97 result is fine)",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.boiler_h_steam).speed(10.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_h_steam_unit,
                        &[
                            ("kJ/kg", "kJ/kg"),
                            ("kcal/kg", "kcal/kg"),
                            ("Btu/lb", "Btu/lb"),
                        ],
                    );
                    ui.end_row();
                    label_with_tip(
                        ui,
                        &txt("gui.boiler.h_fw", "Feedwater enthalpy [kJ/kg]"),
                        &txt("gui.boiler.h_fw_tip", "Feedwater enthalpy"),
                    );
                    ui.add(egui::DragValue::new(&mut self.boiler_h_fw).speed(5.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_h_fw_unit,
                        &[
                            ("kJ/kg", "kJ/kg"),
                            ("kcal/kg", "kcal/kg"),
                            ("Btu/lb", "Btu/lb"),
                        ],
                    );
                    ui.end_row();
                });
            if ui
                .button(txt("gui.boiler.run_basic", "Calculate efficiency"))
                .clicked()
            {
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
                self.boiler_result = Some(fill_template(
                    &txt(
                        "gui.boiler.result_basic",
                        "Efficiency={eff:.2} %, Useful heat={useful:.1} kW, Fuel heat={fuel:.1} kW",
                    ),
                    &[
                        ("eff", format!("{:.2}", res.efficiency * 100.0)),
                        ("useful", format!("{:.1}", res.useful_heat_kw)),
                        ("fuel", format!("{:.1}", res.fuel_heat_kw)),
                    ],
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
            &txt(
                "gui.boiler.ptc.heading",
                "PTC 4.0 extended (stack/radiation/blowdown losses)",
            ),
            &txt(
                "gui.boiler.ptc.tip",
                "Include flue gas losses, excess air, radiation and blowdown.",
            ),
        );
        egui::Frame::group(ui.style()).show(ui, |ui| {
            egui::Grid::new("boiler_ptc_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(
                        ui,
                        &txt("gui.boiler.ptc.fg_flow", "Flue gas flow"),
                        &txt("gui.boiler.ptc.fg_flow_tip", "Flue gas mass flow"),
                    );
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

                    label_with_tip(
                        ui,
                        &txt("gui.boiler.ptc.fg_cp", "Flue gas cp [kJ/kgK]"),
                        &txt("gui.boiler.ptc.fg_cp_tip", "Average flue gas cp"),
                    );
                    ui.add(egui::DragValue::new(&mut self.boiler_fg_cp).speed(0.01));
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.boiler.ptc.stack_temp", "Stack temperature"),
                        &txt(
                            "gui.boiler.ptc.stack_temp_tip",
                            "Stack/duct outlet temperature",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.boiler_stack_temp).speed(1.0));
                    unit_combo(ui, &mut self.boiler_temp_unit, temperature_unit_options());
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.boiler.ptc.ambient_temp", "Ambient temperature"),
                        &txt(
                            "gui.boiler.ptc.ambient_temp_tip",
                            "Reference/combustion air temperature",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.boiler_ambient_temp).speed(1.0));
                    unit_combo(ui, &mut self.boiler_temp_unit, temperature_unit_options());
                    ui.end_row();

                    ui.small(txt(
                "gui.valve.tip_mmhg",
                "Tip: mmHg is treated as gauge (0=atm, -760mmHg=vacuum).",
            ));

                    label_with_tip(
                        ui,
                        &txt("gui.boiler.ptc.excess_air", "Excess air ratio"),
                        &txt(
                            "gui.boiler.ptc.excess_air_tip",
                            "Actual air vs theoretical air ratio",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.boiler_excess_air).speed(0.01));
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.boiler.ptc.rad_loss", "Radiation/surface loss [%]"),
                        &txt(
                            "gui.boiler.ptc.rad_loss_tip",
                            "Surface radiation/convection loss fraction",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.boiler_rad_loss).speed(0.005));
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.boiler.ptc.blowdown_rate", "Blowdown rate"),
                        &txt(
                            "gui.boiler.ptc.blowdown_rate_tip",
                            "Boiler blowdown fraction",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.boiler_blowdown_rate).speed(0.005));
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.boiler.ptc.blowdown_h", "Blowdown enthalpy"),
                        &txt("gui.boiler.ptc.blowdown_h_tip", "Blowdown effluent enthalpy"),
                    );
                    ui.add(egui::DragValue::new(&mut self.boiler_blowdown_h).speed(5.0));
                    unit_combo(
                        ui,
                        &mut self.boiler_blowdown_h_unit,
                        &[("kJ/kg", "kJ/kg"), ("Btu/lb", "Btu/lb")],
                    );
                    ui.end_row();
                });

            if ui
                .button(txt("gui.boiler.ptc.run", "Calculate PTC 4.0 efficiency"))
                .clicked()
            {
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
                self.boiler_result = Some(fill_template(
                    &txt(
                        "gui.boiler.ptc.result",
                        "PTC efficiency={eff:.2} %, Useful heat={useful:.1} kW, Fuel heat={fuel:.1} kW",
                    ),
                    &[
                        ("eff", format!("{:.2}", res.efficiency * 100.0)),
                        ("useful", format!("{:.1}", res.useful_heat_kw)),
                        ("fuel", format!("{:.1}", res.fuel_heat_kw)),
                    ],
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
        let tr = self.tr.clone();
        let txt = move |key: &str, default: &str| {
            tr.lookup(key).unwrap_or_else(|| default.to_string())
        };
        heading_with_tip(
            ui,
            &txt(
                "gui.cooling.heading",
                "Cooling / Condenser / NPSH / Drain Cooler",
            ),
            &txt(
                "gui.cooling.tip",
                "Condenser heat balance, cooling tower range/approach, pump NPSH, drain/reheater LMTD",
            ),
        );
        label_with_tip(
            ui,
            &txt(
                "gui.cooling.subheading",
                "Condenser heat balance, cooling tower range/approach, pump NPSH, drain/reheater LMTD",
            ),
            &txt(
                "gui.cooling.subhint",
                "Fill each card to compute instantly.",
            ),
        );
        ui.add_space(8.0);

        // 콘덴서
        egui::Frame::group(ui.style()).show(ui, |ui| {
            heading_with_tip(
                ui,
                &txt(
                    "gui.cooling.cond.heading",
                    "Condenser Heat Balance / Vacuum",
                ),
                &txt(
                    "gui.cooling.cond.tip",
                    "Card to compute steam Tsat/vacuum/LMTD together",
                ),
            );
            ui.small(txt(
                "gui.cooling.cond.note",
                "Steam Tsat/LMTD auto calc; mmHg is gauge (0=atm).",
            ));
            egui::Grid::new("condenser_grid")
                .num_columns(4)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.checkbox(
                        &mut self.condenser_auto_condensing_from_pressure,
                        txt("gui.cooling.cond.auto_tsat", "auto Tsat"),
                    )
                    .on_hover_text(txt(
                        "gui.cooling.cond.auto_tsat_tip",
                        "Use pressure to auto-calc Tsat/Psat.",
                    ));
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.cond.steam_p", "Steam pressure"),
                        &txt(
                            "gui.cooling.cond.steam_p_tip",
                            "Condenser steam/non-condensable pressure",
                        ),
                    );
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
                            txt("gui.steam.mode.gauge", "Gauge (G)"),
                        );
                        ui.selectable_value(
                            &mut self.condenser_pressure_mode,
                            conversion::PressureMode::Absolute,
                            txt("gui.steam.mode.absolute", "Absolute (A)"),
                        );
                    });
                    ui.end_row();

                    ui.checkbox(
                        &mut self.condenser_auto_backpressure_from_temp,
                        txt("gui.cooling.cond.auto_psat", "auto Psat"),
                    )
                    .on_hover_text(txt(
                        "gui.cooling.cond.auto_psat_tip",
                        "Use Tsat to auto-calc Psat.",
                    ));
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.cond.steam_t", "Steam temperature"),
                        &txt(
                            "gui.cooling.cond.steam_t_tip",
                            "Condenser steam temperature (auto Tsat possible)",
                        ),
                    );
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_temp_c).speed(0.5))
                        .changed()
                    {
                        self.condenser_auto_condensing_from_pressure = false;
                        self.condenser_auto_backpressure_from_temp = false;
                        self.condenser_use_manual_temp = true;
                    }
                    unit_combo(ui, &mut self.condenser_cw_temp_unit, temperature_unit_options());
                    ui.checkbox(
                        &mut self.condenser_use_manual_temp,
                        txt("gui.cooling.cond.manual_input", "Manual input"),
                    );
                    ui.end_row();

                    ui.checkbox(
                        &mut self.condenser_auto_cw_out_from_range,
                        txt("gui.cooling.cond.auto_tout", "auto Tout"),
                    )
                    .on_hover_text(txt(
                        "gui.cooling.cond.auto_tout_tip",
                        "Use range target to auto-calc outlet temp.",
                    ));
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.cond.cw_in_out", "Cooling water in/out"),
                        &txt(
                            "gui.cooling.cond.cw_in_out_tip",
                            "Circulating cooling water inlet/outlet temps (auto range supported)",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.condenser_cw_in).speed(0.5));
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_cw_out).speed(0.5))
                        .changed()
                    {
                        self.condenser_auto_cw_out_from_range = false;
                    }
                    unit_combo(ui, &mut self.condenser_cw_temp_unit, temperature_unit_options());
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.cooling.cond.range_target", "Range target (in-out)"),
                        &txt(
                            "gui.cooling.cond.range_target_tip",
                            "Cooling water inlet-outlet temperature difference target",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.ct_range_target).speed(0.2));
                    ui.label("°C");
                    ui.end_row();

                    ui.label("");
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.cond.cw_flow", "Cooling water flow"),
                        &txt(
                            "gui.cooling.cond.cw_flow_tip",
                            "Circulating cooling water flow",
                        ),
                    );
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

                    ui.checkbox(
                        &mut self.condenser_auto_ua_from_area_u,
                        txt("gui.cooling.cond.auto_ua", "auto UA"),
                    )
                    .on_hover_text(txt(
                        "gui.cooling.cond.auto_ua_tip",
                        "Auto-calc UA from area × U",
                    ));
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.cond.ua", "UA [kW/K]"),
                        &txt("gui.cooling.cond.ua_tip", "Area × U"),
                    );
                    if ui
                        .add(egui::DragValue::new(&mut self.condenser_ua).speed(1.0))
                        .changed()
                    {
                        self.condenser_auto_ua_from_area_u = false;
                    }
                    ui.end_row();

                    ui.checkbox(
                        &mut self.condenser_auto_area_required,
                        txt("gui.cooling.cond.auto_area", "auto area (required)"),
                    )
                    .on_hover_text(txt(
                        "gui.cooling.cond.auto_area_tip",
                        "Auto-calc required area; uncheck to validate entered area.",
                    ));
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.cond.area_u", "Area / U"),
                        &txt(
                            "gui.cooling.cond.area_u_tip",
                            "Enter heat transfer area and U to validate",
                        ),
                    );
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

                    ui.checkbox(
                        &mut self.condenser_auto_backpressure_from_temp,
                        txt("gui.cooling.cond.auto_backpressure", "auto backpressure"),
                    );
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.cond.backpressure", "Target backpressure"),
                        &txt(
                            "gui.cooling.cond.backpressure_tip",
                            "Enter compressor/turbine backpressure target or auto-calc from Tsat",
                        ),
                    );
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
                            txt("gui.steam.mode.absolute", "Absolute (A)"),
                        );
                        ui.selectable_value(
                            &mut self.condenser_backpressure_mode,
                            conversion::PressureMode::Gauge,
                            txt("gui.steam.mode.gauge", "Gauge (G)"),
                        );
                    });
                    ui.end_row();
                });
            ui.collapsing(txt("gui.cooling.cond.help", "Input help"), |ui| {
                ui.label(txt(
                    "gui.cooling.cond.help_backpressure",
                    "Backpressure/Psat: Psat = condenser vacuum. Gauge is atm-referenced.",
                ));
                ui.label(txt(
                    "gui.cooling.cond.help_ua",
                    "UA: U[W/m²K] × Area[m²] / 1000 = kW/K.",
                ));
                ui.label(txt(
                    "gui.cooling.cond.help_range",
                    "Range: CW inlet-outlet ΔT. Auto checked → outlet auto-calculated.",
                ));
                ui.label(txt(
                    "gui.cooling.cond.help_mmhg",
                    "mmHg is gauge (0=atm, -760=vacuum).",
                ));
            });
            if ui
                .button(txt("gui.cooling.cond.run", "Run condenser calc"))
                .clicked()
            {
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
                        let mut text = fill_template(
                            &txt(
                                "gui.cooling.cond.result",
                                "Tsat={tsat:.2} {t_unit}, Psat={psat:.4} {p_unit}{mode}, LMTD={lmtd:.2} K, Q≈{q:.1} kW",
                            ),
                            &[
                                ("tsat", format!("{:.2}", cond_temp_out)),
                                ("t_unit", self.condenser_cw_temp_unit.clone()),
                                ("psat", format!("{:.4}", cond_press_out)),
                                ("p_unit", self.condenser_pressure_unit.clone()),
                                (
                                    "mode",
                                    if self.condenser_pressure_mode == conversion::PressureMode::Gauge {
                                        "g".into()
                                    } else {
                                        "a".into()
                                    },
                                ),
                                ("lmtd", format!("{:.2}", res.lmtd_k)),
                                ("q", format!("{:.1}", res.heat_duty_kw)),
                            ],
                        );
                        if !res.warnings.is_empty() {
                            text.push_str(&txt("gui.cooling.cond.warn_prefix", "\nWarning: "));
                            text.push_str(&res.warnings.join(" / "));
                        }
                        // 면적/UA 관련 추가 정보
                        if self.condenser_auto_area_required && self.condenser_u > 0.0 {
                            let area_req =
                                (res.heat_duty_kw * 1000.0) / (self.condenser_u * res.lmtd_k.max(1e-6));
                            self.condenser_area = area_req;
                            text.push_str(&fill_template(
                                &txt(
                                    "gui.cooling.cond.area_req",
                                    "\nRequired area≈{area:.2} m² (U={u:.1} W/m²K)",
                                ),
                                &[
                                    ("area", format!("{:.2}", area_req)),
                                    ("u", format!("{:.1}", self.condenser_u)),
                                ],
                            ));
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
                            text.push_str(&fill_template(
                                &txt(
                                    "gui.cooling.cond.area_check",
                                    "\nArea={area:.2} m², U={u:.1} W/m²K → Qcap≈{qcap:.1} kW, load ratio≈{lr:.2}x",
                                ),
                                &[
                                    ("area", format!("{:.2}", self.condenser_area)),
                                    ("u", format!("{:.1}", self.condenser_u)),
                                    ("qcap", format!("{:.1}", q_cap)),
                                    ("lr", format!("{:.2}", load_ratio)),
                                ],
                            ));
                            if load_ratio > 1.0 {
                                text.push_str(&fill_template(
                                    &txt(
                                        "gui.cooling.cond.area_warn_over",
                                        "\n⚠ Load exceeds design. Operable to about {pct:.0}% (Qcap basis). Lower CW temp/raise flow or increase area/U.",
                                    ),
                                    &[("pct", format!("{:.0}", capable_pct))],
                                ));
                            } else {
                                text.push_str(&txt(
                                    "gui.cooling.cond.area_warn_ok",
                                    "\nWithin design load (load ≤ capacity).",
                                ));
                            }
                        }
                        text
                    }
                    Err(e) => match e {
                        condenser::CoolingError::NegativeDeltaT => {
                            txt(
                                "gui.cooling.cond.error.delta_t",
                                "Error: cooling water temperature crosses saturation temperature.",
                            )
                            .to_string()
                        }
                        condenser::CoolingError::If97(msg) => fill_template(
                            &txt("gui.cooling.cond.error.if97", "Saturation calc error: {msg}"),
                            &[("msg", msg)],
                        ),
                    },
                });
            }
            if let Some(res) = &self.condenser_result {
                ui.separator();
                for line in res.lines() {
                    if line.starts_with(&txt("gui.cooling.cond.warn_prefix", "Warning:")) {
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
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.npsh.suction_p", "Suction pressure"),
                        &txt(
                            "gui.cooling.npsh.suction_p_tip",
                            "Pump suction pressure (gauge/absolute)",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.npsh_suction_p).speed(0.1));
                    unit_combo(ui, &mut self.npsh_suction_unit, pressure_unit_options());
                    ui.selectable_value(
                        &mut self.npsh_suction_mode,
                        conversion::PressureMode::Gauge,
                        txt("gui.steam.mode.gauge", "Gauge (G)"),
                    );
                    ui.selectable_value(
                        &mut self.npsh_suction_mode,
                        conversion::PressureMode::Absolute,
                        txt("gui.steam.mode.absolute", "Absolute (A)"),
                    );
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.cooling.npsh.temp", "Liquid temperature"),
                        &txt(
                            "gui.cooling.npsh.temp_tip",
                            "Suction liquid temperature (for vapor pressure)",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.npsh_temp).speed(0.5));
                    unit_combo(ui, &mut self.npsh_temp_unit, temperature_unit_options());
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.cooling.npsh.head_friction", "Static head / friction [m]"),
                        &txt(
                            "gui.cooling.npsh.head_friction_tip",
                            "Static head from surface to pump / friction head loss",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.npsh_static_head).speed(0.2));
                    ui.add(egui::DragValue::new(&mut self.npsh_friction).speed(0.2));
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.cooling.npsh.rho_npshr", "Density / NPSHr"),
                        &txt(
                            "gui.cooling.npsh.rho_npshr_tip",
                            "Suction liquid density and manufacturer NPSHr",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.npsh_rho).speed(1.0));
                    unit_combo(
                        ui,
                        &mut self.npsh_rho_unit,
                        &[("kg/m3", "kg/m3"), ("lb/ft3", "lb/ft3")],
                    );
                    ui.add(egui::DragValue::new(&mut self.npsh_required).speed(0.2));
                    ui.end_row();
                });
            if ui
                .button(txt("gui.cooling.npsh.run", "Run NPSH calc"))
                .clicked()
            {
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
                let mut msg = fill_template(
                    &txt(
                        "gui.cooling.npsh.result",
                        "NPSHa={npsha:.2} m, Margin={margin:.2}",
                    ),
                    &[
                        ("npsha", format!("{:.2}", res.npsha_m)),
                        ("margin", format!("{:.2}", res.margin_ratio)),
                    ],
                );
                if !res.warnings.is_empty() {
                    msg.push_str(&txt("gui.cooling.npsh.warn_prefix", "\nWarning: "));
                    msg.push_str(&res.warnings.join(" / "));
                }
                self.npsh_result = Some(msg);
            }
            if let Some(res) = &self.npsh_result {
                ui.separator();
                for line in res.lines() {
                    if line.starts_with(&txt("gui.cooling.npsh.warn_prefix", "Warning:")) {
                        ui.colored_label(ui.visuals().warn_fg_color, line);
                    } else {
                        ui.label(line);
                    }
                }
                ui.small(txt(
                    "gui.cooling.npsh.note",
                    "Note: Margin<1.1 ⇒ high cavitation risk. Raise suction pressure / lower temperature / cut friction.",
                ));
            }
        });

        ui.add_space(8.0);
        // 드레인/재열기
        egui::Frame::group(ui.style()).show(ui, |ui| {
            heading_with_tip(
                ui,
                &txt(
                    "gui.cooling.drain.heading",
                    "Drain Cooler / Reheater Heat Balance",
                ),
                &txt(
                    "gui.cooling.drain.tip",
                    "Compute LMTD and heat balance from shell/tube inlet/outlet temps and flows",
                ),
            );
            egui::Grid::new("drain_grid")
                .num_columns(3)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.drain.shell_in_out", "Shell IN/OUT"),
                        &txt("gui.cooling.drain.shell_in_out_tip", "Shell-side inlet/outlet temperature"),
                    );
                    ui.add(egui::DragValue::new(&mut self.drain_shell_in).speed(0.5));
                    ui.add(egui::DragValue::new(&mut self.drain_shell_out).speed(0.5));
                    unit_combo(ui, &mut self.drain_temp_unit, temperature_unit_options());
                    ui.end_row();
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.drain.tube_in_out", "Tube IN/OUT"),
                        &txt("gui.cooling.drain.tube_in_out_tip", "Tube-side inlet/outlet temperature"),
                    );
                    ui.add(egui::DragValue::new(&mut self.drain_tube_in).speed(0.5));
                    ui.add(egui::DragValue::new(&mut self.drain_tube_out).speed(0.5));
                    unit_combo(ui, &mut self.drain_temp_unit, temperature_unit_options());
                    ui.end_row();
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.drain.flow", "Shell/Tube flow"),
                        &txt("gui.cooling.drain.flow_tip", "Shell-side / tube-side flow"),
                    );
                    ui.add(egui::DragValue::new(&mut self.drain_shell_flow).speed(1.0));
                    ui.add(egui::DragValue::new(&mut self.drain_tube_flow).speed(1.0));
                    unit_combo(
                        ui,
                        &mut self.drain_flow_unit,
                        &[("m3/h", "m3/h"), ("gpm", "gpm")],
                    );
                    ui.end_row();
                    label_with_tip(
                        ui,
                        &txt("gui.cooling.drain.ua_area_u", "UA or Area/U"),
                        &txt(
                            "gui.cooling.drain.ua_area_u_tip",
                            "Enter UA directly or area/U to compute UA",
                        ),
                    );
                    ui.add(egui::DragValue::new(&mut self.drain_ua).speed(1.0));
                    ui.add(egui::DragValue::new(&mut self.drain_area).speed(0.5));
                    ui.add(egui::DragValue::new(&mut self.drain_u).speed(5.0));
                    ui.end_row();
                });
            if ui
                .button(txt("gui.cooling.drain.run", "Run heat balance"))
                .clicked()
            {
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
                    overall_u_w_m2k: if self.drain_u > 0.0 {
                        Some(self.drain_u)
                    } else {
                        None
                    },
                });
                let mut msg = fill_template(
                    &txt(
                        "gui.cooling.drain.result",
                        "LMTD={lmtd:.2} K, Shell Q={shell:.1} kW, Tube Q={tube:.1} kW, Imbalance={imb:.1} kW",
                    ),
                    &[
                        ("lmtd", format!("{:.2}", res.lmtd_k)),
                        ("shell", format!("{:.1}", res.shell_heat_kw)),
                        ("tube", format!("{:.1}", res.tube_heat_kw)),
                        ("imb", format!("{:.1}", res.imbalance_kw)),
                    ],
                );
                if !res.warnings.is_empty() {
                    msg.push_str(&txt("gui.cooling.drain.warn_prefix", "\nWarning: "));
                    msg.push_str(&res.warnings.join(" / "));
                }
                self.drain_result = Some(msg);
            }
            if let Some(res) = &self.drain_result {
                ui.separator();
                for line in res.lines() {
                    if line.starts_with(&txt("gui.cooling.drain.warn_prefix", "Warning:")) {
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
        let tr = self.tr.clone();
        let txt = move |key: &str, default: &str| {
            tr.lookup(key).unwrap_or_else(|| default.to_string())
        };
        heading_with_tip(
            ui,
            &txt("gui.plant.heading", "Plant Piping"),
            &txt(
                "gui.plant.tip",
                "Orifice/nozzle flow, thermal expansion, pressure rating",
            ),
        );
        label_with_tip(
            ui,
            &txt("gui.plant.subheading", "Orifice/nozzle check, thermal expansion, pressure rating"),
            &txt(
                "gui.plant.subhint",
                "Compressibility(Y), expansion, and pressure rating on one screen",
            ),
        );
        ui.add_space(8.0);

        egui::Frame::group(ui.style()).show(ui, |ui| {
            heading_with_tip(
                ui,
                &txt("gui.plant.orifice.heading", "Orifice / Nozzle flow check"),
                &txt(
                    "gui.plant.orifice.tip",
                    "Verify differential-pressure meter or nozzle flow",
                ),
            );
            egui::Grid::new("plant_orifice")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    label_with_tip(
                        ui,
                        &txt("gui.plant.orifice.up_p", "{t_up_p}"),
                        &txt(
                            "gui.plant.orifice.up_p_tip",
                            "Nozzle/orifice {t_up_p} (gauge/absolute)",
                        ),
                    );
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.plant_up_p).speed(0.1));
                        unit_combo(ui, &mut self.plant_up_unit, pressure_unit_options());
                        ui.selectable_value(
                            &mut self.plant_up_mode,
                            conversion::PressureMode::Gauge,
                            txt("gui.steam.mode.gauge", "Gauge (G)"),
                        );
                        ui.selectable_value(
                            &mut self.plant_up_mode,
                            conversion::PressureMode::Absolute,
                            txt("gui.steam.mode.absolute", "Absolute (A)"),
                        );
                    });
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.plant.orifice.dp", "ΔP"),
                        &txt(
                            "gui.plant.orifice.dp_tip",
                            "Pressure drop across orifice/nozzle",
                        ),
                    );
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut self.plant_dp).speed(0.1));
                        unit_combo(ui, &mut self.plant_dp_unit, pressure_unit_options());
                        ui.selectable_value(
                            &mut self.plant_dp_mode,
                            conversion::PressureMode::Gauge,
                            txt("gui.steam.mode.gauge", "Gauge (G)"),
                        );
                        ui.selectable_value(
                            &mut self.plant_dp_mode,
                            conversion::PressureMode::Absolute,
                            txt("gui.steam.mode.absolute", "Absolute (A)"),
                        );
                    });
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.plant.orifice.rho", "Fluid density"),
                        &txt("gui.plant.orifice.rho_tip", "Density at operating condition"),
                    );
                    ui.add(egui::DragValue::new(&mut self.plant_rho).speed(1.0));
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.plant.orifice.diameter", "Diameter"),
                        &txt(
                            "gui.plant.orifice.diameter_tip",
                            "Orifice/nozzle effective diameter (m or mm)",
                        ),
                    );
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.plant_diameter_m)
                                .speed(0.001)
                                .clamp_range(0.0..=5.0),
                        );
                        unit_combo(ui, &mut self.plant_diam_unit, &[("m", "m"), ("mm", "mm")]);
                    });
                    ui.end_row();

                    label_with_tip(
                        ui,
                        &txt("gui.plant.orifice.shape_cd", "Shape / Cd"),
                        &txt(
                            "gui.plant.orifice.shape_cd_tip",
                            "Select shape to set Cd; adjust if needed",
                        ),
                    );
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

                    label_with_tip(
                        ui,
                        &txt("gui.plant.orifice.beta_k", "Beta(diameter ratio) / k(specific heat ratio)"),
                        &txt(
                            "gui.plant.orifice.beta_k_tip",
                            "beta=orifice/pipe diameter ratio, k=gamma",
                        ),
                    );
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

                    label_with_tip(
                        ui,
                        &txt("gui.plant.orifice.compressible", "Use compressible correction"),
                        &txt(
                            "gui.plant.orifice.compressible_tip",
                            "Apply Y-factor for steam/gas flow",
                        ),
                    );
                    ui.checkbox(
                        &mut self.plant_compressible,
                        txt("gui.plant.orifice.compressible_toggle", "Compressible (Y)"),
                    );
                    ui.end_row();
                });
            if ui
                .button(txt("gui.plant.orifice.run", "Calculate flow"))
                .clicked()
            {
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
                    self.plant_result = Some(txt(
                        "gui.plant.orifice.error.input",
                        "Error: ΔP, density, and diameter must be > 0.",
                    )
                    .to_string());
                } else {
                    let dp_pa = dp_bar * 1.0e5;
                    let area = std::f64::consts::PI * (d_m.powi(2)) / 4.0;
                    if self.plant_compressible {
                        if pu_bar_abs <= dp_bar {
                            self.plant_result = Some(txt(
                                "gui.plant.orifice.error.up_lt_dp",
                                "Error: upstream pressure must exceed ΔP (compressible).",
                            )
                            .to_string());
                        } else {
                            let beta = self.plant_beta.clamp(0.1, 0.99);
                            let k = self.plant_gamma.clamp(1.0, 1.7);
                            let ratio = (dp_bar / pu_bar_abs).min(0.9);
                            let y = (1.0 - (0.41 + 0.35 * beta * beta) * ratio).clamp(0.1, 1.0);
                            let c = self.plant_cd / (1.0 - beta.powi(4)).sqrt();
                            let m_kg_s = c * y * area * (2.0 * self.plant_rho * dp_pa).sqrt();
                            let m_kg_h = m_kg_s * 3600.0;
                            let q_m3_h = m_kg_h / self.plant_rho;
                            self.plant_result = Some(fill_template(
                                &txt(
                                    "gui.plant.orifice.result.comp",
                                    "Compressible: Q≈{q:.3} m³/h, m≈{m:.2} kg/h (Cd={cd:.2}, Y={y:.3}, beta={beta:.2}, k={k:.2}, dp={dp:.3} bar)",
                                ),
                                &[
                                    ("q", format!("{:.3}", q_m3_h)),
                                    ("m", format!("{:.2}", m_kg_h)),
                                    ("cd", format!("{:.2}", self.plant_cd)),
                                    ("y", format!("{:.3}", y)),
                                    ("beta", format!("{:.2}", beta)),
                                    ("k", format!("{:.2}", k)),
                                    ("dp", format!("{:.3}", dp_bar)),
                                ],
                            ));
                        }
                    } else {
                        let q_m3_s = self.plant_cd * area * (2.0 * dp_pa / self.plant_rho).sqrt();
                        let q_m3_h = q_m3_s * 3600.0;
                        let m_kg_h = q_m3_h * self.plant_rho;
                        self.plant_result = Some(fill_template(
                            &txt(
                                "gui.plant.orifice.result.incomp",
                                "Incompressible: Q≈{q:.3} m³/h, m≈{m:.2} kg/h (Cd={cd:.2}, dp={dp:.3} bar)",
                            ),
                            &[
                                ("q", format!("{:.3}", q_m3_h)),
                                ("m", format!("{:.2}", m_kg_h)),
                                ("cd", format!("{:.2}", self.plant_cd)),
                                ("dp", format!("{:.3}", dp_bar)),
                            ],
                        ));
                    }
                }
            }
            if let Some(res) = &self.plant_result {
                ui.label(res);
                legend_toggle(
                    ui,
                    &txt("legend.plant.title", "Legend / notes"),
                    &txt("legend.plant.body", "Formula: incompressible Q = Cd·A·√(2·ΔP/ρ); compressible uses Y·C(1-β⁴)^-0.5"),
                    &mut self.show_legend_plant,
                );
            }
        });
        ui.add_space(10.0);
        self.ui_bypass_panels(ui);
    }

}

impl App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // 최초 1회 화면 크기 조정
        if self.apply_initial_view_size {
            if let Some(screen) = ctx.input(|i| {
                let r = i.screen_rect();
                if r.is_positive() {
                    Some(r.size())
                } else {
                    None
                }
            }) {
                let target = egui::vec2((screen.x * 0.60).max(1000.0), (screen.y * 0.60).max(700.0));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(target));
                self.apply_initial_view_size = false;
            }
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        }));

        // 투명도 적용 + 라벨 복사 방지 스타일
        let mut style = (*ctx.style()).clone();
        style.interaction.selectable_labels = false;
        style.visuals.window_fill = style.visuals.window_fill.linear_multiply(self.window_alpha);
        style.visuals.panel_fill = style.visuals.panel_fill.linear_multiply(self.window_alpha);
        ctx.set_style(style);

        let tr = self.tr.clone();
        let txt = move |key: &str, default: &str| {
            tr.lookup(key).unwrap_or_else(|| default.to_string())
        };

        // 외부 진공 포화 온도 창
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
                        vacuum_table_ui(ui, &txt);
                    });
                },
            );
            if close_flag.get() {
                self.show_vacuum_table_viewport = false;
            }
        }

        // 상단 바
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(txt("gui.nav.app_title", "Steam Engineering Toolbox"));
                ui.label(" | Desktop GUI");
                ui.separator();
                if ui.button(txt("gui.formula.button", "Formula reference")).clicked() {
                    self.show_formula_modal = true;
                }
                if ui.button(txt("gui.settings.title", "Settings")).clicked() {
                    self.show_settings_modal = true;
                }
                if ui.button(txt("gui.about.title", "Help / About")).clicked() {
                    self.show_help_modal = true;
                }
            });
        });

        // 설정 모달
        if self.show_settings_modal {
            let mut new_unit_system = self.config.unit_system;
            egui::Window::new(txt("gui.settings.title", "Program Settings"))
                .collapsible(false)
                .resizable(true)
                .open(&mut self.show_settings_modal)
                .show(ctx, |ui| {
                    ui.heading(txt("gui.settings.general", "General"));
                    ui.separator();
                    ui.label(txt("gui.settings.unit_preset", "Unit system preset"));
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
                    ui.separator();
                    ui.label(txt("gui.settings.ui_scale", "UI scale"));
                    let scale_slider = egui::Slider::new(&mut self.ui_scale, 0.8..=1.6).suffix(" x");
                    if ui.add(scale_slider).changed() {
                        ctx.set_pixels_per_point(self.ui_scale);
                    }
                    ui.separator();
                    ui.checkbox(&mut self.always_on_top, txt("gui.settings.always_on_top", "Always on top"));
                    ui.separator();
                    ui.label(txt("gui.settings.alpha", "Window transparency"));
                    ui.add(egui::Slider::new(&mut self.window_alpha, 0.3..=1.0).text("alpha"));

                    ui.separator();
                    ui.label(txt("gui.settings.lang", "Language"));
                    egui::ComboBox::from_id_source("lang_choice")
                        .selected_text(&self.lang_input)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.lang_input, "auto".into(), txt("gui.settings.lang.auto", "System"));
                            ui.selectable_value(&mut self.lang_input, "en-us".into(), "English (US)");
                            ui.selectable_value(&mut self.lang_input, "ko-kr".into(), "한국어");
                            ui.selectable_value(&mut self.lang_input, "de-de".into(), "Deutsch");
                        });
                    if ui.button(txt("gui.settings.save", "Save settings")).clicked() {
                        self.config.language = self.lang_input.clone();
                        self.config.window_alpha = self.window_alpha;
                        // 즉시 번역기 반영
                        let resolved = i18n::resolve_language(&self.config.language, self.config.language_pack_dir.as_deref());
                        self.tr = i18n::Translator::new_with_pack(&resolved, self.config.language_pack_dir.as_deref());
                        if let Err(e) = self.config.save() {
                            self.lang_save_status = Some(format!("Save error: {e}"));
                        } else {
                            self.lang_save_status = Some(txt("gui.settings.saved", "Saved."));
                        }
                    }
                    if let Some(msg) = &self.lang_save_status {
                        ui.label(msg);
                    }
                });
            if new_unit_system != self.config.unit_system {
                self.config.unit_system = new_unit_system;
                self.apply_unit_preset(new_unit_system);
            }
        }

        // 도움말 모달
        if self.show_help_modal {
            egui::Window::new(txt("gui.about.title", "Help / About"))
                .collapsible(false)
                .resizable(true)
                .open(&mut self.show_help_modal)
                .show(ctx, |ui| {
                    ui.heading(txt("gui.about.app", "Offline calculator for steam/water/piping/valves"));
                    ui.label(txt("gui.about.version", "Version: 0.1a"));
                    ui.label(txt("gui.about.author", "Author: ruassu"));
                    ui.separator();
                    ui.label(txt("gui.about.units.title", "Unit guide"));
                    ui.label(txt("gui.about.units.mmHg", "- Pressure mmHg: gauge basis (0=atm, -760mmHg=vacuum)"));
                    ui.label(txt("gui.about.units.ga", "- g=gauge, a=absolute"));
                    ui.label(txt("gui.about.hint", "Adjust units/font in settings if you see issues."));
                });
        }

        if self.show_formula_modal {
            egui::Window::new(txt("gui.formula.title", "Formula reference"))
                .collapsible(true)
                .resizable(true)
                .open(&mut self.show_formula_modal)
                .show(ctx, |ui| {
                    ui.style_mut().wrap = Some(true);
                    ui.heading(txt("gui.formula.steam", "Steam: IF97 saturation/superheat; mmHg treated as gauge."));
                    ui.separator();
                    ui.heading(txt("gui.formula.pipe_sizing", "Pipe sizing: mdot = rho * Q, v = Q/A, Re = rho * v * D / mu."));
                    ui.label(txt("gui.formula.pipe_loss", "Pressure loss: ΔP = f (L/D) (rho v^2/2) + ΣK (rho v^2/2); f=64/Re (laminar) else Haaland/Petukhov."));
                    ui.separator();
                    ui.heading(txt("gui.formula.valve", "Valve Cv/Kv: Q = Cv * sqrt(ΔP / SG) (incompressible); mass = rho*Q."));
                    ui.label(txt("gui.formula.orifice", "Orifice/nozzle: incompressible Q = Cd·A·√(2·ΔP/ρ); compressible uses Y·C(1-β^4)^-0.5."));
                    ui.label(txt("gui.formula.pressure_rating", "Pressure rating: thin-wall (Barlow) vs thick-wall (Lame) using allowable stress S(T), weld eff. E, design factor F, CA, mill tolerance."));
                    ui.label(txt("gui.formula.expansion", "Thermal expansion: ΔL = α * L * ΔT."));
                    ui.separator();
                    ui.heading(txt("gui.formula.boiler_basic", "Boiler basic eff.: η = (m_s*h_s - m_fw*h_fw) / (Fuel_LHV*Fuel_flow)."));
                    ui.label(txt("gui.formula.boiler_ptc", "PTC: include flue-gas sensible losses, excess air, radiation, blowdown enthalpy."));
                    ui.separator();
                    ui.heading(txt("gui.formula.cooling_cond", "Condenser/vacuum: LMTD with Tsat(P) from IF97; Q = m·cp·ΔT; mmHg gauge = vacuum."));
                    ui.label(txt("gui.formula.cooling_ct", "Cooling tower: Range = T_hot - T_cold, Approach = T_cold - T_wb; simple heat balance."));
                    ui.label(txt("gui.formula.npsh", "NPSH: NPSHa = (Psuction - Pvap)/ρg + z - h_loss; compare to NPSHr."));
                    ui.label(txt("gui.formula.drain", "Drain/reheater: LMTD; UA or Area/U to compute Q_shell and Q_tube, check imbalance."));
                });
        }

        // 좌측 네비 + 본문
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
                .show(ui, |ui| match self.tab {
                    Tab::UnitConv => self.ui_unit_conv(ui),
                    Tab::SteamTables => self.ui_steam_tables(ui),
                    Tab::SteamPiping => self.ui_steam_piping(ui),
                    Tab::SteamValves => self.ui_steam_valves(ui),
                    Tab::Boiler => self.ui_boiler(ui),
                    Tab::Cooling => self.ui_cooling(ui),
                    Tab::PlantPiping => self.ui_plant_piping(ui),
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
        QuantityKind::Temperature => &[("Celsius (°C)", "C"), ("Kelvin (K)", "K"), ("Fahrenheit (°F)", "F"), ("Rankine (R)", "R")],
        QuantityKind::TemperatureDifference => &[("Δ°C", "C"), ("ΔK", "K"), ("Δ°F", "F"), ("ΔR", "R")],
        QuantityKind::Pressure => &[("bar(g)", "bar"), ("bar(a)", "bara"), ("kPa", "kPa"), ("MPa", "MPa"), ("psi", "psi"), ("atm", "atm"), ("Pa", "Pa"), ("mmHg", "mmHg")],
        QuantityKind::Length => &[("m", "m"), ("mm", "mm"), ("cm", "cm"), ("inch", "in"), ("ft", "ft"), ("yd", "yd"), ("km", "km")],
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
    &[("bar(g)", "bar"), ("bar(a)", "bara"), ("kPa", "kPa"), ("MPa", "MPa"), ("psi", "psi"), ("atm", "atm"), ("Pa", "Pa"), ("mmHg", "mmHg")]
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
    let to_lower = |s: &str| s.to_ascii_lowercase();
    let from_l = to_lower(from);
    let to_l = to_lower(to);
    if from_l == to_l {
        return v;
    }
    let kg_per_h = match from_l.as_str() {
        "kg/h" => v,
        "t/h" | "ton/h" | "tonne/h" => v * 1000.0,
        "kg/s" => v * 3600.0,
        "lb/h" => v * 0.45359237,
        _ => v,
    };
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
    if from.eq_ignore_ascii_case("kg/h") {
        v / convert_density_gui(rho, rho_unit, "kg/m3")
    } else if from.eq_ignore_ascii_case("t/h") {
        (v * 1000.0) / convert_density_gui(rho, rho_unit, "kg/m3")
    } else if from.eq_ignore_ascii_case("kg/s") {
        (v * 3600.0) / convert_density_gui(rho, rho_unit, "kg/m3")
    } else if from.eq_ignore_ascii_case("lb/h") {
        let kg_h = v * 0.45359237;
        kg_h / convert_density_gui(rho, rho_unit, "kg/m3")
    } else if from.eq_ignore_ascii_case("gpm") {
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
            state_pressure_bar_abs: Some(1.01325),
            state_temperature_c: Some(100.0),
        };
        let res = pressure_loss(input).unwrap();
        assert!((res.mach - 0.71).abs() < 0.02, "mach={}", res.mach);
    }
}
