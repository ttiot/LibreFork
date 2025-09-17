package main

// mustAssetStyles returns embedded CSS used to style the UI.
// This keeps the app visually closer to a dark VCS UI.
func mustAssetStyles() string {
    return `
/* Overall dark theme helpers */
.toolbar { padding: 6px; }
.compact-toolbar button { margin-right: 4px; }

/* Header buttons */
button.suggested-action { background: #3a6ea5; color: #fff; }
button.suggested-action:hover { background: #4b84c6; }

/* Sidebar */
.sidebar { background-color: @theme_bg_color; padding: 8px; }
.dim-label { opacity: 0.7; }

/* Commit list */
row { padding: 4px 8px; }
row:selected { background-color: rgba(90, 140, 200, 0.25); }

/* Diff colors (TextView fallback; exact colors handled via tags when possible) */
/* We still set some global colors if theme supports it */
`
}

