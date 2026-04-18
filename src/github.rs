use serde::Deserialize;

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
