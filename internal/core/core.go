package core

import (
    "context"
    "fmt"
    "path/filepath"
    "regexp"
    "sort"
    "strings"
)

type CommitInfo struct {
    OID          string
    ShortID      string
    Summary      string
    Author       string
    Email        string
    Time         string
    Parents      []string
    Refs         []string
    FilesChanged int
    Insertions   int
    Deletions    int
}

type DiffLine struct {
    Left  *string
    Right *string
}

type FileDiff struct {
    Path   string
    Status string // A, M, D, ?
    Lines  []DiffLine
}

type BranchStatus struct {
    Name   string
    Ahead  int
    Behind int
}

type RepoHandle struct {
    Path string
}

func Open(path string) (*RepoHandle, error) {
    // Discover: try exact or ancestors for .git
    abs, _ := filepath.Abs(path)
    // Verify by running git rev-parse
    if _, _, err := runGit(context.Background(), abs, "rev-parse", "--git-dir"); err != nil {
        return nil, fmt.Errorf("impossible d'ouvrir le dépôt: %s: %w", path, err)
    }
    return &RepoHandle{Path: abs}, nil
}

func (r *RepoHandle) Head() (string, bool, error) {
    // Return branch name if on branch; otherwise OID
    out, _, err := runGit(context.Background(), r.Path, "rev-parse", "--abbrev-ref", "HEAD")
    if err != nil {
        return "", false, nil // treat as no head (empty repo)
    }
    name := strings.TrimSpace(out)
    if name != "HEAD" {
        return name, true, nil
    }
    oidOut, _, err := runGit(context.Background(), r.Path, "rev-parse", "HEAD")
    if err != nil {
        return "", false, nil
    }
    return strings.TrimSpace(oidOut), false, nil
}

func (r *RepoHandle) ListBranches() ([]string, error) {
    out, _, err := runGit(context.Background(), r.Path, "branch", "--format=%(refname:short)")
    if err != nil {
        return nil, err
    }
    var names []string
    for _, line := range strings.Split(strings.TrimSpace(out), "\n") {
        s := strings.TrimSpace(line)
        if s != "" {
            names = append(names, s)
        }
    }
    sort.Strings(names)
    return names, nil
}

func (r *RepoHandle) ListBranchesWithUpstream() ([]BranchStatus, error) {
    branches, err := r.ListBranches()
    if err != nil {
        return nil, err
    }
    stats := make([]BranchStatus, 0, len(branches))
    for _, b := range branches {
        ahead, behind := 0, 0
        if up, ok := r.branchUpstream(b); ok {
            // git rev-list --left-right --count local...upstream
            out, _, err := runGit(context.Background(), r.Path, "rev-list", "--left-right", "--count", fmt.Sprintf("%s...%s", b, up))
            if err == nil {
                // format: "A\tB\n"
                parts := strings.Fields(out)
                if len(parts) >= 2 {
                    fmt.Sscanf(parts[0], "%d", &behind) // left are reachable from first, i.e., upstream? actually --left-right: left commits are from first arg
                    fmt.Sscanf(parts[1], "%d", &ahead)
                }
            }
        }
        stats = append(stats, BranchStatus{Name: b, Ahead: ahead, Behind: behind})
    }
    return stats, nil
}

func (r *RepoHandle) branchUpstream(branch string) (string, bool) {
    out, _, err := runGit(context.Background(), r.Path, "rev-parse", "--abbrev-ref", branch+"@{upstream}")
    if err != nil {
        return "", false
    }
    return strings.TrimSpace(out), true
}

func (r *RepoHandle) ListRemotes() ([]string, error) {
    out, _, err := runGit(context.Background(), r.Path, "remote")
    if err != nil {
        return nil, err
    }
    var remotes []string
    for _, line := range strings.Split(strings.TrimSpace(out), "\n") {
        s := strings.TrimSpace(line)
        if s != "" {
            remotes = append(remotes, s)
        }
    }
    sort.Strings(remotes)
    return remotes, nil
}

