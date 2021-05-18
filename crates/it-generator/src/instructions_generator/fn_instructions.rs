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

mod args_it_generator;
mod output_type_it_generator;

use super::ITGenerator;
use super::ITResolver;
use super::utils::*;
use crate::Result;
use crate::default_export_api_config::RELEASE_OBJECTS;

use marine_macro_impl::FnType;
use wasmer_it::interpreter::Instruction;

impl ITGenerator for FnType {
    fn generate_it<'a>(&'a self, it_resolver: &mut ITResolver<'a>) -> Result<()> {
        generate_it_types(self, it_resolver)?;
        generate_instructions(self, it_resolver)
    }
}

fn generate_it_types<'f>(fn_type: &'f FnType, it_resolver: &mut ITResolver<'f>) -> Result<()> {
    use wasmer_it::ast::Type;

    let arguments = generate_it_args(&fn_type.signature, it_resolver)?;
    let output_types = generate_it_output_type(&fn_type.signature, it_resolver)?;

    let interfaces = &mut it_resolver.interfaces;
    interfaces.types.push(Type::Function {
        arguments: arguments.clone(),
        output_types: output_types.clone(),
    });

    // TODO: replace with Wasm types
    interfaces.types.push(Type::Function {
        arguments,
        output_types,
    });

    let export_idx = (interfaces.types.len() - 1) as u32;

    interfaces.exports.push(wasmer_it::ast::Export {
        name: &fn_type.signature.name,
        function_type: export_idx,
    });

    Ok(())
}

fn generate_instructions<'f>(fn_type: &'f FnType, it_resolver: &mut ITResolver<'f>) -> Result<()> {
    use args_it_generator::ArgumentITGenerator;
    use output_type_it_generator::OutputITGenerator;
    use wasmer_it::ast::Adapter;

    let mut instructions = fn_type
        .signature
        .arguments
        .iter()
        .enumerate()
        .try_fold::<_, _, Result<_>>(Vec::new(), |mut instructions, (arg_id, arg)| {
            let new_instructions = arg
                .ty
                .generate_instructions_for_arg(arg_id as _, it_resolver)?;

            instructions.extend(new_instructions);
            Ok(instructions)
        })?;

    let export_function_index = (it_resolver.interfaces.exports.len() - 1) as u32;
    instructions.push(Instruction::CallCore {
        function_index: export_function_index,
    });

    let mut should_generate_release = false;
    let mut instructions = fn_type
        .signature
        .output_types
        .iter()
        .try_fold::<_, _, Result<_>>(instructions, |mut instructions, ty| {
            let new_instructions = ty.generate_instructions_for_output_type(it_resolver)?;
            instructions.extend(new_instructions);

            should_generate_release |= ty.is_complex_type();
            Ok(instructions)
        })?;

    if should_generate_release {
        instructions.push(Instruction::CallCore {
            function_index: RELEASE_OBJECTS.id,
        });
    }

    let interfaces = &mut it_resolver.interfaces;
    let adapter_idx = (interfaces.types.len() - 2) as u32;
    let export_idx = (interfaces.types.len() - 1) as u32;

    let adapter = Adapter {
        function_type: adapter_idx,
        instructions,
    };

    interfaces.adapters.push(adapter);

    let implementation = wasmer_it::ast::Implementation {
        core_function_type: export_idx,
        adapter_function_type: adapter_idx,
    };

    interfaces.implementations.push(implementation);

    Ok(())
}
