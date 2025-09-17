package core

import (
    "os"
    "os/exec"
    "path/filepath"
    "strings"
    "testing"
)

func mustRun(t *testing.T, dir string, name string, args ...string) string {
    t.Helper()
    cmd := exec.Command(name, args...)
    cmd.Dir = dir
    out, err := cmd.CombinedOutput()
    if err != nil {
        t.Fatalf("%s %v failed: %v\n%s", name, args, err, string(out))
    }
    return string(out)
}

func TestCoreBasic(t *testing.T) {
    tmp := t.TempDir()
    // init repo
    mustRun(t, tmp, "git", "init", ".")
    mustRun(t, tmp, "git", "config", "user.name", "Test")
    mustRun(t, tmp, "git", "config", "user.email", "test@example.com")
    // commit 1
    os.WriteFile(filepath.Join(tmp, "a.txt"), []byte("hello\n"), 0o644)
    mustRun(t, tmp, "git", "add", "a.txt")
    mustRun(t, tmp, "git", "commit", "-m", "first")
    // commit 2
    os.WriteFile(filepath.Join(tmp, "a.txt"), []byte("hello\nworld\n"), 0o644)
    mustRun(t, tmp, "git", "add", "a.txt")
    mustRun(t, tmp, "git", "commit", "-m", "second")

    rh, err := Open(tmp)
    if err != nil {
        t.Fatalf("Open failed: %v", err)
    }

    head, isBranch, err := rh.Head()
    if err != nil {
        t.Fatalf("Head: %v", err)
    }
    if !isBranch || head != "master" && head != "main" { // git may default to master or main
        t.Fatalf("unexpected head: %s (branch=%v)", head, isBranch)
    }

    branches, err := rh.ListBranches()
    if err != nil || len(branches) == 0 {
        t.Fatalf("ListBranches: %v len=%d", err, len(branches))
    }

    commits, err := rh.ListCommitsPaginated(0, 10)
    if err != nil || len(commits) < 2 {
        t.Fatalf("ListCommitsPaginated: %v len=%d", err, len(commits))
    }
    oid := commits[0].OID

    patch, err := rh.GetCommitPatchText(oid)
    if err != nil || !strings.Contains(patch, "diff --git") {
        t.Fatalf("GetCommitPatchText: %v\n%s", err, patch)
    }

    url, ok := rh.CommitRemoteURL(oid)
    if ok || url != "" { // No remote configured; must be false
        t.Fatalf("CommitRemoteURL expected none; got %v %q", ok, url)
    }

    // StageFile should succeed even if nothing to stage (no error)
    if err := rh.StageFile("a.txt"); err != nil {
        t.Fatalf("StageFile: %v", err)
    }

    // Create a branch at HEAD^ and switch to it
    if err := rh.CreateBranchAt("test-branch", commits[1].OID); err != nil {
        t.Fatalf("CreateBranchAt: %v", err)
    }
    if err := rh.CheckoutBranch("test-branch"); err != nil {
        t.Fatalf("CheckoutBranch: %v", err)
    }
}

