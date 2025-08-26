use anyhow::{Context, Result};
use git2::{ApplyLocation, BranchType, Oid, Patch, Repository, Sort};
use std::path::Path;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: String,
    pub short_id: String,
    pub summary: String,
    pub author: String,
    pub email: String,
    pub time: String,
    pub parents: usize,
    pub refs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    pub lines: Vec<DiffLine>,
}

pub struct RepoHandle {
    repo: Repository,
    pub path: String,
}

impl RepoHandle {
    pub fn open(path: &str) -> Result<Self> {
        let repo = Repository::open(path)
            .or_else(|_| Repository::discover(path))
            .with_context(|| format!("Impossible d'ouvrir le dépôt: {}", path))?;

        let repo_path = repo.path().display().to_string(); // <- lire avant le move

        Ok(Self {
            repo,            // move ici
            path: repo_path, // utiliser la copie
        })
    }

    pub fn head(&self) -> Result<Option<String>> {
        match self.repo.head() {
            Ok(head) => {
                if head.is_branch() {
                    Ok(head.shorthand().map(|s| s.to_string()))
                } else {
                    Ok(head.target().map(|oid| oid.to_string()))
                }
            }
            Err(_) => Ok(None),
        }
    }

    pub fn list_branches(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        for branch in self.repo.branches(Some(BranchType::Local))? {
            let (b, _) = branch?;
            if let Some(name) = b.name()? {
                names.push(name.to_string());
            }
        }
        Ok(names)
    }

    pub fn list_commits_paginated(&self, skip: usize, max: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

        if let Ok(head) = self.repo.head() {
            if let Some(id) = head.target() {
                revwalk.push(id)?;
            } else {
                return Ok(vec![]);
            }
        } else {
            return Ok(vec![]);
        }

        let mut commits = Vec::new();
        for oid_res in revwalk.skip(skip).take(max) {
            let oid = oid_res?;
            let commit = self.repo.find_commit(oid)?;
            let author = commit.author();
            let name = author.name().unwrap_or("unknown").to_string();
            let email = author.email().unwrap_or("").to_string();
            let time = commit.time();
            let time_utc = OffsetDateTime::from_unix_timestamp(time.seconds())
                .unwrap_or(OffsetDateTime::UNIX_EPOCH)
                .to_offset(time::UtcOffset::UTC);
            let time_str = time_utc.format(&Rfc3339).unwrap_or_else(|_| "".into());

            let short_id = self
                .repo
                .find_object(oid, None)?
                .short_id()?
                .as_str()
                .unwrap_or("")
                .to_string();

            let mut refs = Vec::new();
            for reference in self.repo.references()? {
                if let Ok(r) = reference {
                    if let Some(target) = r.target() {
                        if target == oid {
                            if let Some(name) = r.shorthand() {
                                refs.push(name.to_string());
                            }
                        }
                    }
                }
            }

            commits.push(CommitInfo {
                oid: oid.to_string(),
                short_id,
                summary: commit.summary().unwrap_or("<no subject>").to_string(),
                author: name,
                email,
                time: time_str,
                parents: commit.parent_count() as usize,
                refs,
            });
        }
        Ok(commits)
    }

    pub fn list_commits(&self, max: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

        // Essayez d'abord HEAD; s'il n'existe pas (repo vide), retourner vide
        if let Ok(head) = self.repo.head() {
            if let Some(id) = head.target() {
                revwalk.push(id)?;
            } else {
                return Ok(vec![]);
            }
        } else {
            return Ok(vec![]);
        }

        let mut commits = Vec::new();
        for oid_res in revwalk.take(max) {
            let oid = oid_res?;
            let commit = self.repo.find_commit(oid)?;
            let author = commit.author();
            let name = author.name().unwrap_or("unknown").to_string();
            let email = author.email().unwrap_or("").to_string();
            let time = commit.time();
            let time_utc = OffsetDateTime::from_unix_timestamp(time.seconds())
                .unwrap_or(OffsetDateTime::UNIX_EPOCH)
                .to_offset(time::UtcOffset::UTC);
            let time_str = time_utc.format(&Rfc3339).unwrap_or_else(|_| "".into());

            // Short id
            let short_id = self
                .repo
                .find_object(oid, None)?
                .short_id()?
                .as_str()
                .unwrap_or("")
                .to_string();

            // Refs (branches/tags) pointant sur ce commit
            let mut refs = Vec::new();
            for reference in self.repo.references()? {
                if let Ok(r) = reference {
                    if let Some(target) = r.target() {
                        if target == oid {
                            if let Some(name) = r.shorthand() {
                                refs.push(name.to_string());
                            }
                        }
                    }
                }
            }

            commits.push(CommitInfo {
                oid: oid.to_string(),
                short_id,
                summary: commit.summary().unwrap_or("<no subject>").to_string(),
                author: name,
                email,
                time: time_str,
                parents: commit.parent_count() as usize,
                refs,
            });
        }
        Ok(commits)
    }

