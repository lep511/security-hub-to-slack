mod utils;
mod loader;
mod models;
mod ui;
use anyhow::{Context, Result};
use utils::AwsScpManager;
use clap::Parser;
use colored::*;
use loader::TemplateLoader;
use models::ScpTemplate;
use ui::{InteractiveMenu, MainMenuOption};

#[derive(Parser, Debug)]
#[command(
    name = "scp-generator",
    author = "AWS SCP Generator",
    version = "0.1.0",
    about = "Interactive Service Control Policy generator for AWS Organizations"
)]
struct Args {
    /// Directory where SCP templates are located
    #[arg(short, long, default_value = "./scp-templates")]
    templates_dir: String,
    // Usage: -t ./custom-dir  or  --templates-dir ./custom-dir

    /// Non-interactive mode - only list templates
    #[arg(short, long)]
    list_only: bool,
    // Usage: -l  or  --list-only

    /// Create sample templates
    #[arg(long)]
    init: bool,
    // Usage: --init
}

#[tokio::main] 
async fn main() -> Result<()> {
    let args = Args::parse();
    print_banner();

    let loader = TemplateLoader::new(args.templates_dir.clone());
    let templates = loader
        .load_all_templates()
        .context("Error loading templates")?;  // Adds context to error

    // Validate that templates exist
    if templates.is_empty() {
        println!("{}", "⚠️  No SCP templates found".yellow());
        println!(
            "{}",
            format!(
                "Run '{}' to create sample templates",
                "scp-generator --init".bold()
            )
        );
        return Ok(());
    }

    println!(
        "{}",
        format!("✅ Loaded {} templates", templates.len())
            .green()
            .bold()
    );

    if args.list_only {
        InteractiveMenu::show_templates_by_category(&templates);
        return Ok(());
    }

    let aws_manager = AwsScpManager::new()
        .await  // Async call
        .context("Error connecting to AWS")?;

    loop {
        match InteractiveMenu::show_main_menu()? {
            MainMenuOption::ViewAll => {
                handle_view_all(&templates)?;
            }
            MainMenuOption::SelectByCategory => {
                handle_select_by_category(&templates, &aws_manager).await?;
            }
            MainMenuOption::Search => {
                handle_search(&templates, &aws_manager).await?;
            }
            MainMenuOption::ViewDeployed => {
                handle_view_deployed(&aws_manager).await?;
            }
            MainMenuOption::Exit => {
                println!("{}", "\n👋 Goodbye!".cyan().bold());
                break;  // Exit loop
            }
        }
    }

    Ok(())
}

fn handle_view_all(templates: &[ScpTemplate]) -> Result<()> {
    InteractiveMenu::show_templates_by_category(templates);
    Ok(())
}

async fn handle_select_by_category(
    templates: &[ScpTemplate],
    aws_manager: &AwsScpManager,
) -> Result<()> {
    // 1. User selects category
    if let Some(category) = InteractiveMenu::select_category(templates)? {
        
        // 2. Filter templates by category
        let filtered: Vec<ScpTemplate> = templates
            .iter()
            .filter(|t| t.category == category)
            .cloned()
            .collect();

        // 3. User selects a specific template
        if let Some(index) = InteractiveMenu::select_template(&filtered)? {
            handle_template_selection(&filtered[index], aws_manager).await?;
        }
    }
    Ok(())
}

async fn handle_search(
    templates: &[ScpTemplate],
    aws_manager: &AwsScpManager,
) -> Result<()> {
    let results = InteractiveMenu::search_templates(templates)?;

    if !results.is_empty() {
        if let Some(index) = InteractiveMenu::select_template(&results)? {
            handle_template_selection(&results[index], aws_manager).await?;
        }
    }

    Ok(())
}

async fn handle_view_deployed(aws_manager: &AwsScpManager) -> Result<()> {
    println!("{}", "🔍 Querying SCPs in AWS Organizations...".cyan());

    let scps = aws_manager.list_scps().await?;
    InteractiveMenu::show_deployed_scps(&scps);

    Ok(())
}

async fn handle_template_selection(
    template: &ScpTemplate,
    aws_manager: &AwsScpManager,
) -> Result<()> {
    // 1. Show template details
    InteractiveMenu::show_template_details(template);

    // 2. Confirm creation
    if !InteractiveMenu::confirm_create_scp(template)? {
        println!("{}", "❌ Operation cancelled".yellow());
        return Ok(());
    }

    // 3. Customize name & description
    let pascal_name = InteractiveMenu::to_pascal_case(&template.name);
    let name = InteractiveMenu::get_custom_name(&pascal_name)?;
    let description = InteractiveMenu::get_custom_description(&template.name)
        .unwrap_or_default();

    // 4. Create the SCP in AWS
    let policy_content = template.to_json_string()?;
    let policy_id = aws_manager
        .create_scp(&name, &description, &policy_content)
        .await?;

    // 5. Optionally attach to OU/account
    if InteractiveMenu::confirm_attach_policy()? {
        handle_attach_policy(aws_manager, &policy_id).await?;
    }

    Ok(())
}

async fn handle_attach_policy(aws_manager: &AwsScpManager, policy_id: &str) -> Result<()> {
    println!("{}", "🔍 Getting organization structure...".cyan());

    // 1. Get organization roots
    let roots = aws_manager.list_roots().await?;
    if roots.is_empty() {
        println!("{}", "⚠️  No roots found in organization".yellow());
        return Ok(());
    }

    // 2. Show attachment options
    let options = vec![
        "📁 Attach to organization root",
        "🗂️  Attach to a specific OU",
        "❌ Cancel",
    ];

    let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Where do you want to attach the SCP?")
        .items(&options)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            // Attach to root (entire organization)
            if let Some(target_id) = InteractiveMenu::select_target(&roots, "the root")? {
                aws_manager.attach_policy(policy_id, &target_id).await?;
            }
        }
        1 => {
            // Attach to a specific OU
            let root_id = &roots[0].0;
            let ous = aws_manager.list_ous(root_id).await?;
            
            if let Some(target_id) = InteractiveMenu::select_target(&ous, "an OU")? {
                aws_manager.attach_policy(policy_id, &target_id).await?;
            }
        }
        _ => {
            println!("{}", "❌ Operation cancelled".yellow());
        }
    }

    Ok(())
}

fn print_banner() {
    println!("\n{}", "╔══════════════════════════════════════════════╗".cyan().bold());
    println!("{}", "║                                              ║".cyan().bold());
    println!(
        "{}",
        "║       AWS SCP Generator v0.1.0               ║"
            .cyan()
            .bold()
    );
    println!("{}", "║   Service Control Policy Management Tool     ║".cyan().bold());
    println!("{}", "║                                              ║".cyan().bold());
    println!("{}", "╚══════════════════════════════════════════════╝".cyan().bold());
}