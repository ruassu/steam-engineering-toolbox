use std::collections::HashMap;
use std::fs;
use std::path::Path;
use sys_locale::get_locale;

/// 문자열 키를 모아두는 네임스페이스.
pub mod keys {
    pub const ERROR_PREFIX: &str = "general.error_prefix";
    pub const APP_EXIT: &str = "general.app_exit";

    pub const MAIN_MENU_TITLE: &str = "main_menu.title";
    pub const MAIN_MENU_UNIT_CONVERSION: &str = "main_menu.unit_conversion";
    pub const MAIN_MENU_STEAM_TABLES: &str = "main_menu.steam_tables";
    pub const MAIN_MENU_STEAM_PIPING: &str = "main_menu.steam_piping";
    pub const MAIN_MENU_STEAM_VALVES: &str = "main_menu.steam_valves";
    pub const MAIN_MENU_SETTINGS: &str = "main_menu.settings";
    pub const MAIN_MENU_EXIT: &str = "main_menu.exit";
    pub const PROMPT_MENU_SELECT: &str = "prompt.menu_select";
    pub const INVALID_SELECTION_RETRY: &str = "error.invalid_selection_retry";

    pub const UNIT_CONVERSION_HEADING: &str = "unit_conversion.heading";
    pub const UNIT_CONVERSION_OPTIONS_LINE1: &str = "unit_conversion.options_line1";
    pub const UNIT_CONVERSION_OPTIONS_LINE2: &str = "unit_conversion.options_line2";
    pub const UNIT_CONVERSION_NOTE_MMHG: &str = "unit_conversion.note_mmhg";
    pub const UNIT_CONVERSION_PROMPT_KIND: &str = "unit_conversion.prompt_kind";
    pub const UNIT_CONVERSION_PROMPT_VALUE: &str = "unit_conversion.prompt_value";
    pub const UNIT_CONVERSION_PROMPT_FROM_UNIT: &str = "unit_conversion.prompt_from_unit";
    pub const UNIT_CONVERSION_PROMPT_TO_UNIT: &str = "unit_conversion.prompt_to_unit";
    pub const UNIT_CONVERSION_RESULT: &str = "unit_conversion.result";
    pub const UNIT_CONVERSION_UNSUPPORTED: &str = "unit_conversion.unsupported";

    pub const STEAM_TABLES_HEADING: &str = "steam_tables.heading";
    pub const STEAM_TABLES_NOTE: &str = "steam_tables.note";
    pub const STEAM_TABLES_OPTIONS: &str = "steam_tables.options";
    pub const PROMPT_SELECT: &str = "prompt.select";
    pub const PROMPT_PRESSURE_VALUE: &str = "prompt.pressure_value";
    pub const PROMPT_TEMPERATURE_VALUE: &str = "prompt.temperature_value";

    pub const STEAM_PIPING_HEADING: &str = "steam_piping.heading";
    pub const STEAM_PIPING_OPTION_SIZING: &str = "steam_piping.option_sizing";
    pub const STEAM_PIPING_OPTION_PRESSURE_DROP: &str = "steam_piping.option_pressure_drop";
    pub const PROMPT_MASS_FLOW: &str = "prompt.mass_flow";
    pub const PROMPT_OPERATING_PRESSURE: &str = "prompt.operating_pressure";
    pub const PROMPT_OPERATING_PRESSURE_MODE: &str = "prompt.operating_pressure_mode";
    pub const PROMPT_OPERATING_TEMPERATURE: &str = "prompt.operating_temperature";
    pub const PROMPT_TARGET_VELOCITY: &str = "prompt.target_velocity";
    pub const RESULT_RECOMMENDED_ID: &str = "result.recommended_id";
    pub const RESULT_EXPECTED_VELOCITY: &str = "result.expected_velocity";
    pub const PROMPT_DENSITY_OPTIONAL: &str = "prompt.density_optional";
    pub const PROMPT_DIAMETER: &str = "prompt.diameter";
    pub const PROMPT_LENGTH: &str = "prompt.length";
    pub const PROMPT_EQ_LENGTH: &str = "prompt.eq_length";
    pub const PROMPT_FITTINGS_K: &str = "prompt.fittings_k";
    pub const PROMPT_ROUGHNESS: &str = "prompt.roughness";
    pub const PROMPT_VISCOSITY: &str = "prompt.viscosity";
    pub const PROMPT_SOUND_SPEED: &str = "prompt.sound_speed";
    pub const RESULT_PRESSURE_DROP: &str = "result.pressure_drop";

