package core

import (
    "bytes"
    "context"
    "fmt"
    "os/exec"
    "strings"
    "time"
)

// runGit runs a git command in the given repository path and returns stdout as string.
func runGit(ctx context.Context, repoPath string, args ...string) (string, string, error) {
    // Ensure a reasonable timeout to avoid hanging
    if ctx == nil {
        var cancel context.CancelFunc
        ctx, cancel = context.WithTimeout(context.Background(), 15*time.Second)
        defer cancel()
    }
    cmd := exec.CommandContext(ctx, "git", args...)
    if repoPath != "" {
        cmd.Dir = repoPath
    }
    var stdout, stderr bytes.Buffer
    cmd.Stdout = &stdout
    cmd.Stderr = &stderr
    err := cmd.Run()
    outStr := stdout.String()
    errStr := stderr.String()
    if err != nil {
        // Include stderr in error to help troubleshooting
        if strings.TrimSpace(errStr) == "" {
            errStr = err.Error()
        }
        return outStr, errStr, fmt.Errorf("git %v failed: %s", strings.Join(args, " "), errStr)
    }
    return outStr, errStr, nil
}

