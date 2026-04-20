# duckman Configuration Guide

Configuration file location: **`$HOME/.duckdb/duckman.toml`**

---

## Top-level fields

```toml
# Default DuckDB version used when no version is specified.
# Can be overridden by the DUCKDB_VERSION environment variable at any time.
default = "v1.5.2"

[profile.myprofile]
# ... see below
```

| Field     | Type   | Description                              |
|-----------|--------|------------------------------------------|
| `default` | string | Default DuckDB version (e.g. `"v1.5.2"`) |
| `profile` | table  | Named profile definitions                |

---

## Profile overview

A **profile** bundles a set of DuckDB capabilities — extensions, secrets, environment
variables, attached databases, and DuckLake connections — into a named, reusable unit.

When a profile is activated, duckman translates each field into the appropriate DuckDB
startup commands (`-cmd`) or environment variables before launching duckdb.

```
duckman run v1.5.2 --profile myprofile
```

---

## Profile fields

### Basic info

```toml
[profile.analytics]
description = "Analytics workload with S3 and Delta Lake"
duckdb_version = "v1.5.2"   # optional — overrides the top-level default
init_sql = '''
'''
```

| Field            | Type   | Description                                                  |
|------------------|--------|--------------------------------------------------------------|
| `description`    | string | Human-readable description                                   |
| `duckdb_version` | string | Pin this profile to a specific DuckDB version                |
| `init_sql`       | string | SQL commands to execute at startup after all resources setup |

---

### extensions

List of extensions to load (or install if not yet present).

```toml
[profile.analytics]
extensions = ["json", "parquet", "httpfs", "delta", "aws"]
```

- Core extensions (e.g. `json`, `parquet`, `httpfs`) are installed with:
  `INSTALL <ext>;`
- Community extensions are installed with:
  `INSTALL <ext> FROM community;`
- Already-installed extensions are loaded with:
  `LOAD <ext>;`

Full list of core extensions: `duckman ext list --remote`

---

### environment

Environment variables injected before duckdb starts. Keys are uppercased automatically.

```toml
[profile.analytics.environment]
AWS_DEFAULT_REGION = "us-east-1"
HOME_DIR = "/data"
DUCKDB_NO_UNSIGNED_EXTENSIONS = "1"
```

Translates to shell environment: `AWS_DEFAULT_REGION=us-east-1 duckdb ...`

---

### secret

Named DuckDB secrets, converted to `CREATE OR REPLACE SECRET <name> (...)` statements
executed at startup.

Each key-value pair inside the secret table becomes a parameter. All values are
single-quoted strings unless you provide a raw `sql` override.

#### S3 / AWS

```toml
[profile.analytics.secret.my_s3]
type = "S3"
key_id = "AKIAIOSFODNN7EXAMPLE"
secret = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
region = "us-east-1"
```

Generated SQL:

```sql
CREATE
OR REPLACE SECRET my_s3 (
  type 'S3', 
  key_id 'AKIAIOSFODNN7EXAMPLE',
  secret 'wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY', 
  region 'us-east-1'
);
```

#### Azure

```toml
[profile.azure_profile.secret.my_azure]
type = "AZURE"
connection_string = "DefaultEndpointsProtocol=https;AccountName=...;AccountKey=...;"
```

---

### bucket

S3-compatible storage buckets, also converted to `CREATE OR REPLACE SECRET` statements.
Unlike `secret`, `type` and `provider` values are **not** quoted (DuckDB keyword syntax).

```toml
[profile.analytics.bucket.minio_local]
type = "S3"
key_id = "minioadmin"
secret = "minioadmin"
endpoint = "http://localhost:9000"
url_style = "path"
use_ssl = "false"

[profile.analytics.bucket.r2_store]
type = "S3"
key_id = "your-r2-access-key"
secret = "your-r2-secret-key"
endpoint = "https://<account>.r2.cloudflarestorage.com"
region = "auto"
```

Generated SQL (type/provider unquoted):

```sql
CREATE
OR REPLACE SECRET minio_local (
  type S3, key_id 'minioadmin', secret 'minioadmin',
  endpoint 'http://localhost:9000', url_style 'path', use_ssl 'false'
);
```

---

### parquet_key

A 256-bit encryption key for reading/writing encrypted Parquet files.

```toml
[profile.analytics]
parquet_key = "my-32-byte-encryption-key-here!!"
```

Generated SQL:

```sql
PRAGMA
add_parquet_key('key256', 'my-32-byte-encryption-key-here!!');
```

You can use `openssl rand --base64 32` to generate a random 256-bit key.

---

### attached databases

Databases to `ATTACH` at startup. Each entry is a named table under `[profile.<name>.attached.<db-name>]`.

| Field            | Type   | Description                                                   |
|------------------|--------|---------------------------------------------------------------|
| `path`           | string | File path, connection string, or `md:` URI for Motherduck     |
| `type`           | string | DB type: `sqlite`, `postgres`, `mysql`, `duckdb` … (optional) |
| `encryption_key` | string | Decryption key for encrypted DuckDB files (optional)          |
| `sql`            | string | Raw `ATTACH` SQL override (optional)                          |

#### SQLite