    pub const STEAM_VALVES_HEADING: &str = "steam_valves.heading";
    pub const STEAM_VALVES_OPTION_REQUIRED: &str = "steam_valves.option_required";
    pub const STEAM_VALVES_OPTION_FLOW: &str = "steam_valves.option_flow";
    pub const PROMPT_VOLUMETRIC_FLOW: &str = "prompt.volumetric_flow";
    pub const PROMPT_DELTA_P: &str = "prompt.delta_p";
    pub const PROMPT_DENSITY_GENERIC: &str = "prompt.density_generic";
    pub const RESULT_REQUIRED_KV_CV: &str = "result.required_kv_cv";
    pub const PROMPT_INPUT_MODE_KV_CV: &str = "prompt.input_mode_kv_cv";
    pub const PROMPT_KV_CV_VALUE: &str = "prompt.kv_cv_value";
    pub const PROMPT_UPSTREAM_PRESSURE: &str = "prompt.upstream_pressure";
    pub const RESULT_POSSIBLE_FLOW: &str = "result.possible_flow";

    pub const SETTINGS_HEADING: &str = "settings.heading";
    pub const SETTINGS_CURRENT_UNIT_SYSTEM: &str = "settings.current_unit_system";
    pub const SETTINGS_OPTIONS: &str = "settings.options";
    pub const SETTINGS_PROMPT_CHANGE: &str = "settings.prompt_change";
    pub const SETTINGS_INVALID: &str = "settings.invalid";
    pub const SETTINGS_SAVED: &str = "settings.saved";

    pub const PRESSURE_UNIT_OPTIONS: &str = "unit.pressure_options";
    pub const TEMPERATURE_UNIT_OPTIONS: &str = "unit.temperature_options";

    pub const ERROR_INVALID_NUMBER: &str = "error.invalid_number";

    pub const STATE_SATURATION_T: &str = "state.saturation_temperature";
    pub const STATE_SATURATION_P: &str = "state.saturation_pressure";
    pub const STATE_ENTHALPY_VOLUME: &str = "state.enthalpy_volume";
    pub const STATE_SUPERHEATED_ENTHALPY: &str = "state.superheated_enthalpy";

    pub const HELP_UNIT_CONVERSION: &str = "help.unit_conversion";
    pub const HELP_STEAM_TABLES: &str = "help.steam_tables";
    pub const HELP_STEAM_PIPING_SIZING: &str = "help.steam_piping_sizing";
    pub const HELP_STEAM_PIPING_DROP: &str = "help.steam_piping_drop";
    pub const HELP_STEAM_VALVES_REQUIRED: &str = "help.steam_valves_required";
    pub const HELP_STEAM_VALVES_FLOW: &str = "help.steam_valves_flow";
    pub const HELP_SETTINGS: &str = "help.settings";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Ko,
    En,
}

impl Language {
    fn from_code(code: &str) -> Self {
        let c = code.to_lowercase();
        if c.starts_with("en") {
            Language::En
        } else {
            Language::Ko
        }
    }

    pub fn as_code(&self) -> &'static str {
        match self {
            Language::Ko => "ko",
            Language::En => "en",
        }
    }
}

/// 런타임 언어 번들을 제공한다.
#[derive(Debug, Clone)]
pub struct Translator {
    lang: Language,
    overrides: Option<HashMap<String, String>>,
}

impl Translator {
    /// 언어 코드(ko/en)에 따라 번역기를 생성한다. 알 수 없는 코드는 ko로 폴백한다.
    pub fn new(lang_code: &str) -> Self {
        Self {
            lang: Language::from_code(lang_code),
            overrides: None,
        }
    }

