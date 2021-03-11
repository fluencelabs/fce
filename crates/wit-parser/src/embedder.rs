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

use super::custom::ITCustomSection;
use super::errors::WITParserError;
use crate::Result;

use walrus::ModuleConfig;
use wasmer_wit::{
    ast::Interfaces,
    decoders::wat::{parse, Buffer},
};
use wasmer_wit::ToBytes;

use std::path::PathBuf;

/// Embed provided WIT to a Wasm file by path.
pub fn embed_text_wit(in_wasm_path: PathBuf, out_wasm_path: PathBuf, wit: &str) -> Result<()> {
    let module = ModuleConfig::new()
        .parse_file(&in_wasm_path)
        .map_err(WITParserError::CorruptedWasmFile)?;

    let buffer = Buffer::new(wit)?;
    let ast = parse(&buffer)?;

    let mut module = embed_wit(module, &ast);
    module
        .emit_wasm_file(&out_wasm_path)
        .map_err(WITParserError::WasmEmitError)?;

    Ok(())
}

/// Embed provided WIT to a Wasm module.
pub fn embed_wit(mut wasm_module: walrus::Module, interfaces: &Interfaces<'_>) -> walrus::Module {
    let mut bytes = vec![];
    // TODO: think about possible errors here
    interfaces.to_bytes(&mut bytes).unwrap();

    let custom = ITCustomSection(bytes);
    wasm_module.customs.add(custom);

    wasm_module
}
