//! mmHg 게이지/절대 변환 회귀 테스트.
use steam_engineering_toolbox::conversion::{convert_pressure_mode, PressureMode};
use steam_engineering_toolbox::units::PressureUnit;

#[test]
fn mmhg_gauge_to_abs_bar() {
    // 0 mmHg(g) => 1 atm abs ≈ 1.01325 barA
    let bar_abs = convert_pressure_mode(
        0.0,
        PressureUnit::MmHg,
        PressureMode::Gauge,
        PressureUnit::Bar,
        PressureMode::Absolute,
    );
    assert!((bar_abs - 1.01325).abs() < 1e-4);
}

#[test]
fn mmhg_full_vacuum_to_abs_bar() {
    // -760 mmHg(g) => 0 bar abs
    let bar_abs = convert_pressure_mode(
        -760.0,
        PressureUnit::MmHg,
        PressureMode::Gauge,
        PressureUnit::Bar,
        PressureMode::Absolute,
    );
    assert!(bar_abs.abs() < 1e-5);
}

#[test]
fn mmhg_abs_to_gauge_mm_hg_roundtrip() {
    // 760 mmHg(abs) => 0 barg => 0 mmHg(g)
    let mmhg_g = convert_pressure_mode(
        760.0,
        PressureUnit::MmHg,
        PressureMode::Absolute,
        PressureUnit::MmHg,
        PressureMode::Gauge,
    );
    assert!(mmhg_g.abs() < 5e-2, "expected ~0 mmHg(g), got {mmhg_g}");
}
