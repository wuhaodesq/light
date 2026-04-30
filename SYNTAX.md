# Light Language Syntax Specification

> Version: 0.1.0

## Overview

Light is a simple, statically-typed programming language designed for embedded systems and AI applications. This document describes the syntax supported by the Light compiler.

---

## Comments

```light
// Single-line comment
let x = 10  // inline comment

/* Multi-line comment
   spanning multiple lines */
```

Single-line comments start with `//` and continue to the end of the line.
Multi-line comments are enclosed in `/*` and `*/`.

---

## Keywords

```
fn      let     return   if      else     while    use
```

---

## Data Types

### Numbers

```light
let integer = 42
let float = 3.14
let negative = -10
```

Numbers are IEEE 754 double-precision floating-point internally.

### Strings

```light
let greeting = "Hello, World!"
let with_escape = "Line1\nLine2"
let quote = "She said \"Hello\""
```

Supported escape sequences:
- `\n` - Newline
- `\t` - Tab
- `\"` - Double quote
- `\\` - Backslash

### Arrays

```light
let numbers = [1, 2, 3, 4, 5]
let first = numbers[0]    // Array indexing
let length = 5
```

Arrays are fixed-size collections of values. Access elements using zero-based index.

### Unit

```light
let empty = ()
```

The unit type represents the absence of a value.

### GPIO Pins

```light
let led = gpio.pin(13)
```

GPIO pins are created via HAL builtins.

---

## Variables

### Declaration

```light
let x = 10
let name = "Alice"
let value = 3.14
```

Variables are declared with `let` and must be initialized.

### Assignment

```light
let x = 10
x = 20        // reassignment
x = x + 5    // with expression
```

Assignment uses the `=` operator.

---

## Functions

### Declaration

```light
fn add(a, b) -> i32 {
    return a + b
}
```

Functions are declared with the `fn` keyword, followed by:
- Parameter list in parentheses
- Return type after `->`
- Body in curly braces

### Return

```light
fn max(a, b) -> i32 {
    if a > b {
        return a
    }
    return b
}
```

Use `return` to return a value from a function.

### Parameters

```light
fn greet(name, age) -> i32 {
    print("Hello, " + name)
    return age
}
```

Multiple parameters are separated by commas.

---

## Control Flow

### If/Else

```light
if condition {
    // code
} else {
    // code
}
```

```light
if score >= 90 {
    print("A")
} else if score >= 80 {
    print("B")
} else {
    print("C")
}
```

### While Loop

```light
let i = 0
while i < 10 {
    print(i)
    i = i + 1
}
```

### For Loop

```light
for (let i = 0; i < 5; i + 1) {
    print("i = " + i)
    i = i + 1
}
```

The for loop has three parts: initializer, condition, and increment.
- Initializer runs once at the start
- Condition is checked each iteration
- Increment runs after each iteration

---

---

## Operators

### Arithmetic

```light
let sum = 10 + 5      // Addition
let diff = 10 - 5     // Subtraction
let prod = 10 * 5     // Multiplication
let quot = 10 / 5     // Division
```

### Comparison

```light
10 == 10    // Equal
10 != 5     // Not equal
5 < 10      // Less than
5 <= 5      // Less than or equal
10 > 5      // Greater than
10 >= 10    // Greater than or equal
```

### Logical

```light
if x > 0 && y > 0 { }
if x > 0 || y > 0 { }
```

Currently, logical operators are expressed via comparison chains in conditionals.

---

## Expressions

### Block Expressions

```light
let result = {
    let x = 10
    let y = 20
    x + y
}
```

### Function Calls

```light
print("Hello")
add(1, 2)
gpio.high(led)
```

### String Concatenation

```light
let full = "Hello" + " " + "World"
```

---

## Built-in Functions

### print

```light
print("Hello")
print(x)
print("Value: " + x)
```

Prints values to stdout.

---

## HAL Builtins

### GPIO

```light
use std.hw.gpio

let led = gpio.pin(13)    // Create GPIO pin
gpio.high(led)            // Set pin high
gpio.low(led)             // Set pin low
```

### Timing

```light
sleep_ms(1000)            // Sleep for 1000 milliseconds
```

---

## Use Statements

```light
use std.hw.gpio
use std.onnx
use std.camera
```

Import external modules or HAL components.

---

## Complete Example

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
        print("x is less than y")
    }

    let count = 0
    while count < 5 {
        print("count: " + count)
        count = count + 1
    }

    let led = gpio.pin(13)
    gpio.high(led)

    print("Factorial of 5: " + factorial(5))
}
```

---

## Grammar Summary

```
program     ::= stmt*
stmt        ::= "let" IDENT "=" expr
            |  IDENT "=" expr
            |  "if" expr "{" stmt* "}" ("else" "{" stmt* "}")?
            |  "while" expr "{" stmt* "}"
            |  "fn" IDENT "(" params? ")" "->" TYPE "{" stmt* "}"
            |  "return" expr
            |  "use" path
            |  expr

expr        ::= STRING
            |  NUMBER
            |  IDENT
            |  "[" expr ("," expr)* "]"
            |  IDENT "(" args? ")"
            |  expr "[" expr "]"
            |  expr op expr
            |  "(" expr ")"

op          ::= "+" | "-" | "*" | "/" | "==" | "!=" | "<" | "<=" | ">" | ">="
```

---

## Not Supported (Yet)

The following features are NOT yet implemented:

- Custom structs/classes
- Match/case expressions
- Modules/files beyond `use` statements
- Generic functions
- Closures/anonymous functions
- Error handling (try/catch)
- Import/export modules from files
