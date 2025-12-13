use clap::Parser;
use steam_engineering_toolbox::i18n::keys;
use steam_engineering_toolbox::{app, config, i18n};

#[derive(Parser, Debug)]
#[command(name = "steam_engineering_toolbox_cli")]
struct CliArgs {
    /// UI language (auto|en-us|en-uk|ko-kr|ko). auto uses config, then system locale, then en-us.
    #[arg(long = "lang", short = 'L', default_value = "auto")]
    lang: String,
}

/// 프로그램의 엔트리 포인트. 설정을 로드한 뒤 CLI 애플리케이션을 실행한다.
fn main() {
    let args = CliArgs::parse();
    if let Err((lang_code, err)) = try_run(&args) {
        let tr = i18n::Translator::new(&lang_code);
        eprintln!("{}: {err}", tr.t(keys::ERROR_PREFIX));
    }
}

fn try_run(args: &CliArgs) -> Result<(), (String, Box<dyn std::error::Error>)> {
    let lang_hint = i18n::resolve_language(&args.lang, None);
    let mut cfg = config::load_or_default().map_err(|e| (lang_hint.clone(), Box::new(e) as _))?;
    let lang_code = i18n::resolve_language(&args.lang, Some(cfg.language.as_str()));
    cfg.language = lang_code.clone();
    let tr = i18n::Translator::new_with_pack(&lang_code, cfg.language_pack_dir.as_deref());
    app::run(&mut cfg, &tr).map_err(|e| (lang_code, Box::new(e) as _))?;
    Ok(())
}
