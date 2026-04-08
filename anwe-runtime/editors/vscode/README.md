# ANWE Language Support for VS Code

Syntax highlighting for the **ANWE** (Autonomous Agent Neural Weave) language.

## Features

- Full syntax highlighting for `.anwe` files
- Bracket matching and auto-closing
- Comment toggling with `Ctrl+/` (uses `--` line comments)
- Code folding for blocks

### Highlighted constructs

- **Declarations**: `agent`, `mind`, `link`, `pattern`, `supervisor`, `record`, `fn`
- **Cognitive primitives**: `think`, `sense`, `express`, `author`, `attend`
- **Actions**: `alert`, `connect`, `sync`, `apply`, `commit`, `reject`, `converge`
- **Signals & temporal**: `signal`, `emit`, `when`, `on`, `every`, `after`, `continuous`
- **Control flow**: `if`, `else`, `match`, `while`, `for`, `in`, `break`, `continue`, `return`
- **Error handling**: `try`, `catch`, `attempt`, `recover`
- **Variables**: `let`, `let mut`
- **Modules**: `import`, `from`, `as`
- **Link operators**: `<->` (bidirectional), `<-` (assignment), `->` (arrow), `~` (sync)
- **Signal qualities**: `recognizing`, `questioning`, `applying`, `completing`, `connecting`, `alerting`
- **Agent states**: `idle`, `alert`, `connected`, `synced`, `committed`, `rejected`, `converged`
- **Strings**: `"double quoted"` and `f"interpolated {expressions}"`
- **Comments**: `-- single line comments`

## Installation

### Option 1: Symlink (recommended for development)

Create a symbolic link from the VS Code extensions directory to this folder:

```bash
# Linux
ln -s /path/to/anwe-runtime/editors/vscode ~/.vscode/extensions/anwe

# macOS
ln -s /path/to/anwe-runtime/editors/vscode ~/.vscode/extensions/anwe

# Windows (PowerShell, run as Administrator)
New-Item -ItemType SymbolicLink -Path "$env:USERPROFILE\.vscode\extensions\anwe" -Target "C:\path\to\anwe-runtime\editors\vscode"
```

Then restart VS Code (or run **Developer: Reload Window** from the command palette).

### Option 2: Copy the extension

Copy the entire `vscode/` directory into your VS Code extensions folder:

```bash
cp -r /path/to/anwe-runtime/editors/vscode ~/.vscode/extensions/anwe
```

Restart VS Code.

### Option 3: Package as VSIX

If you have `vsce` installed, you can package and install as a `.vsix`:

```bash
cd /path/to/anwe-runtime/editors/vscode
npx @vscode/vsce package
code --install-extension anwe-0.1.0.vsix
```

## Verifying the installation

1. Open any `.anwe` file in VS Code.
2. Check the language mode in the bottom status bar -- it should say **ANWE**.
3. You should see syntax highlighting for keywords, strings, numbers, comments, and operators.

## Theme compatibility

The grammar uses standard TextMate scope names that work well with popular themes including:

- One Dark Pro
- Dracula
- Monokai
- GitHub Theme
- Solarized
- Catppuccin
- Nord
