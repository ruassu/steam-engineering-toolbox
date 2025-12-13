//! 핵심 계산 로직을 라이브러리로 분리하여 CLI 뿐 아니라 추후 GUI 확장도 쉽게 한다.

pub mod air;
pub mod app;
pub mod condensate_recovery;
pub mod config;
pub mod conversion;
pub mod cooling;
pub mod gas;
pub mod i18n;
pub mod material_db;
pub mod quantity;
pub mod steam;
pub mod ui_cli;
pub mod units;
pub mod water;