    /// 언어 코드 + 언어팩 디렉터리(locales/ 등)를 받아서 번역기를 생성한다.
    /// 디렉터리가 없거나 파일이 없으면 내장 문자열만 사용한다.
    pub fn new_with_pack(lang_code: &str, pack_dir: Option<&str>) -> Self {
        let overrides = pack_dir
            .and_then(|dir| load_overrides(dir, lang_code))
            .or_else(|| load_overrides("locales", lang_code))
            .or_else(|| built_in_pack(lang_code));
        Self {
            lang: Language::from_code(lang_code),
            overrides,
        }
    }

    pub fn language(&self) -> Language {
        self.lang
    }

    pub fn language_code(&self) -> &'static str {
        self.lang.as_code()
    }

    /// 키를 조회해 문자열을 반환한다. 언어팩에 없으면 None.
    pub fn lookup(&self, key: &str) -> Option<String> {
        self.overrides
            .as_ref()
            .and_then(|m| m.get(key).cloned())
    }

    /// 번역을 가져온다. 영어 번역이 없으면 한국어 문자열을 폴백한다.
    pub fn t(&self, key: &str) -> &'static str {
        if let Some(ref map) = self.overrides {
            if let Some(v) = map.get(key) {
                return Box::leak(v.clone().into_boxed_str());
            }
        }
        match self.lang {
            Language::En => en(key).unwrap_or_else(|| ko(key)),
            Language::Ko => ko(key),
        }
    }
}

/// CLI 플래그/설정/시스템 순으로 언어 코드를 결정한다.
pub fn resolve_language(cli_arg: &str, config_lang: Option<&str>) -> String {
    normalize_lang(cli_arg)
        .or_else(|| config_lang.and_then(normalize_lang))
        .or_else(detect_system_language)
        .unwrap_or_else(|| "en-us".to_string())
}

fn normalize_lang(code: &str) -> Option<String> {
    let c = code.trim().to_lowercase();
    match c.as_str() {
        "ko" => Some("ko".into()),
        "ko-kr" => Some("ko-kr".into()),
        "en" => Some("en".into()),
        "en-us" => Some("en-us".into()),
        "en-uk" => Some("en-us".into()),
        "de" => Some("de-de".into()),
        "de-de" => Some("de-de".into()),
        "auto" | "" => None,
        other if other.starts_with("ko") => Some("ko".into()),
        other if other.starts_with("en") => Some("en-us".into()),
        other if other.starts_with("de") => Some("de-de".into()),
        _ => None,
    }
}

fn normalize_locale_string(loc: &str) -> Option<String> {
    let lang = loc
        .split(['.', '_', '-'])
        .next()
        .unwrap_or_default()
        .to_lowercase();
    match lang.as_str() {
        "ko" => Some("ko".into()),
        "en" => Some("en".into()),
        _ => None,
    }
}

/// 시스템 로케일에서 언어를 추정한다.
pub fn detect_system_language() -> Option<String> {
    if let Some(loc) = get_locale() {
        if let Some(lang) = normalize_locale_string(&loc) {
            return Some(lang);
        }
    }
    if let Ok(lang) = std::env::var("LANG") {
        if let Some(code) = normalize_locale_string(&lang) {
            return Some(code);
        }
    }
    if let Ok(lang) = std::env::var("LC_ALL") {
        if let Some(code) = normalize_locale_string(&lang) {
            return Some(code);
        }
    }
    None
}

/// TOML 기반 언어팩을 로드한다. 형식: key = "value" 로 구성된 플랫 맵.
fn load_overrides(dir: &str, lang: &str) -> Option<HashMap<String, String>> {
    let try_load = |code: &str| -> Option<HashMap<String, String>> {
        let path = Path::new(dir).join(format!("{code}.toml"));
        let content = fs::read_to_string(path).ok()?;
        parse_toml_to_map(&content)
    };

    // 1) full code (e.g., en-us)
    if let Some(map) = try_load(lang) {
        return Some(map);
    }
    // 2) base code (e.g., en)
    if let Some((base, _)) = lang.split_once(['-', '_']) {
        if let Some(map) = try_load(base) {
            return Some(map);
        }
    }
    None
}