    pub fn get_commit_details(&self, oid_str: &str) -> Result<(CommitInfo, String, Vec<FileDiff>)> {
        let oid = Oid::from_str(oid_str)?;
        let commit = self.repo.find_commit(oid)?;
        let author = commit.author();
        let name = author.name().unwrap_or("unknown").to_string();
        let email = author.email().unwrap_or("").to_string();
        let time = commit.time();
        let time_utc = OffsetDateTime::from_unix_timestamp(time.seconds())
            .unwrap_or(OffsetDateTime::UNIX_EPOCH)
            .to_offset(time::UtcOffset::UTC);
        let time_str = time_utc.format(&Rfc3339).unwrap_or_else(|_| "".into());

        let short_id = self
            .repo
            .find_object(oid, None)?
            .short_id()?
            .as_str()
            .unwrap_or("")
            .to_string();

        let mut refs = Vec::new();
        for reference in self.repo.references()? {
            if let Ok(r) = reference {
                if let Some(target) = r.target() {
                    if target == oid {
                        if let Some(name) = r.shorthand() {
                            refs.push(name.to_string());
                        }
                    }
                }
            }
        }

        let info = CommitInfo {
            oid: oid.to_string(),
            short_id,
            summary: commit.summary().unwrap_or("<no subject>").to_string(),
            author: name,
            email,
            time: time_str,
            parents: commit.parent_count() as usize,
            refs,
        };

        let message = commit.message().unwrap_or("").to_string();

        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };

        let diff = self
            .repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

        let mut files = Vec::new();
        for (i, delta) in diff.deltas().enumerate() {
            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .unwrap()
                .to_string_lossy()
                .to_string();

            if let Some(patch) = Patch::from_diff(&diff, i)? {
                let mut lines = Vec::new();
                for hunk_idx in 0..patch.num_hunks() {
                    let (_hunk, _) = patch.hunk(hunk_idx).unwrap();
                    for line_idx in 0..patch.num_lines_in_hunk(hunk_idx)? {
                        let line = patch.line_in_hunk(hunk_idx, line_idx).unwrap();
                        let content = std::str::from_utf8(line.content())
                            .unwrap_or("")
                            .trim_end_matches('\n')
                            .to_string();
                        match line.origin() {
                            '-' => lines.push(DiffLine {
                                left: Some(content),
                                right: None,
                            }),
                            '+' => lines.push(DiffLine {
                                left: None,
                                right: Some(content),
                            }),
                            ' ' => lines.push(DiffLine {
                                left: Some(content.clone()),
                                right: Some(content),
                            }),
                            _ => {}
                        }
                    }
                }
                files.push(FileDiff { path, lines });
            }
        }

        Ok((info, message, files))
    }

    pub fn stage_file(&self, path: &str) -> Result<()> {
        let mut index = self.repo.index()?;
        index.add_path(Path::new(path))?;
        index.write()?;
        Ok(())
    }

    pub fn stage_hunk(&self, patch_text: &str) -> Result<()> {
        let diff = git2::Diff::from_buffer(patch_text.as_bytes())?;
        self.repo
            .apply(&diff, ApplyLocation::Index, None)
            .map_err(|e| e.into())
    }
}