func (r *RepoHandle) ListTags() ([]string, error) {
    out, _, err := runGit(context.Background(), r.Path, "tag", "--list")
    if err != nil {
        return nil, err
    }
    var tags []string
    for _, line := range strings.Split(strings.TrimSpace(out), "\n") {
        s := strings.TrimSpace(line)
        if s != "" {
            tags = append(tags, s)
        }
    }
    sort.Strings(tags)
    return tags, nil
}

func (r *RepoHandle) ListStashes() ([]string, error) {
    out, _, err := runGit(context.Background(), r.Path, "stash", "list", "--format=%gd")
    if err != nil {
        // No stashes returns empty
        return []string{}, nil
    }
    var stashes []string
    for _, line := range strings.Split(strings.TrimSpace(out), "\n") {
        s := strings.TrimSpace(line)
        if s != "" {
            stashes = append(stashes, s)
        }
    }
    return stashes, nil
}

func (r *RepoHandle) ListSubmodules() ([]string, error) {
    // Use git config to read .gitmodules entries
    out, _, err := runGit(context.Background(), r.Path, "config", "--file", ".gitmodules", "--name-only", "--get-regexp", "^submodule\\..*\\.path$")
    if err != nil {
        return []string{}, nil // No submodules
    }
    var names []string
    re := regexp.MustCompile(`^submodule\.([^\.]+)\.`)
    for _, line := range strings.Split(strings.TrimSpace(out), "\n") {
        if m := re.FindStringSubmatch(strings.TrimSpace(line)); m != nil {
            names = append(names, m[1])
        }
    }
    sort.Strings(names)
    return names, nil
}

// ListCommitsPaginated returns commits starting at skip with max results, including short stats.
func (r *RepoHandle) ListCommitsPaginated(skip, max int) ([]CommitInfo, error) {
    // Format fields separated by NUL; then include shortstat lines.
    // Use a clear end marker per commit to split reliably
    format := "%H|%h|%s|%an|%ae|%aI|%P|%D%n---END---"
    args := []string{"log", "--date=iso-strict", "--pretty=format:" + format}
    if max > 0 {
        args = append(args, fmt.Sprintf("--max-count=%d", max))
    }
    if skip > 0 {
        args = append(args, fmt.Sprintf("--skip=%d", skip))
    }
    // Stats omitted for now to keep parsing reliable without nulls
    out, _, err := runGit(context.Background(), r.Path, args...)
    if err != nil {
        return nil, err
    }
    blocks := strings.Split(out, "\n---END---")
    commits := make([]CommitInfo, 0, len(blocks))
    for _, blk := range blocks {
        blk = strings.TrimSpace(blk)
        if blk == "" {
            continue
        }
        lines := strings.Split(strings.TrimSpace(blk), "\n")
        header := lines[0]
        fields := strings.Split(header, "|")
        if len(fields) < 8 {
            continue
        }
        parents := []string{}
        if fields[6] != "" {
            parents = strings.Fields(fields[6])
        }
        refs := []string{}
        if fields[7] != "" {
            // refs like "HEAD -> main, origin/main, tag: v1.0"
            for _, ref := range strings.Split(fields[7], ",") {
                refs = append(refs, strings.TrimSpace(ref))
            }
        }
        ci := CommitInfo{
            OID:     fields[0],
            ShortID: fields[1],
            Summary: fields[2],
            Author:  fields[3],
            Email:   fields[4],
            Time:    fields[5],
            Parents: parents,
            Refs:    refs,
        }
        // Stats can be computed later when needed
        commits = append(commits, ci)
    }
    return commits, nil
}

func (r *RepoHandle) GetCommitPatchText(oid string) (string, error) {
    out, _, err := runGit(context.Background(), r.Path, "show", "--format=", "--patch", oid)
    if err != nil {
        return "", err
    }
    return out, nil
}

