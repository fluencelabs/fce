/*
 * Copyright 2021 Fluence Labs Limited
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

use std::cell::Cell;

pub(crate) struct MemoryWriter<'m> {
    memory: &'m [Cell<u8>],
}

impl<'m> MemoryWriter<'m> {
    pub(crate) fn new(memory: &'m [Cell<u8>]) -> Self {
        Self { memory }
    }

    pub(crate) fn write_array<const N: usize>(&self, offset: usize, values: [u8; N]) {
        self.memory[offset..offset + N]
            .iter()
            .zip(values.iter())
            .for_each(|(cell, &byte)| cell.set(byte));
    }

    // specialization of write_array for u8
    pub(super) fn write_u8(&self, offset: usize, value: u8) {
        self.memory[offset].set(value);
    }

    // specialization of write_array for u32
    pub(super) fn write_u32(&self, offset: usize, value: u32) {
        let value = value.to_le_bytes();
        self.memory[offset].set(value[0]);
        self.memory[offset + 1].set(value[1]);
        self.memory[offset + 2].set(value[2]);
        self.memory[offset + 3].set(value[3]);
    }

    pub(super) fn write_bytes(&self, offset: usize, bytes: &[u8]) {
        self.memory[offset..offset + bytes.len()]
            .iter()
            .zip(bytes)
            .for_each(|(cell, &byte)| cell.set(byte));
    }
}
