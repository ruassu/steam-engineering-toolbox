# Steam Engineering Toolbox

Process-engineering toolbox for steam/air/gas/water piping calculations, steam tables, and valve/cooling utilities. Ships with both GUI (eframe) and CLI frontends.

## Overview & Intent
- Purpose: offline thermomechanical calculator for grid-connected thermal/combined-cycle plant maintenance engineers and related utility roles.
- Scope: quick sizing/verification using standard correlations (Darcy-Weisbach, IF97) so results stay transparent and reproducible, even on closed intranets.
- Direction: keep dependencies light, formulas visible, and offer both GUI convenience and CLI automation.
- Openness: anyone in the field is welcome to use and contribute.

## Design Goals
- Security: usable in closed intranet environments without external calls.
- Accuracy: trustable engineering precision with clear methods.
- Convenience: GUI and CLI for both interactive checks and scripts.
- Reliability: runs on low-spec processors without heavy runtime overhead.

## Program Details
- Property engine: IF97 steam via `seuif97`, optional user overrides for density/viscosity, and saturated vs. superheated handling.
- Piping solver: Darcy-Weisbach with Haaland/Petukhov friction factor, laminar 64/Re cutoff, fittings by K-factor or equivalent length, Mach awareness when you supply speed of sound.
- Outputs: pressure drop, velocity, Reynolds, friction factor, Mach (when available), plus key property echoes so you can verify assumptions.
- UX: GUI for interactive what-if checks; CLI for batch runs and scripting; both read optional defaults from `config.toml`.

## Features
- Piping pressure drop for steam/air/gas/water/condensate using Darcy-Weisbach with Haaland/Petukhov turbulence correlations, laminar 64/Re, and fittings via K or equivalent length.
- Steam properties via IF97 (seuif97) with automatic density/viscosity estimation from pressure/temperature and fallback to user inputs; supports saturated and superheated states.
- Valve/orifice Cv/Kv sizing plus cooling tower, condenser, and NPSH utilities.
- Unit conversion and a default `config.toml` for baseline options.

## Build & Test
Requires Rust 1.75+.
```
cargo test
cargo build --release
```
Release artifacts: `target/release/steam_engineering_toolbox.exe` and `steam_engineering_toolbox_cli.exe` (see the `release/` folder for bundled builds).

## Run
- GUI: `steam_engineering_toolbox.exe`
- CLI: `steam_engineering_toolbox_cli --help` for usage
- Config: adjust default units/options via `config.toml` in the run directory

## Consumer Usage (GUI/CLI package)
- Download the release zip from GitHub Releases and extract it to a writable folder (no install needed).
- Run `steam_engineering_toolbox.exe` for the GUI; keep `config.toml` in the same folder if you want defaults loaded automatically.
- For scripting or terminal use, run `steam_engineering_toolbox_cli --help` to see commands; typical usage is `steam_engineering_toolbox_cli pressure-drop --help`.
- If property estimation looks off, override density/viscosity in the input fields or CLI flags; the app will use your manual values.

## Release/Distribution Guide
- Commit source only; upload built binaries to GitHub Releases.
- Example package: `steam_engineering_toolbox_1.0.0-a_windows.zip` containing `steam_engineering_toolbox.exe`, `steam_engineering_toolbox_cli.exe`, and `config.toml`.

## Input Tips
- Default pipe roughness epsilon: carbon steel ~0.000045 m.
- If unsure of speed of sound for Mach calculations, use ~400-500 m/s.
- For pressure-drop runs, entering pressure (bar a) and temperature (°C) triggers IF97-based density/viscosity; manual inputs are used when property estimation fails.

## Dependencies
- `seuif97` (steam IF97 calculations)
- `eframe/egui` (desktop GUI)

## 한국어 요약
- 개요: 중앙전력망 연계 기력/복합발전소 유지보수·유틸리티 엔지니어용 오프라인 열기계 계산기입니다.
- 지향점: 폐쇄망에서 안전하게 쓰고, 표준 상관식(Darcy-Weisbach, IF97)으로 정확하고 재현성 있게 계산합니다. 업계 종사자는 누구나 참여 환영입니다.
- 동작: IF97 증기 물성(밀도/점도 수동 입력 가능), Darcy-Weisbach+Haaland/Petukhov 마찰계수, 층류 64/Re, 피팅 K·등가길이, 음속 입력 시 Mach 계산. GUI는 상호작용, CLI는 배치/스크립트용이며 둘 다 `config.toml` 기본값을 읽습니다.
- 핵심 기능: 압손, Cv/Kv/오리피스, 냉각탑·콘덴서·NPSH, 단위 변환, 기본 설정 파일.
- 빌드/테스트: Rust 1.75+; `cargo test`, `cargo build --release`; 산출물은 `target/release/steam_engineering_toolbox.exe`, `steam_engineering_toolbox_cli.exe`.
- 소비자용 사용법: Releases zip을 풀어 GUI는 `steam_engineering_toolbox.exe`, CLI는 `steam_engineering_toolbox_cli --help` 후 `pressure-drop` 등 서브커맨드 사용. 물성이 의심되면 밀도/점도를 수동 입력해 바로 반영합니다.
- 배포/가치: 소스만 커밋하고, 빌드 결과는 Releases에 `steam_engineering_toolbox.exe`, `steam_engineering_toolbox_cli.exe`, `config.toml`을 묶어 올립니다. 목표는 보안성(폐쇄망), 정확성(공학 정밀도), 편리성(GUI/CLI), 신뢰성(저사양 호환)입니다.
