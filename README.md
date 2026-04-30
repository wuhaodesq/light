# Light Language (MVP Foundation)

This repository currently implements the first **seven phases** in order:

1. **阶段1：解释器** - `light run` executes `main` through an interpreter.
2. **阶段2：AST + JSON** - `light ast --json` outputs stable JSON AST.
3. **阶段3：语义分析** - `light check` validates undefined variables and basic call arity.
4. **阶段4：后端起步** - `light build` lowers AST into a textual Light IR.
5. **阶段5：Linux 构建落盘** - `light build` emits `.elf/.bin/.hex` artifacts.
6. **阶段6：ARM64/多目标构建目录** - build outputs are namespaced per target (`build/<target>/...`) including `aarch64-linux`.
7. **阶段7：no_std 约束检查** - `--no-std` build paths reject forbidden APIs (`std.fs`, `std.net`) and dynamic allocation patterns.

## Quick start

```bash
cargo run -- run examples/hello.light
cargo run -- ast examples/hello.light --json
cargo run -- check examples/hello.light
cargo run -- build examples/hello.light --target x86_64-linux
cargo run -- build examples/hello.light --target aarch64-linux
cargo run -- firmware build examples/hello.light --target thumbv7em-none-eabihf
cargo run -- targets list
```
