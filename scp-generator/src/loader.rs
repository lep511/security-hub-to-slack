use crate::models::{ScpPolicy, ScpTemplate};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub struct TemplateLoader {
    templates_dir: String,
}

impl TemplateLoader {
    pub fn new(templates_dir: String) -> Self {
        Self { templates_dir }
    }

    /// Carga todos los templates de SCP desde el directorio
    pub fn load_all_templates(&self) -> Result<Vec<ScpTemplate>> {
        let mut templates = Vec::new();
        
        if !Path::new(&self.templates_dir).exists() {
            return Ok(templates);
        }

        for entry in WalkDir::new(&self.templates_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
        {
            if let Ok(template) = self.load_template_from_file(entry.path()) {
                templates.push(template);
            }
        }

        templates.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
        Ok(templates)
    }

    /// Carga un template desde un archivo JSON
    fn load_template_from_file(&self, path: &Path) -> Result<ScpTemplate> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Error leyendo archivo: {:?}", path))?;

        let policy: ScpPolicy = serde_json::from_str(&content)
            .with_context(|| format!("Error parseando JSON: {:?}", path))?;

        // Extraer metadata del path o del contenido
        let file_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let category = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("general")
            .to_string();

        let description = self.extract_description(&policy);
        let name = self.format_name(&file_name);

        Ok(ScpTemplate {
            name,
            description,
            category,
            policy,
            file_path: path.to_string_lossy().to_string(),
        })
    }

    /// Extrae descripción del primer Sid o usa el nombre del archivo
    fn extract_description(&self, policy: &ScpPolicy) -> String {
        policy
            .statement
            .first()
            .and_then(|s| s.sid.clone())
            .unwrap_or_else(|| "Sin descripción".to_string())
    }

    /// Formatea el nombre del archivo para mostrarlo mejor
    fn format_name(&self, file_name: &str) -> String {
        file_name
            .replace('-', " ")
            .replace('_', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}