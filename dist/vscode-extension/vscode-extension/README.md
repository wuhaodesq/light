# Light Language VS Code Extension

## Features

- Syntax highlighting for Light language
- Support for `.light` file extension
- Comments (`//`)
- Keyword highlighting (`fn`, `let`, `return`, `if`, `else`, `while`, `use`)
- String highlighting
- Number highlighting
- Operator highlighting

## Installation

### From Source

1. Copy this folder to `.vscode/extensions/` folder
2. Restart VS Code

Or use VS Code's "Install from VSIX" command.

### Build .vsix Package

```bash
npm install -g vsce
vsce package
```

## File Types

- `.light` - Light source files

## Example

```light
fn main() {
    let x = 10
    if x > 5 {
        print("x is greater than 5")
    }
}
```

## License

MIT
