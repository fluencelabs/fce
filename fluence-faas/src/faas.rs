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

use super::FaaSError;
use super::faas_interface::FaaSInterface;
use super::faas_interface::FaaSModuleInterface;

use fce::FCE;
use super::IValue;
use fce::FCEModuleConfig;

use std::fs;
use std::path::PathBuf;
use crate::RawCoreModulesConfig;
use crate::misc::{CoreModulesConfig, make_fce_config};
use std::convert::TryInto;
use std::fs::DirEntry;

/// FluenceFaas isn't thread safe.
// impl !Sync for FluenceFaaS {}

// TODO: remove and use mutex instead
unsafe impl Send for FluenceFaaS {}

pub struct FluenceFaaS {
    fce: FCE,

    // names of core modules loaded to FCE
    module_names: Vec<String>,

    // config for code loaded by call_code function
    faas_code_config: FCEModuleConfig,
}

impl FluenceFaaS {
    /// Creates FaaS from config on filesystem.
    pub fn new<P: Into<PathBuf>>(config_file_path: P) -> Result<Self, FaaSError> {
        let core_modules_config = crate::misc::parse_config_from_file(config_file_path.into())?;
        Self::with_config(core_modules_config)
    }

    /// Creates FaaS from config deserialized from TOML.
    pub fn with_raw_config(config: RawCoreModulesConfig) -> Result<Self, FaaSError> {
        let core_modules_config = crate::misc::from_raw_config(config)?;
        Self::with_config(core_modules_config)
    }

    /// Creates FaaS with given modules.
    pub fn with_modules<I, C>(modules: I, config: C) -> Result<Self, FaaSError>
    where
        I: IntoIterator<Item = (String, Vec<u8>)>,
        C: TryInto<CoreModulesConfig>,
        FaaSError: From<C::Error>,
    {
        let mut fce = FCE::new();
        let mut module_names = Vec::new();
        let mut config = config.try_into()?;

        for (name, bytes) in modules {
            let module_config = crate::misc::make_fce_config(config.modules_config.remove(&name))?;
            fce.load_module(name.clone(), &bytes, module_config)?;
            module_names.push(name);
        }

        let faas_code_config = make_fce_config(config.rpc_module_config)?;

        Ok(Self {
            fce,
            module_names,
            faas_code_config,
        })
    }

    /// Creates FaaS from prepared config.
    pub(crate) fn with_config(config: CoreModulesConfig) -> Result<Self, FaaSError> {
        let entries = fs::read_dir(&config.core_modules_dir)?.collect::<Result<Vec<_>, _>>()?;

        let modules = entries
            .into_iter()
            // skip directories
            .filter(|e| !e.path().is_dir())
            .map::<Result<(String, Vec<u8>), FaaSError>, _>(|entry: DirEntry| {
                let module_name = entry.path().file_name().unwrap().to_os_string();
                let module_name = module_name
                    .into_string()
                    .map_err(|name| FaaSError::IOError(format!("invalid file name: {:?}", name)))?;
                let module_bytes = fs::read(entry.path())?;
                Ok((module_name, module_bytes))
            })
            .collect::<Result<Vec<(String, Vec<u8>)>, _>>()?;

        Self::with_modules(modules, config)
    }

    /// Executes provided Wasm code in the internal environment (with access to module exports).
    pub fn call_code(
        &mut self,
        wasm: &[u8],
        func_name: &str,
        args: &[IValue],
    ) -> Result<Vec<IValue>, FaaSError> {
        // We need this because every wasm code loaded into VM needs a module name
        let anonymous_module = "anonymous_module_name";

        self.fce
            .load_module(anonymous_module, wasm, self.faas_code_config.clone())?;

        let call_result = self.fce.call(anonymous_module, func_name, args)?;
        self.fce.unload_module(anonymous_module)?;

        Ok(call_result)
    }

    /// Call a specified function of loaded on a startup module by its name.
    pub fn call_module(
        &mut self,
        module_name: &str,
        func_name: &str,
        args: &[IValue],
    ) -> Result<Vec<IValue>, FaaSError> {
        self.fce
            .call(module_name, func_name, args)
            .map_err(Into::into)
    }

    /// Return all export functions (name and signatures) of loaded on a startup modules.
    pub fn get_interface(&self) -> FaaSInterface {
        let mut modules = Vec::with_capacity(self.module_names.len());

        for module_name in self.module_names.iter() {
            let functions = self.fce.get_interface(module_name).unwrap();
            modules.push(FaaSModuleInterface {
                name: module_name,
                functions,
            })
        }

        FaaSInterface { modules }
    }
}
