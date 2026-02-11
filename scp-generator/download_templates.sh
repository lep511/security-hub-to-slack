#!/bin/bash

# Script para descargar SCPs del repositorio oficial de AWS
# https://github.com/aws-samples/service-control-policy-examples

set -e

REPO_URL="https://github.com/aws-samples/service-control-policy-examples"
TEMPLATES_DIR="./scp-templates"
TEMP_DIR="/tmp/scp-examples-$$"

echo "📦 Descargando templates de SCP desde el repositorio de AWS..."

# Crear directorio temporal
mkdir -p "$TEMP_DIR"

# Clonar repositorio
git clone --depth 1 "$REPO_URL" "$TEMP_DIR"

# Crear estructura de directorios
mkdir -p "$TEMPLATES_DIR"

echo "📁 Organizando templates por categoría..."

# Función para copiar y organizar archivos
organize_templates() {
    local source_dir="$1"
    local category="$2"
    
    if [ -d "$source_dir" ]; then
        mkdir -p "$TEMPLATES_DIR/$category"
        
        # Copiar archivos JSON
        find "$source_dir" -name "*.json" -type f -exec cp {} "$TEMPLATES_DIR/$category/" \;
        
        # Contar archivos copiados
        count=$(find "$TEMPLATES_DIR/$category" -name "*.json" -type f | wc -l)
        echo "   ✅ $category: $count políticas"
    fi
}

# Organizar por categorías comunes del repositorio
organize_templates "$TEMP_DIR/Deny-changes-to-security-services" "security-services"
organize_templates "$TEMP_DIR/Require-encryption" "encryption"
organize_templates "$TEMP_DIR/Deny-leaving-orgs" "organizational-control"
organize_templates "$TEMP_DIR/Deny-root-user" "security"
organize_templates "$TEMP_DIR/Require-MFA" "security"
organize_templates "$TEMP_DIR/Deny-regions" "compliance"
organize_templates "$TEMP_DIR/Require-tagging" "governance"

# Buscar y copiar cualquier otro archivo JSON
find "$TEMP_DIR" -name "*.json" -type f ! -path "$TEMPLATES_DIR/*" -exec sh -c '
    category=$(dirname "$1" | xargs basename | tr "[:upper:]" "[:lower:]" | sed "s/[^a-z0-9]/-/g")
    mkdir -p "'"$TEMPLATES_DIR"'/$category"
    cp "$1" "'"$TEMPLATES_DIR"'/$category/"
' _ {} \;

# Limpiar
rm -rf "$TEMP_DIR"

# Contar total
total=$(find "$TEMPLATES_DIR" -name "*.json" -type f | wc -l)

echo ""
echo "✅ Descarga completada!"
echo "📊 Total de templates: $total"
echo "📁 Ubicación: $TEMPLATES_DIR"
echo ""
echo "Para usar el generador:"
echo "  cargo run -- --templates-dir $TEMPLATES_DIR"