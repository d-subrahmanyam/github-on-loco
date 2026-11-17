//! Helpers shared by request tests.
//!
//! Config is read at boot, so `REPO_BASE_DIR` must be pointed at a temporary
//! directory *before* `request::<App, _, _>` boots the app. Tests using this
//! module are `#[serial]` because they change process-wide state.

use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

/// A throwaway git storage root, removed when the value is dropped.
pub struct TestRepoRoot {
    pub path: PathBuf,
}

impl TestRepoRoot {
    #[must_use]
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("github-loco-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("temp repo root should be creatable");
        // SAFETY: tests using this are serialised with `#[serial]`.
        std::env::set_var("REPO_BASE_DIR", &path);
        Self { path }
    }

    #[must_use]
    pub fn bare_repo(&self, owner: &str, name: &str) -> PathBuf {
        self.path.join(owner).join(format!("{name}.git"))
    }
}

impl Drop for TestRepoRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn git(args: &[&str], dir: &Path) {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// Pushes one commit (with a README) into a bare repository and makes `main`
/// its default branch.
pub fn seed_commit(bare: &Path) {
    let work = std::env::temp_dir().join(format!("github-loco-work-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&work).expect("worktree should be creatable");

    git(&["init", "-q"], &work);
    git(&["config", "user.email", "test@example.com"], &work);
    git(&["config", "user.name", "Test User"], &work);
    std::fs::write(work.join("README.md"), "# hello\n").expect("README should be writable");
    git(&["add", "."], &work);
    git(&["commit", "-q", "-m", "initial commit"], &work);
    git(&["branch", "-M", "main"], &work);
    git(
        &["push", "-q", bare.to_str().expect("valid path"), "main"],
        &work,
    );
    git(&["symbolic-ref", "HEAD", "refs/heads/main"], bare);

    let _ = std::fs::remove_dir_all(&work);
}
