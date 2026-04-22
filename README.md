# GitHub Init CLI

A command-line tool to initialize GitHub repositories from your terminal, built with Rust.

## Features

- Create new GitHub repositories with a single command
- Support for private repositories
- Option to auto-initialize repositories with a README
- User-friendly error messages
- Environment variable support for GitHub token

## Installation

### Prerequisites

- Rust 1.60+ (for building from source)
- GitHub Personal Access Token with `repo` scope

### Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd github-init-cli

# Build the project
cargo build --release

# Move the binary to a directory in your PATH
mv target/release/github-init-cli ~/.local/bin/
```

## Configuration

Set your GitHub Personal Access Token as an environment variable:

### Linux/macOS

```bash
echo "export GITHUB_TOKEN=your-token-here" >> ~/.bashrc
source ~/.bashrc
```

### Windows (PowerShell)

```powershell
$env:GITHUB_TOKEN="your-token-here"
# To make it persistent
[Environment]::SetEnvironmentVariable("GITHUB_TOKEN", "your-token-here", "User")
```

## Usage

### Basic Usage

```bash
github-init-cli --name my-new-repo
```

### With Options

```bash
github-init-cli --name my-new-repo --directory ./my-project --private --auto-init
```

### Command Line Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--name` | `-n` | Repository name (required) | N/A |
| `--directory` | `-d` | Local directory path | Empty string (uses repo name) |
| `--private` | `-p` | Create a private repository | `false` |
| `--auto-init` | `-a` | Auto-initialize with README | `false` |
| `--help` | `-h` | Print help information | N/A |
| `--version` | `-V` | Print version information | N/A |

## Examples

### Create a public repository

```bash
github-init-cli --name my-public-repo
```

### Create a private repository with description

```bash
github-init-cli --name my-private-repo --description "Internal project" --private
```

### Create a repository with auto-initialization

```bash
github-init-cli --name my-init-repo --auto-init
```

## Error Handling

The tool provides clear error messages for common issues:

- Missing GitHub token
- Invalid token permissions
- Repository name already exists
- Network errors

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
