// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::Local;
use dotenvy::var;
use rocket::{fs::relative, launch};
use std::fs;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};
use xapi_rs::build;

#[launch]
async fn rocket() -> _ {
    fs::create_dir_all(relative!("logs")).expect("Failed creating 'logs' dir :(");
    let rust_log = var("RUST_LOG").expect("Missing RUST_LOG :(");
    let filter = tracing_subscriber::EnvFilter::builder()
        .parse(rust_log)
        .expect("Failed parsing RUST_LOG :(");
    let now = Local::now();
    let file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("logs/xapi-{}.log", now.format("%Y%m%d-%H%M%S")))
        .unwrap();
    let file_logger = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(file)
        .with_ansi(false)
        .with_filter(filter);

    let console_logger = tracing_subscriber::fmt::layer().with_filter(LevelFilter::INFO);

    tracing_subscriber::registry()
        .with(file_logger)
        .with(console_logger)
        .init();

    build(false) // false == not for testing
}
