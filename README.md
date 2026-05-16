Duckman: DuckDB version manager and toolchain
=============================================

Duckman(赶鸭人) is a DuckDB version manager and toolchain CLI.

![Duckman](duckman.jpg)

Features:

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

## profiles

Profile is a collection of settings to run DuckDB.

- basic info: name, description, DuckDB version
- required extensions
- secrets
- environment variables
- S3 bucket list
- attached db
- ducklake
- parquet key

If profile name is `default`, and it means that this profile will be used as the default profile when running DuckDB.

Fore more information about profile, please refer [Duckman Config](duckman-config.md).

# extension sub command

- install/uninstall extension
- list extensions
- init: create a new extension with Rust/C++

# Environment variable

- `DUCKDB_VERSION`: default DuckDB version to use
- `DUCKDB_PROFILE`: default profile to run DuckDB

# FAQ

### How does Duckman choose DuckDB version?

- option first: `--duckdb 1.5.2`
- environment variable: `DUCKDB_VERSION`
- profile default: `duckdb_version` in profile
- global default: `default` in config file

Or you can create `.env` file with environment variables:

```
DUCKDB_VERSION=v1.5.2
```

### How to migrate extensions from DuckDB v1.4.4 to v1.5.2?

Migration is not real one, just install the extensions on new DuckDB version.

```shell
$ duckman --duckdb 1.5.2 ext migrate 1.4.4
```

### How to install DuckDB for Duckman from local zip file?

```
$ unzip -d $HOME/.duckdb/versions/v1.5.2 duckdb_cli-osx-amd64.zip
```

### How to install DuckDB from local path?

```
$ duckman install ~/Downloads/duckdb
```

### How to install DuckDB from PATH environment?

```
$ duckman install system
```

### How to manage snippets?

Create snippet Markdown file under `~/.duckdb/snippets` directory, and content as following:

~~~markdown
---
summary: Filter column names using a pattern
tags: [sql]
---

```sql
-- select only the column names that start with the dim_
SELECT COLUMNS('^dim_') FROM fact_table;
```
~~~

Then use `duckman snippet list` or `duckman snippet show <name>` to list or show snippet.

### How does Duckman choose profile?

- option first: `--profile xxx`
- environment variable: `DUCKDB_PROFILE`

Or you can create `.env` file with environment variables:

```
DUCKDB_PROFILE=xxx
```

### What is Duckman shim?

Duckman shim is a wrapper for DuckDB executable, it is used to switch DuckDB version and profile.

```
$ ln -s /path/to/duckman.shim ~/bin/duckdb
$ ~/bin/duckdb --version
```

### How to add MontherDuck support?

```toml
[profile.analytics.environment]
MOTHERDUCK_TOKEN = "xxxx"

[profile.analytics.attached.mydb]
path = "md:mydb"
```

### How to add Quack server support

Add `quack` extension and init_sql to start quack server.

```toml
# profile of polyglot
[profile.polyglot]
description = "profile name"
duckdb_version = "v1.5.2"
extensions = ["parquet","quack"]
init_sql = '''
CALL quack_serve('quack:0.0.0.0:9494', token = 'super_secret', allow_other_hostname => true);
'''
```

### How to add Quack client support?

```toml
[profile.analytics.attached.remote_db]
type = "quack"
path = "quack:localhost"
options = { TOKEN = "super_secret" }
```

### How to add Iceberg support?

```toml
[profile.analytics.secret.iceberg_secret]
type = "iceberg"
token = "bearer_token"

[profile.analytics.attached.myberg]
type = "iceberg"
path = "warehouse"
options = { SECRET = "iceberg_secret", ENDPOINT = "https://rest_endpoint.com" }
```

### How to manage snippets?

- `duckman snippet edit <name>`: edit/create snippet
- `duckman snippet list`: list all snippets
- `duckman snippet show <name>`: show snippet

# References

* DuckDB: https://duckdb.org/
* DuckLake: https://ducklake.select/
* MotherDuck: https://motherduck.com/