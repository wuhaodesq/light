# Light Language

## AI + Embedded Native Programming Language (Full MVP)

---

## 项目定位

Light 是一门新一代编程语言，目标是：

```text
AI 友好 + 高性能 + 内存安全 + 可运行于硬件
```

不仅用于 AI 应用开发，还可直接编译为：

* 本地程序（Linux / Windows）
* 边缘设备（Jetson / ARM）
* 嵌入式固件（STM32 / ESP32 / RISC-V）

---

## 核心目标

Light 需要同时满足：

### AI 友好

* 稳定 AST（JSON 输出）
* 机器可读错误
* 强约束语法（低歧义）
* 官方格式化工具

---

### 高性能

* 编译执行（非纯解释）
* LLVM / Cranelift 后端
* 接近 C / Rust 性能

---

### 内存安全

* 默认无 GC
* 所有权 + 生命周期（简化版）
* unsafe 显式标记

---

### 硬件支持

* 支持裸机（no_std）
* 支持固件构建
* 支持直接烧录硬件
* 提供 HAL 标准库

---

## 技术栈

* 编译器：Rust
* 解析器：手写递归下降
* AST：自定义结构 + serde
* 后端：

  * MVP：解释器
  * 中期：Cranelift
  * 最终：LLVM
* CLI：clap

---

## 项目结构

```text
light/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── lexer/
│   ├── parser/
│   ├── ast/
│   ├── semantic/
│   ├── interpreter/
│   ├── backend/
│   ├── diagnostics/
│   ├── formatter/
│   ├── hal/
│   └── utils/
└── examples/
    ├── hello.light
    ├── led.light
    └── ai.light
```

---

## 示例代码

### 基础示例

```light
fn main() {
    print("Hello, Light")
}
```

---

### 函数示例

```light
fn add(a: i32, b: i32) -> i32 {
    return a + b
}
```

---

### 硬件示例（GPIO）

```light
use std.hw.gpio

fn main() {
    let led = gpio.pin(13)
    led.output()

    while true {
        led.high()
        sleep_ms(500)
        led.low()
        sleep_ms(500)
    }
}
```

---

### AI 推理示例

```light
use std.onnx
use std.camera

fn main() {
    let cam = camera.open(0)
    let model = onnx.load("drone.onnx")

    while true {
        let frame = cam.read()
        let result = model.run(frame)

        if result.has_object("drone") {
            print("detected")
        }
    }
}
```

---

## CLI 命令

```bash
light run main.light
light ast main.light --json
light check main.light
light fmt main.light

light build main.light
light build main.light --target aarch64-linux
light build main.light --target stm32f407 --no-std

light firmware build main.light --target stm32f407
light flash build/firmware/main.bin --target stm32 --interface stlink

light targets list
```

---

## 支持的 Target

```text
x86_64-linux
aarch64-linux
armv7-linux
riscv64-linux
thumbv7em-none-eabihf
riscv32imac-none-elf
esp32-none-elf
```

---

## 编译输出

```text
.elf
.bin
.hex
```

---

## 编译流程

```text
源代码
 → Lexer
 → Parser
 → AST
 → 语义分析
 → IR
 → 后端（解释器 / LLVM）
 → 可执行文件 / 固件
```

---

## Lexer 设计

```rust
pub enum Token {
    Fn,
    Let,
    Return,
    If,
    Else,
    Identifier(String),
    Number(f64),
    String(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Arrow,
}
```

---

## AST 设计

```rust
#[derive(Serialize)]
pub enum Expr {
    Number(f64),
    String(String),
    Identifier(String),
    Binary(Box<Expr>, String, Box<Expr>),
}

#[derive(Serialize)]
pub enum Stmt {
    Let(String, Expr),
    Return(Expr),
    Expr(Expr),
}

#[derive(Serialize)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Serialize)]
pub struct Program {
    pub functions: Vec<Function>,
}
```

---

## AI 核心能力

### AST 输出

```bash
light ast main.light --json
```

---

### 机器可读错误

```json
{
  "error": "UNDEFINED_VARIABLE",
  "message": "variable x not found",
  "line": 10
}
```

---

### 代码格式化

```bash
light fmt main.light
```

---

## 硬件支持

---

### 构建固件

```bash
light firmware build main.light --target stm32f407
```

输出：

```text
build/firmware/main.elf
build/firmware/main.bin
build/firmware/main.hex
```

---

### 烧录固件

#### STM32

```bash
light flash main.bin --target stm32 --interface stlink
```

#### ESP32

```bash
light flash main.bin --target esp32 --port COM3
```

---

## HAL 标准库

```text
std.hw.gpio
std.hw.uart
std.hw.spi
std.hw.i2c
std.hw.pwm
std.hw.adc
std.hw.timer
std.hw.interrupt
```

---

## 裸机模式

```bash
light build main.light --target stm32f407 --no-std
```

限制：

```text
禁止 std.fs
禁止 std.net
禁止动态分配
```

---

## 内存模型

```light
static BUFFER: [u8; 1024]

let data: [u8; 64]

arena frame {
    let buf = arena.alloc<u8>(256)
}
```

---

## 寄存器访问

```light
use std.hw.mmio

unsafe {
    let reg = mmio.ptr<u32>(0x40020000)
    reg.write(1)
}
```

---

## 中断支持

```light
interrupt TIM2 {
    led.toggle()
}
```

---

## Linker Script

```bash
light build main.light --linker stm32.ld
```

---

## 实现阶段

```text
阶段1：解释器
阶段2：AST + JSON
阶段3：语义分析
阶段4：LLVM 后端
阶段5：Linux 编译
阶段6：ARM64 编译
阶段7：no_std
阶段8：STM32 固件
阶段9：ESP32 固件
阶段10：烧录工具
阶段11：HAL
阶段12：AI 推理支持
```

---

## 验收标准

必须满足：

* AST 可解析
* JSON AST 可输出
* 程序可运行
* 错误结构化
* CLI 可用
* 可编译 ELF/BIN/HEX
* 可写入硬件
* GPIO 可控制
* 可在 ARM / MCU 运行

---

## 强约束

```text
必须手写 Parser
必须有 AST
禁止跳过语义分析
必须支持 JSON AST
必须支持 no_std
必须支持硬件编译
```

---

## 最终目标

```text
Light = AI 原生 + 系统级 + 可嵌入硬件 的编程语言
```

---

## 执行说明

请严格按阶段顺序实现，每一步完成后再进入下一阶段，不允许跳跃开发。