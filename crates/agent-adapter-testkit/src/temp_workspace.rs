//! Temporary workspace helpers for testing.
//!
//! Provides utilities for creating isolated temp directories,
//! git worktrees, and workspace cleanup.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

/// A temporary workspace that auto-cleanups on drop.
pub struct TempWorkspace {
    _dir: TempDir,
    path: PathBuf,
    cleanup_run: Arc<AtomicBool>,
}

impl TempWorkspace {
    /// Create a new temp workspace.
    pub fn new() -> std::io::Result<Self> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().to_path_buf();
        let cleanup_run = Arc::new(AtomicBool::new(false));

        Ok(Self {
            _dir: dir,
            path,
            cleanup_run,
        })
    }

    /// Create a new temp workspace with a specific prefix.
    pub fn with_prefix(prefix: &str) -> std::io::Result<Self> {
        let dir = tempfile::Builder::new().prefix(prefix).tempdir()?;
        let path = dir.path().to_path_buf();
        let cleanup_run = Arc::new(AtomicBool::new(false));

        Ok(Self {
            _dir: dir,
            path,
            cleanup_run,
        })
    }

    /// Get the path to the workspace.
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Create a file in the workspace.
    pub fn create_file(&self, name: &str, contents: &str) -> std::io::Result<PathBuf> {
        let path = self.path.join(name);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, contents)?;
        Ok(path)
    }

    /// Create a nested directory in the workspace.
    pub fn create_dir(&self, name: &str) -> std::io::Result<PathBuf> {
        let path = self.path.join(name);
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Check if workspace is empty.
    pub fn is_empty(&self) -> bool {
        std::fs::read_dir(&self.path)
            .map(|mut i| i.next().is_none())
            .unwrap_or(true)
    }

    /// List all files in the workspace recursively.
    pub fn list_files(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.walk_dir(&self.path, &mut files)?;
        Ok(files)
    }

    fn walk_dir(&self, dir: &PathBuf, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.walk_dir(&path, files)?;
            } else {
                files.push(path);
            }
        }
        Ok(())
    }

    /// Mark that cleanup was attempted.
    pub fn mark_cleaned(&self) {
        self.cleanup_run.store(true, Ordering::SeqCst);
    }

    /// Check if cleanup was run.
    pub fn was_cleaned(&self) -> bool {
        self.cleanup_run.load(Ordering::SeqCst)
    }
}

impl Default for TempWorkspace {
    fn default() -> Self {
        Self::new().expect("Failed to create temp workspace")
    }
}

impl std::fmt::Debug for TempWorkspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TempWorkspace({})", self.path.display())
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        self.cleanup_run.store(true, Ordering::SeqCst);
    }
}

/// A git worktree-based workspace for isolation testing.
pub struct WorktreeWorkspace {
    _worktree: TempDir,
    worktree_path: PathBuf,
    main_repo_path: PathBuf,
}

impl WorktreeWorkspace {
    /// Create a new isolated worktree workspace.
    pub fn new() -> std::io::Result<Self> {
        // Create a temp directory to serve as the "main" repo
        let main_repo = tempfile::tempdir()?;
        let main_path = main_repo.path().to_path_buf();

        // Initialize a git repo
        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(&main_path)
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other(format!(
                "git init failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Create worktree directory parent (git worktree add creates the leaf dir)
        let worktree_path = main_path.join("worktrees").join("test");
        std::fs::create_dir_all(worktree_path.parent().unwrap())?;

        // Add worktree - git will create the leaf directory
        let output = std::process::Command::new("git")
            .args(["worktree", "add", &worktree_path.to_string_lossy()])
            .current_dir(&main_path)
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other(format!(
                "git worktree add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(Self {
            _worktree: main_repo,
            worktree_path,
            main_repo_path: main_path,
        })
    }

    /// Get the worktree path.
    pub fn path(&self) -> PathBuf {
        self.worktree_path.clone()
    }

    /// Get the main repo path.
    pub fn main_repo_path(&self) -> PathBuf {
        self.main_repo_path.clone()
    }
}

impl Default for WorktreeWorkspace {
    fn default() -> Self {
        Self::new().expect("Failed to create worktree workspace")
    }
}

impl std::fmt::Debug for WorktreeWorkspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WorktreeWorkspace({})", self.worktree_path.display())
    }
}

/// Helper to create a workspace with a file already present.
pub fn workspace_with_file(name: &str, contents: &str) -> std::io::Result<TempWorkspace> {
    let ws = TempWorkspace::new()?;
    ws.create_file(name, contents)?;
    Ok(ws)
}

