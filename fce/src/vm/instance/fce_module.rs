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

use super::wit_prelude::*;
use super::{IType, IValue, WValue};

use fce_wit_interfaces::extract_fce_wit;
use fce_wit_interfaces::FCEWITInterfaces;
use wasmer_wit::interpreter::Interpreter;
use wasmer_runtime::{compile, ImportObject};
use wasmer_core::Instance as WasmerInstance;
use wasmer_core::import::Namespace;

use std::collections::HashMap;
use std::convert::TryInto;
use std::mem::MaybeUninit;
use std::sync::Arc;

type WITInterpreter =
    Interpreter<WITInstance, WITExport, WITFunction, WITMemory, WITMemoryView<'static>>;

struct WITModuleFunc {
    interpreter: WITInterpreter,
    inputs: Vec<IType>,
    outputs: Vec<IType>,
}

pub struct FCEModule {
    // it is needed because of WITInstance contains dynamic functions
    // that internally keep pointer to Wasmer instance.
    #[allow(unused)]
    wamser_instance: WasmerInstance,
    wit_instance: Arc<WITInstance>,
    exports_funcs: HashMap<String, WITModuleFunc>,
}

impl FCEModule {
    pub fn new(
        wasm_bytes: &[u8],
        imports: ImportObject,
        modules: &HashMap<String, Arc<FCEModule>>,
    ) -> Result<Self, FCEError> {
        let wasmer_module = compile(&wasm_bytes)?;
        let wit = extract_fce_wit(&wasmer_module)?;
        let wit_exports = Self::instantiate_wit_exports(&wit)?;

        let mut wit_instance = Arc::new_uninit();
        let mut import_object = Self::adjust_wit_imports(&wit, wit_instance.clone())?;
        import_object.extend(imports);

        let wasmer_instance = wasmer_module.instantiate(&import_object)?;

        let wit_instance = unsafe {
            // get_mut_unchecked here is safe because currently only this modules have reference to
            // it and the environment is single-threaded
            *Arc::get_mut_unchecked(&mut wit_instance) =
                MaybeUninit::new(WITInstance::new(&wasmer_instance, &wit, modules)?);
            std::mem::transmute::<_, Arc<WITInstance>>(wit_instance)
        };

        Ok(Self {
            wamser_instance: wasmer_instance,
            wit_instance,
            exports_funcs: wit_exports,
        })
    }

    pub fn call(&mut self, function_name: &str, args: &[IValue]) -> Result<Vec<IValue>, FCEError> {
        use wasmer_wit::interpreter::stack::Stackable;

        match self.exports_funcs.get(function_name) {
            Some(func) => {
                let result = func
                    .interpreter
                    .run(args, Arc::make_mut(&mut self.wit_instance))?
                    .as_slice()
                    .to_owned();
                Ok(result)
            }
            None => Err(FCEError::NoSuchFunction(format!(
                "{} hasn't been found while calling",
                function_name
            ))),
        }
    }

    pub fn get_func_signature(
        &self,
        function_name: &str,
    ) -> Result<(&Vec<IType>, &Vec<IType>), FCEError> {
        match self.exports_funcs.get(function_name) {
            Some(func) => Ok((&func.inputs, &func.outputs)),
            None => Err(FCEError::NoSuchFunction(format!(
                "{} has't been found during its signature looking up",
                function_name
            ))),
        }
    }

    fn instantiate_wit_exports(
        wit: &FCEWITInterfaces<'_>,
    ) -> Result<HashMap<String, WITModuleFunc>, FCEError> {
        use fce_wit_interfaces::WITAstType;

        wit
            .implementations()
            .filter_map(|(adapter_function_type, core_function_type)|
                match wit.export_by_type(*core_function_type) {
                    Some(export_function_name) => Some((adapter_function_type, *export_function_name)),
                    // pass functions that aren't export
                    None => None
                }
            )
            .map(|(adapter_function_type, export_function_name)| {
                let adapter_instructions = wit.adapter_by_type(*adapter_function_type)
                    .ok_or_else(|| FCEError::IncorrectWIT(
                        format!("adapter function with idx = {} hasn't been found during extracting exports by implementations", adapter_function_type)
                    ))?;

                let wit_type = wit.type_by_idx(*adapter_function_type).ok_or_else(
                // TODO: change error type
                || FCEError::IncorrectWIT(format!(
                    "{} function id is bigger than WIT interface types count",
                    adapter_function_type
                )))?;

                match wit_type {
                    WITAstType::Function { inputs, outputs, .. } => {
                        let interpreter: WITInterpreter = adapter_instructions.try_into().map_err(|_| FCEError::IncorrectWIT(
                            format!("failed to parse instructions for adapter type {}", adapter_function_type)
                        ))?;

                        Ok((
                            export_function_name.to_string(),
                            WITModuleFunc {
                                interpreter,
                                inputs: inputs.clone(),
                                outputs: outputs.clone(),
                            }
                        ))
                    },
                    _ => Err(FCEError::IncorrectWIT(format!(
                        "type with idx = {} isn't a function type",
                        adapter_function_type
                    )))
                }
            })
            .collect::<Result<HashMap<String, WITModuleFunc>, FCEError>>()
    }

