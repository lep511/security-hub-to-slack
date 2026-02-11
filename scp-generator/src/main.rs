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
    about = "Generador interactivo de Service Control Policies para AWS Organizations"
)]
struct Args {
    /// Directorio donde están los templates de SCP
    #[arg(short, long, default_value = "./scp-templates")]
    templates_dir: String,
    // Uso: -t ./custom-dir  o  --templates-dir ./custom-dir

    /// Modo no interactivo - solo listar templates
    #[arg(short, long)]
    list_only: bool,
    // Uso: -l  o  --list-only

    /// Crear templates de ejemplo
    #[arg(long)]
    init: bool,
    // Uso: --init
}

#[tokio::main] 
async fn main() -> Result<()> {
    let args = Args::parse();
    print_banner();

    let loader = TemplateLoader::new(args.templates_dir.clone());
    let templates = loader
        .load_all_templates()
        .context("Error al cargar templates")?;  // Agrega contexto al error

    // Validar que haya templates
    if templates.is_empty() {
        println!("{}", "⚠️  No se encontraron templates de SCP".yellow());
        println!(
            "{}",
            format!(
                "Ejecuta '{}' para crear templates de ejemplo",
                "scp-generator --init".bold()
            )
        );
        return Ok(());
    }

    println!(
        "{}",
        format!("✅ Se cargaron {} templates", templates.len())
            .green()
            .bold()
    );

    if args.list_only {
        InteractiveMenu::show_templates_by_category(&templates);
        return Ok(());
    }

    let aws_manager = AwsScpManager::new()
        .await  // Async call
        .context("Error al conectar con AWS")?;

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
                println!("{}", "\n👋 ¡Hasta luego!".cyan().bold());
                break;  // Salir del loop
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
    // 1. Usuario selecciona categoría
    if let Some(category) = InteractiveMenu::select_category(templates)? {
        
        // 2. Filtrar templates por categoría
        let filtered: Vec<ScpTemplate> = templates
            .iter()
            .filter(|t| t.category == category)
            .cloned()
            .collect();

        // 3. Usuario selecciona un template específico
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
    println!("{}", "🔍 Consultando SCPs en AWS Organizations...".cyan());

    let scps = aws_manager.list_scps().await?;
    InteractiveMenu::show_deployed_scps(&scps);

    Ok(())
}

async fn handle_template_selection(
    template: &ScpTemplate,
    aws_manager: &AwsScpManager,
) -> Result<()> {
    // 1. Mostrar detalles del template
    InteractiveMenu::show_template_details(template);

    // 2. Confirmar creación
    if !InteractiveMenu::confirm_create_scp(template)? {
        println!("{}", "❌ Operación cancelada".yellow());
        return Ok(());
    }

    // 3. Personalizar nombre y descripción
    let name = InteractiveMenu::get_custom_name(&template.name)?;
    let description = InteractiveMenu::get_custom_description(&template.description)?;

    // 4. Crear la SCP en AWS
    let policy_content = template.to_json_string()?;
    let policy_id = aws_manager
        .create_scp(&name, &description, &policy_content)
        .await?;

    // 5. Opcionalmente adjuntar a OU/cuenta
    if InteractiveMenu::confirm_attach_policy()? {
        handle_attach_policy(aws_manager, &policy_id).await?;
    }

    Ok(())
}

async fn handle_attach_policy(aws_manager: &AwsScpManager, policy_id: &str) -> Result<()> {
    println!("{}", "🔍 Obteniendo estructura de la organización...".cyan());

    // 1. Obtener roots de la organización
    let roots = aws_manager.list_roots().await?;
    if roots.is_empty() {
        println!("{}", "⚠️  No se encontraron roots en la organización".yellow());
        return Ok(());
    }

    // 2. Mostrar opciones de adjuntado
    let options = vec![
        "📁 Adjuntar a la raíz de la organización",
        "🗂️  Adjuntar a una OU específica",
        "❌ Cancelar",
    ];

    let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("¿Dónde deseas adjuntar la SCP?")
        .items(&options)
        .default(0)
        .interact()?;

    match selection {
        0 => {
            // Adjuntar a root (toda la organización)
            if let Some(target_id) = InteractiveMenu::select_target(&roots, "la raíz")? {
                aws_manager.attach_policy(policy_id, &target_id).await?;
            }
        }
        1 => {
            // Adjuntar a una OU específica
            let root_id = &roots[0].0;
            let ous = aws_manager.list_ous(root_id).await?;
            
            if let Some(target_id) = InteractiveMenu::select_target(&ous, "una OU")? {
                aws_manager.attach_policy(policy_id, &target_id).await?;
            }
        }
        _ => {
            println!("{}", "❌ Operación cancelada".yellow());
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
