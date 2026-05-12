//! Schema lint for TOML configuration files.
//!
//! This module provides linting utilities to ensure TOML config files are valid
//! and conform to expected schemas.

use std::fs;
use std::path::{Path, PathBuf};

/// Result of a schema lint check.
#[derive(Debug, Clone)]
pub struct SchemaLintResult {
    pub passed: bool,
    pub errors: Vec<SchemaLintError>,
}

impl SchemaLintResult {
    pub fn success() -> Self {
        Self {
            passed: true,
            errors: Vec::new(),
        }
    }

    pub fn failure(errors: Vec<SchemaLintError>) -> Self {
        Self {
            passed: false,
            errors,
        }
    }
}

/// A schema lint error.
#[derive(Debug, Clone)]
pub struct SchemaLintError {
    pub file: PathBuf,
    pub line: Option<usize>,
    pub error: String,
}

/// Lint TOML files in the project.
pub fn lint_toml_files(root_dir: &Path) -> SchemaLintResult {
    let mut errors = Vec::new();

    // Check Cargo.toml files
    errors.extend(lint_cargo_toml(root_dir));

    // Check for config.toml or similar TOML in docs
    errors.extend(lint_docs_toml(root_dir));

    if errors.is_empty() {
        SchemaLintResult::success()
    } else {
        SchemaLintResult::failure(errors)
    }
}

/// Lint Cargo.toml files.
fn lint_cargo_toml(root_dir: &Path) -> Vec<SchemaLintError> {
    let mut errors = Vec::new();

    // Find all Cargo.toml files
    let walker = walkdir::WalkDir::new(root_dir)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in walker {
        let path = entry.path();
        if path.file_name().map(|n| n == "Cargo.toml").unwrap_or(false) {
            if let Err(e) = lint_toml_file(path) {
                errors.push(e);
            }
        }
    }

    errors
}

/// Lint TOML files in docs directory.
fn lint_docs_toml(root_dir: &Path) -> Vec<SchemaLintError> {
    let mut errors = Vec::new();

    let docs_dir = root_dir.join("docs");
    if !docs_dir.exists() {
        return errors;
    }

    // Look for TOML files in docs
    if let Ok(entries) = std::fs::read_dir(&docs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                if let Err(e) = lint_toml_file(&path) {
                    errors.push(e);
                }
            }
        }
    }

    errors
}

/// Lint a single TOML file.
fn lint_toml_file(path: &Path) -> Result<(), SchemaLintError> {
    let content = fs::read_to_string(path).map_err(|e| SchemaLintError {
        file: path.to_path_buf(),
        line: None,
        error: format!("Could not read file: {}", e),
    })?;

    toml::from_str::<toml::Value>(&content).map_err(|e| SchemaLintError {
        file: path.to_path_buf(),
        line: None,
        error: format!("Invalid TOML: {}", e),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lint_valid_toml() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.toml");

        // Write valid TOML
        fs::write(&path, "key = \"value\"\n[section]\nfoo = 123\n").unwrap();

        let result = lint_toml_file(&path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lint_invalid_toml() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.toml");

        // Write invalid TOML
        fs::write(&path, "key = \"value\"\n[section\nfoo = ").unwrap();

        let result = lint_toml_file(&path);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.error.contains("Invalid TOML"));
    }

    #[test]
    fn test_lint_toml_files_integration() {
        let temp = TempDir::new().unwrap();

        // Create a Cargo.toml-like structure
        let cargo_toml = temp.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "test"
version = "0.1.0"
"#,
        )
        .unwrap();

        let result = lint_toml_files(temp.path());
        assert!(
            result.passed,
            "Expected passed but got errors: {:?}",
            result.errors
        );
    }
}
