use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use rmcp::{service::ServiceExt, transport::stdio};
use tracing_subscriber::{filter::LevelFilter, EnvFilter};

mod config;
mod fs_ops;
mod server;
mod web;

#[derive(Parser, Debug)]
#[command(
    name = "nosce",
    version,
    about = "MCP server and web frontend for nosce docs and reports",
    long_about = "nosce serves generated documentation and daily sync reports. \
        It can run as an MCP server (for AI assistants) or as a \
        standalone web frontend with a browsable UI.",
    after_help = "\x1b[1mExamples:\x1b[0m
  # Start the MCP server (default mode, communicates over stdio)
  nosce --output-dir ~/nosce-output

  # Start the web UI on port 8080
  nosce --output-dir ~/nosce-output serve -p 8080

  # Start the web UI as a background daemon
  nosce --output-dir ~/nosce-output serve -d

  # Stop a running daemon
  nosce stop

  # Increase log verbosity (-v info, -vv debug, -vvv trace)
  nosce -vv --output-dir ~/nosce-output serve

  # Use an environment variable instead of --output-dir
  export NOSCE_OUTPUT_DIR=~/nosce-output
  nosce serve"
)]
struct Cli {
    /// Path to the nosce output directory containing docs/ and reports/
    #[arg(short, long, env = "NOSCE_OUTPUT_DIR", global = true)]
    output_dir: Option<String>,

    /// Increase log verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the MCP server over stdio (default if no subcommand given).
    /// This mode is used by AI assistants that speak the Model Context Protocol.
    /// The server reads/writes JSON-RPC messages on stdin/stdout.
    Mcp,
    /// Start the web frontend to browse docs and reports in a browser.
    /// Opens an HTTP server with a REST API and single-page application UI.
    Serve {
        /// Port to listen on
        #[arg(short, long, env = "PORT", default_value_t = 3000)]
        port: u16,

        /// Host to bind to (use 0.0.0.0 for Docker/public exposure)
        #[arg(long, env = "HOST", default_value = "127.0.0.1")]
        host: String,

        /// Base path prefix for all routes (e.g. "/nosce" to serve at /nosce/*)
        #[arg(long, env = "BASE_PATH", default_value = "/")]
        base_path: String,

        /// Run the server as a background daemon
        #[arg(short, long)]
        detach: bool,
    },
    /// Stop a running nosce daemon started with `serve -d`
    Stop,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Show help when invoked with no arguments and no env var fallback
    if cli.output_dir.is_none() && cli.command.is_none() {
        Cli::command().print_help()?;
        eprintln!();
        std::process::exit(0);
    }

    // Handle stop subcommand (no tokio runtime needed)
    if matches!(cli.command, Some(Commands::Stop)) {
        return stop_daemon();
    }

    // Handle detach before spawning tokio threads
    let is_detach = matches!(cli.command, Some(Commands::Serve { detach: true, .. }));
    if is_detach {
        daemonize_server(&cli)?;
    }

    tokio::runtime::Runtime::new()?.block_on(async_main(cli))
}

