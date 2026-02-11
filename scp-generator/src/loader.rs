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

    /// Loads all SCP templates from the directory
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

    /// Loads a template from a JSON file
    fn load_template_from_file(&self, path: &Path) -> Result<ScpTemplate> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Error reading file: {:?}", path))?;

        let policy: ScpPolicy = serde_json::from_str(&content)
            .with_context(|| format!("Error parsing JSON: {:?}", path))?;

        // Extract metadata from path
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

        let name = self.format_name(&file_name);

        Ok(ScpTemplate {
            name,
            category,
            policy,
            file_path: path.to_string_lossy().to_string(),
        })
    }

    /// Formats the file name for better display
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