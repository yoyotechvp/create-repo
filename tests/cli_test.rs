use clap::Parser;

#[derive(Parser, Debug)]
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

#[test]
fn test_cli_args() {
    let args = Args::try_parse_from(["github-init-cli", "--name", "test-repo"]).unwrap();
    assert_eq!(args.name, "test-repo");
    assert_eq!(args.directory, "");
    assert_eq!(args.private, false);
    assert_eq!(args.auto_init, false);
}

#[test]
fn test_cli_args_with_directory() {
    let args = Args::try_parse_from([
        "github-init-cli",
        "--name",
        "test-repo",
        "--directory",
        "./projects",
    ])
    .unwrap();
    assert_eq!(args.name, "test-repo");
    assert_eq!(args.directory, "./projects");
    assert_eq!(args.private, false);
    assert_eq!(args.auto_init, false);
}

#[test]
fn test_cli_args_with_options() {
    let args = Args::try_parse_from([
        "github-init-cli",
        "--name",
        "test-repo",
        "--directory",
        "./projects",
        "--private",
        "--auto-init",
    ])
    .unwrap();
    assert_eq!(args.name, "test-repo");
    assert_eq!(args.directory, "./projects");
    assert_eq!(args.private, true);
    assert_eq!(args.auto_init, true);
}
