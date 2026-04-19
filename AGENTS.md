# Project overview

Duckman is a DuckDB version manager and toolchain CLI.

## Tech stack

1. Clap.rs: CLI interface
2. reqwest: HTTP client
3. tokio: Asynchronous runtime
4. indicatif: download progress

## Duckman CLI help

```
duckman - a DuckDB version manager and toolchain CLI

Usage: duckman [OPTIONS] [COMMAND]

Commands:
  list        list installed/remote DuckDB versions
  install     Install DuckDB
  uninstall   Uninstall DuckDB
  run         Run a specific version of DuckDB
  default     Set a version of DuckDB as default one to use
  ext         Manage DuckDB extensions
  extension   Manage DuckDB extensions
  profile     Manage profiles
  completion  Output auto-completion script for bash/zsh/fish/powershell
  help        Print this message or the help of the given subcommand(s)

Flags:
  -d, --duckdb <duckdb>  DuckDB version
  -h, --help             Print help
  -V, --version          Print version

Use "duckman [command] --help" for more information about a command.
```

## Duckman config

- duckman home: `$HOME/.duckdb`
- duckman versions: `$HOME/.duckdb/versions/`
- duckdb binary path: `$HOME/.duckdb/versions/xxxx/duckdb`