// GetCommitDetails returns CommitInfo, commit message, and per-file diffs with line-level changes.
func (r *RepoHandle) GetCommitDetails(oid string) (CommitInfo, string, []FileDiff, error) {
    // First, get the header fields similar to ListCommitsPaginated for a single commit
    format := "%H|%h|%s|%an|%ae|%aI|%P|%D\n%b"
    out, _, err := runGit(context.Background(), r.Path, "show", "--date=iso-strict", "--pretty=format:"+format, "--patch", oid)
    if err != nil {
        return CommitInfo{}, "", nil, err
    }
    // Split message body from header by first newline not part of NUL-separated header
    // The header stops at first '\n' after the fields; before that, we have 7 NULs
    // Extract first line up to first newline
    nl := strings.IndexByte(out, '\n')
    if nl < 0 {
        nl = len(out)
    }
    header := out[:nl]
    bodyAndPatch := ""
    if nl < len(out) {
        bodyAndPatch = out[nl+1:]
    }
    hf := strings.Split(header, "|")
    if len(hf) < 8 {
        return CommitInfo{}, "", nil, fmt.Errorf("unexpected commit header format")
    }
    parents := []string{}
    if hf[6] != "" {
        parents = strings.Fields(hf[6])
    }
    refs := []string{}
    if hf[7] != "" {
        for _, ref := range strings.Split(hf[7], ",") {
            refs = append(refs, strings.TrimSpace(ref))
        }
    }
    info := CommitInfo{
        OID:     hf[0],
        ShortID: hf[1],
        Summary: hf[2],
        Author:  hf[3],
        Email:   hf[4],
        Time:    hf[5],
        Parents: parents,
        Refs:    refs,
    }
    // Compute shortstat for this commit
    statOut, _, _ := runGit(context.Background(), r.Path, "show", "--shortstat", "--format=", oid)
    var files, ins, del int
    _, _ = fmt.Sscanf(statOut, "%d file%*s changed, %d insertion%*s, %d deletion%*s", &files, &ins, &del)
    info.FilesChanged, info.Insertions, info.Deletions = files, ins, del

    // Now parse the patch in bodyAndPatch
    filesDiff := parseUnifiedDiff(bodyAndPatch)
    // Message is the commit body up to before first diff --git
    msg := bodyAndPatch
    if idx := strings.Index(msg, "\ndiff --git "); idx >= 0 {
        msg = msg[:idx]
    }
    msg = strings.TrimRight(msg, "\n")
    return info, msg, filesDiff, nil
}

// parseUnifiedDiff parses a unified diff text into per-file diffs.
func parseUnifiedDiff(patch string) []FileDiff {
    var files []FileDiff
    var cur *FileDiff
    var currentPath string
    lines := strings.Split(patch, "\n")
    for _, l := range lines {
        if strings.HasPrefix(l, "diff --git ") {
            // start new file
            if cur != nil {
                files = append(files, *cur)
            }
            cur = &FileDiff{Path: "", Status: "?", Lines: []DiffLine{}}
            currentPath = ""
            continue
        }
        if cur == nil {
            continue
        }
        if strings.HasPrefix(l, "--- a/") || strings.HasPrefix(l, "+++ b/") {
            // extract path from +++ b/
            if strings.HasPrefix(l, "+++ b/") {
                currentPath = strings.TrimSpace(strings.TrimPrefix(l, "+++ b/"))
                cur.Path = currentPath
            }
            continue
        }
        if strings.HasPrefix(l, "new file mode") {
            cur.Status = "A"
            continue
        }
        if strings.HasPrefix(l, "deleted file mode") {
            cur.Status = "D"
            continue
        }
        if strings.HasPrefix(l, "index ") {
            if cur.Status == "?" {
                cur.Status = "M"
            }
            continue
        }
        if strings.HasPrefix(l, "@@ ") {
            // start of hunk header; we don't store header lines as content
            continue
        }
        if len(l) > 0 {
            switch l[0] {
            case '+':
                s := l[1:]
                cur.Lines = append(cur.Lines, DiffLine{Left: nil, Right: &s})
            case '-':
                s := l[1:]
                cur.Lines = append(cur.Lines, DiffLine{Left: &s, Right: nil})
            case ' ':
                s := l[1:]
                s2 := s
                cur.Lines = append(cur.Lines, DiffLine{Left: &s, Right: &s2})
            default:
                // ignore other lines (\ No newline at end of file, etc.)
            }
        }
    }
    if cur != nil {
        files = append(files, *cur)
    }
    return files
}

