#!/bin/bash
set -e

# Install additional OCR languages if specified
if [ -n "$READUR_EXTRA_OCR_LANGUAGES" ]; then
    echo "Installing additional OCR languages: $READUR_EXTRA_OCR_LANGUAGES"

    IFS=',' read -ra LANGS <<< "$READUR_EXTRA_OCR_LANGUAGES"
    PACKAGES=()

    for lang in "${LANGS[@]}"; do
        # Trim whitespace
        lang=$(echo "$lang" | xargs)
        # Skip empty entries
        [ -z "$lang" ] && continue
        # Convert underscore to hyphen for package names (e.g., chi_sim -> chi-sim)
        pkg_lang=$(echo "$lang" | sed 's/_/-/g')
        PACKAGES+=("tesseract-ocr-$pkg_lang")
    done

    if [ ${#PACKAGES[@]} -gt 0 ]; then
        apt-get update
        if ! apt-get install -y "${PACKAGES[@]}"; then
            echo "Warning: Some language packages may not have been found"
        fi
        rm -rf /var/lib/apt/lists/*
    fi
fi

exec "$@"
