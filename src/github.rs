use crate::duckman_config::{DuckmanConfig, normalize_duckdb_version};
use anyhow::bail;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::fs;
use std::io::Cursor;

#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub published_at: Option<String>,
    pub prerelease: bool,
    pub draft: bool,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

impl GitHubRelease {
    pub fn find_asset(&self, asset_name: &str) -> Option<&GitHubAsset> {
        self.assets.iter().find(|a| a.name == asset_name)
    }
}

pub async fn download_duckdb(version: &str) -> anyhow::Result<()> {
    let version = normalize_duckdb_version(version);
    // Fetch release metadata from GitHub
    eprintln!("Fetching release info for {}...", version);
    let release = fetch_release(&version).await?;
    let release = match release {
        Some(r) => r,
        None => {
            bail!("Version {} not found on GitHub releases.", version);
        }
    };

    let asset_name = platform_asset_name(&version);
    let asset = match release.find_asset(asset_name) {
        Some(a) => a,
        None => {
            bail!(
                "No binary found for your platform (expected asset: {}). \
                 Available assets: {}",
                asset_name,
                release
                    .assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    };

    // Print download url
    eprintln!("Begin to download from {}", &asset.browser_download_url);
    let client = reqwest::Client::new();
    let response = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "Duckman/0.1.0")
        .send()
        .await?
        .error_for_status()?;
    // Prefer the actual Content-Length over the size reported by the release metadata
    let total_size = response.content_length().unwrap_or(asset.size);

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} Downloading [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({binary_bytes_per_sec}, {eta})",
            )?
            .progress_chars("=>-"),
    );

    let mut stream = response.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(total_size as usize);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        pb.inc(chunk.len() as u64);
        buf.extend_from_slice(&chunk);
    }
    pb.finish_and_clear();
    let bytes = buf;

    // Extract binary from zip
    eprintln!("Extracting...");
    let version_dir = DuckmanConfig::version_dir(&version);
    fs::create_dir_all(&version_dir)?;
    let binary_path = DuckmanConfig::version_binary(&version);

    // Extract to a temp file first, then rename, so an interrupted install never leaves a partial binary
    let tmp_path = binary_path.with_extension("tmp");
    let result = if asset_name.ends_with(".tar.gz") {
        extract_binary_from_tar_gz(&bytes, &tmp_path, &binary_path)
    } else {
        zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(anyhow::Error::from)
            .and_then(|mut archive| extract_binary(&mut archive, &tmp_path, &binary_path))
    };
    if let Err(e) = result {
        let _ = fs::remove_dir_all(&version_dir);
        return Err(e);
    }

    Ok(())
}

fn extract_binary_from_tar_gz(
    bytes: &[u8],
    tmp_path: &std::path::Path,
    binary_path: &std::path::Path,
) -> anyhow::Result<()> {
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder);
    let mut found = false;
    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path()?;
        // Match "duckdb" or "duckdb.exe" anywhere in the archive path
        let base = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if base == "duckdb" || base == "duckdb.exe" {
            let mut out = fs::File::create(tmp_path)?;
            std::io::copy(&mut entry, &mut out)?;
            out.sync_all()?;
            found = true;
            break;
        }
    }

    if !found {
        bail!("Could not find duckdb binary inside the downloaded archive.");
    }

    finalize_binary(tmp_path, binary_path)
}

fn extract_binary<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    tmp_path: &std::path::Path,
    binary_path: &std::path::Path,
) -> anyhow::Result<()> {
    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_owned()?;
        // Match "duckdb" or "duckdb.exe" anywhere in the archive path
        let base = name.split('/').last().unwrap_or(&name);
        if base == "duckdb" || base == "duckdb.exe" {
            let mut out = fs::File::create(tmp_path)?;
            std::io::copy(&mut entry, &mut out)?;
            out.sync_all()?;
            found = true;
            break;
        }
    }

    if !found {
        bail!("Could not find duckdb binary inside the downloaded archive.");
    }

    finalize_binary(tmp_path, binary_path)
}

