#!/bin/sh

# This script builds all subprojects and puts all created Wasm modules in one dir
(
  cd effector || exit;
  cargo update --aggressive;
  marine build --release;
)

(
  cd pure || exit;
  cargo update --aggressive;
  marine build --release;
)

rm artifacts/* || true
mkdir -p artifacts

cp ../../target/wasm32-wasi/release/ipfs_effector.wasm artifacts/
cp ../../target/wasm32-wasi/release/ipfs_pure.wasm artifacts/