    // this function deals only with import functions that have an adaptor implementation
    fn adjust_wit_imports(
        wit: &FCEWITInterfaces<'_>,
        wit_instance: Arc<MaybeUninit<WITInstance>>,
    ) -> Result<ImportObject, FCEError> {
        use fce_wit_interfaces::WITAstType;
        use super::type_converters::{itype_to_wtype, wval_to_ival};
        use wasmer_core::typed_func::DynamicFunc;
        use wasmer_core::types::FuncSig;
        use wasmer_core::vm::Ctx;

        // returns function that will be called from imports of Wasmer module
        fn dyn_func_from_imports(
            inputs: Vec<IType>,
            func: Box<dyn Fn(&mut Ctx, &[WValue]) -> Vec<WValue> + 'static>,
        ) -> DynamicFunc<'static> {
            let signature = inputs.iter().map(itype_to_wtype).collect::<Vec<_>>();
            DynamicFunc::new(Arc::new(FuncSig::new(signature, vec![])), func)
        }

        fn create_raw_import(
            wit_instance: Arc<MaybeUninit<WITInstance>>,
            interpreter: WITInterpreter,
        ) -> Box<dyn for<'a, 'b> Fn(&'a mut Ctx, &'b [WValue]) -> Vec<WValue> + 'static> {
            Box::new(move |_: &mut Ctx, inputs: &[WValue]| -> Vec<WValue> {
                // copy here because otherwise wit_instance will be consumed by the closure
                let wit_instance_callable = wit_instance.clone();
                let converted_inputs = inputs.iter().map(wval_to_ival).collect::<Vec<_>>();
                unsafe {
                    // error here will be propagated by the special error instruction
                    let _ = interpreter.run(
                        &converted_inputs,
                        Arc::make_mut(&mut wit_instance_callable.assume_init()),
                    );
                }

                // wit import functions should only change the stack state -
                // the result will be returned by an export function
                vec![]
            })
        }

        let namespaces = wit
            .implementations()
            .filter_map(|(adapter_function_type, core_function_type)|
                match wit.import_by_type(*core_function_type) {
                    Some(import) => Some((adapter_function_type, *import)),
                    // pass functions that aren't export
                    None => None
                }
            )
            .map(|(adapter_function_type, (import_namespace, import_name))| {
                let adapter_instructions = wit.adapter_by_type(*adapter_function_type)
                    .ok_or_else(|| FCEError::IncorrectWIT(
                        format!("adapter function with idx = {} hasn't been found during extracting adjusting wit imports", adapter_function_type)
                    ))?;

                let wit_type = wit.type_by_idx(*adapter_function_type).ok_or_else(
                    || FCEError::IncorrectWIT(format!(
                        "{} function id is bigger than WIT interface types count",
                        adapter_function_type
                    )))?;

                match wit_type {
                    WITAstType::Function { inputs, .. } => {
                        let interpreter: WITInterpreter = adapter_instructions.try_into().map_err(|_| FCEError::IncorrectWIT(
                            format!("failed to parse instructions for adapter type {}", adapter_function_type)
                        ))?;

                        let inner_import = create_raw_import(wit_instance.clone(), interpreter);
                        let wit_import = dyn_func_from_imports(inputs.clone(), inner_import);
                        let mut namespace = Namespace::new();
                        namespace.insert(import_name, wit_import);

                        Ok((
                            import_namespace.to_string(),
                            namespace
                        ))
                    },
                    _ => Err(FCEError::IncorrectWIT(format!(
                        "type with idx = {} isn't a function type",
                        adapter_function_type
                    )))
                }
            }).collect::<Result<HashMap<String, Namespace>, FCEError>>()?;

        let mut import_object = ImportObject::new();

        for (namespace_name, namespace) in namespaces {
            import_object.register(namespace_name, namespace);
        }

        Ok(import_object)
    }
}