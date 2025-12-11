//! IAPWS-IF97 계산을 seuif97 크레이트로 위임한 래퍼.
//! 입력: 압력(bar, 절대), 온도(°C)
//! 출력: (엔탈피[J/kg], 비체적[m³/kg], 엔트로피[J/kg·K])

use seuif97::{pt, OH, OS, OV};

// ---------------- Region 4 (포화) ----------------
const P4_STAR_MPA: f64 = 22.064;
const T4_STAR_K: f64 = 647.096;
const R4_N: [f64; 6] = [
    -7.859_517_83,
    1.844_082_59,
    -11.786_649_7,
    22.680_741_1,
    -15.961_871_9,
    1.801_225_02,
];

fn nan_err() -> Result<(f64, f64, f64), &'static str> {
    Err("IF97 계산 실패(유효 범위 밖이거나 수렴 실패)")
}

/// Region 1(압축수) 강제 계산. 입력은 bar(abs)/°C.
pub fn region1_props(p_bar_abs: f64, t_c: f64) -> Result<(f64, f64, f64), &'static str> {
    let p_mpa = p_bar_abs / 10.0;
    let h_kj = pt(p_mpa, t_c, (OH, 1));
    let v = pt(p_mpa, t_c, (OV, 1));
    let s_kj = pt(p_mpa, t_c, (OS, 1));
    if h_kj.is_nan() || v.is_nan() || s_kj.is_nan() {
        return nan_err();
    }
    Ok((h_kj * 1000.0, v, s_kj * 1000.0))
}

/// Region 2(과열 증기) 강제 계산. 입력은 bar(abs)/°C.
pub fn region2_props(p_bar_abs: f64, t_c: f64) -> Result<(f64, f64, f64), &'static str> {
    let p_mpa = p_bar_abs / 10.0;
    let h_kj = pt(p_mpa, t_c, (OH, 2));
    let v = pt(p_mpa, t_c, (OV, 2));
    let s_kj = pt(p_mpa, t_c, (OS, 2));
    if h_kj.is_nan() || v.is_nan() || s_kj.is_nan() {
        return nan_err();
    }
    Ok((h_kj * 1000.0, v, s_kj * 1000.0))
}

/// Region 3(포화 부근 고밀도) 강제 계산. 입력은 bar(abs)/°C.
pub fn region3_props(p_bar_abs: f64, t_c: f64) -> Result<(f64, f64, f64), &'static str> {
    let p_mpa = p_bar_abs / 10.0;
    let h_kj = pt(p_mpa, t_c, (OH, 3));
    let v = pt(p_mpa, t_c, (OV, 3));
    let s_kj = pt(p_mpa, t_c, (OS, 3));
    if h_kj.is_nan() || v.is_nan() || s_kj.is_nan() {
        return nan_err();
    }
    Ok((h_kj * 1000.0, v, s_kj * 1000.0))
}

/// Region 5(고온) 강제 계산. 입력은 bar(abs)/°C.
pub fn region5_props(p_bar_abs: f64, t_c: f64) -> Result<(f64, f64, f64), &'static str> {
    let p_mpa = p_bar_abs / 10.0;
    let h_kj = pt(p_mpa, t_c, (OH, 5));
    let v = pt(p_mpa, t_c, (OV, 5));
    let s_kj = pt(p_mpa, t_c, (OS, 5));
    if h_kj.is_nan() || v.is_nan() || s_kj.is_nan() {
        return nan_err();
    }
    Ok((h_kj * 1000.0, v, s_kj * 1000.0))
}

/// 온도(°C)·압력(bar abs)에 따라 자동 영역을 판정해 h/v/s를 반환한다.
pub fn region_props(p_bar_abs: f64, t_c: f64) -> Result<(f64, f64, f64), &'static str> {
    let p_mpa = p_bar_abs / 10.0;
    let h_kj = pt(p_mpa, t_c, OH);
    let v = pt(p_mpa, t_c, OV);
    let s_kj = pt(p_mpa, t_c, OS);
    if h_kj.is_nan() || v.is_nan() || s_kj.is_nan() {
        return nan_err();
    }
    Ok((h_kj * 1000.0, v, s_kj * 1000.0))
}

/// 포화압력(bar abs) - 입력 온도는 °C.
pub fn saturation_pressure_bar_abs_from_temp_c(t_c: f64) -> Result<f64, &'static str> {
    let t_k = t_c + 273.15;
    if t_k <= 0.0 || t_k > T4_STAR_K {
        return Err("IF97 Region4 유효 범위를 벗어났습니다 (0~374°C)");
    }
    let theta = 1.0 - t_k / T4_STAR_K;
    let exp_term = (T4_STAR_K / t_k)
        * (R4_N[0] * theta
            + R4_N[1] * theta.powf(1.5)
            + R4_N[2] * theta.powi(3)
            + R4_N[3] * theta.powf(3.5)
            + R4_N[4] * theta.powi(4)
            + R4_N[5] * theta.powf(7.5));
    let p_mpa = P4_STAR_MPA * exp_term.exp();
    Ok(p_mpa * 10.0)
}

/// 포화온도(°C) - 입력 압력은 bar abs.
pub fn saturation_temp_c_from_pressure_bar_abs(p_bar_abs: f64) -> Result<f64, &'static str> {
    if p_bar_abs <= 0.0 {
        return Err("압력은 양수여야 합니다.");
    }
    let mut t_k = 373.15_f64;
    for _ in 0..30 {
        let theta = 1.0 - t_k / T4_STAR_K;
        let f = (T4_STAR_K / t_k)
            * (R4_N[0] * theta
                + R4_N[1] * theta.powf(1.5)
                + R4_N[2] * theta.powi(3)
                + R4_N[3] * theta.powf(3.5)
                + R4_N[4] * theta.powi(4)
                + R4_N[5] * theta.powf(7.5))
            - (p_bar_abs / 10.0 / P4_STAR_MPA).ln();
        let dtheta_dt = -1.0 / T4_STAR_K;
        let dfdtheta = R4_N[0]
            + 1.5 * R4_N[1] * theta.powf(0.5)
            + 3.0 * R4_N[2] * theta.powi(2)
            + 3.5 * R4_N[3] * theta.powf(2.5)
            + 4.0 * R4_N[4] * theta.powi(3)
            + 7.5 * R4_N[5] * theta.powf(6.5);
        let dfd_t = (-(T4_STAR_K / t_k.powi(2))
            * (R4_N[0] * theta
                + R4_N[1] * theta.powf(1.5)
                + R4_N[2] * theta.powi(3)
                + R4_N[3] * theta.powf(3.5)
                + R4_N[4] * theta.powi(4)
                + R4_N[5] * theta.powf(7.5)))
            + (T4_STAR_K / t_k) * dfdtheta * dtheta_dt;
        let delta = f / dfd_t;
        t_k -= delta;
        if delta.abs() < 1e-8 {
            break;
        }
    }
    Ok(t_k - 273.15)
}
