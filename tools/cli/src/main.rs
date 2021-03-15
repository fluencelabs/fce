/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod args;
mod build;
mod errors;

use fce_module_info_parser::manifest;
use fce_module_info_parser::sdk_version;

pub(crate) type CLIResult<T> = std::result::Result<T, crate::errors::CLIError>;

pub fn main() -> Result<(), anyhow::Error> {
    let app = clap::App::new(args::DESCRIPTION)
        .version(args::VERSION)
        .author(args::AUTHORS)
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .subcommand(args::build())
        .subcommand(args::embed_wit())
        .subcommand(args::embed_version())
        .subcommand(args::show_manifest())
        .subcommand(args::show_wit())
        .subcommand(args::repl());
    let arg_matches = app.get_matches();

    match arg_matches.subcommand() {
        ("build", Some(args)) => build(args),
        ("embed_it", Some(args)) => embed_wit(args),
        ("embed_ver", Some(args)) => embed_version(args),
        ("it", Some(args)) => it(args),
        ("info", Some(args)) => info(args),
        ("repl", Some(args)) => repl(args),
        (c, _) => Err(crate::errors::CLIError::NoSuchCommand(c.to_string()).into()),
    }
}

fn build(args: &clap::ArgMatches<'_>) -> Result<(), anyhow::Error> {
    let trailing_args: Vec<&str> = args.values_of("optional").unwrap_or_default().collect();

    crate::build::build(trailing_args)?;

    Ok(())
}

fn embed_wit(args: &clap::ArgMatches<'_>) -> Result<(), anyhow::Error> {
    let in_wasm_path = args.value_of(args::IN_WASM_PATH).unwrap();
    let it_path = args.value_of(args::WIT_PATH).unwrap();
    let out_wasm_path = match args.value_of(args::OUT_WASM_PATH) {
        Some(path) => path,
        None => in_wasm_path,
    };

    let it = std::fs::read(it_path)?;
    let it = String::from_utf8(it)?;

    fce_wit_parser::embed_text_wit(in_wasm_path, out_wasm_path, &it)?;

    println!("interface types were successfully embedded");

    Ok(())
}

fn embed_version(args: &clap::ArgMatches<'_>) -> Result<(), anyhow::Error> {
    use std::str::FromStr;

    let in_wasm_path = args.value_of(args::IN_WASM_PATH).unwrap();
    let version = args.value_of(args::SDK_VERSION).unwrap();
    let out_wasm_path = match args.value_of(args::OUT_WASM_PATH) {
        Some(path) => path,
        None => in_wasm_path,
    };

    let version = semver::Version::from_str(version)?;
    sdk_version::embed_from_path(in_wasm_path, out_wasm_path, version)?;

    println!("the version was successfully embedded");

    Ok(())
}

fn it(args: &clap::ArgMatches<'_>) -> Result<(), anyhow::Error> {
    let wasm_path = args.value_of(args::IN_WASM_PATH).unwrap();

    let it = fce_wit_parser::extract_text_wit(wasm_path)?;
    println!("{}", it);

    Ok(())
}

fn info(args: &clap::ArgMatches<'_>) -> Result<(), anyhow::Error> {
    let wasm_path = args.value_of(args::IN_WASM_PATH).unwrap();

    let wasm_module = walrus::ModuleConfig::new().parse_file(wasm_path)?;
    let sdk_version = sdk_version::extract_from_module(&wasm_module)?;
    let module_manifest = manifest::extract_from_module(&wasm_module)?;
    let it_version = fce_wit_parser::extract_version_from_module(&wasm_module)?;

    println!("it version:  {}", it_version);
    match sdk_version {
        Some(sdk_version) => println!("sdk version: {}", sdk_version),
        None => println!("module doesn't contain sdk version"),
    }

    match module_manifest {
        Some(manifest) => println!("{}", manifest),
        None => println!("module doesn't contain module manifest"),
    }

    Ok(())
}

fn repl(args: &clap::ArgMatches<'_>) -> Result<(), anyhow::Error> {
    use std::process::Command;
    // use UNIX-specific API for replacing process image
    use std::os::unix::process::CommandExt;

    let trailing_args: Vec<&str> = args.values_of("optional").unwrap_or_default().collect();

    let mut repl = Command::new("fce-repl");
    repl.args(trailing_args);
    let error = repl.exec();
    if error.kind() == std::io::ErrorKind::NotFound {
        println!("fce-repl not found, run `cargo +nightly install frepl` to install it");
    } else {
        // this branch should be executed if exec was successful, so just else if fine here
        println!("error occurred: {:?}", error);
    }

    Ok(())
}
