//! Read-only access to bare git repositories on disk.
//!
//! Cloning and pushing go through `git http-backend` (see
//! `controllers/git_http.rs`). Browsing needs a different interface: structured
//! access to refs, commits and trees. This module wraps the `git` CLI for that
//! purpose. Keeping it behind [`GitRepo`] means a future switch to `git2` is a
//! single-file change.
//!
//! Git remains the source of truth for repository contents. Nothing here writes
//! to a repository.

use crate::dtos::repositories::{Branch, Commit, CommitDetail, TreeEntry};
use crate::models::repositories;
use loco_rs::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// A git revision name that is safe to pass as a command-line argument.
///
/// Rejects leading `-` (option injection) and `..` (revision-range tricks),
/// plus anything outside the small character set git refs use.
#[must_use]
pub fn is_safe_ref(reference: &str) -> bool {
    !reference.is_empty()
        && !reference.starts_with('-')
        && !reference.contains("..")
        && !reference.contains('\0')
        && reference.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.' | '+' | '^' | '~')
        })
}

/// A path inside a tree that is safe to append to `<rev>:<path>`.
#[must_use]
pub fn is_safe_path(path: &str) -> bool {
    !path.starts_with('/') && !path.contains("..") && !path.contains('\0') && !path.contains("://")
}

/// An open handle to one bare repository.
#[derive(Clone, Debug)]
pub struct GitRepo {
    path: PathBuf,
}

impl GitRepo {
    /// Resolves `owner/repo` to a directory under `base` and checks that it is
    /// actually a git repository.
    ///
    /// # Errors
    /// Returns a 400 for an unsafe owner/name, or 404 when the repo is absent.
    pub fn open(base: &str, owner: &str, repo: &str) -> Result<Self> {
        let path = repositories::repo_dir(base, owner, repo)?;
        if !path.join("HEAD").is_file() {
            return not_found();
        }
        Ok(Self { path })
    }

    /// The on-disk directory of this repository.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Runs `git -C <repo> <args>`, capturing both output streams.
    async fn git(&self, args: &[&str]) -> Result<Vec<u8>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|e| Error::string(&format!("failed to run git: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(?args, %stderr, "git command failed");
            return Err(Error::string("git command failed"));
        }
        Ok(output.stdout)
    }

    /// The short name of the branch `HEAD` points at.
    ///
    /// # Errors
    /// Returns an error if git cannot be run or the repository has no HEAD.
    pub async fn default_branch(&self) -> Result<String> {
        let out = self.git(&["symbolic-ref", "--short", "HEAD"]).await?;
        Ok(String::from_utf8_lossy(&out).trim().to_owned())
    }

    /// Resolves a revision to a full commit SHA.
    ///
    /// # Errors
    /// Returns an error for an unsafe or unknown revision.
    pub async fn resolve(&self, reference: &str) -> Result<String> {
        if !is_safe_ref(reference) {
            return bad_request("invalid revision");
        }
        let spec = format!("{reference}^{{commit}}");
        let out = self.git(&["rev-parse", "--verify", &spec]).await?;
        Ok(String::from_utf8_lossy(&out).trim().to_owned())
    }

    /// Lists branches with the SHA each one points at.
    ///
    /// # Errors
    /// Returns an error if git cannot be run.
    pub async fn branches(&self) -> Result<Vec<Branch>> {
        let out = self
            .git(&[
                "for-each-ref",
                "--format=%(refname:short)%09%(objectname)",
                "refs/heads",
            ])
            .await?;
        Ok(String::from_utf8_lossy(&out)
            .lines()
            .filter_map(|line| {
                let (name, sha) = line.split_once('\t')?;
                Some(Branch {
                    name: name.to_owned(),
                    sha: sha.trim().to_owned(),
                })
            })
            .collect())
    }

