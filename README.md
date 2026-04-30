# Light Language (MVP Foundation)

This repository implements a custom programming language with support for AI and embedded systems.

## Implemented Phases (1-12 + Language Features)

1. **阶段1：解释器** - `light run` executes `main` through an interpreter.
2. **阶段2：AST + JSON** - `light ast --json` outputs stable JSON AST.
3. **阶段3：语义分析** - `light check` validates undefined variables and basic call arity.
4. **阶段4：后端起步** - `light build` lowers AST into a textual Light IR.
5. **阶段5：Linux 构建落盘** - `light build` emits `.elf/.bin/.hex` artifacts.
6. **阶段6：ARM64/多目标构建目录** - build outputs are namespaced per target (`build/<target>/...`) including `aarch64-linux`.
7. **阶段7：no_std 约束检查** - `--no-std` build paths reject forbidden APIs (`std.fs`, `std.net`) and dynamic allocation patterns.
8. **阶段8：STM32 固件** - `light build` supports `stm32f407`, `stm32f103` with linker scripts.
9. **阶段9：ESP32 固件** - `esp32-none-elf` target with `esp32.ld` linker script.
10. **阶段10：烧录工具** - `light flash` supports STLink, JLink, OpenOCD, esptool, serial.
11. **阶段11：HAL** - `use` statements, HAL builtins (`gpio.pin`, `sleep_ms`, etc.).
12. **阶段12：AI 推理支持** - `use std.onnx`, `use std.camera` (stub).

## Language Features

- **Variables**: `let x = 10`
- **Assignment**: `x = x + 1`
- **Functions**: `fn add(a, b) -> i32 { return a + b }`
- **Control Flow**: `if/else`, `while` loops, `for` loops
- **Comparison**: `==`, `!=`, `<`, `<=`, `>`, `>=`
- **Arithmetic**: `+`, `-`, `*`, `/`
- **Strings**: Concatenation (`+`), equality (`==`, `!=`)
- **Arrays**: `let arr = [1, 2, 3]`, `arr[0]`
- **Structs**: `struct Point { x: i32, y: i32 }`
- **Enums**: `enum Color { Red, Green, Blue }`
- **Comments**: `// single-line`, `/* multi-line */`
- **HAL Builtins**: `gpio.pin()`, `sleep_ms()`, etc.
- **Error Reporting**: Source spans with line:column location

## Quick start

```bash
# Run
cargo run -- run examples/hello.light
cargo run -- run examples/control_flow.light

# Parse and check
cargo run -- ast examples/hello.light --json
cargo run -- check examples/hello.light
cargo run -- fmt examples/hello.light

# Build for Linux
cargo run -- build examples/hello.light --target x86_64-linux
cargo run -- build examples/hello.light --target aarch64-linux

# Build for embedded
cargo run -- build examples/hello.light --target stm32f407 --no-std
cargo run -- build examples/hello.light --target thumbv7em-none-eabihf --no-std
cargo run -- build examples/hello.light --target esp32-none-elf --no-std

# Firmware build with linker script
cargo run -- firmware build examples/hello.light --target stm32f407
cargo run -- firmware build examples/hello.light --target stm32f407 --linker linker/stm32f407.ld

# Flash firmware
cargo run -- flash build/firmware/stm32f407/main.bin --target stm32f407 --interface stlink
cargo run -- flash build/firmware/stm32f407/main.bin --target stm32f407 --interface jlink

# List targets and interfaces
cargo run -- targets list
cargo run -- interfaces list
```

## Supported Targets

```
x86_64-linux
aarch64-linux
armv7-linux
riscv64-linux
thumbv7em-none-eabihf
riscv32imac-none-elf
esp32-none-elf
stm32f407
stm32f103
```

## Supported Flash Interfaces

```
stlink, st    - STLink (STM32)
jlink, jl     - JLink (ARM Cortex)
esptool, esp  - esptool.py (ESP32)
openocd, ocd  - OpenOCD (multi-vendor)
serial, uart  - Serial/UART bootloader
```

## HAL Builtins

```light
use std.hw.gpio

fn main() {
    let led = gpio.pin(13)
    gpio.high(led)
    sleep_ms(500)
    gpio.low(led)
}
```

## Examples

See `examples/` directory:
- `demo.light` - Complete demo with all language features
- `hello.light` - Basic function and print
- `control_flow.light` - If/else, while loops, comparisons
- `string_test.light` - String operations
- `gpio.light` - GPIO HAL demo
- `led_blink.light` - LED blink demo
- `ai.light` - AI inference stub
- `led.light` - Simple LED demo

## VS Code Extension

A VS Code extension for Light language is available in `vscode-extension/`.

**Features**:
- Syntax highlighting for `.light` files
- Keywords: `fn`, `let`, `return`, `if`, `else`, `while`, `use`
- String, number, and operator highlighting
- Comment support (`//`)

**Installation**:

1. **Copy to VS Code extensions folder**:
   ```bash
   # Windows
   copy vscode-extension %USERPROFILE%\.vscode\extensions\

   # Linux/macOS
   cp -r vscode-extension ~/.vscode/extensions/
   ```

2. **Build .vsix package** (requires Node.js):
   ```bash
   npm install -g @vscode/vsce
   cd vscode-extension
   vsce package
   # Install: code --install-extension light-language-0.1.0.vsix
   ```

## Not Yet Supported

The following features are planned but not yet implemented:

- Match/case expressions (structs/enums with qualified names)
- Multi-file modules (beyond `use` statements)
- Generic functions
- Closures/anonymous functions
- Error handling (try/catch)
- File import/export modules

## Implementation Roadmap

### Phase 1: Core Language (v0.2.0) ✅ Complete
- [x] Multi-line comments (`/* ... */`)
- [x] For loops
- [x] Arrays: `let arr = [1, 2, 3]`
- [x] Array indexing: `arr[0]`

### Phase 2: Advanced Types (v0.3.0) ✅ Partial
- [x] Struct definitions
- [x] Enum definitions
- [ ] Struct/Enum instantiation with qualified names (Color::Red)
- [ ] Tuple types
- [ ] Pattern matching (match/case)

### Phase 3: Functions (v0.4.0)
- [ ] Generic functions: `fn identity<T>(x: T) -> T`
- [ ] Closures/anonymous functions
- [ ] Higher-order functions

### Phase 4: Error Handling (v0.5.0)
- [ ] Result types
- [ ] try/catch error handling
- [ ] Option type with None

### Phase 5: Modules (v0.6.0)
- [ ] Multi-file projects
- [ ] Module imports from files
- [ ] Standard library
- [ ] Package manager

### Phase 6: Tooling (v0.7.0)
- [ ] Language Server Protocol (LSP)
- [ ] Debugger support
- [ ] REPL
- [ ] Better IDE integration

### Phase 7: Advanced Features (v0.8.0)
- [ ] Traits/interfaces
- [ ] Async/await
- [ ] Macro system
- [ ] Metaprogramming

## Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for development setup and guidelines.

## License

This project is licensed under the MIT License - see [LICENSE](LICENSE) for details.