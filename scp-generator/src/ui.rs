use crate::models::ScpTemplate;
use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::collections::HashMap;

pub struct InteractiveMenu;

impl InteractiveMenu {
    /// Muestra el menú principal
    pub fn show_main_menu() -> Result<MainMenuOption> {
        println!("\n{}", "=== AWS SCP Generator ===".bold().cyan());

        let options = vec![
            "📋 Ver todas las SCPs disponibles",
            "🎯 Seleccionar SCP por categoría",
            "🔍 Buscar SCP por nombre",
            "📤 Ver SCPs desplegadas en AWS",
            "❌ Salir",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("¿Qué deseas hacer?")
            .items(&options)
            .default(0)
            .interact()?;

        Ok(match selection {
            0 => MainMenuOption::ViewAll,
            1 => MainMenuOption::SelectByCategory,
            2 => MainMenuOption::Search,
            3 => MainMenuOption::ViewDeployed,
            4 => MainMenuOption::Exit,
            _ => MainMenuOption::Exit,
        })
    }

    /// Muestra lista de templates agrupados por categoría
    pub fn select_template(templates: &[ScpTemplate]) -> Result<Option<usize>> {
        if templates.is_empty() {
            println!("{}", "⚠️  No hay templates disponibles".yellow());
            return Ok(None);
        }

        let items: Vec<String> = templates
            .iter()
            .map(|t| format!("{} - {} ({})", "📄".cyan(), t.name.bold(), t.category.bright_black()))
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Selecciona una SCP")
            .items(&items)
            .default(0)
            .interact_opt()?;

        Ok(selection)
    }

    /// Muestra categorías disponibles
    pub fn select_category(templates: &[ScpTemplate]) -> Result<Option<String>> {
        let mut categories: Vec<String> = templates
            .iter()
            .map(|t| t.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        categories.sort();

        if categories.is_empty() {
            return Ok(None);
        }

        let items: Vec<String> = categories
            .iter()
            .map(|c| format!("📁 {}", c))
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Selecciona una categoría")
            .items(&items)
            .default(0)
            .interact_opt()?;

        Ok(selection.map(|i| categories[i].clone()))
    }

    /// Muestra detalles de un template
    pub fn show_template_details(template: &ScpTemplate) {
        println!("\n{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
        println!("{} {}", "📄 Nombre:".bold(), template.name.cyan());
        println!("{} {}", "📁 Categoría:".bold(), template.category.yellow());
        println!("{} {}", "📝 Descripción:".bold(), template.description);
        println!("\n{}", "Contenido de la política:".bold());
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
        
        if let Ok(json) = template.to_json_string() {
            println!("{}", json.bright_black());
        }
        
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
    }

    /// Confirma si desea crear la SCP
    pub fn confirm_create_scp(template: &ScpTemplate) -> Result<bool> {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("¿Deseas crear esta SCP '{}' en AWS?", template.name))
            .default(false)
            .interact()
            .map_err(Into::into)
    }

    /// Solicita nombre personalizado para la SCP
    pub fn get_custom_name(default: &str) -> Result<String> {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Nombre de la SCP")
            .default(default.to_string())
            .interact_text()
            .map_err(Into::into)
    }

    /// Solicita descripción personalizada
    pub fn get_custom_description(default: &str) -> Result<String> {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Descripción")
            .default(default.to_string())
            .interact_text()
            .map_err(Into::into)
    }

    /// Confirma si desea adjuntar la SCP
    pub fn confirm_attach_policy() -> Result<bool> {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("¿Deseas adjuntar esta SCP a una OU o cuenta?")
            .default(false)
            .interact()
            .map_err(Into::into)
    }

    /// Selecciona un target (OU o Root)
    pub fn select_target(targets: &[(String, String)], target_type: &str) -> Result<Option<String>> {
        if targets.is_empty() {
            println!("{}", format!("⚠️  No hay {} disponibles", target_type).yellow());
            return Ok(None);
        }

        let items: Vec<String> = targets
            .iter()
            .map(|(id, name)| format!("{} (ID: {})", name, id.bright_black()))
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Selecciona {}", target_type))
            .items(&items)
            .default(0)
            .interact_opt()?;

        Ok(selection.map(|i| targets[i].0.clone()))
    }

    /// Busca templates por término
    pub fn search_templates(templates: &[ScpTemplate]) -> Result<Vec<ScpTemplate>> {
        let search_term: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Buscar SCP (nombre o descripción)")
            .interact_text()?;

        let search_lower = search_term.to_lowercase();
        let results: Vec<ScpTemplate> = templates
            .iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&search_lower)
                    || t.description.to_lowercase().contains(&search_lower)
                    || t.category.to_lowercase().contains(&search_lower)
            })
            .cloned()
            .collect();

        if results.is_empty() {
            println!("{}", format!("⚠️  No se encontraron resultados para '{}'", search_term).yellow());
        } else {
            println!("{}", format!("✅ Se encontraron {} resultado(s)", results.len()).green());
        }

        Ok(results)
    }

    /// Muestra lista de SCPs desplegadas
    pub fn show_deployed_scps(scps: &[(String, String, String)]) {
        if scps.is_empty() {
            println!("{}", "⚠️  No hay SCPs desplegadas".yellow());
            return;
        }

        println!("\n{}", "SCPs desplegadas en AWS Organizations:".bold().cyan());
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());

        for (id, name, description) in scps {
            println!("\n{} {}", "📋 ID:".bold(), id.yellow());
            println!("{} {}", "   Nombre:".bold(), name.cyan());
            if !description.is_empty() {
                println!("{} {}", "   Descripción:".bold(), description);
            }
        }
        println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
    }

    /// Agrupa templates por categoría
    pub fn group_by_category(templates: &[ScpTemplate]) -> HashMap<String, Vec<ScpTemplate>> {
        let mut grouped: HashMap<String, Vec<ScpTemplate>> = HashMap::new();
        
        for template in templates {
            grouped
                .entry(template.category.clone())
                .or_insert_with(Vec::new)
                .push(template.clone());
        }
        
        grouped
    }

    /// Muestra templates agrupados por categoría
    pub fn show_templates_by_category(templates: &[ScpTemplate]) {
        let grouped = Self::group_by_category(templates);
        let mut categories: Vec<_> = grouped.keys().collect();
        categories.sort();

        println!("\n{}", "SCPs disponibles por categoría:".bold().cyan());

        for category in categories {
            if let Some(templates) = grouped.get(category) {
                println!("\n{} {} ({} políticas)", "📁".yellow(), category.bold(), templates.len());
                for template in templates {
                    println!("   {} {}", "└─".bright_black(), template.name.cyan());
                    println!("      {}", template.description.bright_black());
                }
            }
        }
    }
}

pub enum MainMenuOption {
    ViewAll,
    SelectByCategory,
    Search,
    ViewDeployed,
    Exit,
}