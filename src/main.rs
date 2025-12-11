use steam_engineering_toolbox::{app, config};

/// 프로그램의 엔트리 포인트. 설정을 로드한 뒤 CLI 애플리케이션을 실행한다.
fn main() {
    if let Err(err) = try_run() {
        eprintln!("오류: {err}");
    }
}

fn try_run() -> Result<(), Box<dyn std::error::Error>> {
    let mut cfg = config::load_or_default()?;
    app::run(&mut cfg)?;
    Ok(())
}
