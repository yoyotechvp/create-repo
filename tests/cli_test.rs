use clap::CommandFactory;
use github_init_cli::Args;

#[test]
fn test_cli_args() {
    let args = Args::try_parse_from(["github-init-cli", "--name", "test-repo"]).unwrap();
    assert_eq!(args.name, "test-repo");
    assert_eq!(args.description, "");
    assert_eq!(args.private, false);
    assert_eq!(args.auto_init, false);
}

#[test]
fn test_cli_args_with_options() {
    let args = Args::try_parse_from([
        "github-init-cli",
        "--name",
        "test-repo",
        "--description",
        "Test repository",
        "--private",
        "--auto-init",
    ])
    .unwrap();
    assert_eq!(args.name, "test-repo");
    assert_eq!(args.description, "Test repository");
    assert_eq!(args.private, true);
    assert_eq!(args.auto_init, true);
}
