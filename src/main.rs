use std::process::exit;

#[tokio::main]
async fn main() {
    let opt = cli::Opt::parse();
    let config_path = opt
        .config
        .clone()
        .unwrap_or_else(|| "PiMainteno.toml".to_owned());

    let config = match config::Config::from_file(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load config: {}", e);
            exit(1);
        }
    };

    if let Err(e) = tracing_init::init(&config) {
        eprintln!("Failed to initialize logging: {}", e);
        exit(1);
    }

    if opt.one_shot {
        if let Err(err) = run_once(&config).await {
            eprintln!("Error in one-shot mode: {:?}", err);
            exit(1);
        }
    } else {
        if let Err(err) = start_daemon(config).await {
            eprintln!("Error in daemon mode: {:?}", err);
            exit(1);
        }
    }
}