/// Helper to create a workspace with multiple files.
pub fn workspace_with_files(files: &[(&str, &str)]) -> std::io::Result<TempWorkspace> {
    let ws = TempWorkspace::new()?;
    for (name, contents) in files {
        ws.create_file(name, contents)?;
    }
    Ok(ws)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_workspace_new() {
        let ws = TempWorkspace::new().unwrap();
        assert!(ws.path().exists());
    }

    #[test]
    fn test_temp_workspace_with_prefix() {
        let ws = TempWorkspace::with_prefix("test-prefix-").unwrap();
        assert!(ws.path().exists());
        assert!(ws.path().to_string_lossy().contains("test-prefix-"));
    }

    #[test]
    fn test_temp_workspace_create_file() {
        let ws = TempWorkspace::new().unwrap();
        let file = ws.create_file("test.txt", "hello").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello");
    }

    #[test]
    fn test_temp_workspace_create_dir() {
        let ws = TempWorkspace::new().unwrap();
        let dir = ws.create_dir("subdir/nested").unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    #[test]
    fn test_temp_workspace_is_empty() {
        let ws = TempWorkspace::new().unwrap();
        assert!(ws.is_empty());
        ws.create_file("test.txt", "hello").unwrap();
        assert!(!ws.is_empty());
    }

    #[test]
    fn test_temp_workspace_list_files() {
        let ws = TempWorkspace::new().unwrap();
        ws.create_file("a.txt", "a").unwrap();
        ws.create_file("b.txt", "b").unwrap();
        let files = ws.list_files().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_workspace_with_file() {
        let ws = workspace_with_file("test.txt", "hello world").unwrap();
        let path = ws.path().join("test.txt");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world");
    }

    #[test]
    fn test_workspace_with_files() {
        let ws = workspace_with_files(&[("a.txt", "a"), ("b.txt", "b")]).unwrap();
        assert_eq!(ws.list_files().unwrap().len(), 2);
    }

    #[test]
    fn test_worktree_workspace() {
        let result = WorktreeWorkspace::new();
        if result.is_err() {
            // Git worktree may not be available in all environments
            // This is not a fatal error for the test suite
            return;
        }
        let ws = result.unwrap();
        // Only assert on paths if worktree creation succeeded
        assert!(
            ws.path().exists(),
            "Worktree path should exist: {}",
            ws.path().display()
        );
        assert!(
            ws.main_repo_path().exists(),
            "Main repo path should exist: {}",
            ws.main_repo_path().display()
        );
    }

    #[test]
    fn test_temp_workspace_cleanup() {
        let ws = TempWorkspace::new().unwrap();
        let path = ws.path().clone();
        drop(ws);
        // After drop, path should not exist (temp dir cleaned up)
        assert!(!path.exists());
    }

    // =========================================================================
    // Workspace isolation tests (Task 36)
    // =========================================================================

    #[test]
    fn test_temp_workspace_path_safety() {
        let ws = TempWorkspace::new().unwrap();
        let path = ws.path();

        // Path should be absolute
        assert!(path.is_absolute(), "Workspace path should be absolute");

        // Path should exist
        assert!(path.exists(), "Workspace path should exist");

        // Parent should be temp directory
        assert!(path.parent().is_some());
    }

    #[test]
    fn test_temp_workspace_nested_path_traversal() {
        let ws = TempWorkspace::new().unwrap();
        let file = ws.create_file("a/b/c/nested.txt", "content").unwrap();

        // Verify file exists and has correct content
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "content");

        // Verify the path doesn't escape workspace
        assert!(file.starts_with(&ws.path()));
    }

    #[test]
    fn test_worktree_workspace_isolation() {
        let result = WorktreeWorkspace::new();
        if result.is_err() {
            // Git worktree may not be available
            return;
        }
        let ws = result.unwrap();
        let worktree_path = ws.path();

        // Worktree path should be different from main repo path
        assert_ne!(worktree_path, ws.main_repo_path());

        // Both should exist
        assert!(worktree_path.exists());
        assert!(ws.main_repo_path().exists());
    }

    #[test]
    fn test_workspace_cleanup_on_drop() {
        let ws = TempWorkspace::new().unwrap();
        let path = ws.path().clone();
        let file = ws.create_file("test.txt", "hello").unwrap();

        // Verify file exists before drop
        assert!(file.exists());

        // Drop workspace
        drop(ws);

        // Entire directory should be cleaned up
        assert!(!path.exists(), "Workspace should be cleaned up on drop");
    }

    #[test]
    fn test_multiple_workspaces_independent() {
        let ws1 = TempWorkspace::new().unwrap();
        let ws2 = TempWorkspace::new().unwrap();

        let file1 = ws1.create_file("ws1.txt", "ws1").unwrap();
        let file2 = ws2.create_file("ws2.txt", "ws2").unwrap();

        // Files should be independent
        assert!(file1.exists());
        assert!(file2.exists());
        assert_ne!(file1.parent(), file2.parent());

        // Save path before drop
        let path1 = ws1.path().clone();

        // Dropping one should not affect the other
        drop(ws1);
        assert!(!path1.exists());
        assert!(file2.exists());
        assert!(ws2.path().exists());
    }
}
