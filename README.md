# Steam Engineering Toolbox

Process engineering utilities for steam, gas, and water piping calculations, steam tables, and valve/cooling helpers. Both GUI (eframe) and CLI are available.

## Key Features
- Steam/air/gas/water/condensate piping pressure drop using Darcy-Weisbach with Haaland/Petukhov turbulent correlations, laminar 64/Re, fitting K values, and equivalent length support.
- Steam properties via IF97 (seuif97) with pressure/temperature-driven density/viscosity estimation (falls back to provided inputs) including saturated and superheated calculations.
- Valve/orifice Cv/Kv sizing plus cooling tower, condenser, and NPSH utilities.
- Unit conversions and a default configuration (`config.toml`).

## Build & Test
Requires Rust 1.75+.
```
cargo test
cargo build --release
```
Output binaries: `target/release/steam_engineering_toolbox.exe` and `steam_engineering_toolbox_cli.exe` (see the `release/` folder for bundled artifacts).

## Run
- GUI: `steam_engineering_toolbox.exe`
- CLI: `steam_engineering_toolbox_cli --help` for usage
- Settings: adjust default units/options via `config.toml` in the run directory

## Input Tips
- Default roughness ε: ~0.000045 m for carbon steel pipe.
- Mach is auto-calculated from speed of sound; if unknown, try 400–500 m/s as a starting point.
- For pressure drop, entering pressure (bar a) and temperature (°C) triggers IF97-based density/viscosity lookup; on failure, manual inputs are used.

## Dependencies
- `seuif97` (steam IF97 calculations)
- `eframe/egui` (desktop GUI)

---
---

# 스팀 엔지니어링 툴박스

증기/가스/수 배관 계산, 증기표, 밸브·냉각 유틸리티를 포함한 공정 엔지니어링 도구입니다. GUI(eframe)와 CLI를 모두 제공합니다.

## 주요 기능
- Steam/공기/가스/물/응축수 배관 압력손실: Darcy-Weisbach + Haaland/Petukhov 난류 상관식, 층류 64/Re, 피팅 K 및 등가길이 지원.
- 증기 물성: IF97(seuif97)로 압력·온도 기반 밀도/점도 자동 추정(입력값 폴백), 포화·과열 계산 포함.
- 밸브/오리피스 Cv/Kv 계산, 냉각탑·콘덴서·NPSH 등 유틸리티 계산.
- 단위 변환, 기본 구성(config.toml) 제공.

## 빌드 및 테스트
Rust 1.75+ 기준.
