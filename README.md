# Fluence Compute Engine

![fce version on crates.io](https://img.shields.io/crates/v/fce?color=green&style=flat-square)

FCE is a general purpose Wasm runtime that could be used in different scenarios, especially in programs based on the [ECS](https://en.wikipedia.org/wiki/Entity_component_system) pattern or plugin architecture. It runs multi-module WebAssembly applications with interface-types and shared-nothing linking scheme.

Fluence [nodes](https://github.com/fluencelabs/fluence) use FCE to execute [aquamarine](https://github.com/fluencelabs/aquamarine) and services compiled to Wasm:

<p align="center" width="100%">
    <img alt="fluence stack" align="center" src="images/fluence_stack_overview.png" width="663"/>
</p>

At now, it is in the heavily developing phase, more detailed information could be found in [docs](https://fluence-labs.readme.io/docs/services-development)
