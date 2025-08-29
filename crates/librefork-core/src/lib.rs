use anyhow::{Context, Result};
use git2::{
    build::CheckoutBuilder, ApplyLocation, BranchType, Cred, CredentialType, Delta, FetchOptions,
    Oid, Patch, RemoteCallbacks, Repository, Sort,
};
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
    pub parents: Vec<String>,
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
    pub status: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct BranchStatus {
    pub name: String,
    pub ahead: usize,
    pub behind: usize,
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

    pub fn list_branches_with_upstream(&self) -> Result<Vec<BranchStatus>> {
        let mut result = Vec::new();
        for branch in self.repo.branches(Some(BranchType::Local))? {
            let (b, _) = branch?;
            if let Some(name) = b.name()? {
                let mut ahead = 0;
                let mut behind = 0;
                if let Ok(upstream) = b.upstream() {
                    if let (Some(lo), Some(ro)) = (b.get().target(), upstream.get().target()) {
                        let (a, d) = self.repo.graph_ahead_behind(lo, ro)?;
                        ahead = a;
                        behind = d;
                    }
                }
                result.push(BranchStatus {
                    name: name.to_string(),
                    ahead,
                    behind,
                });
            }
        }
        Ok(result)
    }

    pub fn list_remotes(&self) -> Result<Vec<String>> {
        let mut remotes = Vec::new();
        if let Ok(names) = self.repo.remotes() {
            for name in names.iter().flatten() {
                remotes.push(name.to_string());
            }
        }
        Ok(remotes)
    }

    pub fn list_tags(&self) -> Result<Vec<String>> {
        let mut tags = Vec::new();
        self.repo.tag_foreach(|_, name| {
            if let Ok(name_str) = std::str::from_utf8(name) {
                tags.push(name_str.to_string());
            }
            true
        })?;
        Ok(tags)
    }

    pub fn list_stashes(&self) -> Result<Vec<String>> {
        let mut stashes = Vec::new();
        let mut repo = Repository::open(&self.path)?;
        repo.stash_foreach(|i, name, _oid| {
            let label = if name.is_empty() {
                format!("stash@{{{}}}", i)
            } else {
                name.to_string()
            };
            stashes.push(label);
            true
        })?;
        Ok(stashes)
    }

    pub fn list_submodules(&self) -> Result<Vec<String>> {
        let mut subs = Vec::new();
        for sm in self.repo.submodules()? {
            if let Some(name) = sm.name() {
                subs.push(name.to_string());
            }
        }
        Ok(subs)
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

            let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();

            commits.push(CommitInfo {
                oid: oid.to_string(),
                short_id,
                summary: commit.summary().unwrap_or("<no subject>").to_string(),
                author: name,
                email,
                time: time_str,
                parents,
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

            let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();

            commits.push(CommitInfo {
                oid: oid.to_string(),
                short_id,
                summary: commit.summary().unwrap_or("<no subject>").to_string(),
                author: name,
                email,
                time: time_str,
                parents,
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

        let parents: Vec<String> = commit.parents().map(|p| p.id().to_string()).collect();

        let info = CommitInfo {
            oid: oid.to_string(),
            short_id,
            summary: commit.summary().unwrap_or("<no subject>").to_string(),
            author: name,
            email,
            time: time_str,
            parents,
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

            let status = match delta.status() {
                Delta::Added => "A",
                Delta::Deleted => "D",
                Delta::Modified => "M",
                _ => "?",
            };

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
                files.push(FileDiff {
                    path,
                    status: status.to_string(),
                    lines,
                });
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

    fn default_fetch_options(&self, url_hint: Option<&str>) -> Result<FetchOptions<'static>> {
        // Prepare owned data to avoid borrowing `self`/params inside the closure
        let url_hint_owned = url_hint.map(|s| s.to_string());
        let config_owned = git2::Config::open_default().ok();

        let mut callbacks = RemoteCallbacks::new();

        // Credentials callback to support SSH agent, SSH keys, and HTTPS helpers
        callbacks.credentials(move |url, username_from_url, allowed| {
            let url_eff = url_hint_owned.as_deref().unwrap_or(url);
            let username = username_from_url.unwrap_or("git");

            // 1) SSH (agent or key files)
            if allowed.contains(CredentialType::SSH_KEY) {
                if let Ok(agent_cred) = Cred::ssh_key_from_agent(username) {
                    return Ok(agent_cred);
                }

                // Fallback to common key files
                if let Some(home) = dirs::home_dir() {
                    for key_name in ["id_ed25519", "id_rsa"] {
                        let key_path = home.join(".ssh").join(key_name);
                        if key_path.exists() {
                            if let Ok(file_cred) = Cred::ssh_key(username, None, &key_path, None) {
                                return Ok(file_cred);
                            }
                        }
                    }
                }
            }

            // 2) HTTPS via git credential helpers (if configured)
            if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
                if let Some(cfg) = &config_owned {
                    if let Ok(helper_cred) = Cred::credential_helper(cfg, url_eff, username_from_url) {
                        return Ok(helper_cred);
                    }
                }
            }

            // 3) Some servers probe USERNAME first
            if allowed.contains(CredentialType::USERNAME) {
                return Cred::username(username);
            }

            // 4) Let libgit2 choose platform defaults where applicable
            Cred::default()
        });

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(callbacks);
        Ok(fo)
    }

    pub fn fetch(&self) -> Result<()> {
        let mut remote = self.repo.find_remote("origin")?;
        let url_hint = remote.url();
        let mut opts = self.default_fetch_options(url_hint)?;
        remote.fetch(&[] as &[&str], Some(&mut opts), None)?;
        Ok(())
    }

    pub fn pull(&self) -> Result<()> {
        self.fetch()?;
        if let Some(branch) = self.head()? {
            let local_ref = format!("refs/heads/{}", branch);
            let remote_ref = format!("refs/remotes/origin/{}", branch);
            if let Ok(oid) = self.repo.refname_to_id(&remote_ref) {
                if let Ok(mut reference) = self.repo.find_reference(&local_ref) {
                    reference.set_target(oid, "fast-forward")?;
                }
            }
        }
        Ok(())
    }

    pub fn stash(&mut self, message: &str) -> Result<()> {
        let sig = self.repo.signature()?;
        self.repo.stash_save(&sig, message, None)?;
        Ok(())
    }

    pub fn checkout_branch(&self, name: &str) -> Result<()> {
        let full_ref = format!("refs/heads/{}", name);
        // Point HEAD to the branch
        self.repo.set_head(&full_ref)?;
        // Update working tree to match HEAD
        let mut co = CheckoutBuilder::new();
        co.force();
        self.repo.checkout_head(Some(&mut co))?;
        Ok(())
    }

    pub fn checkout_tag(&self, name: &str) -> Result<()> {
        // Accept either plain tag name or full ref
        let full_ref = if name.starts_with("refs/tags/") {
            name.to_string()
        } else {
            format!("refs/tags/{}", name)
        };
        let reference = self.repo.find_reference(&full_ref)?;
        // Peel tag to a commit (works for annotated and lightweight tags)
        let obj = reference.peel(git2::ObjectType::Commit)?;
        let commit = obj
            .into_commit()
            .map_err(|_| anyhow::anyhow!("Tag does not point to a commit"))?;
        // Detach HEAD at the tagged commit
        self.repo.set_head_detached(commit.id())?;
        let mut co = CheckoutBuilder::new();
        co.force();
        self.repo.checkout_head(Some(&mut co))?;
        Ok(())
    }
}
