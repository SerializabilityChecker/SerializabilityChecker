#!/bin/bash

# SMPT Wrapper Script
# This script activates the SMPT virtual environment and runs SMPT

SMPT_DIR="PATH_TO_SMPT"
VENV_DIR="$SMPT_DIR/myenv"

if [ ! -d "$VENV_DIR" ]; then
    echo "Error: SMPT virtual environment not found at $VENV_DIR"
    echo "Please install SMPT."
    exit 1
fi

echo "$VENV_DIR"

# Activate virtual environment and run SMPT
cd "$SMPT_DIR"
source "$VENV_DIR/bin/activate"
python -m smpt "$@"

