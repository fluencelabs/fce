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

use crate::misc::{CoreModulesConfig, make_fce_config};
use crate::RawCoreModulesConfig;
use crate::Result;

use super::faas_interface::FaaSInterface;
use super::FaaSError;
use super::IValue;

use fce::FCE;
use fce::FCEModuleConfig;

use std::convert::TryInto;
use std::fs;
use std::path::PathBuf;
use crate::faas_interface::FaaSFunctionSignature;
use std::collections::HashSet;

// TODO: remove and use mutex instead
unsafe impl Send for FluenceFaaS {}

/// Strategy for module loading: either `All`, or only those specified in `Named`
pub enum Modules<'a> {
    All,
    Named(&'a HashSet<String>),
}

impl<'a> Modules<'a> {
    #[inline]
    /// Returns true if `module` should be loaded
    pub fn should_load(&self, module: &str) -> bool {
        match self {
            Modules::All => true,
            Modules::Named(set) => set.contains(module),
        }
    }

    #[inline]
    pub fn required_modules(&self) -> usize {
        match self {
            Modules::Named(set) => set.len(),
            _ => 0,
        }
    }

    #[inline]
    /// Returns difference between required and loaded modules
    pub fn missing_modules<'s>(&self, loaded: impl Iterator<Item = &'s String>) -> HashSet<String> {
        match self {
            Modules::Named(set) => {
                let set = (*set).clone();
                loaded.fold(set, |mut set, module| {
                    set.remove(module);
                    set
                })
            }
            _ => <_>::default(),
        }
    }
}

pub struct FluenceFaaS {
    fce: FCE,

    // config for code loaded by call_code function
    faas_code_config: FCEModuleConfig,
}

impl FluenceFaaS {
    /// Creates FaaS from config on filesystem.
    pub fn new<P: Into<PathBuf>>(config_file_path: P) -> Result<Self> {
        let config = crate::misc::load_config(config_file_path.into())?;
        Self::with_raw_config(config)
    }

    /// Creates FaaS from config deserialized from TOML.
    pub fn with_raw_config(config: RawCoreModulesConfig) -> Result<Self> {
        let config = crate::misc::from_raw_config(config)?;
        let modules = config
            .core_modules_dir
            .as_ref()
            .map_or(Ok(vec![]), |dir| Self::load_modules(dir, Modules::All))?;
        Self::with_modules(modules, config)
    }

    /// Creates FaaS with given modules.
    pub fn with_modules<I, C>(modules: I, config: C) -> Result<Self>
    where
        I: IntoIterator<Item = (String, Vec<u8>)>,
        C: TryInto<CoreModulesConfig>,
        FaaSError: From<C::Error>,
    {
        let mut fce = FCE::new();
        let mut config = config.try_into()?;

        for (name, bytes) in modules {
            let module_config = crate::misc::make_fce_config(config.modules_config.remove(&name))?;
            fce.load_module(name.clone(), &bytes, module_config)?;
        }

        let faas_code_config = make_fce_config(config.rpc_module_config)?;

        Ok(Self {
            fce,
            faas_code_config,
        })
    }

    /// Searches for modules in `config.core_modules_dir`, loads only those in the `names` set
    pub fn with_module_names<C>(names: &HashSet<String>, config: C) -> Result<Self>
    where
        C: TryInto<CoreModulesConfig>,
        FaaSError: From<C::Error>,
    {
        let config = config.try_into()?;
        let modules = config.core_modules_dir.as_ref().map_or(Ok(vec![]), |dir| {
            Self::load_modules(dir, Modules::Named(names))
        })?;

        Self::with_modules::<_, CoreModulesConfig>(modules, config)
    }

    /// Loads modules from a directory at a given path. Non-recursive, ignores subdirectories.
    fn load_modules(core_modules_dir: &str, modules: Modules) -> Result<Vec<(String, Vec<u8>)>> {
        use FaaSError::IOError;

        let mut dir_entries = fs::read_dir(core_modules_dir)
            .map_err(|e| IOError(format!("{}: {}", core_modules_dir, e)))?;

        let loaded = dir_entries.try_fold(vec![], |mut vec, entry| {
            let entry = entry?;
            let path = entry.path();
            // Skip directories
            if path.is_dir() {
                return Ok(vec);
            }

            let module_name = path
                .file_name()
                .ok_or_else(|| IOError(format!("No file name in path {:?}", path)))?
                .to_os_string()
                .into_string()
                .map_err(|name| IOError(format!("invalid file name: {:?}", name)))?;

            if modules.should_load(&module_name) {
                let module_bytes = fs::read(path)?;
                vec.push((module_name, module_bytes));
            }

            Result::Ok(vec)
        })?;

        if modules.required_modules() > loaded.len() {
            let loaded = loaded.iter().map(|(n, _)| n);
            let not_found = modules.missing_modules(loaded);
            return Err(FaaSError::ConfigParseError(format!(
                "the following modules were not found: {:?}",
                not_found
            )));
        }

        Ok(loaded)
    }

    /// Executes provided Wasm code in the internal environment (with access to module exports).
    pub fn call_code<S: AsRef<str>>(
        &mut self,
        wasm: &[u8],
        func_name: S,
        args: &[IValue],
    ) -> Result<Vec<IValue>> {
        self.call_code_(wasm, func_name.as_ref(), args)
    }

    pub fn call_code_(
        &mut self,
        wasm: &[u8],
        func_name: &str,
        args: &[IValue],
    ) -> Result<Vec<IValue>> {
        // We need this because every wasm code loaded into VM needs a module name
        let anonymous_module = "anonymous_module_name";

        self.fce
            .load_module(anonymous_module, wasm, self.faas_code_config.clone())?;

        let call_result = self.fce.call(anonymous_module, func_name, args)?;
        self.fce.unload_module(anonymous_module)?;

        Ok(call_result)
    }

    /// Call a specified function of loaded on a startup module by its name.
    pub fn call_module<MN: AsRef<str>, FN: AsRef<str>>(
        &mut self,
        module_name: MN,
        func_name: FN,
        args: &[IValue],
    ) -> Result<Vec<IValue>> {
        self.fce
            .call(module_name, func_name, args)
            .map_err(Into::into)
    }

    /// Return all export functions (name and signatures) of loaded on a startup modules.
    pub fn get_interface(&self) -> FaaSInterface {
        let modules = self
            .fce
            .interface()
            .map(|(name, signatures)| {
                let signatures = signatures
                    .iter()
                    .map(|f| {
                        (
                            f.name,
                            FaaSFunctionSignature {
                                input_types: f.input_types,
                                output_types: f.output_types,
                            },
                        )
                    })
                    .collect();
                (name, signatures)
            })
            .collect();

        FaaSInterface { modules }
    }
}