func (r *RepoHandle) CommitRemoteURL(oid string) (string, bool) {
    out, _, err := runGit(context.Background(), r.Path, "remote", "get-url", "origin")
    if err != nil {
        return "", false
    }
    url := strings.TrimSpace(out)
    webBase := ""
    if strings.HasPrefix(url, "git@") {
        // git@host:user/repo.git
        parts := strings.SplitN(strings.TrimPrefix(url, "git@"), ":", 2)
        if len(parts) == 2 {
            host := parts[0]
            path := strings.TrimSuffix(parts[1], ".git")
            webBase = fmt.Sprintf("https://%s/%s", host, path)
        }
    } else if strings.HasPrefix(url, "http://") || strings.HasPrefix(url, "https://") {
        webBase = strings.TrimSuffix(url, ".git")
    } else {
        return "", false
    }
    return fmt.Sprintf("%s/commit/%s", webBase, oid), true
}

// RemoteWebBase returns an https URL base for a given remote (e.g., https://host/user/repo)
// when possible. For http(s) remotes, it returns the URL with .git suffix trimmed.
// For SSH remotes, it converts git@host:user/repo.git to https://host/user/repo.
func (r *RepoHandle) RemoteWebBase(remote string) (string, bool) {
    out, _, err := runGit(context.Background(), r.Path, "remote", "get-url", remote)
    if err != nil {
        return "", false
    }
    url := strings.TrimSpace(out)
    if strings.HasPrefix(url, "git@") {
        parts := strings.SplitN(strings.TrimPrefix(url, "git@"), ":", 2)
        if len(parts) == 2 {
            host := parts[0]
            path := strings.TrimSuffix(parts[1], ".git")
            return fmt.Sprintf("https://%s/%s", host, path), true
        }
        return "", false
    }
    if strings.HasPrefix(url, "http://") || strings.HasPrefix(url, "https://") {
        return strings.TrimSuffix(url, ".git"), true
    }
    return "", false
}

// --- Repo actions ---

func (r *RepoHandle) StageFile(path string) error {
    _, _, err := runGit(context.Background(), r.Path, "add", "--", path)
    return err
}

func (r *RepoHandle) StageHunk(patchFile string) error {
    // Apply a patch file to index (cached). The patch must be relative to repo root.
    _, _, err := runGit(context.Background(), r.Path, "apply", "--cached", patchFile)
    return err
}

func (r *RepoHandle) Fetch() error {
    _, _, err := runGit(context.Background(), r.Path, "fetch", "origin")
    return err
}

func (r *RepoHandle) Pull() error {
    _, _, err := runGit(context.Background(), r.Path, "pull", "--ff-only")
    return err
}

func (r *RepoHandle) Stash(message string) error {
    _, _, err := runGit(context.Background(), r.Path, "stash", "push", "-m", message)
    return err
}

func (r *RepoHandle) CheckoutBranch(name string) error {
    _, _, err := runGit(context.Background(), r.Path, "checkout", name)
    return err
}

func (r *RepoHandle) CheckoutTag(name string) error {
    // Detach at the tag
    _, _, err := runGit(context.Background(), r.Path, "checkout", "--detach", "tags/"+name)
    return err
}

func (r *RepoHandle) CreateBranchAt(name, oid string) error {
    _, _, err := runGit(context.Background(), r.Path, "branch", name, oid)
    return err
}

func (r *RepoHandle) CreateTag(name, oid string) error {
    _, _, err := runGit(context.Background(), r.Path, "tag", name, oid)
    return err
}

func (r *RepoHandle) ResetHardTo(oid string) error {
    _, _, err := runGit(context.Background(), r.Path, "reset", "--hard", oid)
    return err
}

func (r *RepoHandle) ResetHardToParent(oid string) error {
    parent, _, err := runGit(context.Background(), r.Path, "rev-parse", oid+"^")
    if err != nil {
        return nil
    }
    return r.ResetHardTo(strings.TrimSpace(parent))
}

func (r *RepoHandle) CheckoutCommit(oid string) error {
    _, _, err := runGit(context.Background(), r.Path, "checkout", "--detach", oid)
    return err
}
