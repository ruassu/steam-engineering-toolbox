//! IF97 기준점 회귀 테스트. IAPWS-IF97 공식 문서의 검증 예제 값을 활용한다.
use steam_engineering_toolbox::steam::if97::{
    region1_props, region2_props, region3_props, region5_props, region_props,
};

fn assert_close(label: &str, actual: f64, expected: f64, rel_tol: f64) {
    let denom = expected.abs().max(1.0);
    let diff = (actual - expected).abs();
    assert!(
        diff <= rel_tol * denom,
        "{label} expected {expected:.6} got {actual:.6} (diff {diff:.6}, tol {rel_tol})"
    );
}

#[test]
fn region1_reference_point() {
    // IF97: p = 3 MPa (30 bar abs), T = 300 K (26.85 °C)
    let (h, v, s) = region1_props(30.0, 26.85).expect("region1");
    assert_close("h", h, 115_331.273_021_438_4, 1e-6);
    assert_close("v", v, 0.001_002_151_679_686_694_3, 1e-6);
    assert_close("s", s, 392.294_792_402_624_27, 1e-6);
}

#[test]
fn region2_reference_points() {
    // IF97: p = 0.0035 MPa (0.035 bar abs), T = 300 K (26.85 °C)
    let (h1, v1, s1) = region2_props(0.035, 26.85).expect("region2 low T");
    assert_close("h300K", h1, 2_549_911.450_840_020_3, 1e-6);
    assert_close("v300K", v1, 39.491_386_637_762_98, 1e-6);
    assert_close("s300K", s1, 8_522.389_667_335_792, 1e-6);

    // IF97: p = 0.0035 MPa (0.035 bar abs), T = 700 K (426.85 °C)
    let (h2, v2, s2) = region2_props(0.035, 426.85).expect("region2 high T");
    assert_close("h700K", h2, 3_335_683.753_731_224, 1e-6);
    assert_close("v700K", v2, 92.301_589_817_419_68, 1e-6);
    assert_close("s700K", s2, 10_174.999_578_595_989, 1e-6);
}

#[test]
fn region3_reference_point() {
    // IF97: p = 25 MPa (250 bar abs), T = 650 K (376.85 °C)
    let (h, v, s) = region3_props(250.0, 376.85).expect("region3");
    assert_close("h", h, 1_876_359.122_516_944_4, 5e-4);
    assert_close("v", v, 0.002_045_512_438_704_213_3, 5e-4);
    assert_close("s", s, 4_075.979_000_313_241, 5e-4);
}

#[test]
fn region5_reference_points() {
    // IF97: p = 0.5 MPa (5 bar abs), T = 1500 K (1226.85 °C)
    let (h1, v1, s1) = region5_props(5.0, 1_226.85).expect("region5 low P");
    assert_close("h1500K-0.5MPa", h1, 5_219_768.551_208_338, 1e-6);
    assert_close("v1500K-0.5MPa", v1, 1.384_550_898_781_53, 1e-6);
    assert_close("s1500K-0.5MPa", s1, 9_654.088_753_312_948, 1e-6);

    // IF97: p = 30 MPa (300 bar abs), T = 1500 K (1226.85 °C)
    let (h2, v2, s2) = region5_props(300.0, 1_226.85).expect("region5 high P");
    assert_close("h1500K-30MPa", h2, 5_167_235.140_089_517, 1e-6);
    assert_close("v1500K-30MPa", v2, 0.023_076_129_947_253_575, 1e-6);
    assert_close("s1500K-30MPa", s2, 7_729.701_326_182_764, 1e-6);
}

#[test]
fn region_dispatch_matches_reference_regions() {
    // Region 1 reference point
    let (h1, v1, s1) = region_props(30.0, 26.85).expect("dispatch region1");
    assert_close("h1", h1, 115_331.273_021_438_4, 1e-6);
    assert_close("v1", v1, 0.001_002_151_679_686_694_3, 1e-6);
    assert_close("s1", s1, 392.294_792_402_624_27, 1e-6);

    // Region 2 reference point
    let (h2, v2, s2) = region_props(0.035, 426.85).expect("dispatch region2");
    assert_close("h2", h2, 3_335_683.753_731_224, 1e-6);
    assert_close("v2", v2, 92.301_589_817_419_68, 1e-6);
    assert_close("s2", s2, 10_174.999_578_595_989, 1e-6);

    // Region 3 reference point
    let (h3, v3, s3) = region_props(250.0, 376.85).expect("dispatch region3");
    assert_close("h3", h3, 1_876_359.122_516_944_4, 5e-4);
    assert_close("v3", v3, 0.002_045_512_438_704_213_3, 5e-4);
    assert_close("s3", s3, 4_075.979_000_313_241, 5e-4);

    // Region 5 reference point
    let (h5, v5, s5) = region_props(5.0, 1_226.85).expect("dispatch region5");
    assert_close("h5", h5, 5_219_768.551_208_338, 1e-6);
    assert_close("v5", v5, 1.384_550_898_781_53, 1e-6);
    assert_close("s5", s5, 9_654.088_753_312_948, 1e-6);
}
