use clap::{App, Arg, SubCommand};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

pub const IN_WASM_PATH: &str = "in-wasm-path";

pub fn build<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("build")
        .about("build provided Rust project to Wasm")
        .args(&[Arg::with_name(IN_WASM_PATH)
            .required(true)
            .takes_value(true)
            .short("i")
            .help("path to the wasm file")])
}

pub fn validate<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name("validate")
        .about("validates provided Wasm file to suite Fluence network requirements")
        .args(&[Arg::with_name(IN_WASM_PATH)
            .required(true)
            .takes_value(true)
            .short("i")
            .help("path to the wasm file")])
}
