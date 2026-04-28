# Light Language (MVP Foundation)

This repository now contains the first **three phases** implemented in order:

1. **阶段1：解释器** - `light run` executes `main` through an interpreter.
2. **阶段2：AST + JSON** - `light ast --json` outputs stable JSON AST.
3. **阶段3：语义分析** - `light check` validates undefined variables.

Later phases (LLVM backend, firmware toolchain integration, HAL richness, etc.) remain to be implemented.

## Quick start

```bash
cargo run -- run examples/hello.light
cargo run -- ast examples/hello.light --json
cargo run -- check examples/hello.light
cargo run -- targets list
```
