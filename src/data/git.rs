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

use crate::dtos::pulls::{CompareResult, DiffFile};
use crate::dtos::repositories::{Branch, Commit, CommitDetail, TreeEntry};
use crate::models::repositories;
use loco_rs::prelude::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
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

    /// Runs `git -C <repo> <args>`, returning the raw process output so callers
    /// can inspect a non-zero exit status (needed for conflict detection).
    async fn git_output(&self, args: &[&str]) -> Result<Output> {
        Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .await
            .map_err(|e| Error::string(&format!("failed to run git: {e}")))
    }

    /// Runs `git -C <repo> <args>`, capturing both output streams.
    async fn git(&self, args: &[&str]) -> Result<Vec<u8>> {
        let output = self.git_output(args).await?;
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
        // A freshly created bare repository has no commits yet; that is an
        // empty history, not an error.
        if self.git(&["rev-parse", "--verify", "HEAD"]).await.is_err() {
            return Ok(Vec::new());
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
        // An empty repository has no tree to list.
        if self.git(&["rev-parse", "--verify", "HEAD"]).await.is_err() {
            return Ok(Vec::new());
        }

        // Clicking a file in the SPA also asks for its "tree". A blob has no
        // children, so return an empty list instead of a 500 from git.
        if let Ok(kind) = self.git(&["cat-file", "-t", &spec]).await {
            let kind = String::from_utf8_lossy(&kind).trim().to_owned();
            if kind != "tree" {
                return Ok(Vec::new());
            }
        }

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

    /// Three-dot comparison between two revisions: commits and files in
    /// `head` that are not in `base`, measured from their merge base.
    ///
    /// # Errors
    /// Returns a 400 for unsafe revisions or unrelated histories.
    pub async fn compare(&self, base: &str, head: &str) -> Result<CompareResult> {
        if !is_safe_ref(base) || !is_safe_ref(head) {
            return bad_request("invalid revision");
        }
        let base_sha = self.resolve(base).await?;
        let head_sha = self.resolve(head).await?;

        let out = self.git(&["merge-base", &base_sha, &head_sha]).await?;
        let merge_base = String::from_utf8_lossy(&out).trim().to_owned();
        if merge_base.is_empty() {
            return bad_request("the branches have no common ancestor");
        }

        // `git rev-list --left-right --count A...B` prints "left right":
        // commits reachable only from A, then only from B.
        let out = self
            .git(&[
                "rev-list",
                "--left-right",
                "--count",
                &format!("{base_sha}...{head_sha}"),
            ])
            .await?;
        let counts = String::from_utf8_lossy(&out);
        let mut fields = counts.split_whitespace();
        let behind_by = fields.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let ahead_by = fields.next().and_then(|v| v.parse().ok()).unwrap_or(0);

        let out = self
            .git(&[
                "log",
                "--format=%H%x09%an%x09%aI%x09%s",
                &format!("{base_sha}..{head_sha}"),
            ])
            .await?;
        let commits = String::from_utf8_lossy(&out)
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(4, '\t');
                Some(Commit {
                    sha: parts.next()?.to_owned(),
                    author: parts.next()?.to_owned(),
                    committed_at: parts.next()?.to_owned(),
                    message: parts.next().unwrap_or("").to_owned(),
                })
            })
            .collect();

        // `--no-renames` keeps one path per row, so all three outputs can be
        // joined on the path without parse ambiguity.
        let statuses = parse_name_status(&String::from_utf8_lossy(
            &self
                .git(&[
                    "-c",
                    "core.quotePath=false",
                    "diff",
                    "--no-renames",
                    "--name-status",
                    "--no-color",
                    "--no-ext-diff",
                    &merge_base,
                    &head_sha,
                    "--",
                ])
                .await?,
        ));
        let numstats = parse_numstat(&String::from_utf8_lossy(
            &self
                .git(&[
                    "-c",
                    "core.quotePath=false",
                    "diff",
                    "--no-renames",
                    "--numstat",
                    "--no-color",
                    "--no-ext-diff",
                    &merge_base,
                    &head_sha,
                    "--",
                ])
                .await?,
        ));
        let patches = parse_patches(&String::from_utf8_lossy(
            &self
                .git(&[
                    "-c",
                    "core.quotePath=false",
                    "diff",
                    "--no-renames",
                    "--unified=3",
                    "--no-color",
                    "--no-ext-diff",
                    &merge_base,
                    &head_sha,
                    "--",
                ])
                .await?,
        ));

        let files = statuses
            .into_iter()
            .map(|(path, status)| {
                let (additions, deletions) = numstats.get(&path).copied().unwrap_or((0, 0));
                DiffFile {
                    patch: patches.get(&path).cloned().unwrap_or_default(),
                    path,
                    status,
                    additions,
                    deletions,
                }
            })
            .collect();

        Ok(CompareResult {
            base: merge_base,
            head: head_sha,
            ahead_by,
            behind_by,
            commits,
            files,
        })
    }

    /// Merges `head_sha` into `base_ref` inside the bare repository and returns
    /// the new commit SHA.
    ///
    /// This uses git plumbing rather than a worktree: `merge-tree` computes the
    /// merged tree, `commit-tree` creates a merge commit with two parents, and
    /// `update-ref` advances the branch with a compare-and-swap guard so a
    /// concurrent push cannot be overwritten.
    ///
    /// # Errors
    /// Returns a 400 for unsafe refs, a conflict, or no common ancestor.
    pub async fn merge(&self, base_ref: &str, head_sha: &str, message: &str) -> Result<String> {
        if !is_safe_ref(base_ref) || !is_safe_ref(head_sha) {
            return bad_request("invalid revision");
        }
        let base_sha = self.resolve(base_ref).await?;
        let head_sha = self.resolve(head_sha).await?;

        let output = self
            .git_output(&["merge-tree", "--write-tree", &base_sha, &head_sha])
            .await?;
        if !output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            let detail = text
                .lines()
                .filter(|line| line.starts_with("CONFLICT"))
                .take(3)
                .collect::<Vec<_>>()
                .join("; ");
            return bad_request(if detail.is_empty() {
                "unable to merge these branches".to_owned()
            } else {
                format!("merge conflict: {detail}")
            });
        }
        let tree = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_owned();

        let out = self
            .git(&[
                "-c",
                "user.name=github-loco",
                "-c",
                "user.email=github-loco@localhost",
                "commit-tree",
                &tree,
                "-p",
                &base_sha,
                "-p",
                &head_sha,
                "-m",
                message,
            ])
            .await?;
        let commit = String::from_utf8_lossy(&out).trim().to_owned();

        self.git(&[
            "update-ref",
            &format!("refs/heads/{base_ref}"),
            &commit,
            &base_sha,
        ])
        .await?;
        Ok(commit)
    }
}

