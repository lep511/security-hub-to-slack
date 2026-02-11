use crate::models::ScpTemplate;
use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::collections::HashMap;

pub struct InteractiveMenu;

impl InteractiveMenu {
    /// Shows the main menu
    pub fn show_main_menu() -> Result<MainMenuOption> {
        println!("\n{}", "=== AWS SCP Generator ===".bold().cyan());

        let options = vec![
            "View all available SCPs",
            "Select SCP by category",
            "Search SCP by name",
            "View deployed SCPs in AWS",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What do you want to do?\nUse arrow keys to navigate and Enter to select\n")
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

    /// Shows list of templates grouped by category
    pub fn select_template(templates: &[ScpTemplate]) -> Result<Option<usize>> {
        if templates.is_empty() {
            println!("{}", "No templates available".yellow());
            return Ok(None);
        }

        let items: Vec<String> = templates
            .iter()
            .map(|t| format!("{} ({})", t.name.bold(), t.category.bright_black()))
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select an SCP")
            .items(&items)
            .default(0)
            .interact_opt()?;

        Ok(selection)
    }

    /// Shows available categories
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
            .map(|c| c.to_string())
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a category")
            .items(&items)
            .default(0)
            .interact_opt()?;

        Ok(selection.map(|i| categories[i].clone()))
    }

    /// Shows template details
    pub fn show_template_details(template: &ScpTemplate) {
        println!("\n{}", "----------------------------------------".bright_black());
        println!("{} {}", "Name:".bold(), template.name.cyan());
        println!("{} {}", "Category:".bold(), template.category.yellow());
        println!("\n{}", "Policy content:".bold());
        println!("{}", "----------------------------------------".bright_black());
        
        if let Ok(json) = template.to_json_string() {
            println!("{}", json.bright_black());
        }
        
        println!("{}", "----------------------------------------".bright_black());
    }

    /// Confirms if user wants to create the SCP
    pub fn confirm_create_scp(template: &ScpTemplate) -> Result<bool> {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Do you want to create this SCP '{}' in AWS?", template.name))
            .default(false)
            .interact()
            .map_err(Into::into)
    }

    /// Requests custom name for the SCP
    pub fn get_custom_name(default: &str) -> Result<String> {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("SCP name")
            .default(default.to_string())
            .interact_text()?;
        
        // Convert to PascalCase
        Ok(Self::to_pascal_case(&input))
    }

    /// Requests custom comment/description
    pub fn get_custom_description(default: &str) -> Result<String> {
        let input: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Comment (optional)")
            .default(default.to_string())
            .interact_text()?;
        
        Ok(input)
    }

    /// Converts a string to PascalCase
    pub fn to_pascal_case(input: &str) -> String {
        input
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().chain(chars.map(|c| c.to_lowercase()).flatten()).collect::<String>(),
                    None => String::new(),
                }
            })
            .collect::<String>()
    }

    /// Confirms if user wants to attach the SCP
    pub fn confirm_attach_policy() -> Result<bool> {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to attach this SCP to an OU or account?")
            .default(false)
            .interact()
            .map_err(Into::into)
    }

    /// Selects a target (OU or Root)
    pub fn select_target(targets: &[(String, String)], target_type: &str) -> Result<Option<String>> {
        if targets.is_empty() {
            println!("{}", format!("No {} available", target_type).yellow());
            return Ok(None);
        }

        let items: Vec<String> = targets
            .iter()
            .map(|(id, name)| format!("{} (ID: {})", name, id.bright_black()))
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Select {}", target_type))
            .items(&items)
            .default(0)
            .interact_opt()?;

        Ok(selection.map(|i| targets[i].0.clone()))
    }

    /// Searches templates by term
    pub fn search_templates(templates: &[ScpTemplate]) -> Result<Vec<ScpTemplate>> {
        let search_term: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Search SCP name or category")
            .interact_text()?;

        let search_lower = search_term.to_lowercase();
        let results: Vec<ScpTemplate> = templates
            .iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&search_lower)
                    || t.category.to_lowercase().contains(&search_lower)
            })
            .cloned()
            .collect();

        if results.is_empty() {
            println!("{}", format!("No results found for '{}'", search_term).yellow());
        } else {
            println!("{}", format!("Found {} result(s)", results.len()).green());
        }

        Ok(results)
    }

    /// Shows list of deployed SCPs
    pub fn show_deployed_scps(scps: &[(String, String)]) {
        if scps.is_empty() {
            println!("{}", "No deployed SCPs".yellow());
            return;
        }

        println!("\n{}", "SCPs deployed in AWS Organizations:".bold().cyan());
        println!("{}", "----------------------------------------".bright_black());

        for (id, name) in scps {
            println!("\n{} {}", "ID:".bold(), id.yellow());
            println!("{} {}", "   Name:".bold(), name.cyan());
        }
        println!("{}", "----------------------------------------".bright_black());
    }

    /// Groups templates by category
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

    /// Shows templates grouped by category
    pub fn show_templates_by_category(templates: &[ScpTemplate]) {
        let grouped = Self::group_by_category(templates);
        let mut categories: Vec<_> = grouped.keys().collect();
        categories.sort();

        println!("\n{}", "Available SCPs by category:".bold().cyan());

        for category in categories {
            if let Some(templates) = grouped.get(category) {
                println!("\n{} ({} policies)", category.bold(), templates.len());
                for template in templates {
                    println!("   - {}", template.name.cyan());
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