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

use fluence_app_service::AppService;
use fluence_app_service::IValue;

use std::path::PathBuf;

const IPFS_MODULES_CONFIG_PATH: &str = "Config.toml";

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let mut ipfs_node = AppService::new(std::iter::empty(), );
    println!("ipfs node interface is\n{}", ipfs_node.get_interface());

    let node_address = ipfs_node.call("ipfs_node.wasm", "get_address", &[])?;
    println!("ipfs node address is:\n{:?}", node_address);

    let result = ipfs_node.call(
        "ipfs_rpc.wasm",
        "get",
        &[IValue::String(
            "QmXdC36pX1B1sdHdbri859vMYctQjAhvTmkWyG9xzhShxb".to_string(),
        )],
    )?;
    println!("execution result {:?}", result);

    Ok(())
}
