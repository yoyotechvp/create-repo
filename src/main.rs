use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    name: String,
    
    #[arg(short, long, default_value = "")]
    description: String,
    
    #[arg(short, long, default_value_t = false)]
    private: bool,
    
    #[arg(short, long, default_value_t = false)]
    auto_init: bool,
}

#[derive(Serialize)]
struct CreateRepoRequest {
    name: String,
    description: String,
    private: bool,
    auto_init: bool,
}

#[derive(Deserialize, Debug)]
struct CreateRepoResponse {
    id: u64,
    name: String,
    html_url: String,
    private: bool,
}

async fn create_github_repo(
    client: &Client,
    token: &str,
    name: &str,
    description: &str,
    private: bool,
    auto_init: bool,
) -> anyhow::Result<CreateRepoResponse> {
    let request = CreateRepoRequest {
        name: name.to_string(),
        description: description.to_string(),
        private,
        auto_init,
    };

    let response = client
        .post("https://api.github.com/user/repos")
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to create repo: {}", response.text().await?));
    }

    let repo = response.json::<CreateRepoResponse>().await?;
    Ok(repo)
}

fn get_github_token() -> anyhow::Result<String> {
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        return Ok(token);
    }

    Err(anyhow::anyhow!("GITHUB_TOKEN environment variable not set"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let args = Args::parse();
    let token = get_github_token()?;
    let client = Client::new();

    println!("Creating GitHub repository '{}'...", args.name);
    
    let repo = create_github_repo(
        &client,
        &token,
        &args.name,
        &args.description,
        args.private,
        args.auto_init,
    )
    .await?;

    println!("Repository created successfully!");
    println!("Name: {}", repo.name);
    println!("URL: {}", repo.html_url);
    println!("Private: {}", repo.private);

    Ok(())
}
