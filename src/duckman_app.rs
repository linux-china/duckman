use clap::{Arg, ArgAction, Command};

pub const VERSION: &str = "0.1.0";

pub fn build_duckman_app() -> Command {
    let run_command = Command::new("run").about("run a specific version of DuckDB");
    let list_command = Command::new("list")
        .about("list DuckDB versions")
        .arg(
            Arg::new("local")
                .long("local")
                .help("Local installed versions")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("remote")
                .long("remote")
                .help("List remote versions")
                .action(ArgAction::SetTrue),
        );
    let install_command = Command::new("install").about("Install DuckDB").arg(
        Arg::new("version")
            .help("DuckDB version to install")
            .index(1)
            .num_args(1)
            .required(true),
    );
    let uninstall_command = Command::new("uninstall").about("Uninstall DuckDB").arg(
        Arg::new("version")
            .help("DuckDB version to uninstall")
            .index(1)
            .num_args(1)
            .required(true),
    );

    let default_command = Command::new("default")
        .about("Set a version of DuckDB as default one to use")
        .arg(
            Arg::new("version")
                .help("DuckDB version as default")
                .index(1)
                .num_args(1)
                .required(true),
        );
    // extension manager
    let extension_command = Command::new("extension").about("Manage DuckDB extensions");
    let profile_command = Command::new("profile").about("Manage profiles");
    Command::new("duckman")
        .version(VERSION)
        .author("linux_china <libing.chen@gmail.com>")
        .about("duckman - a DuckDB version manager and toolchain CLI")
        .arg(
            Arg::new("duckdb")
                .short('d')
                .long("duckdb")
                .help("DuckDB version")
                .num_args(1)
                .required(false),
        )
        .subcommand(list_command)
        .subcommand(install_command)
        .subcommand(uninstall_command)
        .subcommand(run_command)
        .subcommand(default_command)
        .subcommand(extension_command)
        .subcommand(profile_command)
}