fn parse_toml_to_map(src: &str) -> Option<HashMap<String, String>> {
    let value: toml::Value = toml::from_str(src).ok()?;
    let table = value.as_table()?;
    let mut map = HashMap::new();

    fn walk(prefix: &str, val: &toml::Value, out: &mut HashMap<String, String>) {
        match val {
            toml::Value::String(s) => {
                out.insert(prefix.to_string(), s.to_string());
            }
            toml::Value::Table(t) => {
                for (k, v) in t {
                    let key = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{prefix}.{k}")
                    };
                    walk(&key, v, out);
                }
            }
            _ => {}
        }
    }

    for (k, v) in table {
        walk(k, v, &mut map);
    }

    if map.is_empty() {
        None
    } else {
        Some(map)
    }
}

/// 내장 언어팩(파일이 없어도 동작하도록 빌드 시 포함).
fn built_in_pack(lang: &str) -> Option<HashMap<String, String>> {
    match lang.to_lowercase().as_str() {
        "en-us" | "en" => parse_toml_to_map(include_str!("../locales/en-us.toml")),
        "en-uk" => parse_toml_to_map(include_str!("../locales/en-uk.toml")),
        "ko-kr" | "ko" => parse_toml_to_map(include_str!("../locales/ko-kr.toml")),
        "de-de" | "de" => parse_toml_to_map(include_str!("../locales/de-de.toml")),
        _ => None,
    }
}