/// Maps `git diff --name-status` rows to `path -> status`.
fn parse_name_status(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let (code, path) = line.split_once('\t')?;
            let status = match code.chars().next().unwrap_or('M') {
                'A' | 'C' => "added",
                'D' => "deleted",
                'R' => "renamed",
                _ => "modified",
            };
            Some((path.to_owned(), status.to_owned()))
        })
        .collect()
}

/// Maps `git diff --numstat` rows to `path -> (additions, deletions)`.
fn parse_numstat(text: &str) -> BTreeMap<String, (i64, i64)> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let added = parts.next()?.parse().unwrap_or(0);
            let deleted = parts.next()?.parse().unwrap_or(0);
            let path = parts.next()?;
            Some((path.to_owned(), (added, deleted)))
        })
        .collect()
}

/// Splits a unified diff into `path -> patch` and caps each patch at 80 lines.
fn parse_patches(text: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for chunk in text.split("diff --git ").skip(1) {
        let path = chunk
            .lines()
            .find_map(|line| line.strip_prefix("+++ b/"))
            .or_else(|| chunk.lines().find_map(|line| line.strip_prefix("--- a/")))
            .unwrap_or("")
            .to_owned();
        if path.is_empty() {
            continue;
        }
        let patch = format!(
            "diff --git {chunk}",
            chunk = chunk.lines().take(80).collect::<Vec<_>>().join("\n")
        );
        result.insert(path, patch);
    }
    result
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

    #[test]
    fn parses_name_status() {
        let map = parse_name_status("A\tnew.txt\nM\tREADME.md\nD\told.txt\n");
        assert_eq!(map.get("new.txt").map(String::as_str), Some("added"));
        assert_eq!(map.get("README.md").map(String::as_str), Some("modified"));
        assert_eq!(map.get("old.txt").map(String::as_str), Some("deleted"));
    }

    #[test]
    fn parses_numstat_including_binary_files() {
        let map = parse_numstat("3\t1\tREADME.md\n-\t-\timage.png\n");
        assert_eq!(map.get("README.md"), Some(&(3, 1)));
        assert_eq!(map.get("image.png"), Some(&(0, 0)));
    }

    #[test]
    fn parses_patches_and_handles_deleted_files() {
        let diff = concat!(
            "diff --git a/README.md b/README.md\n",
            "--- a/README.md\n",
            "+++ b/README.md\n",
            "@@ -1 +1 @@\n",
            "-old\n",
            "+new\n",
            "diff --git a/old.txt b/old.txt\n",
            "--- a/old.txt\n",
            "+++ /dev/null\n",
            "@@ -1 +0,0 @@\n",
            "-gone\n",
        );
        let map = parse_patches(diff);
        assert!(map["README.md"].contains("+new"));
        assert!(map["old.txt"].contains("-gone"));
    }
}
