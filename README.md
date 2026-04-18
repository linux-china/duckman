Duckman: DuckDB version manager and toolchain
=============================================

Duckman(赶鸭人) is a DuckDB version manager and toolchain CLI.

![Duckman](duckman.jpg)

Features:

1. Install/Uninstall DuckDB with different versions
2. List installed/remote DuckDB
3. Run duckdb with a specific version of DuckDB and profile
4. Extension Manager
5. Profile Manager: secrets, S3, required extensions etc.
6. MotherDuck integration
7. DuckLake integration
8. Iceberg integration

## profiles

name, description, default duckdb version

- basic info: name, description, DuckDB version
- required extensions
- secrets
- environment variables
- S3 bucket list
- attached db
- ducklake

If profile name is default, and it means that this profile will be used as the default profile when running DuckDB.

# extension sub command

- install/uninstall extension
- list extensions
- init: create a new extension with Rust/C++

# Environment variable

- DUCKDB_VERSION: default DuckDB version to use
- DUCKDB_PROFILE: default profile to run DuckDB

# MontherDuck

DuckDB version: md

- DuckDB version for md:  specified by MotherDuck

# References

* DuckDB: https://duckdb.org/
* DuckLake: https://ducklake.select/
* MotherDuck: https://motherduck.com/