/// 단순한 온도-허용응력/인장강도 테이블과 선형 보간을 제공한다.
/// 값은 참고용이며 설계 시 최신 코드(ASME 등)로 검증해야 한다.

#[derive(Debug, Clone, Copy)]
pub struct TempPoint {
    pub temp_c: f64,
    pub value_mpa: f64,
}

impl TempPoint {
    pub const fn new(temp_c: f64, value_mpa: f64) -> Self {
        Self { temp_c, value_mpa }
    }
}

#[derive(Debug)]
pub struct MaterialData {
    pub code: &'static str,
    pub name: &'static str,
    pub notes: &'static str,
    pub allowable: &'static [TempPoint],
    pub uts: &'static [TempPoint],
}

#[derive(Debug)]
pub struct MaterialValue {
    pub value_mpa: f64,
    pub source_temp_c: f64,
    /// true면 테이블 범위 밖이라 가장자리 값으로 클램프됨을 의미한다.
    pub clamped: bool,
}

pub fn materials() -> &'static [MaterialData] {
    MATERIALS
}

pub fn find_material(code: &str) -> Option<&'static MaterialData> {
    MATERIALS
        .iter()
        .find(|m| m.code.eq_ignore_ascii_case(code) || m.name.eq_ignore_ascii_case(code))
}

pub fn allowable_stress(code: &str, temp_c: f64) -> Option<MaterialValue> {
    let mat = find_material(code)?;
    interpolate(mat.allowable, temp_c)
}

pub fn uts(code: &str, temp_c: f64) -> Option<MaterialValue> {
    let mat = find_material(code)?;
    interpolate(mat.uts, temp_c)
}

fn interpolate(points: &[TempPoint], temp_c: f64) -> Option<MaterialValue> {
    if points.is_empty() {
        return None;
    }
    if points.len() == 1 {
        let p = points[0];
        return Some(MaterialValue {
            value_mpa: p.value_mpa,
            source_temp_c: p.temp_c,
            clamped: true,
        });
    }
    if temp_c <= points[0].temp_c {
        let p = points[0];
        return Some(MaterialValue {
            value_mpa: p.value_mpa,
            source_temp_c: p.temp_c,
            clamped: true,
        });
    }
    if temp_c >= points[points.len() - 1].temp_c {
        let p = points[points.len() - 1];
        return Some(MaterialValue {
            value_mpa: p.value_mpa,
            source_temp_c: p.temp_c,
            clamped: true,
        });
    }
    for win in points.windows(2) {
        let a = win[0];
        let b = win[1];
        if temp_c >= a.temp_c && temp_c <= b.temp_c {
            let frac = (temp_c - a.temp_c) / (b.temp_c - a.temp_c);
            let val = a.value_mpa + frac * (b.value_mpa - a.value_mpa);
            return Some(MaterialValue {
                value_mpa: val,
                source_temp_c: temp_c,
                clamped: false,
            });
        }
    }
    None
}

