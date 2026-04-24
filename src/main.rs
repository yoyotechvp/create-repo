use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process::Command;

// Enum to represent executed steps for rollback
enum Step {
    DirectoryCreated(String),
    GitInitialized(String),
    CommitCreated(String),
    RemoteCreated(String),
    RepoCreatedOnGithub(String),
}

// Struct to track executed steps for rollback
struct RollbackManager {
    steps: Vec<Step>,
}

impl RollbackManager {
    fn new() -> Self {
        Self { steps: Vec::new() }
    }

    fn add_step(&mut self, step: Step) {
        self.steps.push(step);
    }

    fn rollback(&self) {
        println!("\nRolling back changes...");
        
        // Rollback in reverse order
        for step in self.steps.iter().rev() {
            match step {
                Step::DirectoryCreated(path) => {
                    println!("Removing directory: {}", path);
                    if let Err(e) = fs::remove_dir_all(path) {
                        println!("Failed to remove directory: {}", e);
                    }
                }
                Step::GitInitialized(path) => {
                    println!("Removing .git directory: {}", path);
                    let git_path = format!("{}/.git", path);
                    if let Err(e) = fs::remove_dir_all(&git_path) {
                        println!("Failed to remove .git directory: {}", e);
                    }
                }
                Step::CommitCreated(_) => {
                    // No need to rollback commit
                }
                Step::RemoteCreated(path) => {
                    println!("Removing remote origin: {}", path);
                    let output = Command::new("git")
                        .arg("remote")
                        .arg("remove")
                        .arg("origin")
                        .current_dir(path)
                        .output();
                    if let Err(e) = output {
                        println!("Failed to remove remote: {}", e);
                    }
                }
                Step::RepoCreatedOnGithub(repo_name) => {
                    println!("Note: Remote repository '{}' was created on GitHub and needs to be deleted manually", repo_name);
                }
            }
        }
        
        println!("Rollback completed");
    }
}

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
    
    #[arg(long, default_value_t = false)]
    https: bool,
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
    ssh_url: String,
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

fn create_local_repo(directory: &str, name: &str) -> anyhow::Result<Vec<Step>> {
    let mut steps = Vec::new();
    
    // Combine directory and name as final path
    let repo_path = if !directory.is_empty() {
        format!("{}/{}", directory, name)
    } else {
        name.to_string()
    };

    // Check if directory exists
    if !fs::metadata(&repo_path).is_ok() {
        // Create directory
        fs::create_dir_all(&repo_path)?;
        println!("Created directory: {}", repo_path);
        steps.push(Step::DirectoryCreated(repo_path.clone()));
    } else {
        println!("Directory already exists: {}, skipping creation", repo_path);
    }

    // Check if git repository is already initialized
    let git_dir = format!("{}/.git", repo_path);
    if !fs::metadata(&git_dir).is_ok() {
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
        println!("Initialized git repository at: {}", repo_path);
        steps.push(Step::GitInitialized(repo_path.clone()));
    } else {
        println!("Git repository already initialized at: {}, skipping init", repo_path);
    }

    // Check if there are any commits
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .current_dir(&repo_path)
        .output()?;

    if !output.status.success() {
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
        println!("Created initial commit");
        steps.push(Step::CommitCreated(repo_path.clone()));
    } else {
        println!("Repository already has commits, skipping initial commit");
    }

    println!("Local repository ready at: {}", repo_path);
    Ok(steps)
}

fn sync_to_remote(repo_path: &str, remote_url: &str) -> anyhow::Result<Vec<Step>> {
    let mut steps = Vec::new();
    
    // Check if remote origin already exists
    let output = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
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
        println!("Added remote origin: {}", remote_url);
        steps.push(Step::RemoteCreated(repo_path.to_string()));
    } else {
        let existing_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if existing_url == remote_url {
            println!("Remote origin already exists with the same URL, skipping add");
        } else {
            // Update remote URL
            let output = Command::new("git")
                .arg("remote")
                .arg("set-url")
                .arg("origin")
                .arg(remote_url)
                .current_dir(repo_path)
                .output()?;

            if !output.status.success() {
                return Err(anyhow::anyhow!("Git remote set-url failed: {}", String::from_utf8_lossy(&output.stderr)));
            }
            println!("Updated remote origin URL to: {}", remote_url);
        }
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
    Ok(steps)
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

    let mut rollback_manager = RollbackManager::new();

    // Step 1: Create local repository
    println!("Creating local repository...");
    match create_local_repo(&args.directory, &args.name) {
        Ok(steps) => {
            for step in steps {
                rollback_manager.add_step(step);
            }
        }
        Err(e) => {
            rollback_manager.rollback();
            return Err(e);
        }
    }

    // Step 2: Create remote repository
    println!("\nCreating remote repository...");
    let token = get_github_token()?;
    let client = Client::new();
    
    let repo = match create_github_repo(
        &client,
        &token,
        &args.name,
        args.private,
        args.auto_init,
    )
    .await {
        Ok(repo) => {
            rollback_manager.add_step(Step::RepoCreatedOnGithub(args.name.clone()));
            repo
        }
        Err(e) => {
            rollback_manager.rollback();
            return Err(e);
        }
    };

    // Step 3: Sync to remote
    println!("\nSyncing to remote repository...");
    let remote_url = if args.https {
        &repo.html_url
    } else {
        &repo.ssh_url
    };
    
    match sync_to_remote(&repo_path, remote_url) {
        Ok(steps) => {
            for step in steps {
                rollback_manager.add_step(step);
            }
        }
        Err(e) => {
            rollback_manager.rollback();
            return Err(e);
        }
    }

    println!("\nRepository created successfully!");
    println!("Name: {}", repo.name);
    println!("URL: {}", repo.html_url);
    println!("Private: {}", repo.private);

    Ok(())
}
