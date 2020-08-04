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

mod errors;
mod service;

pub(crate) type Result<T> = std::result::Result<T, ServiceError>;

pub use errors::ServiceError;
pub use service::FluenceFaaSService;

pub use fluence_faas::IValue;
pub use fluence_faas::IType;
pub use fluence_faas::FaaSInterface;
pub use fluence_faas::RawModulesConfig;
pub use fluence_faas::RawModuleConfig;
pub use fluence_faas::ModulesConfig;
pub use fluence_faas::ModuleConfig;
pub use fluence_faas::WASIConfig;