use clap::Parser;
use smms::domain::{ManifestGenerator, PathResolver, PlaysetExtractor};

#[derive(Parser)]
#[command(name = "smms")]
#[command(about = "Stellaris Multiplayer Mod Sync")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand)]
enum Commands {
    Init,
    GenKeypair,
    Serve {
        #[arg(long, help = "Port to listen on")]
        port: Option<u16>,
    },
    Fetch {
        host: String,
        #[arg(long, help = "Sync only, do not launch Stellaris")]
        no_launch: bool,
        #[arg(long, help = "Backup files before overwriting")]
        backup: bool,
        #[arg(long, help = "Allow fetch when manifest has no files (dangerous)")]
        allow_empty_manifest: bool,
        #[arg(long, help = "Host port")]
        port: Option<u16>,
    },
    Verify {
        host: String,
        #[arg(long, help = "Host port")]
        port: Option<u16>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Init) => smms::init::run_init()?,
        Some(Commands::GenKeypair) => smms::signing::run_gen_keypair()?,
        Some(Commands::Serve { port }) => run_serve(port).await?,
        Some(Commands::Fetch {
            host,
            no_launch,
            backup,
            allow_empty_manifest,
            port,
        }) => smms::client::fetch(&host, no_launch, backup, allow_empty_manifest, port).await?,
        Some(Commands::Verify { host, port }) => smms::client::verify(&host, port).await?,
    }
    Ok(())
}

async fn run_serve(port_override: Option<u16>) -> Result<(), Box<dyn std::error::Error>> {
    let path_resolver = smms::path_resolver::SteamPathResolver::new();
    let paths = path_resolver
        .resolve()
        .map_err(|e| format!("Path resolution failed: {}", e))?;
    eprintln!(
        "✓ Stellaris at {} (workshop: {})",
        paths.game_path.display(),
        paths.workshop_path.display()
    );
    let playset = smms::playset::DlcLoadPlaysetExtractor::new();
    let load_order = playset
        .active_playset(&paths)
        .map_err(|e| format!("Playset extraction failed: {}", e))?;
    let manifest_gen = smms::manifest_gen::Blake3ManifestGenerator::new();
    let manifest = manifest_gen
        .generate(&paths, &load_order)
        .map_err(|e| format!("Manifest generation failed: {}", e))?;
    let backend = smms::manifest_gen::build_file_backend(&paths, &load_order);
    eprintln!("✓ Manifest: {} files", manifest.files.len());
    // FIXED: fail closed when config/signing setup is broken, instead of silently falling back to unsigned manifests.
    let signed_manifest = if let Some(key_path) = smms::config::signing_key_path_for_auth()? {
        let sig = smms::signing::sign_manifest(&manifest, &key_path).map_err(|e| {
            format!(
                "Signing failed (signing_key_path configured): {}. Aborting.",
                e
            )
        })?;
        Some(smms::domain::SignedManifest {
            manifest: manifest.clone(),
            signature: sig,
        })
    } else {
        None
    };
    let state = smms::server::AppState {
        manifest: manifest.clone(),
        signed_manifest,
        files: Some(backend),
    };
    let port = port_override.unwrap_or_else(smms::config::port_from_config);
    let app = smms::server::router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    eprintln!("✓ Listening on 0.0.0.0:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