    /// Lists up to `limit` commits reachable from `reference`.
    ///
    /// # Errors
    /// Returns an error for an unsafe revision or if git cannot be run.
    pub async fn commits(&self, reference: &str, limit: usize) -> Result<Vec<Commit>> {
        if !is_safe_ref(reference) {
            return bad_request("invalid revision");
        }
        let limit = limit.clamp(1, 100).to_string();
        let out = self
            .git(&[
                "log",
                "--format=%H%x09%an%x09%aI%x09%s",
                &format!("-n{limit}"),
                reference,
                "--",
            ])
            .await?;
        Ok(String::from_utf8_lossy(&out)
            .lines()
            .filter_map(|line| {
                let mut fields = line.splitn(4, '\t');
                Some(Commit {
                    sha: fields.next()?.to_owned(),
                    author: fields.next()?.to_owned(),
                    committed_at: fields.next()?.to_owned(),
                    message: fields.next().unwrap_or("").to_owned(),
                })
            })
            .collect())
    }

    /// Loads one commit, including its full message and parent SHAs.
    ///
    /// # Errors
    /// Returns an error for an unsafe SHA or an unknown commit.
    pub async fn commit(&self, sha: &str) -> Result<CommitDetail> {
        let sha = self.resolve(sha).await?;
        let out = self
            .git(&["show", "-s", "--format=%H%n%P%n%an%n%ae%n%aI%n%B", &sha])
            .await?;
        let text = String::from_utf8_lossy(&out);
        let mut lines = text.lines();
        Ok(CommitDetail {
            sha: lines.next().unwrap_or("").to_owned(),
            parents: lines
                .next()
                .unwrap_or("")
                .split_whitespace()
                .map(str::to_owned)
                .collect(),
            author: lines.next().unwrap_or("").to_owned(),
            author_email: lines.next().unwrap_or("").to_owned(),
            committed_at: lines.next().unwrap_or("").to_owned(),
            message: lines.collect::<Vec<_>>().join("\n").trim().to_owned(),
        })
    }

    /// Lists the direct children of `<rev>:<path>` (a tree).
    ///
    /// # Errors
    /// Returns an error for an unsafe revision/path or a missing tree.
    pub async fn tree(&self, reference: &str, path: &str) -> Result<Vec<TreeEntry>> {
        if !is_safe_ref(reference) || !is_safe_path(path) {
            return bad_request("invalid revision or path");
        }
        let spec = if path.is_empty() {
            format!("{reference}^{{tree}}")
        } else {
            format!("{reference}:{path}")
        };
        let out = self.git(&["ls-tree", "--long", &spec]).await?;

        Ok(String::from_utf8_lossy(&out)
            .lines()
            .filter_map(|line| {
                let (meta, name) = line.split_once('\t')?;
                let mut parts = meta.split_whitespace();
                let _mode = parts.next()?;
                let kind = parts.next()?.to_owned();
                let sha = parts.next()?.to_owned();
                let size = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                let child_path = if path.is_empty() {
                    name.to_owned()
                } else {
                    format!("{path}/{name}")
                };
                Some(TreeEntry {
                    name: name.to_owned(),
                    path: child_path,
                    kind,
                    size,
                    sha,
                })
            })
            .collect())
    }

    /// Reads the raw bytes of `<rev>:<path>` (a blob).
    ///
    /// # Errors
    /// Returns an error for an unsafe revision/path or a missing blob.
    pub async fn blob(&self, reference: &str, path: &str) -> Result<Vec<u8>> {
        if !is_safe_ref(reference) || path.is_empty() || !is_safe_path(path) {
            return bad_request("invalid revision or path");
        }
        self.git(&["show", &format!("{reference}:{path}")]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_revisions_are_accepted() {
        for reference in ["main", "HEAD", "feature/foo", "v1.2.3", "HEAD~2", "a^b"] {
            assert!(
                is_safe_ref(reference),
                "expected {reference:?} to be accepted"
            );
        }
    }

    #[test]
    fn unsafe_revisions_are_rejected() {
        for reference in ["", "--all", "main..other", "a b", "main\0x", "../../etc"] {
            assert!(
                !is_safe_ref(reference),
                "expected {reference:?} to be rejected"
            );
        }
    }

    #[test]
    fn safe_paths_are_accepted() {
        assert!(is_safe_path("src/main.rs"));
        assert!(is_safe_path("README.md"));
        assert!(is_safe_path(""));
    }

    #[test]
    fn unsafe_paths_are_rejected() {
        for path in ["/etc/passwd", "../secret", "a/../b", "http://evil", "a\0b"] {
            assert!(!is_safe_path(path), "expected {path:?} to be rejected");
        }
    }
}
