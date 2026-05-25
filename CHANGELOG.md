<!-- Keep a Changelog guide -> https://keepachangelog.com -->

# Task Keeper Changelog

## [Unreleased]

## [0.1.4] - 2026-05-25

Core features:

1. Install/Uninstall DuckDB with different versions
2. List installed/remote DuckDB
3. Run duckdb with a specific version of DuckDB and profile
4. Extension Manager: install/uninstall/update/migrate extensions
5. Profile Manager: secrets, S3, required extensions etc.
6. Snippet manager: new/edit/list snippets from `~/.duckdb/snippets`
7. MotherDuck integration
8. DuckLake integration
9. Iceberg integration
10. dotenv support: `.env` autoload into environment variables for `getenv('XXX')`
11. Duckman shim: a wrapper for DuckDB executable, it is used to switch DuckDB version and profile
12. [dotenvx-rs](https://github.com/linux-china/dotenvx-rs) integration to encrypt sensitive data in duckman.toml



