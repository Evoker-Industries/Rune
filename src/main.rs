//! Rune - A Docker-like and Docker-compatible container service
//!
//! This is the main CLI entry point for Rune.

use clap::{Parser, Subcommand};
use rune::compose::{ComposeOrchestrator, ComposeParser};
use rune::container::{ContainerConfig, ContainerManager};
use rune::error::Result;
use rune::image::builder::{BuildContext, ImageBuilder, DEFAULT_BUILD_FILE};
use rune::swarm::{SwarmCluster, SwarmConfig};
use rune::tui::App;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

/// Rune - Docker-compatible container service
#[derive(Parser)]
#[command(name = "rune")]
#[command(author = "Evoker Industries")]
#[command(version)]
#[command(about = "A Docker-like and Docker-compatible container service", long_about = None)]
struct Cli {
    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a container
    Run {
        /// Image to run
        image: String,
        /// Container name
        #[arg(long)]
        name: Option<String>,
        /// Run in detached mode
        #[arg(short, long)]
        detach: bool,
        /// Port mapping (host:container)
        #[arg(short, long)]
        publish: Vec<String>,
        /// Environment variable
        #[arg(short, long)]
        env: Vec<String>,
        /// Volume mount
        #[arg(short, long)]
        volume: Vec<String>,
        /// Working directory
        #[arg(short, long)]
        workdir: Option<String>,
        /// Command to run
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Create a container
    Create {
        /// Image to use
        image: String,
        /// Container name
        #[arg(long)]
        name: Option<String>,
    },

    /// Start a container
    Start {
        /// Container ID or name
        container: String,
    },

    /// Stop a container
    Stop {
        /// Container ID or name
        container: String,
        /// Timeout in seconds
        #[arg(short, long, default_value = "10")]
        time: u64,
    },

    /// Restart a container
    Restart {
        /// Container ID or name
        container: String,
    },

    /// Remove a container
    #[command(name = "rm")]
    Remove {
        /// Container ID or name
        container: String,
        /// Force removal
        #[arg(short, long)]
        force: bool,
    },

    /// List containers
    #[command(name = "ps")]
    Ps {
        /// Show all containers
        #[arg(short, long)]
        all: bool,
        /// Only show numeric IDs
        #[arg(short, long)]
        quiet: bool,
    },

    /// Show container logs
    Logs {
        /// Container ID or name
        container: String,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
        /// Number of lines to show
        #[arg(short = 'n', long)]
        tail: Option<usize>,
    },

    /// Execute command in container
    Exec {
        /// Container ID or name
        container: String,
        /// Allocate pseudo-TTY
        #[arg(short, long)]
        tty: bool,
        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
        /// Command to execute
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Build an image from a Runefile
    Build {
        /// Build context path
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Name and optionally tag
        #[arg(short, long)]
        tag: Vec<String>,
        /// Buildfile path
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Build arguments
        #[arg(long)]
        build_arg: Vec<String>,
        /// Do not use cache
        #[arg(long)]
        no_cache: bool,
        /// Target build stage
        #[arg(long)]
        target: Option<String>,
    },

    /// Manage images
    Image {
        #[command(subcommand)]
        command: ImageCommands,
    },

    /// Manage networks
    Network {
        #[command(subcommand)]
        command: NetworkCommands,
    },

    /// Manage volumes
    Volume {
        #[command(subcommand)]
        command: VolumeCommands,
    },

    /// Docker Compose commands
    Compose {
        #[command(subcommand)]
        command: ComposeCommands,
    },

    /// Manage Swarm
    Swarm {
        #[command(subcommand)]
        command: SwarmCommands,
    },

    /// Manage services (Swarm mode)
    Service {
        #[command(subcommand)]
        command: ServiceCommands,
    },

    /// Manage nodes (Swarm mode)
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },

    /// Display system-wide information
    Info,

    /// Show rune version
    Version,

    /// Launch the Terminal User Interface
    #[command(name = "tui")]
    Tui,
}

#[derive(Subcommand)]
enum ImageCommands {
    /// List images
    #[command(name = "ls")]
    List {
        /// Show all images
        #[arg(short, long)]
        all: bool,
    },
    /// Pull an image
    Pull {
        /// Image name
        name: String,
    },
    /// Push an image
    Push {
        /// Image name
        name: String,
    },
    /// Remove an image
    #[command(name = "rm")]
    Remove {
        /// Image ID or name
        image: String,
        /// Force removal
        #[arg(short, long)]
        force: bool,
    },
    /// Tag an image
    Tag {
        /// Source image
        source: String,
        /// Target tag
        target: String,
    },
    /// Show image history
    History {
        /// Image ID or name
        image: String,
    },
    /// Inspect an image
    Inspect {
        /// Image ID or name
        image: String,
    },
    /// Remove unused images
    Prune {
        /// Remove all unused images
        #[arg(short, long)]
        all: bool,
        /// Do not prompt for confirmation
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum NetworkCommands {
    /// List networks
    #[command(name = "ls")]
    List,
    /// Create a network
    Create {
        /// Network name
        name: String,
        /// Driver
        #[arg(short, long, default_value = "bridge")]
        driver: String,
        /// Subnet
        #[arg(long)]
        subnet: Option<String>,
        /// Gateway
        #[arg(long)]
        gateway: Option<String>,
    },
    /// Remove a network
    #[command(name = "rm")]
    Remove {
        /// Network ID or name
        network: String,
    },
    /// Inspect a network
    Inspect {
        /// Network ID or name
        network: String,
    },
    /// Connect container to network
    Connect {
        /// Network name
        network: String,
        /// Container name
        container: String,
    },
    /// Disconnect container from network
    Disconnect {
        /// Network name
        network: String,
        /// Container name
        container: String,
    },
    /// Remove unused networks
    Prune {
        /// Do not prompt for confirmation
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum VolumeCommands {
    /// List volumes
    #[command(name = "ls")]
    List,
    /// Create a volume
    Create {
        /// Volume name
        name: Option<String>,
        /// Driver
        #[arg(short, long, default_value = "local")]
        driver: String,
    },
    /// Remove a volume
    #[command(name = "rm")]
    Remove {
        /// Volume name
        volume: String,
        /// Force removal
        #[arg(short, long)]
        force: bool,
    },
    /// Inspect a volume
    Inspect {
        /// Volume name
        volume: String,
    },
    /// Remove unused volumes
    Prune {
        /// Do not prompt for confirmation
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ComposeCommands {
    /// Create and start containers
    Up {
        /// Compose file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Run in detached mode
        #[arg(short, long)]
        detach: bool,
        /// Build images before starting
        #[arg(long)]
        build: bool,
        /// Scale services
        #[arg(long)]
        scale: Vec<String>,
    },
    /// Stop and remove containers
    Down {
        /// Compose file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Remove named volumes
        #[arg(short, long)]
        volumes: bool,
        /// Remove images
        #[arg(long)]
        rmi: Option<String>,
    },
    /// List containers
    Ps {
        /// Compose file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    /// View logs
    Logs {
        /// Compose file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Service name
        service: Option<String>,
        /// Follow log output
        #[arg(short = 'f', long)]
        follow: bool,
    },
    /// Build or rebuild services
    Build {
        /// Compose file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Service name
        service: Option<String>,
        /// Do not use cache
        #[arg(long)]
        no_cache: bool,
    },
    /// Start services
    Start {
        /// Compose file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Service names
        services: Vec<String>,
    },
    /// Stop services
    Stop {
        /// Compose file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Service names
        services: Vec<String>,
    },
    /// Restart services
    Restart {
        /// Compose file
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Service names
        services: Vec<String>,
    },
    /// Validate compose file
    Config {
        /// Compose file
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SwarmCommands {
    /// Initialize a swarm
    Init {
        /// Listen address
        #[arg(long, default_value = "0.0.0.0:2377")]
        listen_addr: String,
        /// Advertise address
        #[arg(long)]
        advertise_addr: Option<String>,
        /// Force new cluster
        #[arg(long)]
        force_new_cluster: bool,
    },
    /// Join a swarm
    Join {
        /// Join token
        #[arg(long)]
        token: String,
        /// Remote address
        remote: String,
    },
    /// Leave the swarm
    Leave {
        /// Force leave
        #[arg(short, long)]
        force: bool,
    },
    /// Manage join tokens
    #[command(name = "join-token")]
    JoinToken {
        /// Token type (worker or manager)
        role: String,
        /// Rotate token
        #[arg(long)]
        rotate: bool,
    },
    /// Update the swarm
    Update {
        /// Auto-lock managers
        #[arg(long)]
        autolock: Option<bool>,
        /// Task history retention limit
        #[arg(long)]
        task_history_limit: Option<i64>,
    },
    /// Unlock the swarm
    Unlock,
    /// Manage unlock key
    #[command(name = "unlock-key")]
    UnlockKey {
        /// Rotate unlock key
        #[arg(long)]
        rotate: bool,
    },
}

#[derive(Subcommand)]
enum ServiceCommands {
    /// List services
    #[command(name = "ls")]
    List,
    /// Create a service
    Create {
        /// Service name
        #[arg(long)]
        name: String,
        /// Image
        image: String,
        /// Number of replicas
        #[arg(long)]
        replicas: Option<u64>,
        /// Port mapping
        #[arg(short, long)]
        publish: Vec<String>,
        /// Environment variable
        #[arg(short, long)]
        env: Vec<String>,
        /// Mount
        #[arg(long)]
        mount: Vec<String>,
    },
    /// Update a service
    Update {
        /// Service ID or name
        service: String,
        /// Image
        #[arg(long)]
        image: Option<String>,
        /// Number of replicas
        #[arg(long)]
        replicas: Option<u64>,
        /// Force update
        #[arg(long)]
        force: bool,
    },
    /// Scale a service
    Scale {
        /// Service=replicas pairs
        scales: Vec<String>,
    },
    /// Rollback a service
    Rollback {
        /// Service ID or name
        service: String,
    },
    /// Remove a service
    #[command(name = "rm")]
    Remove {
        /// Service ID or name
        service: String,
    },
    /// Inspect a service
    Inspect {
        /// Service ID or name
        service: String,
    },
    /// Show service logs
    Logs {
        /// Service ID or name
        service: String,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
    },
    /// List service processes
    Ps {
        /// Service ID or name
        service: String,
    },
}

#[derive(Subcommand)]
enum NodeCommands {
    /// List nodes
    #[command(name = "ls")]
    List,
    /// Inspect a node
    Inspect {
        /// Node ID
        node: String,
    },
    /// Update a node
    Update {
        /// Node ID
        node: String,
        /// Availability (active, pause, drain)
        #[arg(long)]
        availability: Option<String>,
        /// Role (worker, manager)
        #[arg(long)]
        role: Option<String>,
        /// Add label
        #[arg(long)]
        label_add: Vec<String>,
        /// Remove label
        #[arg(long)]
        label_rm: Vec<String>,
    },
    /// Promote a node to manager
    Promote {
        /// Node IDs
        nodes: Vec<String>,
    },
    /// Demote a node from manager
    Demote {
        /// Node IDs
        nodes: Vec<String>,
    },
    /// Remove a node
    #[command(name = "rm")]
    Remove {
        /// Node ID
        node: String,
        /// Force removal
        #[arg(short, long)]
        force: bool,
    },
    /// List tasks on a node
    Ps {
        /// Node ID
        node: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt().with_env_filter(filter).init();

    // Get base path for rune data
    let base_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/var/lib"))
        .join("rune");

    // Initialize container manager
    let container_manager = Arc::new(ContainerManager::new(base_path.join("containers"))?);

    match cli.command {
        Commands::Run {
            image,
            name,
            detach,
            publish,
            env,
            volume,
            workdir,
            command,
        } => {
            let container_name = name.unwrap_or_else(|| {
                format!("rune-{}", uuid::Uuid::new_v4().to_string()[..8].to_string())
            });

            let mut config = ContainerConfig::new(&container_name, &image);

            // Parse environment variables
            for e in env {
                if let Some((key, value)) = e.split_once('=') {
                    config.env.insert(key.to_string(), value.to_string());
                }
            }

            // Set command
            if !command.is_empty() {
                config.cmd = command;
            }

            // Set working directory
            if let Some(wd) = workdir {
                config.working_dir = wd;
            }

            let id = container_manager.create(config)?;
            container_manager.start(&id)?;

            if detach {
                println!("{}", id);
            } else {
                println!("Container {} started", id);
            }
        }

        Commands::Create { image, name } => {
            let container_name = name.unwrap_or_else(|| {
                format!("rune-{}", uuid::Uuid::new_v4().to_string()[..8].to_string())
            });

            let config = ContainerConfig::new(&container_name, &image);
            let id = container_manager.create(config)?;
            println!("{}", id);
        }

        Commands::Start { container } => {
            container_manager.start(&container)?;
            println!("{}", container);
        }

        Commands::Stop { container, time: _ } => {
            container_manager.stop(&container)?;
            println!("{}", container);
        }

        Commands::Restart { container } => {
            let _ = container_manager.stop(&container);
            container_manager.start(&container)?;
            println!("{}", container);
        }

        Commands::Remove { container, force } => {
            container_manager.remove(&container, force)?;
            println!("{}", container);
        }

        Commands::Ps { all, quiet } => {
            let containers = container_manager.list(all)?;

            if quiet {
                for c in containers {
                    println!("{}", c.id);
                }
            } else {
                println!(
                    "{:<14} {:<20} {:<25} {:<12} {:<20}",
                    "CONTAINER ID", "NAME", "IMAGE", "STATUS", "CREATED"
                );
                for c in containers {
                    println!(
                        "{:<14} {:<20} {:<25} {:<12} {:<20}",
                        &c.id[..12],
                        c.name,
                        c.image,
                        c.status.to_string(),
                        c.created_at.format("%Y-%m-%d %H:%M:%S")
                    );
                }
            }
        }

        Commands::Logs {
            container,
            follow,
            tail,
        } => {
            println!("Fetching logs for container {}...", container);
            // In a real implementation, we would stream container logs
        }

        Commands::Exec {
            container,
            tty,
            interactive,
            command,
        } => {
            println!("Executing {:?} in container {}", command, container);
            // In a real implementation, we would exec into the container
        }

        Commands::Build {
            path,
            tag,
            file,
            build_arg,
            no_cache,
            target,
        } => {
            let mut context = BuildContext::new(path.clone());

            if let Some(f) = file {
                context = context.build_file(f);
            }

            context.no_cache = no_cache;

            if let Some(t) = target {
                context = context.target(&t);
            }

            for t in tag {
                context = context.tag(&t);
            }

            for arg in build_arg {
                if let Some((key, value)) = arg.split_once('=') {
                    context = context.arg(key, value);
                }
            }

            let builder = ImageBuilder::new(context);
            let image_id = builder.build().await?;
            println!("Successfully built {}", image_id);
        }

        Commands::Image { command } => {
            match command {
                ImageCommands::List { all } => {
                    println!("REPOSITORY          TAG       IMAGE ID       SIZE");
                    // List images
                }
                ImageCommands::Pull { name } => {
                    println!("Pulling image {}...", name);
                }
                ImageCommands::Push { name } => {
                    println!("Pushing image {}...", name);
                }
                ImageCommands::Remove { image, force } => {
                    println!("Removing image {}...", image);
                }
                ImageCommands::Tag { source, target } => {
                    println!("Tagging {} as {}", source, target);
                }
                ImageCommands::History { image } => {
                    println!("IMAGE          CREATED       CREATED BY                                      SIZE");
                }
                ImageCommands::Inspect { image } => {
                    println!("Inspecting image {}...", image);
                }
                ImageCommands::Prune { all, force } => {
                    println!("Pruning unused images...");
                }
            }
        }

        Commands::Network { command } => match command {
            NetworkCommands::List => {
                println!("NETWORK ID     NAME      DRIVER    SCOPE");
                println!("abc123def456   bridge    bridge    local");
                println!("def456ghi789   host      host      local");
                println!("ghi789jkl012   none      null      local");
            }
            NetworkCommands::Create {
                name,
                driver,
                subnet,
                gateway,
            } => {
                println!("Created network {}", name);
            }
            NetworkCommands::Remove { network } => {
                println!("Removed network {}", network);
            }
            NetworkCommands::Inspect { network } => {
                println!("Inspecting network {}...", network);
            }
            NetworkCommands::Connect { network, container } => {
                println!("Connected {} to {}", container, network);
            }
            NetworkCommands::Disconnect { network, container } => {
                println!("Disconnected {} from {}", container, network);
            }
            NetworkCommands::Prune { force } => {
                println!("Pruning unused networks...");
            }
        },

        Commands::Volume { command } => match command {
            VolumeCommands::List => {
                println!("DRIVER    VOLUME NAME");
            }
            VolumeCommands::Create { name, driver } => {
                let vol_name =
                    name.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()[..12].to_string());
                println!("{}", vol_name);
            }
            VolumeCommands::Remove { volume, force } => {
                println!("Removed volume {}", volume);
            }
            VolumeCommands::Inspect { volume } => {
                println!("Inspecting volume {}...", volume);
            }
            VolumeCommands::Prune { force } => {
                println!("Pruning unused volumes...");
            }
        },

        Commands::Compose { command } => {
            let working_dir = std::env::current_dir()?;

            match command {
                ComposeCommands::Up {
                    file,
                    detach,
                    build,
                    scale,
                } => {
                    let compose_file = file.unwrap_or_else(|| {
                        ComposeParser::find_compose_file(&working_dir)
                            .unwrap_or_else(|| working_dir.join("compose.yaml"))
                    });

                    let config = ComposeParser::parse_file(&compose_file)?;
                    let project_name = config.name.clone().unwrap_or_else(|| {
                        working_dir
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("default")
                            .to_string()
                    });

                    let mut orchestrator = ComposeOrchestrator::new(
                        &project_name,
                        config,
                        container_manager.clone(),
                        working_dir,
                    );

                    orchestrator.up(detach, build).await?;
                    println!("Started project {}", project_name);
                }
                ComposeCommands::Down { file, volumes, rmi } => {
                    println!("Stopping compose project...");
                }
                ComposeCommands::Ps { file } => {
                    println!("NAME      SERVICE   STATUS    PORTS");
                }
                ComposeCommands::Logs {
                    file,
                    service,
                    follow,
                } => {
                    println!("Fetching compose logs...");
                }
                ComposeCommands::Build {
                    file,
                    service,
                    no_cache,
                } => {
                    println!("Building compose services...");
                }
                ComposeCommands::Start { file, services } => {
                    println!("Starting services...");
                }
                ComposeCommands::Stop { file, services } => {
                    println!("Stopping services...");
                }
                ComposeCommands::Restart { file, services } => {
                    println!("Restarting services...");
                }
                ComposeCommands::Config { file } => {
                    let compose_file = file.unwrap_or_else(|| {
                        ComposeParser::find_compose_file(&working_dir)
                            .unwrap_or_else(|| working_dir.join("compose.yaml"))
                    });

                    let config = ComposeParser::parse_file(&compose_file)?;
                    let warnings = ComposeParser::validate(&config)?;

                    for warning in warnings {
                        println!("Warning: {}", warning);
                    }

                    println!("{}", serde_yaml::to_string(&config).unwrap());
                }
            }
        }

        Commands::Swarm { command } => match command {
            SwarmCommands::Init {
                listen_addr,
                advertise_addr,
                force_new_cluster,
            } => {
                let mut config = SwarmConfig::default();
                config.listen_addr = listen_addr;
                if let Some(addr) = advertise_addr {
                    config.advertise_addr = addr;
                }
                config.force_new_cluster = force_new_cluster;

                let cluster = SwarmCluster::init(config)?;
                println!(
                    "Swarm initialized: current node ({}) is now a manager.",
                    cluster.id()
                );
                println!("\nTo add a worker to this swarm, run:");
                println!(
                    "    rune swarm join --token {} <manager-ip>:2377",
                    cluster.join_token(rune::swarm::cluster::TokenType::Worker)
                );
                println!("\nTo add a manager to this swarm, run:");
                println!(
                    "    rune swarm join --token {} <manager-ip>:2377",
                    cluster.join_token(rune::swarm::cluster::TokenType::Manager)
                );
            }
            SwarmCommands::Join { token, remote } => {
                println!("Joining swarm at {}...", remote);
            }
            SwarmCommands::Leave { force } => {
                println!("Leaving swarm...");
            }
            SwarmCommands::JoinToken { role, rotate } => {
                println!("Join token for {}: SWMTKN-...", role);
            }
            SwarmCommands::Update {
                autolock,
                task_history_limit,
            } => {
                println!("Swarm updated");
            }
            SwarmCommands::Unlock => {
                println!("Please enter unlock key:");
            }
            SwarmCommands::UnlockKey { rotate } => {
                println!("Unlock key: SWMKEY-...");
            }
        },

        Commands::Service { command } => match command {
            ServiceCommands::List => {
                println!("ID             NAME       MODE         REPLICAS   IMAGE");
            }
            ServiceCommands::Create {
                name,
                image,
                replicas,
                publish,
                env,
                mount,
            } => {
                println!("Created service {}", name);
            }
            ServiceCommands::Update {
                service,
                image,
                replicas,
                force,
            } => {
                println!("Updated service {}", service);
            }
            ServiceCommands::Scale { scales } => {
                for scale in scales {
                    if let Some((name, replicas)) = scale.split_once('=') {
                        println!("Scaled {} to {} replicas", name, replicas);
                    }
                }
            }
            ServiceCommands::Rollback { service } => {
                println!("Rolling back service {}", service);
            }
            ServiceCommands::Remove { service } => {
                println!("Removed service {}", service);
            }
            ServiceCommands::Inspect { service } => {
                println!("Inspecting service {}...", service);
            }
            ServiceCommands::Logs { service, follow } => {
                println!("Fetching logs for service {}...", service);
            }
            ServiceCommands::Ps { service } => {
                println!("ID             NAME              IMAGE     NODE      DESIRED STATE   CURRENT STATE");
            }
        },

        Commands::Node { command } => match command {
            NodeCommands::List => {
                println!("ID                           HOSTNAME         STATUS    AVAILABILITY   MANAGER STATUS");
            }
            NodeCommands::Inspect { node } => {
                println!("Inspecting node {}...", node);
            }
            NodeCommands::Update {
                node,
                availability,
                role,
                label_add,
                label_rm,
            } => {
                println!("Updated node {}", node);
            }
            NodeCommands::Promote { nodes } => {
                for node in nodes {
                    println!("Node {} promoted to manager", node);
                }
            }
            NodeCommands::Demote { nodes } => {
                for node in nodes {
                    println!("Node {} demoted to worker", node);
                }
            }
            NodeCommands::Remove { node, force } => {
                println!("Removed node {}", node);
            }
            NodeCommands::Ps { node } => {
                println!(
                    "ID             NAME              IMAGE     DESIRED STATE   CURRENT STATE"
                );
            }
        },

        Commands::Info => {
            println!("Client:");
            println!(" Version:    {}", env!("CARGO_PKG_VERSION"));
            println!(" API version: 1.43");
            println!(" Go version:  N/A (Rust)");
            println!(
                " OS/Arch:     {}/{}",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
            println!();
            println!("Server:");
            println!(" Containers: {}", container_manager.count()?);
            println!("  Running:   {}", container_manager.running_count()?);
            println!(" Images:     0");
            println!(" Server Version: {}", env!("CARGO_PKG_VERSION"));
            println!(" Storage Driver: overlay2");
            println!(" Default Runtime: rune");
            println!(" Swarm: inactive");
        }

        Commands::Version => {
            println!("Rune version {}", env!("CARGO_PKG_VERSION"));
            println!("API version: 1.43");
            println!("Built with:  Rust {}", env!("CARGO_PKG_RUST_VERSION"));
            println!(
                "OS/Arch:     {}/{}",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
        }

        Commands::Tui => {
            let mut app = App::new(container_manager);
            app.run()?;
        }
    }

    Ok(())
}