```toml
[profile.analytics.attached.app_db]
type = "sqlite"
path = "/var/data/app.sqlite"
```

Generated SQL: `ATTACH '/var/data/app.sqlite' AS app_db (type sqlite);`

#### PostgreSQL

```toml
[profile.analytics.attached.pg_prod]
type = "postgres"
path = "dbname=prod host=pg.internal user=analyst password=secret"
```

Generated SQL: `ATTACH 'dbname=prod host=pg.internal ...' AS pg_prod (type postgres);`

#### Encrypted DuckDB file

```toml
[profile.analytics.attached.secure_db]
path = "/data/sensitive.duckdb"
encryption_key = "my-aes-256-key-here"
```

Generated SQL: `ATTACH '/data/sensitive.duckdb' AS secure_db (ENCRYPTION_KEY 'my-aes-256-key-here');`

#### MotherDuck

```toml
[profile.analytics.environment]
MOTHERDUCK_TOKEN = "xxxx"

[profile.analytics.attached.md_warehouse]
path = "md:my_warehouse"
```

Generated SQL: `ATTACH 'md:my_warehouse' AS md_warehouse;`

#### Raw SQL override

```toml
[profile.analytics.attached.custom]
sql = "ATTACH 'host=pg.internal dbname=prod' AS custom (TYPE postgres, READ_ONLY)"
```

---

### ducklake

DuckLake catalogs to attach at startup.
Each entry maps a name to a catalog endpoint and data path.

| Field              | Type   | Description                             |
|--------------------|--------|-----------------------------------------|
| `catalog_endpoint` | string | Catalog service URL or DuckDB file path |
| `data_path`        | string | Data storage path (local or cloud)      |
| `sql`              | string | Raw `ATTACH` SQL override (optional)    |

#### Local DuckLake (SQLite catalog)

```toml
[profile.analytics.ducklake.local_lake]
catalog_endpoint = "/data/catalog.db"
data_path = "/data/lake"
```

Generated SQL: `ATTACH '/data/catalog.db' AS local_lake (DATA_PATH '/data/lake');`

#### Cloud DuckLake (catalog on S3)

```toml
[profile.analytics.ducklake.prod_lake]
catalog_endpoint = "ducklake:postgres:dbname=ducklake host=127.0.0.1 port=5432 user=ducklake password=123456"
data_path = "s3://my-bucket/lake-data"
```

Generated SQL:

```sql
ATTACH
'ducklake:postgres:dbname=ducklake host=127.0.0.1 port=5432 user=ducklake password=123456' AS prod_lake (DATA_PATH 's3://my-bucket/lake-data');
```

#### Raw SQL override

```toml
[profile.analytics.ducklake.custom_lake]
sql = "ATTACH 'ducklake:postgres:dbname=catalog' AS custom_lake (DATA_PATH 's3://bucket/data')"
```

---

## Complete example

```toml
# $HOME/.duckdb/duckman.toml

default = "v1.5.2"

# ── Analytics profile ──────────────────────────────────────────────────────────
[profile.analytics]
description = "Full analytics stack with S3, Postgres and DuckLake"
duckdb_version = "v1.5.2"
extensions = ["json", "parquet", "httpfs", "delta", "aws", "postgres"]
parquet_key = "my-32-byte-encryption-key-here!!"

[profile.analytics.environment]
AWS_DEFAULT_REGION = "us-east-1"
DUCKDB_NO_UNSIGNED_EXTENSIONS = "1"

[profile.analytics.secret.aws_prod]
type = "S3"
key_id = "AKIAIOSFODNN7EXAMPLE"
secret = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
region = "us-east-1"

[profile.analytics.bucket.minio]
type = "S3"
key_id = "minioadmin"
secret = "minioadmin"
endpoint = "http://localhost:9000"
url_style = "path"
use_ssl = "false"

[profile.analytics.attached.app_sqlite]
type = "sqlite"
endpoint = "/var/data/app.sqlite"

[profile.analytics.attached.pg_prod]
type = "postgres"
endpoint = "dbname=prod host=pg.internal user=analyst password=secret"

[profile.analytics.ducklake.prod_lake]
catalog_endpoint = "ducklake:s3://my-bucket/catalog.db"
data_path = "s3://my-bucket/lake-data"

# ── Lightweight in-memory profile ─────────────────────────────────────────────
[profile.memory]
description = "Quick scratch queries, no persistence"
extensions = ["json", "parquet"]
```

---

## Environment variable override

The `DUCKDB_VERSION` environment variable always takes precedence over both the
profile's `duckdb_version` and the top-level `default`:

```bash
DUCKDB_VERSION=v1.4.4 duckman run --profile analytics
```

---

## Profile activation order (inject_profile)

When a profile is activated, duckman injects startup commands in this order:

1. **extensions** — install (if missing) or load each extension
2. **environment** — export all key=value pairs as uppercase env vars
3. **secrets** — execute `CREATE OR REPLACE SECRET` for each entry
4. **buckets** — execute `CREATE OR REPLACE SECRET` for each entry
5. **parquet_key** — execute `PRAGMA add_parquet_key`
6. **attached** — execute `ATTACH` for each database
7. **ducklake** — execute `ATTACH` for each DuckLake catalog
