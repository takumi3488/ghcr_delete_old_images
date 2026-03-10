use std::env;

use reqwest::{header, Client, Response};
use serde_json::Value;

const BASE_URL: &str = "https://api.github.com";

struct GithubClient {
    client: Client,
}

impl GithubClient {
    fn new(token: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", "application/vnd.github+json".parse()?);
        headers.insert("Authorization", format!("Bearer {token}").parse()?);
        headers.insert("X-Github-Api-Version", "2022-11-28".parse()?);
        headers.insert("User-Agent", "reqwest".parse()?);
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self { client })
    }

    async fn get(&self, path: &str) -> Result<Response, reqwest::Error> {
        self.client
            .get(format!("{BASE_URL}{path}"))
            .send()
            .await
    }

    async fn delete(&self, path: &str) -> Result<Response, reqwest::Error> {
        self.client
            .delete(format!("{BASE_URL}{path}"))
            .send()
            .await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gh_token = env::var("GH_TOKEN").map_err(|_| "GH_TOKEN is not set")?;
    let client = GithubClient::new(&gh_token)?;

    // パッケージ一覧の取得
    let response = client
        .get("/user/packages?package_type=container")
        .await
        .map_err(|e| format!("failed to get packages: {e}"))?;
    let packages: Value = serde_json::from_str(
        &response
            .text()
            .await
            .map_err(|e| format!("failed to read packages response: {e}"))?,
    )
    .map_err(|e| format!("failed to parse packages response: {e}"))?;

    let packages_array = packages
        .as_array()
        .ok_or("packages response is not an array")?;

    for package in packages_array {
        let package_name = package["name"]
            .as_str()
            .ok_or("package name is not a string")?;

        // パッケージのバージョン一覧の取得
        let response: Response = client
            .get(&format!(
                "/user/packages/container/{package_name}/versions",
            ))
            .await
            .map_err(|e| format!("failed to get versions for {package_name}: {e}"))?;
        let versions_res: Value = serde_json::from_str(
            &response
                .text()
                .await
                .map_err(|e| format!("failed to read versions response: {e}"))?,
        )
        .map_err(|e| format!("failed to parse versions response: {e}"))?;
        let mut versions = versions_res
            .as_array()
            .ok_or("versions response is not an array")?
            .clone();

        // 更新日時でソート
        versions.sort_by(|a, b| {
            let a = a["updated_at"].as_str().unwrap_or("");
            let b = b["updated_at"].as_str().unwrap_or("");
            b.cmp(a)
        });

        // 最新以外を削除
        for version in versions.iter().skip(1) {
            let version_id = version["id"]
                .as_u64()
                .ok_or("version id is not a u64")?;
            client
                .delete(&format!(
                    "/user/packages/container/{package_name}/versions/{version_id}",
                ))
                .await
                .map_err(|e| format!("failed to delete {version_id}: {e}"))?;
        }
    }

    Ok(())
}
