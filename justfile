#!/usr/bin/env -S just --justfile

test:
  cargo test --bins --lib & cargo test --doc

integration-test:
  cargo test --test live_integration_tests

doc:
  cargo +nightly rustdoc --all-features -- --cfg docsrs
