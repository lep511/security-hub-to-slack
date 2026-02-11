#!/bin/bash

# Script to download SCPs from the official AWS repository
# https://github.com/aws-samples/service-control-policy-examples

set -e

REPO_URL="https://github.com/aws-samples/service-control-policy-examples"
TEMPLATES_DIR="./scp-templates"
TEMP_DIR="/tmp/scp-examples-$$"

echo "📦 Downloading SCP templates from AWS repository..."

# Create temporary directory
mkdir -p "$TEMP_DIR"

# Clone repository
git clone --depth 1 "$REPO_URL" "$TEMP_DIR"

# Create directory structure
mkdir -p "$TEMPLATES_DIR"

echo "📁 Organizing templates by category..."

# Function to copy and organize files
organize_templates() {
    local source_dir="$1"
    local category="$2"
    
    if [ -d "$source_dir" ]; then
        mkdir -p "$TEMPLATES_DIR/$category"
        
        # Copy JSON files
        find "$source_dir" -name "*.json" -type f -exec cp {} "$TEMPLATES_DIR/$category/" \;
        
        # Count copied files
        count=$(find "$TEMPLATES_DIR/$category" -name "*.json" -type f | wc -l)
        echo "   ✅ $category: $count policies"
    fi
}

# Organize by common repository categories
organize_templates "$TEMP_DIR/Deny-changes-to-security-services" "security-services"
organize_templates "$TEMP_DIR/Require-encryption" "encryption"
organize_templates "$TEMP_DIR/Deny-leaving-orgs" "organizational-control"
organize_templates "$TEMP_DIR/Deny-root-user" "security"
organize_templates "$TEMP_DIR/Require-MFA" "security"
organize_templates "$TEMP_DIR/Deny-regions" "compliance"
organize_templates "$TEMP_DIR/Require-tagging" "governance"

# Find and copy any other JSON files
find "$TEMP_DIR" -name "*.json" -type f ! -path "$TEMPLATES_DIR/*" -exec sh -c '
    category=$(dirname "$1" | xargs basename | tr "[:upper:]" "[:lower:]" | sed "s/[^a-z0-9]/-/g")
    mkdir -p "'"$TEMPLATES_DIR"'/$category"
    cp "$1" "'"$TEMPLATES_DIR"'/$category/"
' _ {} \;

# Clean up
rm -rf "$TEMP_DIR"

# Count total
total=$(find "$TEMPLATES_DIR" -name "*.json" -type f | wc -l)

echo ""
echo "✅ Download completed!"
echo "📊 Total templates: $total"
echo "📁 Location: $TEMPLATES_DIR"
echo ""
echo "To use the generator:"
echo "  cargo run -- --templates-dir $TEMPLATES_DIR"