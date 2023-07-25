# guitar-tab-generator

[![Build + Test](https://github.com/noahbaculi/guitar-tab-generator/actions/workflows/rust_build_and_test.yml/badge.svg)](https://github.com/noahbaculi/guitar-tab-generator/actions/workflows/rust_build_and_test.yml)
[![Coverage](https://codecov.io/gh/noahbaculi/guitar-tab-generator/branch/main/graph/badge.svg?token=BB01PPL4LF)](https://codecov.io/gh/noahbaculi/guitar-tab-generator)

Guitar tab generator from note names considering difficulty of different finger positions.

Old versions:

- [Java](https://github.com/noahbaculi/guitar-tab-generator_java) (2019 - 2022)
- [Typescript](https://github.com/noahbaculi/guitar-tab-generator_typescript) (2022)

Commands:

```shell
# Run code
cargo run --example hello
# Background code runner
bacon -- --example hello


# Background code checker
cargo clippy
# Background code checker
bacon

# Calculate code coverage
cargo tarpaulin --out Html --output-dir dev/tarpaulin-coverage
cargo llvm-cov --open

# Screen for potentially unused feature flags
unused-features analyze --report-dir 'dev/unused-features-report'
unused-features build-report --input 'dev/unused-features-report/report.json'

# Build WASM binary using [wasm-pack](https://rustwasm.github.io/docs/wasm-pack/introduction.html) and [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/introduction.html)
wasm-pack build --target web
ls -l pkg/guitar_tab_generator_bg.wasm  # get size in bytes
```

Running To-Dos:

- [ ] add filter for max_fret_span in `arrangements`
- [ ] re-examine namespace of functions (object functions vs standalone) (public vs private)
- [ ] filter unplayable fingering options from beat_fingering_candidates (based on the fret span and whether there are any candidates with smaller fret spans)
- [ ] [property testing](https://altsysrq.github.io/proptest-book/)
- [ ] borrowed types vs box vs RC
- [ ] [Rayon](https://docs.rs/rayon/latest/rayon/#how-to-use-rayon) parallelism
