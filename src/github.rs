use crate::duckman_config::DuckmanConfig;
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
    // Fetch release metadata from GitHub
    println!("Fetching release info for {}...", version);
    let release = fetch_release(&version).await?;
    let release = match release {
        Some(r) => r,
        None => {
            bail!("Version {} not found on GitHub releases.", version);
        }
    };

    let asset_name = platform_asset_name();
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

    let pb = ProgressBar::new(asset.size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} Downloading [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({binary_bytes_per_sec}, {eta})",
            )?
            .progress_chars("=>-"),
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "Duckman/0.1.0")
        .send()
        .await?;

    let mut stream = response.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(asset.size as usize);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        pb.inc(chunk.len() as u64);
        buf.extend_from_slice(&chunk);
    }
    pb.finish_and_clear();
    let bytes = buf;

    // Extract binary from zip
    println!("Extracting...");
    let cursor = Cursor::new(bytes.to_vec());
    let mut archive = zip::ZipArchive::new(cursor)?;

    let version_dir = DuckmanConfig::version_dir(&version);
    fs::create_dir_all(&version_dir)?;
    let binary_path = DuckmanConfig::version_binary(&version);

    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_owned();
        // Match "duckdb" or "duckdb.exe" anywhere in the archive path
        let base = name.split('/').last().unwrap_or(&name);
        if base == "duckdb" || base == "duckdb.exe" {
            let mut out = fs::File::create(&binary_path)?;
            std::io::copy(&mut entry, &mut out)?;
            found = true;
            break;
        }
    }

    if !found {
        fs::remove_dir_all(&version_dir)?;
        bail!("Could not find duckdb binary inside the downloaded archive.");
    }
    Ok(())
}

fn platform_asset_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "duckdb_cli-osx-universal.zip"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "duckdb_cli-linux-aarch64.zip"
    } else if cfg!(target_os = "linux") {
        "duckdb_cli-linux-amd64.zip"
    } else if cfg!(target_os = "windows") {
        "duckdb_cli-windows-amd64.zip"
    } else {
        "duckdb_cli-linux-amd64.zip"
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
    if response.status().is_success() {
        Ok(Some(response.json::<GitHubRelease>().await?))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

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
