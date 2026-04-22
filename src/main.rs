use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    name: String,
    
    #[arg(short, long, default_value = "")]
    directory: String,
    
    #[arg(short, long, default_value_t = false)]
    private: bool,
    
    #[arg(short, long, default_value_t = false)]
    auto_init: bool,
}

#[derive(Serialize)]
struct CreateRepoRequest {
    name: String,
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
    private: bool,
    auto_init: bool,
) -> anyhow::Result<CreateRepoResponse> {
    let request = CreateRepoRequest {
        name: name.to_string(),
        private,
        auto_init,
    };

    let response = client
        .post("https://api.github.com/user/repos")
        .header("User-Agent", "github-init-cli")
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

fn create_local_repo(directory: &str, name: &str) -> anyhow::Result<()> {
    // Combine directory and name as final path
    let repo_path = if !directory.is_empty() {
        format!("{}/{}", directory, name)
    } else {
        name.to_string()
    };

    // Create directory
    fs::create_dir_all(&repo_path)?;

    // Initialize git repository
    let output = Command::new("git")
        .arg("init")
        .current_dir(&repo_path)
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Git init failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    // Create empty commit
    let output = Command::new("git")
        .arg("commit")
        .arg("--allow-empty")
        .arg("-m")
        .arg("Initial commit")
        .current_dir(&repo_path)
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Git commit failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    println!("Local repository created at: {}", repo_path);
    Ok(())
}

fn sync_to_remote(repo_path: &str, remote_url: &str) -> anyhow::Result<()> {
    // Add remote
    let output = Command::new("git")
        .arg("remote")
        .arg("add")
        .arg("origin")
        .arg(remote_url)
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Git remote add failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    // Push to main branch
    let output = Command::new("git")
        .arg("push")
        .arg("-u")
        .arg("origin")
        .arg("main")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Git push failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    println!("Repository synced to remote: {}", remote_url);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let args = Args::parse();

    // Calculate repo path
    let repo_path = if !args.directory.is_empty() {
        format!("{}/{}", args.directory, args.name)
    } else {
        args.name.to_string()
    };

    // Step 1: Create local repository
    println!("Creating local repository...");
    create_local_repo(&args.directory, &args.name)?;

    // Step 2: Create remote repository
    println!("\nCreating remote repository...");
    let token = get_github_token()?;
    let client = Client::new();
    
    let repo = create_github_repo(
        &client,
        &token,
        &args.name,
        args.private,
        args.auto_init,
    )
    .await?;

    // Step 3: Sync to remote
    println!("\nSyncing to remote repository...");
    sync_to_remote(&repo_path, &repo.html_url)?;

    println!("\nRepository created successfully!");
    println!("Name: {}", repo.name);
    println!("URL: {}", repo.html_url);
    println!("Private: {}", repo.private);

    Ok(())
}
