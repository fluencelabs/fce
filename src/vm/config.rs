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

use std::path::PathBuf;
use wasmer_wasi::WasiVersion;

#[derive(Clone, Debug, PartialEq)]
pub struct WASIConfig {
    /// Desired WASI version.
    pub version: WasiVersion,

    /// Environment variables for loaded modules.
    pub envs: Vec<Vec<u8>>,

    /// List of available directories for loaded modules.
    pub preopened_files: Vec<PathBuf>,

    /// Mapping between paths.
    pub mapped_dirs: Vec<(String, PathBuf)>,
}

impl Default for WASIConfig {
    fn default() -> Self {
        Self {
            version: WasiVersion::Latest,
            envs: vec![],
            preopened_files: vec![],
            mapped_dirs: vec![],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Count of Wasm memory pages that will be preallocated on the VM startup.
    /// Each Wasm pages is 65536 bytes long.
    pub mem_pages_count: i32,

    /// If true, registers the logger Wasm module with name 'logger'.
    /// This functionality is just for debugging, and this module will be disabled in future.
    pub logger_enabled: bool,

    /// The name of the main module handler function.
    pub invoke_function_name: String,

    /// The name of function that should be called for allocation memory. This function
    /// is used for passing array of bytes to the main module.
    pub allocate_function_name: String,

    /// The name of function that should be called for deallocation of
    /// previously allocated memory by allocateFunction.
    pub deallocate_function_name: String,

    /// Config for WASI subsystem initialization. None means that module should be loaded
    /// without WASI.
    pub wasi_config: Option<WASIConfig>,
}

impl Default for Config {
    fn default() -> Self {
        // some reasonable defaults
        Self {
            // 65536*1600 ~ 100 Mb
            mem_pages_count: 1600,
            invoke_function_name: "invoke".to_string(),
            allocate_function_name: "allocate".to_string(),
            deallocate_function_name: "deallocate".to_string(),
            logger_enabled: true,
            wasi_config: Some(WASIConfig::default()),
        }
    }
}
