# guitar-tab-generator

[![Build + Test](https://github.com/noahbaculi/guitar-tab-generator/actions/workflows/rust_build_and_test.yml/badge.svg)](https://github.com/noahbaculi/guitar-tab-generator/actions/workflows/rust_build_and_test.yml)
[![Coverage](https://codecov.io/gh/noahbaculi/guitar-tab-generator/branch/main/graph/badge.svg?token=BB01PPL4LF)](https://codecov.io/gh/noahbaculi/guitar-tab-generator)

Guitar tab generator from note names considering difficulty of different finger positions.

Old versions:

- [Java](https://github.com/noahbaculi/guitar-tab-generator_java) (2019 - 2022)
- [Typescript](https://github.com/noahbaculi/guitar-tab-generator_typescript) (2022)

Commands:

```shell
# Background code checker
bacon

# Calculate code coverage
cargo tarpaulin --exclude-files src/main.rs --out Html
cargo llvm-cov --ignore-filename-regex src/main.rs --open

# Build WASM binary using [wasm-pack](https://rustwasm.github.io/docs/wasm-pack/introduction.html) and [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/introduction.html)
wasm-pack build --target web
ls -l pkg\guitar_tab_generator_bg.wasm  # get size in bytes
```

Running To-Dos:

- [ ] re-examine namespace of functions (object functions vs standalone) (public vs private)
- [ ] code coverage badge
- [ ] handle measure breaks and commented lines and test
- [ ] `let non_zero_fret_avg = non_zero_frets.iter().sum::<usize>() as f32 / non_zero_frets.len() as f32;`
- [ ] filter unplayable fingering options from beat_fingering_candidates (based on the fret span and whether there are any candidates with smaller fret spans)
- [ ] [pathfinding](https://docs.rs/pathfinding/latest/pathfinding/)
- [ ] [property testing](https://altsysrq.github.io/proptest-book/)
- [ ] benchmarking via [Criterion](https://crates.io/crates/criterion)
- [ ] borrowed types vs box vs RC
- [ ] [Rayon](https://docs.rs/rayon/latest/rayon/#how-to-use-rayon) parallelism
- [ ] 