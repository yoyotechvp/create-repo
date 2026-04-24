use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process::Command;
use std::path::Path;

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

// 记录已执行的步骤，用于回滚
#[derive(Debug)]
enum ExecutedStep {
    LocalRepoCreated(String),  // 本地仓库路径
    RemoteRepoCreated(String), // 远程仓库名称
    RemoteAdded(String, String), // 本地路径和远程URL
}

// 检查本地仓库是否已存在且为 git 仓库
fn check_local_repo(directory: &str, name: &str) -> anyhow::Result<bool> {
    let repo_path = if !directory.is_empty() {
        format!("{}/{}", directory, name)
    } else {
        name.to_string()
    };

    let repo_path = Path::new(&repo_path);
    if !repo_path.exists() {
        return Ok(false);
    }

    // 检查是否为 git 仓库
    let git_dir = repo_path.join(".git");
    if !git_dir.exists() || !git_dir.is_dir() {
        return Ok(false);
    }

    // 检查是否有 main 分支
    let output = Command::new("git")
        .arg("branch")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Ok(false);
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    Ok(output_str.contains("main"))
}

// 检查远程仓库是否存在
async fn check_remote_repo(
    client: &Client,
    token: &str,
    name: &str,
) -> anyhow::Result<bool> {
    let response = client
        .get(format!("https://api.github.com/repos/{}/{}", env::var("USER").unwrap_or_default(), name))
        .header("User-Agent", "github-init-cli")
        .header("Authorization", format!("token {}", token))
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    Ok(response.status().is_success())
}

// 检查是否已添加远程并推送
fn check_sync_status(repo_path: &str, remote_url: &str) -> anyhow::Result<bool> {
    // 检查是否已添加远程
    let output = Command::new("git")
        .arg("remote")
        .arg("-v")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        return Ok(false);
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    if !output_str.contains("origin") || !output_str.contains(remote_url) {
        return Ok(false);
    }

    // 检查是否已推送
    let output = Command::new("git")
        .arg("log")
        .arg("--oneline")
        .arg("origin/main")
        .current_dir(repo_path)
        .output()?;

    Ok(output.status.success())
}

// 回滚已执行的步骤
fn rollback(steps: &[ExecutedStep]) -> anyhow::Result<()> {
    println!("\nRolling back changes...");

    // 反向遍历步骤，从最后一个开始回滚
    for step in steps.iter().rev() {
        match step {
            ExecutedStep::LocalRepoCreated(path) => {
                println!("Removing local repository: {}", path);
                if Path::new(path).exists() {
                    fs::remove_dir_all(path)?;
                }
            }
            ExecutedStep::RemoteRepoCreated(name) => {
                println!("Deleting remote repository: {}", name);
                // 这里可以添加删除远程仓库的逻辑
                // 由于需要额外的 API 调用，暂时跳过
            }
            ExecutedStep::RemoteAdded(path, _) => {
                println!("Removing remote from local repository: {}", path);
                let output = Command::new("git")
                    .arg("remote")
                    .arg("remove")
                    .arg("origin")
                    .current_dir(path)
                    .output()?;
                if !output.status.success() {
                    println!("Warning: Failed to remove remote: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
        }
    }

    Ok(())
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

    // Initialize git repository with main branch
    let output = Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
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

    // 记录已执行的步骤，用于回滚
    let mut executed_steps: Vec<ExecutedStep> = Vec::new();

    // 使用 Result 包裹主要逻辑，以便在失败时回滚
    let result = async {
        // Step 1: Check and create local repository
        println!("Checking local repository...");
        if check_local_repo(&args.directory, &args.name)? {
            println!("Local repository already exists, skipping creation.");
        } else {
            println!("Creating local repository...");
            create_local_repo(&args.directory, &args.name)?;
            executed_steps.push(ExecutedStep::LocalRepoCreated(repo_path.clone()));
        }

        // Step 2: Check and create remote repository
        println!("\nChecking remote repository...");
        let token = get_github_token()?;
        let client = Client::new();
        
        let repo = if check_remote_repo(&client, &token, &args.name).await? {
            println!("Remote repository already exists, skipping creation.");
            // 获取现有仓库信息
            let response = client
                .get(format!("https://api.github.com/repos/{}/{}", env::var("USER").unwrap_or_default(), args.name))
                .header("User-Agent", "github-init-cli")
                .header("Authorization", format!("token {}", token))
                .header("Accept", "application/vnd.github.v3+json")
                .send()
                .await?
                .json::<CreateRepoResponse>()
                .await?;
            response
        } else {
            println!("Creating remote repository...");
            let repo = create_github_repo(
                &client,
                &token,
                &args.name,
                args.private,
                args.auto_init,
            )
            .await?;
            executed_steps.push(ExecutedStep::RemoteRepoCreated(args.name.clone()));
            repo
        };

        // Step 3: Check and sync to remote
        println!("\nChecking sync status...");
        if check_sync_status(&repo_path, &repo.html_url)? {
            println!("Repository already synced, skipping syncing.");
        } else {
            println!("Syncing to remote repository...");
            sync_to_remote(&repo_path, &repo.html_url)?;
            executed_steps.push(ExecutedStep::RemoteAdded(repo_path.clone(), repo.html_url.clone()));
        }

        println!("\nRepository created successfully!");
        println!("Name: {}", repo.name);
        println!("URL: {}", repo.html_url);
        println!("Private: {}", repo.private);

        Ok(())
    }.await;

    // 处理结果，失败时回滚
    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("Error: {}", e);
            if let Err(rollback_err) = rollback(&executed_steps) {
                println!("Rollback failed: {}", rollback_err);
            }
            Err(e)
        }
    }
}
