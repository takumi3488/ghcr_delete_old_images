use std::env;

use reqwest::{header, Client};
use serde_json::Value;

const BASE_URL: &'static str = "https://api.github.com";

struct GithubClient {
    client: Client,
}

impl GithubClient {
    fn new(token: String) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", "application/vnd.github+json".parse().unwrap());
        headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        headers.insert("X-Github-Api-Version", "2022-11-28".parse().unwrap());
        headers.insert("User-Agent", "reqwest".parse().unwrap());
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self { client }
    }

    async fn get(&self, path: &str) -> Result<String, reqwest::Error> {
        self.client
            .get(format!("{}{}", BASE_URL, path))
            .send()
            .await
            .unwrap()
            .text()
            .await
    }

    async fn delete(&self, path: &str) -> Result<String, reqwest::Error> {
        self.client
            .delete(format!("{}{}", BASE_URL, path))
            .send()
            .await
            .unwrap()
            .text()
            .await
    }
}

#[tokio::main]
async fn main() {
    let gh_token = env::var("GH_TOKEN").expect("GH_TOKEN is not set");
    let client = GithubClient::new(gh_token);

    // パッケージ一覧の取得
    let response = client
        .get("/user/packages?package_type=container&per_page=100")
        .await
        .unwrap();
    println!("{}", response);
    let packages: Value = serde_json::from_str(&response).unwrap();

    for package in packages.as_array().unwrap() {
        // パッケージのバージョン一覧の取得
        let response = client
            .get(&format!(
                "/user/packages/container/{}/versions",
                package["name"].as_str().unwrap()
            ))
            .await
            .unwrap();
        let versions_res: Value = serde_json::from_str(&response).unwrap();
        let mut versions = versions_res.as_array().unwrap().clone();

        // 更新日時でソート
        versions.sort_by(|a, b| {
            let a = a["updated_at"].as_str().unwrap();
            let b = b["updated_at"].as_str().unwrap();
            b.cmp(a)
        });

        // 最新以外を削除
        for version in versions.iter().skip(1) {
            client
                .delete(&format!(
                    "/user/packages/container/{}/versions/{}",
                    package["name"].as_str().unwrap(),
                    version["id"].as_u64().unwrap()
                ))
                .await
                .expect(format!("failed to delete {}", version["id"].as_u64().unwrap()).as_str());
        }
        break;
    }
}
