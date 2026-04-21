#!/usr/bin/env node
const { program } = require('commander');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const { Octokit } = require('@octokit/rest');
require('dotenv').config();
const chalk = require('chalk').default;

program
  .name('github-init')
  .description('Initialize a new GitHub repository locally and remotely')
  .version('1.0.0')
  .option('-n, --name <name>', 'Repository name', 'my-project')
  .option('-p, --private', 'Create private repository', false)
  .option('-d, --directory <path>', 'Local directory path', '.')
  .parse(process.argv);

const options = program.opts();

async function init() {
  try {
    console.log(chalk.green('Starting GitHub repository initialization...'));
    
    // Create local directory
    const repoPath = path.join(options.directory, options.name);
    if (!fs.existsSync(repoPath)) {
      fs.mkdirSync(repoPath, { recursive: true });
      console.log(chalk.blue(`Created directory: ${repoPath}`));
    }
    
    // Initialize git repository
    process.chdir(repoPath);
    execSync('git init', { stdio: 'inherit' });
    console.log(chalk.blue('Initialized git repository'));
    
    // Create initial commit
    execSync('git config user.name "GitHub CLI"', { stdio: 'inherit' });
    execSync('git config user.email "github-cli@example.com"', { stdio: 'inherit' });
    execSync('git add .', { stdio: 'inherit' });
    execSync('git commit -m "initial commit"', { stdio: 'inherit' });
    console.log(chalk.blue('Created initial commit'));
    
    // Create main branch
    execSync('git branch -M main', { stdio: 'inherit' });
    console.log(chalk.blue('Created main branch'));
    
    // Create GitHub repository
    const octokit = new Octokit({
      auth: process.env.GITHUB_TOKEN
    });
    
    const repo = await octokit.repos.createForAuthenticatedUser({
      name: options.name,
      private: options.private
    });
    
    console.log(chalk.blue(`Created GitHub repository: ${repo.data.html_url}`));
    
    // Push to remote
    execSync(`git remote add origin ${repo.data.ssh_url}`, { stdio: 'inherit' });
    execSync('git push -u origin main', { stdio: 'inherit' });
    console.log(chalk.blue('Pushed to remote repository'));
    
    console.log(chalk.green('Repository initialized successfully!'));
  } catch (error) {
    console.error(chalk.red('Error:', error.message));
    process.exit(1);
  }
}

init();