/// Make the extracted binary executable and move it into place
fn finalize_binary(tmp_path: &std::path::Path, binary_path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(tmp_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(tmp_path, perms)?;
    }

    fs::rename(tmp_path, binary_path)?;
    Ok(())
}

fn platform_asset_name(version: &str) -> &'static str {
    if version.starts_with("v2.0") {
        return platform_asset_name_2();
    }
    if cfg!(target_os = "macos") && !version.starts_with("v1.5") {
        // use universal binary for < 1.5
        "duckdb_cli-osx-universal.zip"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "duckdb_cli-osx-arm64.zip"
    } else if cfg!(target_os = "macos") {
        "duckdb_cli-osx-amd64.zip"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "duckdb_cli-linux-arm64.zip"
    } else if cfg!(target_os = "linux") {
        "duckdb_cli-linux-amd64.zip"
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "aarch64") {
        "duckdb_cli-windows-arm64.zip"
    } else if cfg!(target_os = "windows") {
        "duckdb_cli-windows-amd64.zip"
    } else {
        "duckdb_cli-linux-amd64.zip"
    }
}

fn platform_asset_name_2() -> &'static str {
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "duckdb-cli-osx-arm64.tar.gz"
    } else if cfg!(target_os = "macos") {
        "duckdb-cli-osx-amd64.tar.gz"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "duckdb-cli-linux-arm64.tar.gz"
    } else if cfg!(target_os = "linux") {
        "duckdb-cli-linux-amd64.tar.gz"
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "aarch64") {
        "duckdb-cli-windows-arm64.tar.gz"
    } else if cfg!(target_os = "windows") {
        "duckdb-cli-windows-amd64.tar.gz"
    } else {
        "duckdb-cli-linux-amd64.tar.gz"
    }
}

fn build_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::new())
}

fn add_auth(mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    req
}

pub async fn fetch_releases() -> anyhow::Result<Vec<GitHubRelease>> {
    let client = build_client()?;
    let req = client
        .get("https://api.github.com/repos/duckdb/duckdb/releases")
        .header("User-Agent", "Duckman/0.1.0")
        .header("Accept", "application/vnd.github.v3+json");
    let releases = add_auth(req)
        .send()
        .await?
        .json::<Vec<GitHubRelease>>()
        .await?;
    Ok(releases)
}

pub async fn fetch_release(version: &str) -> anyhow::Result<Option<GitHubRelease>> {
    let url = format!(
        "https://api.github.com/repos/duckdb/duckdb/releases/tags/{}",
        version
    );
    let client = build_client()?;
    let req = client
        .get(&url)
        .header("User-Agent", "duckman/0.1.0")
        .header("Accept", "application/vnd.github.v3+json");
    let response = add_auth(req).send().await?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let hint = if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            " (GitHub API rate limit may be exceeded, set GITHUB_TOKEN to raise the limit)"
        } else {
            ""
        };
        bail!(
            "Failed to fetch release {} from GitHub: HTTP {}{}\n{}",
            version,
            status,
            hint,
            body.trim()
        );
    }
    Ok(Some(response.json::<GitHubRelease>().await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    #[test]
    fn test_extract_binary_from_tar_gz() -> TestResult {
        let content = b"fake duckdb binary";
        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::default(),
        ));
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append_data(&mut header, "duckdb", &content[..])?;
        let bytes = builder.into_inner()?.finish()?;

        let dir = std::env::temp_dir().join("duckman-test-tar-gz");
        fs::create_dir_all(&dir)?;
        let binary_path = dir.join("duckdb");
        extract_binary_from_tar_gz(&bytes, &binary_path.with_extension("tmp"), &binary_path)?;
        assert_eq!(fs::read(&binary_path)?, content);
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_fetch_releases() -> TestResult {
        let releases = fetch_releases().await?;
        assert!(!releases.is_empty());
        for release in releases {
            println!(
                "{},{}",
                release.tag_name,
                release.published_at.as_ref().unwrap()
            );
        }
        Ok(())
    }
}
