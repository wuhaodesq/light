# Light 语言语法规范

> 版本: 0.1.0

## 概述

Light 是一种简单、静类型的编程语言，专为嵌入式系统和 AI 应用设计。本文档描述 Light 编译器支持的语法。

---

## 注释

```light
// 单行注释
let x = 10  // 行内注释

/* 多行注释
   跨多行 */
```

单行注释以 `//` 开头，结束于行尾。
多行注释以 `/*` 开头，`*/` 结尾。

---

## 关键字

```
fn      let     return   if      else     while    use
```

---

## 数据类型

### 数字

```light
let integer = 42      // 整数
let float = 3.14      // 浮点数
let negative = -10    // 负数
```

数字内部使用 IEEE 754 双精度浮点数表示。

### 字符串

```light
let greeting = "你好，世界!"
let with_escape = "第一行\n第二行"
let quote = "他说 \"你好\""
```

支持的转义序列：
- `\n` - 换行
- `\t` - 制表符
- `\"` - 双引号
- `\\` - 反斜杠

### 数组

```light
let numbers = [1, 2, 3, 4, 5]
let first = numbers[0]    // 数组索引
let length = 5
```

数组是固定大小的值集合。使用从零开始的索引访问元素。

### Unit 类型

```light
let empty = ()
```

Unit 类型表示无返回值。

### GPIO 引脚

```light
let led = gpio.pin(13)
```

GPIO 引脚通过 HAL 内置函数创建。

---

## 变量

### 声明

```light
let x = 10
let name = "张三"
let value = 3.14
```

变量使用 `let` 声明，必须初始化。

### 赋值

```light
let x = 10
x = 20        // 重新赋值
x = x + 5    // 表达式赋值
```

赋值使用 `=` 运算符。

---

## 函数

### 定义

```light
fn add(a, b) -> i32 {
    return a + b
}
```

函数使用 `fn` 关键字定义，后跟：
- 圆括号中的参数列表
- `->` 后的返回类型
- 花括号中的函数体

### 返回

```light
fn max(a, b) -> i32 {
    if a > b {
        return a
    }
    return b
}
```

使用 `return` 返回函数值。

### 参数

```light
fn greet(name, age) -> i32 {
    print("你好，" + name)
    return age
}
```

多个参数用逗号分隔。

---

## 控制流

### 条件语句

```light
if 条件 {
    // 代码
} else {
    // 代码
}
```

```light
if 分数 >= 90 {
    print("优秀")
} else if 分数 >= 80 {
    print("良好")
} else {
    print("及格")
}
```

### 循环

```light
let i = 0
while i < 10 {
    print(i)
    i = i + 1
}
```

### For 循环

```light
for (let i = 0; i < 5; i + 1) {
    print("i = " + i)
    i = i + 1
}
```

for 循环有三个部分：初始化器、条件、增量。
- 初始化器在开始时执行一次
- 条件在每次迭代前检查
- 增量在每次迭代后执行

---

---

## 运算符

### 算术运算

```light
let sum = 10 + 5      // 加法
let diff = 10 - 5      // 减法
let prod = 10 * 5      // 乘法
let quot = 10 / 5      // 除法
```

### 比较运算

```light
10 == 10    // 等于
10 != 5     // 不等于
5 < 10      // 小于
5 <= 5      // 小于等于
10 > 5      // 大于
10 >= 10    // 大于等于
```

### 逻辑运算

```light
if x > 0 && y > 0 { }   // 且
if x > 0 || y > 0 { }   // 或
```

当前逻辑运算符通过条件判断表达式链实现。

---

## 表达式

### 代码块表达式

```light
let result = {
    let x = 10
    let y = 20
    x + y
}
```

### 函数调用

```light
print("你好")
add(1, 2)
gpio.high(led)
```

### 字符串拼接

```light
let full = "你好" + " " + "世界"
```

---

## 内置函数

### print

```light
print("你好")
print(x)
print("值: " + x)
```

打印值到标准输出。

---

## HAL 内置函数

### GPIO

```light
use std.hw.gpio

let led = gpio.pin(13)    // 创建 GPIO 引脚
gpio.high(led)            // 设置引脚高电平
gpio.low(led)             // 设置引脚低电平
```

### 定时

```light
sleep_ms(1000)            // 延时 1000 毫秒
```

---

## 导入语句

```light
use std.hw.gpio    // 导入 GPIO 硬件模块
use std.onnx       // 导入 ONNX AI 推理模块
use std.camera     // 导入摄像头模块
```

导入外部模块或 HAL 组件。

---

## 完整示例

```light
use std.hw.gpio

fn factorial(n) -> i32 {
    let result = 1
    let i = 1
    while i <= n {
        result = result * i
        i = i + 1
    }
    return result
}

fn main() {
    let x = 10
    let y = 20

    if x < y {
        print("x 小于 y")
    }

    let count = 0
    while count < 5 {
        print("计数: " + count)
        count = count + 1
    }

    let led = gpio.pin(13)
    gpio.high(led)

    print("5 的阶乘: " + factorial(5))
}
```

---

## 语法摘要

```
程序       ::= 语句*
语句       ::= "let" 标识符 "=" 表达式
            |  标识符 "=" 表达式
            |  "if" 表达式 "{" 语句* "}" ("else" "{" 语句* "}")?
            |  "while" 表达式 "{" 语句* "}"
            |  "for" "(" 语句? ";" 表达式? ";" 表达式? ")" "{" 语句* "}"
            |  "fn" 标识符 "(" 参数? ")" "->" 类型 "{" 语句* "}"
            |  "return" 表达式
            |  "use" 路径
            |  表达式

表达式     ::= 字符串
            |  数字
            |  标识符
            |  "[" 表达式 ("," 表达式)* "]"
            |  标识符 "(" 参数? ")"
            |  标识符 "[" 表达式 "]"
            |  表达式 运算符 表达式
            |  "(" 表达式 ")"

运算符     ::= "+" | "-" | "*" | "/" | "==" | "!=" | "<" | "<=" | ">" | ">="
```

---

## 暂不支持的功能

以下功能尚未实现：

- 自定义结构体/类
- match/case 表达式
- 多文件模块 (除 `use` 语句外)
- 泛型函数
- 闭包/匿名函数
- 错误处理 (try/catch)
- 文件导入/导出模块
