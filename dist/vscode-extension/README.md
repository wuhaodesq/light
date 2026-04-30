# Light Language VS Code Extension

## Features

- Syntax highlighting for Light language
- Support for `.light` file extension
- Comments (`//`)
- Keyword highlighting (`fn`, `let`, `return`, `if`, `else`, `while`, `use`)
- String highlighting
- Number highlighting
- Operator highlighting

## Adding an Icon

The extension icon must be a **PNG** file (128x128 or 256x256 recommended).

1. Create or obtain a 256x256 PNG icon named `icon.png`
2. Place it in the `vscode-extension/` folder
3. Add to `package.json`:
   ```json
   "icon": "./icon.png"
   ```

You can use online tools to convert SVG to PNG, or design your own icon.

## Installation

### From Source

1. Copy this folder to `.vscode/extensions/` folder
2. Restart VS Code

Or use VS Code's "Install from VSIX" command.

### Build .vsix Package

```bash
npm install -g @vscode/vsce
cd vscode-extension
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
