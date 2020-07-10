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

mod path;

use crate::path::to_full_path;

use fluence::fce;

const RESULT_FILE_PATH: &str = "/tmp/ipfs_rpc_file";
const IPFS_ADDR_ENV_NAME: &str = "IPFS_ADDR";
const TIMEOUT_ENV_NAME: &str = "timeout";

pub fn main() {}

/// Put file from specified path to IPFS and return its hash.
#[fce]
pub fn put(file_path: String) -> String {
    let file_path = to_full_path(file_path);

    let timeout = std::env::var(TIMEOUT_ENV_NAME).unwrap_or_else(|_| "1s".to_string());
    let cmd = format!("add --timeout {} -Q {}", timeout, file_path);

    call_ipfs(cmd)
}

/// Get file by provided hash from IPFS, saves it to a temporary file and returns a path to it.
#[fce]
pub fn get(hash: String) -> String {
    let result_file_path = to_full_path(RESULT_FILE_PATH);

    let timeout = std::env::var(TIMEOUT_ENV_NAME).unwrap_or_else(|_| "1s".to_string());
    let cmd = format!(
        "get --timeout {} -o {}  {}",
        timeout, result_file_path, hash
    );

    call_ipfs(cmd);

    RESULT_FILE_PATH.to_string()
}

#[fce]
pub fn get_address() -> String {
    match std::env::var(IPFS_ADDR_ENV_NAME) {
        Ok(addr) => addr,
        Err(e) => format!(
            "getting {} env variable failed with error {:?}",
            IPFS_ADDR_ENV_NAME, e
        ),
    }
}

fn call_ipfs(cmd: String) -> String {
    unsafe {
        // TODO: better error handling
        match ipfs(cmd.as_ptr() as _, cmd.len() as _) {
            0 => String::from_raw_parts(
                fluence::internal::get_result_ptr() as _,
                fluence::internal::get_result_size(),
                fluence::internal::get_result_size(),
            ),
            _ => "host ipfs call failed".to_string(),
        }
    }
}

#[link(wasm_import_module = "host")]
extern "C" {
    /// Put a file to ipfs, returns ipfs hash of the file.
    fn ipfs(ptr: i32, size: i32) -> i32;
}
