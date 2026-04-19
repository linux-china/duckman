use clap::{Arg, ArgAction, Command};

pub const VERSION: &str = "0.1.0";

pub fn build_duckman_app() -> Command {
    let run_command = Command::new("run")
        .about("Run a specific version of DuckDB")
        .arg(
            Arg::new("profile")
                .long("profile")
                .help("Profile to run")
                .num_args(1)
                .required(false),
        )
        .arg(
            Arg::new("version")
                .help("DuckDB version to run (default: $DUCKDB_VERSION or configured default)")
                .index(1)
                .required(false),
        );
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
    // extension manager (ext)
    let ext_list = Command::new("list").about("List extensions").arg(
        Arg::new("remote")
            .long("remote")
            .help("List available core and community extensions")
            .action(ArgAction::SetTrue),
    );
    let ext_install = Command::new("install").about("Install an extension").arg(
        Arg::new("name")
            .help("Extension name")
            .index(1)
            .num_args(1)
            .required(true),
    );
    let ext_uninstall = Command::new("uninstall")
        .about("Uninstall an extension")
        .arg(
            Arg::new("name")
                .help("Extension name")
                .index(1)
                .num_args(1)
                .required(true),
        );
    let ext_update = Command::new("update").about("Update all installed extensions");
    let ext_command = Command::new("ext")
        .about("Manage DuckDB extensions")
        .subcommand_required(true)
        .subcommand(ext_list)
        .subcommand(ext_install)
        .subcommand(ext_uninstall)
        .subcommand(ext_update);
    let extension_command = Command::new("extension").about("Manage DuckDB extensions");
    // profile manager
    let profile_list = Command::new("list").about("List all profiles");
    let profile_command = Command::new("profile")
        .about("Manage profiles")
        .subcommand_required(true)
        .subcommand(profile_list);
    // completion
    let completion_command = Command::new("completion")
        .about("Output auto-completion script for bash/zsh/fish/powershell")
        .arg(
            Arg::new("shell")
                .long("shell")
                .help("shell name: bash, zsh, fish, powershell")
                .value_parser(["bash", "zsh", "first", "powershell"])
                .num_args(1)
                .required(false),
        );
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
        .subcommand(ext_command)
        .subcommand(extension_command)
        .subcommand(profile_command)
        .subcommand(completion_command)
}