fn ko(key: &str) -> &'static str {
    use keys::*;
    match key {
        ERROR_PREFIX => "오류",
        APP_EXIT => "프로그램을 종료합니다.",
        MAIN_MENU_TITLE => "\n=== Steam Engineering Toolbox ===",
        MAIN_MENU_UNIT_CONVERSION => "1) 단위 변환기",
        MAIN_MENU_STEAM_TABLES => "2) Steam Tables",
        MAIN_MENU_STEAM_PIPING => "3) Steam Piping",
        MAIN_MENU_STEAM_VALVES => "4) Steam Valves & Orifices",
        MAIN_MENU_SETTINGS => "5) 설정",
        MAIN_MENU_EXIT => "0) 종료",
        PROMPT_MENU_SELECT => "메뉴 선택: ",
        INVALID_SELECTION_RETRY => "잘못된 입력입니다. 다시 선택하세요.",
        UNIT_CONVERSION_HEADING => "\n-- 단위 변환 --",
        UNIT_CONVERSION_OPTIONS_LINE1 => "1) 온도  2) 온도차  3) 압력  4) 길이  5) 면적  6) 체적",
        UNIT_CONVERSION_OPTIONS_LINE2 => {
            "7) 속도  8) 질량  9) 점도 10) 에너지 11) 열전달율 12) 열전도율 13) 비엔탈피"
        }
        UNIT_CONVERSION_NOTE_MMHG => {
            "참고: mmHg는 게이지 기준(0=대기, -760mmHg=완전진공)으로 처리됩니다."
        }
        UNIT_CONVERSION_PROMPT_KIND => "항목 번호를 입력: ",
        UNIT_CONVERSION_PROMPT_VALUE => "값 입력: ",
        UNIT_CONVERSION_PROMPT_FROM_UNIT => "입력 단위(ex: C, bar, m): ",
        UNIT_CONVERSION_PROMPT_TO_UNIT => "변환 단위(ex: K, psi, ft): ",
        UNIT_CONVERSION_RESULT => "변환 결과:",
        UNIT_CONVERSION_UNSUPPORTED => "지원하지 않는 번호입니다.",
        STEAM_TABLES_HEADING => "\n-- Steam Tables --",
        STEAM_TABLES_NOTE => "참고: 압력 mmHg 입력 시 0=대기, -760mmHg=완전진공으로 해석합니다.",
        STEAM_TABLES_OPTIONS => "1) By Pressure  2) By Temperature  3) Superheated (압력+온도)",
        PROMPT_SELECT => "선택: ",
        PROMPT_PRESSURE_VALUE => "압력 값: ",
        PROMPT_TEMPERATURE_VALUE => "온도 값: ",
        STEAM_PIPING_HEADING => "\n-- Steam Piping --",
        STEAM_PIPING_OPTION_SIZING => "1) 목표 유속 기준 사이징",
        STEAM_PIPING_OPTION_PRESSURE_DROP => "2) 압력손실 계산",
        PROMPT_MASS_FLOW => "질량 유량 [kg/h]: ",
        PROMPT_OPERATING_PRESSURE => "운전 압력 값: ",
        PROMPT_OPERATING_PRESSURE_MODE => "운전 압력 값 (절대/게이지 선택): ",
        PROMPT_OPERATING_TEMPERATURE => "운전 온도 값: ",
        PROMPT_TARGET_VELOCITY => "허용 유속 [m/s]: ",
        RESULT_RECOMMENDED_ID => "추천 내경:",
        RESULT_EXPECTED_VELOCITY => "예상 유속:",
        PROMPT_DENSITY_OPTIONAL => "증기 밀도 [kg/m3] (0 입력 시 IF97 기반 자동 계산): ",
        PROMPT_DIAMETER => "배관 내경 [mm] (in/\" 입력 가능): ",
        PROMPT_LENGTH => "배관 길이 [m]: ",
        PROMPT_EQ_LENGTH => "등가 길이 [m] (없으면 0): ",
        PROMPT_FITTINGS_K => "피팅 K 합계 (없으면 0): ",
        PROMPT_ROUGHNESS => "거칠기 ε [m] (탄소강 배관 약 0.000045): ",
        PROMPT_VISCOSITY => "동점도 [Pa·s] (증기 기본값 1.2e-5 추천): ",
        PROMPT_SOUND_SPEED => "음속 [m/s] (기본 450 정도): ",
        RESULT_PRESSURE_DROP => "압력손실 결과:",
        STEAM_VALVES_HEADING => "\n-- Steam Valves & Orifices --",
        STEAM_VALVES_OPTION_REQUIRED => "1) 필요한 Cv/Kv 계산",
        STEAM_VALVES_OPTION_FLOW => "2) Cv/Kv로 가능한 유량 계산",
        PROMPT_VOLUMETRIC_FLOW => "볼류메트릭 유량 [m3/h]: ",
        PROMPT_DELTA_P => "차압 [bar]: ",
        PROMPT_DENSITY_GENERIC => "밀도 [kg/m3]: ",
        RESULT_REQUIRED_KV_CV => "필요 Kv/Cv:",
        PROMPT_INPUT_MODE_KV_CV => "입력 단위 선택 (1=Kv, 2=Cv): ",
        PROMPT_KV_CV_VALUE => "Kv/Cv 값: ",
        PROMPT_UPSTREAM_PRESSURE => "상류 압력 [bar(a)]: ",
        RESULT_POSSIBLE_FLOW => "가능한 유량:",
        SETTINGS_HEADING => "\n-- 설정 --",
        SETTINGS_CURRENT_UNIT_SYSTEM => "현재 단위 시스템:",
        SETTINGS_OPTIONS => "1) SI(Bar)  2) SI  3) MKS  4) Imperial",
        SETTINGS_PROMPT_CHANGE => "변경할 번호(취소하려면 엔터): ",
        SETTINGS_INVALID => "잘못된 입력이므로 변경하지 않습니다.",
        SETTINGS_SAVED => "단위 시스템이 변경되었습니다:",
        PRESSURE_UNIT_OPTIONS => "압력 단위: 1=bar 2=kPa 3=MPa 4=psi 5=atm",
        TEMPERATURE_UNIT_OPTIONS => "온도 단위: 1=°C 2=K 3=°F 4=R",
        ERROR_INVALID_NUMBER => "숫자를 입력하세요.",
        STATE_SATURATION_T => "포화 온도:",
        STATE_SATURATION_P => "포화 압력:",
        STATE_ENTHALPY_VOLUME => "비엔탈피/비체적:",
        STATE_SUPERHEATED_ENTHALPY => "과열 비엔탈피:",
        HELP_UNIT_CONVERSION => "도움말: 물리량 번호 → 값 → 입력/변환 단위 순으로 입력 (예: bar/kPa/MPa/psi/atm/mmHg, C/K/F).",
        HELP_STEAM_TABLES => "도움말: 압력/온도 입력 시 단위 선택 필요. mmHg는 게이지, bar/psi/atm은 게이지/절대 선택에 따릅니다.",
        HELP_STEAM_PIPING_SIZING => "도움말: 질량유량[kg/h], 압력(게이지/절대), 온도, 허용 유속 입력. 내경 결과는 mm/in 단위로 표시됩니다.",
        HELP_STEAM_PIPING_DROP => "도움말: 밀도 0 입력 시 IF97 자동. 내경/두께 mm 또는 in 입력 가능. K 합계/등가길이는 없으면 0.",
        HELP_STEAM_VALVES_REQUIRED => "도움말: 유량[m3/h], ΔP[bar], 밀도[kg/m3] 입력 → 필요 Kv/Cv 계산.",
        HELP_STEAM_VALVES_FLOW => "도움말: Kv 또는 Cv 값, ΔP[bar], 밀도[kg/m3], 상류압[bar(a)] 입력 → 가능한 유량 계산.",
        HELP_SETTINGS => "도움말: 단위 시스템 프리셋을 선택하면 기본 단위 세트가 바뀝니다 (SIBar/SI/MKS/Imperial).",
        _ => "[missing translation]",
    }
}

