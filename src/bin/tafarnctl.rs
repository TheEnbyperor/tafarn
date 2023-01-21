use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Refreshes all remote profiles from their home servers")]
    RefreshProfiles {
        #[arg(short, long, help = "Disable fetching the profile's graph (followers and following)")]
        no_graph: bool,
    },
    #[command(about = "Refreshes a specific profile from the home server")]
    RefreshProfile {
        uri: String,
        #[arg(short, long, help = "Disable fetching the profile's graph (followers and following)")]
        no_graph: bool,
    },
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let cli = Cli::parse();

    let app = tafarn::setup().await;
    // let db_pool = diesel::PgConnection::pool("db", &app.rocket).unwrap();

    match cli.command {
        Commands::RefreshProfile { uri, no_graph } => {
            app.celery_app.send_task(tafarn::tasks::accounts::update_account::new(uri.clone(), no_graph)).await.unwrap();
            println!("Update requested for {}", uri);
        }
        Commands::RefreshProfiles { no_graph } => {
            app.celery_app.send_task(tafarn::tasks::accounts::update_accounts::new(no_graph)).await.unwrap();
            println!("Update of all profiles requested");
        }
    }
}