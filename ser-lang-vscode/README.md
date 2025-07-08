# Ser Language Extension for VSCode

This extension provides syntax highlighting for Ser language (`.ser`) files.

## Features

- Syntax highlighting for keywords, variables, operators, and literals
- Auto-closing of brackets and parentheses
- Code folding
- Distinction between local variables (lowercase) and global variables (uppercase)
- Support for "request" declarations
- Support for single-line comments with `//` syntax

## Installation

To install this extension:

```bash
./build-vsix.sh
code --install-extension ser-lang-*.vsix
```

Restart VSCode if necessary.

## Development

To make changes to this extension:

1. Edit the files in this directory
2. Reload the VSCode window (`Developer: Reload Window` command)