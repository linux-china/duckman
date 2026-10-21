use crate::commands::completion_command;
use crate::duckman_app::build_duckman_app;
use crate::duckman_config::DuckmanConfig;
use std::env;
use std::ffi::OsString;

mod commands;
mod duckman_app;
mod duckman_config;
mod ext_commands;
mod github;
mod profile_commands;
pub mod runner;
mod snippet_commands;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    dotenvx_rs::dotenv().ok();
    let matches = build_duckman_app().get_matches();

    // global duckdb version from `-d/--duckdb`, passed explicitly to commands
    let duckdb = matches.get_one::<String>("duckdb").map(|s| s.as_str());
    // sub command match
    match matches.subcommand() {
        Some(("list", m)) => {
            let local = m.get_flag("local");
            let remote = m.get_flag("remote");
            commands::list_versions(local, remote).await?;
        }
        Some(("install", m)) => {
            let version = m.get_one::<String>("version").unwrap();
            commands::install_version(version).await?;
        }
        Some(("uninstall", m)) => {
            let version = m.get_one::<String>("version").unwrap();
            commands::uninstall_version(version).await?;
        }
        Some(("run", m)) => {
            let profile = m.get_one::<String>("profile").map(|s| s.as_str());
            // Collect everything after `--` from the raw process args
            let raw: Vec<String> = env::args().collect();
            let extra_args: Vec<String> = raw
                .iter()
                .skip_while(|a| *a != "--")
                .skip(1)
                .cloned()
                .collect();
            commands::run_duckdb(duckdb, profile, extra_args).await?;
        }
        Some(("which", m)) => {
            let version = m.get_one::<String>("version").map(|s| s.as_str());
            commands::which_duckdb(version, duckdb)?;
        }
        Some(("count", _)) => {
            commands::count_versions()?;
        }
        Some(("default", m)) => {
            if let Some(version) = m.get_one::<String>("version") {
                commands::set_default_version(version)?;
            } else {
                let config = DuckmanConfig::load()?;
                if let Some(default_version) = config.default {
                    println!("Default DuckDB version: {}", default_version);
                } else {
                    println!("No default DuckDB version");
                }
            }
        }
        Some(("ext", m)) => match m.subcommand() {
            Some(("list", sm)) => {
                let remote = sm.get_flag("remote");
                ext_commands::list_extensions(remote, duckdb)?;
            }
            Some(("install", sm)) => {
                let name = sm.get_one::<String>("name").unwrap();
                ext_commands::install_extension(name, duckdb).await?;
            }
            Some(("uninstall", sm)) => {
                let name = sm.get_one::<String>("name").unwrap();
                ext_commands::uninstall_extension(name, duckdb)?;
            }
            Some(("update", _)) => {
                ext_commands::update_extensions(duckdb)?;
            }
            Some(("migrate", sm)) => {
                let version = sm.get_one::<String>("version").unwrap();
                ext_commands::migrate_extensions(version, duckdb)?;
            }
            _ => unreachable!(),
        },
        Some(("profile", m)) => match m.subcommand() {
            Some(("list", _)) => profile_commands::list_profiles()?,
            Some(("dump", sm)) => {
                let name = sm.get_one::<String>("name").unwrap();
                profile_commands::dump_profile(name)?;
            }
            _ => unreachable!(),
        },
        Some(("snippet", m)) => match m.subcommand() {
            Some(("list", _)) => snippet_commands::list_snippets()?,
            Some(("show", sm)) => {
                let name = sm.get_one::<String>("name").unwrap();
                snippet_commands::show_snippet(name)?;
            }
            Some(("edit", sm)) => {
                let name = sm.get_one::<String>("name").unwrap();
                snippet_commands::edit_snippet(name)?;
            }
            _ => unreachable!(),
        },
        Some(("completion", sub_command_matches)) => {
            completion_command(sub_command_matches);
        }
        _ => {
            build_duckman_app().print_help()?;
        }
    }

    Ok(())
}

fn get_sub_command() -> Option<String> {
    let raw_args: Vec<OsString> = env::args_os().collect();
    // get sub command name
    let mut sub_command_name = "".to_owned();
    if raw_args.len() > 1 {
        let arg_1 = raw_args[1].clone().to_str().unwrap().to_owned();
        if arg_1.starts_with('-') {
            if raw_args.len() > 3 {
                sub_command_name = raw_args[3].clone().to_str().unwrap().to_owned();
            }
        } else {
            sub_command_name = arg_1;
        }
    }
    if sub_command_name == "" {
        return None;
    }
    Some(sub_command_name)
}
