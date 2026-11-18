use clap::Parser;
use crs_web::Command;
use dioxus::logger::initialize_default;
use errgonomic::exit_result;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    initialize_default();
    exit_result(Command::parse().run().await)
}
