use steam_engineering_toolbox::{
    conversion::PressureMode,
    cooling::{condenser, cooling_tower, pump_npsh},
    units::PressureUnit,
};

#[test]
fn condenser_lmtd_positive() {
    let res = condenser::compute_condenser(condenser::CondenserInput {
        steam_pressure: 0.3, // bar abs
        steam_pressure_unit: PressureUnit::Bar,
        steam_pressure_mode: PressureMode::Absolute,
        steam_temp_c: None,
        cw_inlet_temp_c: 25.0,
        cw_outlet_temp_c: 35.0,
        cw_flow_m3_per_h: 100.0,
        ua_kw_per_k: None,
        area_m2: None,
        overall_u_w_m2k: None,
        target_back_pressure_bar_abs: Some(0.35),
    })
    .expect("condenser calc");
    assert!(
        res.lmtd_k > 0.0,
        "lmtd={} Tsat={} Psat={}",
        res.lmtd_k,
        res.condensing_temp_c,
        res.condensing_pressure_bar_abs
    );
    assert!(res.condensing_pressure_bar_abs > 0.25);
}

#[test]
fn cooling_tower_range_approach() {
    let res = cooling_tower::compute_cooling_tower(cooling_tower::CoolingTowerInput {
        water_in_c: 40.0,
        water_out_c: 30.0,
        dry_bulb_c: 32.0,
        wet_bulb_c: 26.0,
        water_flow_m3_per_h: 100.0,
        target_range_c: Some(8.0),
        target_approach_c: Some(4.0),
    });
    assert!((res.range_c - 10.0).abs() < 1e-6);
    assert!((res.approach_c - 4.0).abs() < 1e-6);
}

#[test]
fn pump_npsh_margin_above_one() {
    let res = pump_npsh::compute_pump_npsh(pump_npsh::PumpNpshInput {
        suction_pressure_bar: 0.5,
        suction_is_abs: false,
        liquid_temp_c: 25.0,
        static_head_m: 3.0,
        friction_loss_m: 1.0,
        npshr_m: 3.0,
        rho_kg_m3: 998.0,
    });
    assert!(res.margin_ratio > 1.1);
}
