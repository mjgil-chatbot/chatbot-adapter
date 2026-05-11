//! Repo-level acceptance validation.
//!
//! Runs the public workspace validators against the real repository:
//! - lint_docs: validates docs/ tree, link integrity, and release-surface hygiene
//! - validate_design_traceability: validates design.md claims against code symbols
//!
//! This is the canonical release acceptance gate for the documentation
//! and design-traceability policies. It replaces the pattern of running filtered
//! unit tests that only exercise helper logic on fixtures.

use clap::Parser;
use std::path::PathBuf;
use std::process;

use agent_adapter_testkit::{lint_docs, validate_design_traceability};

#[derive(Parser, Debug)]
pub(crate) struct AcceptanceArgs {
    #[arg(long, help = "Output JSON")]
    pub json: bool,
}

/// Run the repo-level acceptance validation and return process exit code.
pub(crate) fn acceptance_command(_args: AcceptanceArgs) -> anyhow::Result<()> {
    let exit_code = run();
    process::exit(exit_code);
}

fn run() -> i32 {
    let repo_root = find_repo_root();

    println!("agent-adapter-acceptance");
    println!("  repository: {}", repo_root.display());
    println!();

    let docs_dir = repo_root.join("docs");
    let design_md = docs_dir.join("design.md");

    let mut failed = false;

    // 1. Run lint_docs against docs/
    println!("[1/2] lint_docs (docs/)...");
    let lint_result = lint_docs(&docs_dir);
    if lint_result.passed {
        println!("  lint_docs: PASS");
    } else {
        println!("  lint_docs: FAIL ({} error(s))", lint_result.errors.len());
        for err in &lint_result.errors {
            let location = match err.line {
                Some(n) => format!(":{}", n),
                None => String::new(),
            };
            eprintln!("    {}:{}{}", err.file.display(), location, err.error);
        }
        failed = true;
    }

    // 2. Run validate_design_traceability against design.md and repo root
    println!();
    println!("[2/2] validate_design_traceability (design.md)...");
    let design_result = validate_design_traceability(&design_md, &repo_root);
    if design_result.passed {
        println!("  validate_design_traceability: PASS");
    } else {
        println!(
            "  validate_design_traceability: FAIL ({} error(s))",
            design_result.errors.len()
        );
        for err in &design_result.errors {
            eprintln!("    [{}] {}", err.claim, err.error);
        }
        failed = true;
    }

    println!();
    if failed {
        println!("ACCEPTANCE: FAIL");
        1
    } else {
        println!("ACCEPTANCE: PASS");
        0
    }
}

/// Locate the repository root by walking up from the binary's own location.
fn find_repo_root() -> PathBuf {
    // Use CARGO_MANIFEST_DIR when available (works for cargo test / cargo run)
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = PathBuf::from(manifest);
        // agent-adapter-cli/Cargo.toml -> repo root
        return manifest_path.join("..").join("..");
    }

    // Fallback: walk up from the executable's location
    let exe = std::env::current_exe().expect("could not determine executable path");
    let mut current = exe
        .parent()
        .expect("executable has no parent directory")
        .to_path_buf();

    loop {
        if current.join("Cargo.toml").exists() && current.join("crates").exists() {
            return current;
        }
        if !current.pop() {
            panic!("could not find repository root");
        }
    }
}