const MATERIALS: &[MaterialData] = &[
    MaterialData {
        code: "A106B",
        name: "ASTM A106 Gr.B",
        notes: "Carbon steel seamless; 참고용 ASME Sec II-D 근사치",
        allowable: &[
            tp(20.0, 138.0),
            tp(100.0, 138.0),
            tp(150.0, 132.0),
            tp(200.0, 124.0),
            tp(250.0, 117.0),
            tp(300.0, 110.0),
            tp(350.0, 100.0),
            tp(400.0, 93.0),
            tp(450.0, 83.0),
            tp(500.0, 69.0),
            tp(550.0, 55.0),
            tp(600.0, 45.0),
            tp(650.0, 36.0),
            tp(700.0, 30.0),
        ],
        uts: &[
            tp(20.0, 415.0),
            tp(200.0, 400.0),
            tp(300.0, 390.0),
            tp(400.0, 380.0),
            tp(500.0, 360.0),
            tp(600.0, 340.0),
            tp(700.0, 320.0),
        ],
    },
    MaterialData {
        code: "A53B",
        name: "ASTM A53 Gr.B",
        notes: "Carbon steel ERW/SM; 참고용 ASME Sec II-D 근사치",
        allowable: &[
            tp(20.0, 138.0),
            tp(100.0, 138.0),
            tp(150.0, 132.0),
            tp(200.0, 124.0),
            tp(250.0, 117.0),
            tp(300.0, 110.0),
            tp(350.0, 100.0),
            tp(400.0, 93.0),
            tp(450.0, 83.0),
            tp(500.0, 69.0),
            tp(550.0, 55.0),
            tp(600.0, 45.0),
            tp(650.0, 36.0),
            tp(700.0, 30.0),
        ],
        uts: &[
            tp(20.0, 415.0),
            tp(200.0, 400.0),
            tp(300.0, 390.0),
            tp(400.0, 380.0),
            tp(500.0, 360.0),
            tp(600.0, 340.0),
            tp(700.0, 320.0),
        ],
    },
    MaterialData {
        code: "A335P11",
        name: "ASTM A335 P11",
        notes: "Cr-Mo 1.25Cr-0.5Mo; 고온용 참고치",
        allowable: &[
            tp(20.0, 120.0),
            tp(100.0, 118.0),
            tp(200.0, 113.0),
            tp(300.0, 105.0),
            tp(400.0, 96.0),
            tp(500.0, 86.0),
            tp(550.0, 78.0),
            tp(600.0, 70.0),
            tp(650.0, 63.0),
            tp(700.0, 55.0),
        ],
        uts: &[
            tp(20.0, 485.0),
            tp(400.0, 470.0),
            tp(500.0, 455.0),
            tp(600.0, 440.0),
            tp(700.0, 420.0),
        ],
    },
    MaterialData {
        code: "A335P12",
        name: "ASTM A335 P12",
        notes: "Cr-Mo 1Cr-0.5Mo; 고온용 참고치",
        allowable: &[
            tp(20.0, 110.0),
            tp(100.0, 107.0),
            tp(200.0, 101.0),
            tp(300.0, 93.0),
            tp(400.0, 85.0),
            tp(500.0, 75.0),
            tp(550.0, 68.0),
            tp(600.0, 60.0),
            tp(650.0, 54.0),
            tp(700.0, 48.0),
        ],
        uts: &[
            tp(20.0, 415.0),
            tp(400.0, 400.0),
            tp(500.0, 385.0),
            tp(600.0, 370.0),
            tp(700.0, 350.0),
        ],
    },
    MaterialData {
        code: "A335P91",
        name: "ASTM A335 P91",
        notes: "9Cr-1Mo-V; 고온 내열강 참고치",
        allowable: &[
            tp(20.0, 165.0),
            tp(100.0, 165.0),
            tp(200.0, 163.0),
            tp(300.0, 160.0),
            tp(400.0, 150.0),
            tp(500.0, 135.0),
            tp(550.0, 125.0),
            tp(600.0, 110.0),
            tp(650.0, 96.0),
            tp(700.0, 84.0),
        ],
        uts: &[
            tp(20.0, 585.0),
            tp(400.0, 570.0),
            tp(500.0, 550.0),
            tp(600.0, 530.0),
            tp(700.0, 500.0),
        ],
    },
    MaterialData {
        code: "A335P92",
        name: "ASTM A335 P92",
        notes: "9Cr-0.5Mo-1.8W-V; 고온 내열강 참고치",
        allowable: &[
            tp(20.0, 170.0),
            tp(100.0, 170.0),
            tp(200.0, 168.0),
            tp(300.0, 165.0),
            tp(400.0, 155.0),
            tp(500.0, 140.0),
            tp(550.0, 130.0),
            tp(600.0, 115.0),
            tp(650.0, 100.0),
            tp(700.0, 88.0),
        ],
        uts: &[
            tp(20.0, 620.0),
            tp(400.0, 600.0),
            tp(500.0, 580.0),
            tp(600.0, 560.0),
            tp(700.0, 530.0),
        ],
    },
    MaterialData {
        code: "TP304",
        name: "ASTM A312 TP304",
        notes: "Austenitic stainless; 참고용",
        allowable: &[
            tp(20.0, 138.0),
            tp(100.0, 138.0),
            tp(200.0, 129.0),
            tp(300.0, 120.0),
            tp(400.0, 108.0),
            tp(500.0, 95.0),
            tp(550.0, 88.0),
            tp(600.0, 80.0),
            tp(650.0, 72.0),
            tp(700.0, 64.0),
        ],
        uts: &[
            tp(20.0, 515.0),
            tp(200.0, 500.0),
            tp(400.0, 480.0),
            tp(600.0, 460.0),
            tp(700.0, 440.0),
        ],
    },
    MaterialData {
        code: "TP304L",
        name: "ASTM A312 TP304L",
        notes: "Austenitic stainless L-grade; 참고용",
        allowable: &[
            tp(20.0, 110.0),
            tp(100.0, 110.0),
            tp(200.0, 103.0),
            tp(300.0, 95.0),
            tp(400.0, 87.0),
            tp(500.0, 75.0),
            tp(550.0, 68.0),
            tp(600.0, 60.0),
            tp(650.0, 54.0),
            tp(700.0, 48.0),
        ],
        uts: &[
            tp(20.0, 485.0),
            tp(200.0, 470.0),
            tp(400.0, 450.0),
            tp(600.0, 430.0),
            tp(700.0, 410.0),
        ],
    },
    MaterialData {
        code: "TP316",
        name: "ASTM A312 TP316",
        notes: "Austenitic stainless Mo; 참고용",
        allowable: &[
            tp(20.0, 138.0),
            tp(100.0, 138.0),
            tp(200.0, 131.0),
            tp(300.0, 122.0),
            tp(400.0, 110.0),
            tp(500.0, 98.0),
            tp(550.0, 90.0),
            tp(600.0, 82.0),
            tp(650.0, 74.0),
            tp(700.0, 66.0),
        ],
        uts: &[
            tp(20.0, 515.0),
            tp(200.0, 500.0),
            tp(400.0, 480.0),
            tp(600.0, 460.0),
            tp(700.0, 440.0),
        ],
    },
    MaterialData {
        code: "TP316L",
        name: "ASTM A312 TP316L",
        notes: "Austenitic stainless Mo L-grade; 참고용",
        allowable: &[
            tp(20.0, 110.0),
            tp(100.0, 110.0),
            tp(200.0, 104.0),
            tp(300.0, 96.0),
            tp(400.0, 88.0),
            tp(500.0, 76.0),
            tp(550.0, 69.0),
            tp(600.0, 61.0),
            tp(650.0, 54.0),
            tp(700.0, 48.0),
        ],
        uts: &[
            tp(20.0, 485.0),
            tp(200.0, 470.0),
            tp(400.0, 450.0),
            tp(600.0, 430.0),
            tp(700.0, 410.0),
        ],
    },
];

const fn tp(temp_c: f64, value_mpa: f64) -> TempPoint {
    TempPoint::new(temp_c, value_mpa)
}

// NOTE:
// - Allowable stress values are approximate, adapted from typical ASME Section II-D / B31 tables (circa 2023) for reference.
// - Points above ~600°C are conservatively extended; always verify against the latest code/standard for design.
// - UTS values are nominal; not for fracture assessments. Consult governing code/standard for certified values.
