# Steam Engineering Toolbox

증기/가스/수 배관 계산, 증기표, 밸브·냉각 유틸리티를 포함한 공정 엔지니어링 도구입니다. GUI(eframe)와 CLI를 모두 제공합니다.

## 주요 기능
- Steam/공기/가스/물/응축수 배관 압력손실: Darcy-Weisbach + Haaland/Petukhov 난류 상관식, 층류 64/Re, 피팅 K 및 등가길이 지원.
- 증기 물성: IF97(seuif97)로 압력·온도 기반 밀도/점도 자동 추정(입력값 폴백), 포화·과열 계산 포함.
- 밸브/오리피스 Cv/Kv 계산, 냉각탑·콘덴서·NPSH 등 유틸리티 계산.
- 단위 변환, 기본 구성(config.toml) 제공.

## 빌드 및 테스트
Rust 1.75+ 기준.
```
cargo test
cargo build --release
```
출력 바이너리: `target/release/steam_engineering_toolbox.exe`, `steam_engineering_toolbox_cli.exe` (release용 번들은 `release/` 폴더 참조).

## 실행
- GUI: `steam_engineering_toolbox.exe`
- CLI: `steam_engineering_toolbox_cli --help`로 사용법 확인
- 설정: 실행 디렉터리의 `config.toml`로 기본 단위/옵션 조정 가능

## 배포/릴리스 가이드
- 저장소에는 코드만 커밋하고, 빌드 산출물은 GitHub Releases에 업로드하세요.
- 예시 패키지: `steam_engineering_toolbox_1.0.0-a_windows.zip` 안에 `steam_engineering_toolbox.exe`, `steam_engineering_toolbox_cli.exe`, `config.toml` 포함.
- 릴리스 노트에 SHA256 해시를 함께 기재해 수신자가 무결성을 확인할 수 있게 합니다.

## 입력 팁
- 거칠기 ε 기본값: 탄소강 배관 약 0.000045 m.
- 음속 입력 시 Mach 자동 계산; 모르면 400~500 m/s 수준을 사용.
- 압력손실 계산에서 압력(bar a)·온도(°C)를 넣으면 IF97 기반 밀도/점도를 자동 사용하며, 실패 시 수동 입력값을 사용합니다.

## 의존성
- `seuif97` (증기 IF97 계산)
- `eframe/egui` (데스크톱 GUI)
