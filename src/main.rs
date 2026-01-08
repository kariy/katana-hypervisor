use anyhow::Result;
use clap::{Parser, Subcommand};
use directories::ProjectDirs;
use katana_hypervisor::{
    instance::StorageManager,
    port::PortAllocator,
    qemu::VmManager,
    state::StateDatabase,
};
use std::path::PathBuf;
use tracing_subscriber;

#[derive(Parser)]
#[command(name = "katana-hypervisor")]
#[command(about = "Hypervisor for managing katana instances in QEMU VMs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// State directory override
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new katana instance
    Create {
        /// Instance name
        name: String,

        /// Number of virtual CPUs
        #[arg(long, default_value = "4")]
        vcpus: u32,

        /// Memory limit (e.g., 4G, 512M)
        #[arg(long, default_value = "4G")]
        memory: String,

        /// Storage quota (e.g., 10G)
        #[arg(long, default_value = "10G")]
        storage: String,

        /// RPC port (auto-assign if not specified)
        #[arg(long)]
        port: Option<u16>,

        /// Enable dev mode
        #[arg(long)]
        dev: bool,

        /// Start immediately after creation
        #[arg(long)]
        auto_start: bool,
    },

    /// Start an instance
    Start {
        /// Instance name
        name: String,
    },

    /// Stop an instance
    Stop {
        /// Instance name
        name: String,
    },

    /// Delete an instance
    Delete {
        /// Instance name
        name: String,

        /// Force deletion even if running
        #[arg(long)]
        force: bool,
    },

    /// List all instances
    List,

    /// View instance logs
    Logs {
        /// Instance name
        name: String,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        tail: usize,
    },

    /// Calculate SEV-SNP launch measurement
    Measure {
        /// Kernel path (optional, uses katana build if not specified)
        #[arg(long)]
        kernel: Option<PathBuf>,

        /// Initrd path (optional, uses katana build if not specified)
        #[arg(long)]
        initrd: Option<PathBuf>,

        /// OVMF firmware path (optional, uses katana build if not specified)
        #[arg(long)]
        ovmf: Option<PathBuf>,

        /// Kernel command line
        #[arg(long)]
        cmdline: Option<String>,

        /// Number of vCPUs
        #[arg(long, default_value = "4")]
        vcpus: u32,

        /// VCPU type for SEV-SNP
        #[arg(long, default_value = "EPYC-v4")]
        vcpu_type: String,

        /// Katana repository path
        #[arg(long, default_value = "/home/ubuntu/katana")]
        katana_repo: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify SEV-SNP attestation for a running instance
    Attest {
        /// Instance name
        name: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    // Get state directory
    let state_dir = cli.state_dir.unwrap_or_else(|| {
        ProjectDirs::from("dev", "katana", "hypervisor")
            .expect("Failed to determine project directory")
            .data_dir()
            .to_path_buf()
    });

    // Initialize components
    let db_path = state_dir.join("state.db");
    let db = StateDatabase::new(&db_path)?;

    let instances_dir = state_dir.join("instances");
    let storage = StorageManager::new(instances_dir);

    let port_allocator = PortAllocator::new(db.clone());
    let vm_manager = VmManager::new();

    // Execute command
    match cli.command {
        Commands::Create {
            name,
            vcpus,
            memory,
            storage: storage_str,
            port,
            dev,
            auto_start,
        } => {
            katana_hypervisor::cli::create::execute(
                &name,
                vcpus,
                &memory,
                &storage_str,
                port,
                dev,
                auto_start,
                &db,
                &storage,
                &port_allocator,
                &vm_manager,
            )?;
        }
        Commands::Start { name } => {
            katana_hypervisor::cli::start::execute(&name, &db, &vm_manager)?;
        }
        Commands::Stop { name } => {
            katana_hypervisor::cli::stop::execute(&name, &db, &vm_manager)?;
        }
        Commands::Delete { name, force } => {
            katana_hypervisor::cli::delete::execute(&name, force, &db, &storage, &vm_manager)?;
        }
        Commands::List => {
            katana_hypervisor::cli::list::execute(&db, &vm_manager)?;
        }
        Commands::Logs { name, follow, tail } => {
            katana_hypervisor::cli::logs::execute(&name, follow, tail, &db)?;
        }
        Commands::Measure {
            kernel,
            initrd,
            ovmf,
            cmdline,
            vcpus,
            vcpu_type,
            katana_repo,
            json,
        } => {
            katana_hypervisor::cli::measure::execute(
                kernel,
                initrd,
                ovmf,
                cmdline,
                vcpus,
                vcpu_type,
                katana_repo,
                json,
            )?;
        }
        Commands::Attest { name, json } => {
            katana_hypervisor::cli::attest::execute(&name, &db, json)?;
        }
    }

    Ok(())
}
