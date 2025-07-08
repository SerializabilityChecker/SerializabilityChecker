#!/bin/bash
set -e

# Move to the extension directory
cd "$(dirname "$0")"

# Clean up any previous VSIX files
echo "Removing previous VSIX files..."
rm -f ser-lang-*.vsix

# Install dependencies
npm install

# Package the extension
npm run package

# Get the version from package.json
VERSION=$(node -p "require('./package.json').version")
VSIX_FILE="ser-lang-${VERSION}.vsix"

# Install the extension
echo -e "\nInstalling the extension..."
code --install-extension "$VSIX_FILE"

echo -e "\nVSCode extension v${VERSION} has been built and installed successfully!"
echo "You may need to restart VSCode to see the changes."