async fn async_main(cli: Cli) -> Result<()> {
    // Build tracing filter: RUST_LOG env var takes priority, otherwise use verbose count
    let env_filter = if std::env::var("RUST_LOG").is_ok() {
        EnvFilter::from_default_env()
    } else {
        let level = match cli.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        };
        EnvFilter::builder()
            .with_default_directive(level.into())
            .from_env_lossy()
    };

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    let output_dir_raw = cli.output_dir.ok_or_else(|| {
        anyhow::anyhow!(
            "Missing required argument --output-dir (or set NOSCE_OUTPUT_DIR env var)"
        )
    })?;
    let output_dir = shellexpand::tilde(&output_dir_raw).to_string();
    let output_path = std::path::PathBuf::from(&output_dir);

    if !fs_ops::path_exists(&output_path).await {
        anyhow::bail!("Output directory does not exist: {output_dir}");
    }

    // Load profiles from nosce.yml (look in cwd then fall back to defaults)
    let config_path = std::path::Path::new("nosce.yml");
    let profiles = config::load_profiles(config_path);
    tracing::info!("Loaded {} profile(s)", profiles.len());

    match cli.command.unwrap_or(Commands::Mcp) {
        Commands::Mcp => {
            tracing::info!("Starting nosce MCP server (stdio), output_dir={output_dir}");
            let server = server::NosceServer::new(output_path, profiles);
            let service = server.serve(stdio()).await?;
            service.waiting().await?;
        }
        Commands::Serve {
            port,
            host,
            base_path,
            ..
        } => {
            // Normalize base_path: ensure it starts with / and doesn't end with /
            let base_path = if base_path == "/" {
                String::new()
            } else {
                let bp = base_path.trim_end_matches('/');
                if bp.starts_with('/') {
                    bp.to_owned()
                } else {
                    format!("/{bp}")
                }
            };

            eprintln!("nosce v{}", env!("CARGO_PKG_VERSION"));
            eprintln!("  URL:       http://{host}:{port}{}/", base_path);
            eprintln!("  Data dir:  {output_dir}");
            if !base_path.is_empty() {
                eprintln!("  Base path: {base_path}");
            }
            eprintln!();

            tracing::info!("Starting nosce web frontend on {host}:{port}, output_dir={output_dir}");
            web::start_server(output_path, &host, port, &base_path, profiles).await?;
        }
        Commands::Stop => unreachable!(),
    }

    Ok(())
}

// -- Daemon helpers --

fn nosce_home() -> std::path::PathBuf {
    dirs_or_home().join(".nosce")
}

fn dirs_or_home() -> std::path::PathBuf {
    std::env::var("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
}

fn pid_file_path() -> std::path::PathBuf {
    nosce_home().join("nosce.pid")
}

fn log_file_path() -> std::path::PathBuf {
    nosce_home().join("nosce.log")
}

fn daemonize_server(cli: &Cli) -> Result<()> {
    let home = nosce_home();
    std::fs::create_dir_all(&home)?;

    let log_path = log_file_path();
    let pid_path = pid_file_path();

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    let stdout_file = log_file.try_clone()?;

    // Resolve output_dir for banner before forking
    let output_dir_raw = cli.output_dir.as_deref().unwrap_or("(not set)");
    let output_dir = shellexpand::tilde(output_dir_raw).to_string();

    // Determine port/host for banner
    let (host, port) = match &cli.command {
        Some(Commands::Serve { host, port, .. }) => (host.as_str(), *port),
        _ => ("127.0.0.1", 3000),
    };

    eprintln!("nosce: starting daemon");
    eprintln!("  URL:      http://{host}:{port}");
    eprintln!("  PID file: {}", pid_path.display());
    eprintln!("  Log file: {}", log_path.display());
    eprintln!("  Data dir: {output_dir}");
    eprintln!("  Stop:     nosce stop");

    let daemon = daemonize::Daemonize::new()
        .pid_file(&pid_path)
        .chown_pid_file(true)
        .working_directory(".")
        .stdout(stdout_file)
        .stderr(log_file);

    daemon.start().map_err(|e| anyhow::anyhow!("Failed to daemonize: {e}"))?;

    // After fork — child continues to async_main via the caller
    Ok(())
}

fn stop_daemon() -> Result<()> {
    let pid_path = pid_file_path();

    let pid_str = match std::fs::read_to_string(&pid_path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("No running daemon found (no PID file at {})", pid_path.display());
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    let pid: i32 = pid_str.trim().parse().map_err(|_| {
        anyhow::anyhow!("Invalid PID in {}: {:?}", pid_path.display(), pid_str.trim())
    })?;

    // Send SIGTERM
    let ret = unsafe { libc::kill(pid, libc::SIGTERM) };
    if ret == 0 {
        eprintln!("Sent SIGTERM to nosce daemon (PID {pid})");
        let _ = std::fs::remove_file(&pid_path);
        Ok(())
    } else {
        let errno = std::io::Error::last_os_error();
        if errno.raw_os_error() == Some(libc::ESRCH) {
            // Process doesn't exist — stale PID file
            eprintln!("Daemon (PID {pid}) is not running (stale PID file). Cleaning up.");
            let _ = std::fs::remove_file(&pid_path);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to kill PID {pid}: {errno}"))
        }
    }
}