fn en(key: &str) -> Option<&'static str> {
    use keys::*;
    Some(match key {
        ERROR_PREFIX => "Error",
        APP_EXIT => "Exiting application.",
        MAIN_MENU_TITLE => "\n=== Steam Engineering Toolbox ===",
        MAIN_MENU_UNIT_CONVERSION => "1) Unit Converter",
        MAIN_MENU_STEAM_TABLES => "2) Steam Tables",
        MAIN_MENU_STEAM_PIPING => "3) Steam Piping",
        MAIN_MENU_STEAM_VALVES => "4) Steam Valves & Orifices",
        MAIN_MENU_SETTINGS => "5) Settings",
        MAIN_MENU_EXIT => "0) Exit",
        PROMPT_MENU_SELECT => "Select menu: ",
        INVALID_SELECTION_RETRY => "Invalid input. Please try again.",
        UNIT_CONVERSION_HEADING => "\n-- Unit Conversion --",
        UNIT_CONVERSION_OPTIONS_LINE1 => "1) Temperature  2) ΔTemperature  3) Pressure  4) Length  5) Area  6) Volume",
        UNIT_CONVERSION_OPTIONS_LINE2 =>
            "7) Velocity  8) Mass  9) Viscosity 10) Energy 11) Heat Transfer 12) Conductivity 13) Specific Enthalpy",
        UNIT_CONVERSION_NOTE_MMHG => "Note: mmHg is treated as gauge (0=atm, -760mmHg=vacuum).",
        UNIT_CONVERSION_PROMPT_KIND => "Enter item number: ",
        UNIT_CONVERSION_PROMPT_VALUE => "Value: ",
        UNIT_CONVERSION_PROMPT_FROM_UNIT => "From unit (ex: C, bar, m): ",
        UNIT_CONVERSION_PROMPT_TO_UNIT => "To unit (ex: K, psi, ft): ",
        UNIT_CONVERSION_RESULT => "Result:",
        UNIT_CONVERSION_UNSUPPORTED => "Unsupported selection.",
        STEAM_TABLES_HEADING => "\n-- Steam Tables --",
        STEAM_TABLES_NOTE => "Note: when using mmHg, 0=atm and -760mmHg=vacuum (gauge).",
        STEAM_TABLES_OPTIONS => "1) By Pressure  2) By Temperature  3) Superheated (P+T)",
        PROMPT_SELECT => "Select: ",
        PROMPT_PRESSURE_VALUE => "Pressure value: ",
        PROMPT_TEMPERATURE_VALUE => "Temperature value: ",
        STEAM_PIPING_HEADING => "\n-- Steam Piping --",
        STEAM_PIPING_OPTION_SIZING => "1) Size by target velocity",
        STEAM_PIPING_OPTION_PRESSURE_DROP => "2) Pressure-drop calculation",
        PROMPT_MASS_FLOW => "Mass flow [kg/h]: ",
        PROMPT_OPERATING_PRESSURE => "Operating pressure value: ",
        PROMPT_OPERATING_PRESSURE_MODE => "Operating pressure value (abs/gauge choice): ",
        PROMPT_OPERATING_TEMPERATURE => "Operating temperature value: ",
        PROMPT_TARGET_VELOCITY => "Target velocity [m/s]: ",
        RESULT_RECOMMENDED_ID => "Recommended ID:",
        RESULT_EXPECTED_VELOCITY => "Expected velocity:",
        PROMPT_DENSITY_OPTIONAL => "Steam density [kg/m3] (0 = auto via IF97): ",
        PROMPT_DIAMETER => "Pipe inner diameter [mm] (in/\"): ",
        PROMPT_LENGTH => "Pipe length [m]: ",
        PROMPT_EQ_LENGTH => "Equivalent length [m] (0 if none): ",
        PROMPT_FITTINGS_K => "Fittings K sum (0 if none): ",
        PROMPT_ROUGHNESS => "Roughness ε [m] (carbon steel ~0.000045): ",
        PROMPT_VISCOSITY => "Dynamic viscosity [Pa·s] (steam ~1.2e-5): ",
        PROMPT_SOUND_SPEED => "Speed of sound [m/s] (default ~450): ",
        RESULT_PRESSURE_DROP => "Pressure-drop result:",
        STEAM_VALVES_HEADING => "\n-- Steam Valves & Orifices --",
        STEAM_VALVES_OPTION_REQUIRED => "1) Required Cv/Kv",
        STEAM_VALVES_OPTION_FLOW => "2) Flow from Cv/Kv",
        PROMPT_VOLUMETRIC_FLOW => "Volumetric flow [m3/h]: ",
        PROMPT_DELTA_P => "ΔP [bar]: ",
        PROMPT_DENSITY_GENERIC => "Density [kg/m3]: ",
        RESULT_REQUIRED_KV_CV => "Required Kv/Cv:",
        PROMPT_INPUT_MODE_KV_CV => "Input mode (1=Kv, 2=Cv): ",
        PROMPT_KV_CV_VALUE => "Kv/Cv value: ",
        PROMPT_UPSTREAM_PRESSURE => "Upstream pressure [bar(a)]: ",
        RESULT_POSSIBLE_FLOW => "Possible flow:",
        SETTINGS_HEADING => "\n-- Settings --",
        SETTINGS_CURRENT_UNIT_SYSTEM => "Current unit system:",
        SETTINGS_OPTIONS => "1) SI(Bar)  2) SI  3) MKS  4) Imperial",
        SETTINGS_PROMPT_CHANGE => "Enter number to change (enter to cancel): ",
        SETTINGS_INVALID => "Invalid input; unit system unchanged.",
        SETTINGS_SAVED => "Unit system changed to:",
        PRESSURE_UNIT_OPTIONS => "Pressure units: 1=bar 2=kPa 3=MPa 4=psi 5=atm",
        TEMPERATURE_UNIT_OPTIONS => "Temperature units: 1=°C 2=K 3=°F 4=R",
        ERROR_INVALID_NUMBER => "Please enter a number.",
        STATE_SATURATION_T => "Saturation temperature:",
        STATE_SATURATION_P => "Saturation pressure:",
        STATE_ENTHALPY_VOLUME => "Enthalpy/volume:",
        STATE_SUPERHEATED_ENTHALPY => "Superheated enthalpy:",
        HELP_UNIT_CONVERSION => "Help: choose quantity → enter value → from/to units (bar/kPa/MPa/psi/atm/mmHg, C/K/F, etc).",
        HELP_STEAM_TABLES => "Help: select unit for pressure/temperature. mmHg is gauge; bar/psi/atm follow your abs/gauge selection.",
        HELP_STEAM_PIPING_SIZING => "Help: mass flow [kg/h], pressure (abs/gauge), temperature, target velocity. ID result shows mm and inches.",
        HELP_STEAM_PIPING_DROP => "Help: density 0 => auto IF97. Diameter accepts mm or inch. K-sum/equivalent length can be 0 if none.",
        HELP_STEAM_VALVES_REQUIRED => "Help: flow [m3/h], ΔP [bar], density [kg/m3] → compute required Kv/Cv.",
        HELP_STEAM_VALVES_FLOW => "Help: Kv or Cv, ΔP [bar], density [kg/m3], upstream P [bar(a)] → compute flow.",
        HELP_SETTINGS => "Help: unit-system preset changes default units (SIBar/SI/MKS/Imperial).",
        _ => return None,
    })
}
