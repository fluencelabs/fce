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

use crate::Result;
use super::AppServiceError;
use super::IValue;

use fluence_faas::FluenceFaaS;
use fluence_faas::ModulesConfig;

use std::convert::TryInto;

const SERVICE_ID_ENV_NAME: &str = "service_id";
const SERVICE_LOCAL_DIR_NAME: &str = "local";
const SERVICE_TMP_DIR_NAME: &str = "tmp";

// TODO: remove and use mutex instead
unsafe impl Send for AppService {}

pub struct AppService {
    faas: FluenceFaaS,
}

impl AppService {
    /// Creates Service with given modules and service id.
    pub fn new<I, C, S>(modules: I, config: C, service_id: S) -> Result<Self>
    where
        I: IntoIterator<Item = String>,
        C: TryInto<ModulesConfig>,
        S: AsRef<str>,
        AppServiceError: From<C::Error>,
    {
        let config: ModulesConfig = config.try_into()?;
        let service_id = service_id.as_ref();
        let config = Self::set_env_and_dirs(config, service_id)?;

        let modules = modules.into_iter().collect();
        let faas = FluenceFaaS::with_module_names(&modules, config)?;

        Ok(Self { faas })
    }

    /// Call a specified function of loaded module by its name.
    // TODO: replace serde_json::Value with Vec<u8>?
    pub fn call_module<MN: AsRef<str>, FN: AsRef<str>>(
        &mut self,
        module_name: MN,
        func_name: FN,
        arguments: serde_json::Value,
    ) -> Result<Vec<IValue>> {
        let arguments = Self::json_to_ivalue(arguments)?;

        self.faas
            .call_module(module_name, func_name, &arguments)
            .map_err(Into::into)
    }

    /// Return all export functions (name and signatures) of loaded modules.
    pub fn get_interface(&self) -> fluence_faas::FaaSInterface<'_> {
        self.faas.get_interface()
    }

    /// Prepare service before starting by:
    ///  1. creating a directory structure in the following form:
    ///     - service_base_dir/service_id/SERVICE_LOCAL_DIR_NAME
    ///     - service_base_dir/service_id/SERVICE_TMP_DIR_NAME
    ///  2. adding service_id to environment variables
    fn set_env_and_dirs(mut config: ModulesConfig, service_id: &str) -> Result<ModulesConfig> {
        let base_dir = match config.service_base_dir {
            Some(ref base_dir) => base_dir,
            // TODO: refactor it later
            None => {
                return Err(AppServiceError::IOError(String::from(
                    "service_base_dir should be specified",
                )))
            }
        };

        let service_dir_path = std::path::Path::new(base_dir).join(service_id);
        std::fs::create_dir(service_dir_path.clone())?; // will return an error if dir is already exists

        let local_dir_path = service_dir_path.join(SERVICE_LOCAL_DIR_NAME);
        std::fs::create_dir(local_dir_path.clone())?; // will return an error if dir is already exists

        let tmp_dir_path = service_dir_path.join(SERVICE_TMP_DIR_NAME);
        std::fs::create_dir(tmp_dir_path.clone())?; // will return an error if dir is already exists

        let local_dir: String = local_dir_path.to_string_lossy().into();
        let tmp_dir: String = tmp_dir_path.to_string_lossy().into();

        let service_id_env = vec![format!("{}={}", SERVICE_ID_ENV_NAME, service_id).into_bytes()];
        let preopened_files = vec![local_dir.clone(), tmp_dir.clone()];
        let mapped_dirs = vec![
            (String::from(SERVICE_LOCAL_DIR_NAME), local_dir),
            (String::from(SERVICE_TMP_DIR_NAME), tmp_dir),
        ];

        config.modules_config = config
            .modules_config
            .into_iter()
            .map(|(name, module_config)| {
                let module_config = module_config
                    .extend_wasi_envs(service_id_env.clone())
                    .extend_wasi_files(preopened_files.clone(), mapped_dirs.clone());

                (name, module_config)
            })
            .collect();

        Ok(config)
    }

    fn json_to_ivalue(arguments: serde_json::Value) -> Result<Vec<IValue>> {
        // If arguments are on of: null, [] or {}, avoid calling `to_interface_value`
        let is_null = arguments.is_null();
        let is_empty_arr = arguments.as_array().map_or(false, |a| a.is_empty());
        let is_empty_obj = arguments.as_object().map_or(false, |m| m.is_empty());
        let arguments = if !is_null && !is_empty_arr && !is_empty_obj {
            Some(fluence_faas::to_interface_value(&arguments).map_err(|e| {
                AppServiceError::InvalidArguments(format!(
                    "can't parse arguments as array of interface types: {}",
                    e
                ))
            })?)
        } else {
            None
        };

        match arguments {
            Some(IValue::Record(arguments)) => Ok(arguments.into_vec()),
            // Convert null, [] and {} into vec![]
            None => Ok(vec![]),
            other => Err(AppServiceError::InvalidArguments(format!(
                "expected array of interface values: got {:?}",
                other
            ))),
        }
    }
}