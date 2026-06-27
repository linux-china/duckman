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
  which       Print the absolute path of a DuckDB binary
  count       Count installed DuckDB versions and extensions(数鸭子)
  ext         Manage DuckDB extensions
  profile     Manage profiles
  snippet     Manage DuckDB snippets
  completion  Output auto-completion script for bash/zsh/fish/powershell
  help        Print this message or the help of the given subcommand(s)

Options:
  -d, --duckdb <duckdb>  Specify a DuckDB version or DUCKDB_VERSION env variable
  -h, --help             Print help
  -V, --version          Print version
```

## Duckman config

- duckman home: `$HOME/.duckdb`
- duckman versions: `$HOME/.duckdb/versions/`
- duckdb binary path: `$HOME/.duckdb/versions/xxxx/duckdb`
