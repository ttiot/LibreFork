package main

// getModernStyles retourne le CSS moderne pour l'interface utilisateur
// inspirée exactement de l'interface Git de référence
func getModernStyles() string {
	return `
/* === VARIABLES ET COULEURS EXACTES === */
@define-color git_bg_primary #1f1f1f;
@define-color git_bg_secondary #2d2d30;
@define-color git_bg_tertiary #3e3e42;
@define-color git_fg_primary #cccccc;
@define-color git_fg_secondary #969696;
@define-color git_fg_dim #6a6a6a;
@define-color git_accent #007acc;
@define-color git_accent_hover #1177bb;
@define-color git_success #4ec9b0;
@define-color git_warning #dcdcaa;
@define-color git_error #f44747;
@define-color git_border #3e3e42;
@define-color git_selection #264f78;

/* === FENÊTRE PRINCIPALE === */
.main-window {
    background-color: @git_bg_primary;
    color: @git_fg_primary;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    font-size: 13px;
}

/* === HEADER BAR === */
.modern-header {
    background-color: @git_bg_secondary;
    border-bottom: 1px solid @git_border;
    min-height: 35px;
}

.modern-header .title {
    font-weight: 400;
    font-size: 13px;
    color: @git_fg_primary;
}

/* === BARRE D'OUTILS === */
.modern-toolbar {
    background-color: @git_bg_secondary;
    border-bottom: 1px solid @git_border;
    padding: 4px 8px;
    min-height: 35px;
}

.toolbar-button {
    min-width: 28px;
    min-height: 28px;
    margin: 0 1px;
    border-radius: 3px;
    border: none;
    background: transparent;
    color: @git_fg_secondary;
}

.toolbar-button:hover {
    background-color: @git_bg_tertiary;
    color: @git_fg_primary;
}

.toolbar-button:active {
    background-color: @git_selection;
    color: @git_fg_primary;
}

/* === PANNEAU LATÉRAL STYLE GIT === */
.git-sidebar {
    background-color: @git_bg_secondary;
    border-right: 1px solid @git_border;
    min-width: 280px;
    font-size: 13px;
}

.project-title {
    font-weight: 600;
    font-size: 14px;
    color: @git_fg_primary;
    padding: 8px 0;
}

/* === ÉLÉMENTS D'ARBRE === */
.tree-item {
    padding: 2px 8px;
    border-radius: 3px;
    transition: background-color 100ms ease;
}

.tree-item:hover {
    background-color: @git_bg_tertiary;
}

.tree-item.selected {
    background-color: @git_selection;
}

.tree-icon {
    color: @git_fg_secondary;
    margin-right: 6px;
}

.tree-label {
    color: @git_fg_primary;
    font-size: 13px;
}

/* === SECTIONS REPLIABLES === */
.expander-header {
    padding: 4px 8px;
    border-radius: 3px;
    transition: background-color 100ms ease;
}

.expander-header:hover {
    background-color: @git_bg_tertiary;
}

.expander-arrow {
    color: @git_fg_secondary;
    margin-right: 4px;
}

.expander-title {
    color: @git_fg_primary;
    font-size: 13px;
    font-weight: 500;
}

.section-icon {
    color: @git_fg_secondary;
    margin-right: 6px;
}

/* === ZONE CENTRALE STYLE GIT === */
.main-paned {
    background-color: @git_bg_primary;
}

.tab-header {
    background-color: @git_bg_secondary;
    border-bottom: 1px solid @git_border;
    padding: 0;
    min-height: 35px;
}

/* === ONGLETS STYLE VS CODE === */
.modern-tabs {
    background-color: transparent;
    border: none;
}

.modern-tabs button {
    background-color: transparent;
    border: none;
    border-radius: 0;
    padding: 8px 12px;
    margin: 0;
    color: @git_fg_secondary;
    font-weight: 400;
    font-size: 13px;
    border-right: 1px solid @git_border;
}

.modern-tabs button:hover {
    background-color: @git_bg_tertiary;
    color: @git_fg_primary;
}

.modern-tabs button:checked {
    background-color: @git_bg_primary;
    color: @git_fg_primary;
    border-bottom: 2px solid @git_accent;
}

/* === GIT GRAPH STYLE COMME DANS L'IMAGE === */
.git-graph-list {
    background-color: @git_bg_primary;
    border: none;
    margin: 0;
    font-size: 12px;
}


.git-graph-list row {
    border-bottom: 1px solid @git_border;
    padding: 0;
    min-height: 30px;
    background: transparent;
}

.git-graph-list row:hover .commit-row {
    background-color: rgba(62, 62, 66, 0.6);
}

.git-graph-list row:selected {
    background-color: transparent;
    color: @git_fg_primary;
}

.git-graph-list row:selected .commit-row {
    background-color: rgba(31, 124, 232, 0.35);
    border-radius: 4px;
}

.git-graph-list row:selected label {
    color: @git_fg_primary;
}

.commit-row {
    padding: 0;
    min-height: 30px;
    border-radius: 4px;
    transition: background-color 120ms ease;
}

.commit-list-header {
    background-color: @git_bg_secondary;
    border-bottom: 1px solid @git_border;
    padding: 6px 0;
    min-height: 28px;
}

.commit-header-label {
    color: @git_fg_dim;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding-right: 12px;
}

.commit-branch-column {
    min-width: 150px;
}

.commit-graph-column {
    min-width: 156px;
}

.commit-message-column {
    padding-right: 12px;
}

.commit-author-column {
    min-width: 140px;
}

.commit-hash-column {
    min-width: 80px;
}

.commit-date-column {
    min-width: 100px;
}

.commit-branch-column label {
    color: @git_fg_secondary;
    font-size: 11px;
}

.commit-author-column label,
.commit-hash-column label,
.commit-date-column label {
    font-size: 11px;
}

.git-graph-area {
    min-width: 156px;
    min-height: 30px;
}

/* === COLONNES DU GIT GRAPH === */
.git-graph-columns {
    background-color: transparent;
    margin-right: 8px;
}

/* === POINTS ET LIGNES === */
.git-dot {
    border-radius: 50%;
    border: 1px solid transparent;
}

.git-line {
    background-color: transparent;
}

/* === COULEURS DES BRANCHES (comme dans l'image) === */
.git-color-0 .git-dot,
.git-color-0 .git-line {
    background-color: #00d4aa; /* Vert cyan comme dans l'image */
}

.git-color-1 .git-dot,
.git-color-1 .git-line {
    background-color: #1f7ce8; /* Bleu comme dans l'image */
}

.git-color-2 .git-dot,
.git-color-2 .git-line {
    background-color: #f9826c; /* Orange comme dans l'image */
}

.git-color-3 .git-dot,
.git-color-3 .git-line {
    background-color: #a855f7; /* Violet comme dans l'image */
}

.git-color-0 {
    color: #00d4aa;
}

.git-color-1 {
    color: #1f7ce8;
}

.git-color-2 {
    color: #f9826c;
}

.git-color-3 {
    color: #a855f7;
}

/* === RENDU GIT GRAPH SOPHISTIQUÉ === */
.git-graph-element {
    font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
    font-size: 14px;
    font-weight: bold;
    text-shadow: 0 0 2px rgba(0, 0, 0, 0.3);
    margin: 0;
    padding: 0;
}

.git-commit-dot {
    font-size: 16px;
    font-weight: bold;
    text-shadow: 0 0 4px rgba(0, 0, 0, 0.5);
    margin: 0;
    padding: 0;
}

.git-merge-dot {
    font-size: 18px;
    font-weight: bold;
    text-shadow: 0 0 6px rgba(0, 0, 0, 0.6);
    margin: 0;
    padding: 0;
}

.git-branch-line {
    font-size: 14px;
    font-weight: bold;
    opacity: 0.9;
    text-shadow: 0 0 2px rgba(0, 0, 0, 0.3);
    margin: 0;
    padding: 0;
}

.git-connection {
    font-size: 12px;
    font-weight: bold;
    opacity: 0.8;
    text-shadow: 0 0 2px rgba(0, 0, 0, 0.4);
    margin: 0;
    padding: 0;
}

.git-empty {
    font-size: 14px;
    margin: 0;
    padding: 0;
}

/* === CONNEXIONS COURBES ET DIAGONALES === */
.git-curve-connection {
    font-size: 12px;
    font-weight: bold;
    color: inherit;
    margin: 0;
    padding: 0;
    text-shadow: 0 0 2px rgba(0, 0, 0, 0.4);
}

.git-diagonal-connection {
    font-size: 12px;
    font-weight: bold;
    color: inherit;
    margin: 0;
    padding: 0;
    text-shadow: 0 0 2px rgba(0, 0, 0, 0.4);
}

.git-merge-connection {
    font-size: 12px;
    font-weight: bold;
    color: inherit;
    margin: 0;
    padding: 0;
    opacity: 0.9;
    text-shadow: 0 0 3px rgba(0, 0, 0, 0.5);
}

/* === COULEURS POUR LES CONNEXIONS AVANCÉES === */
.git-color-0.git-commit-dot {
    border-color: #00d4aa;
    color: #00d4aa;
}

.git-color-1.git-commit-dot {
    border-color: #1f7ce8;
    color: #1f7ce8;
}

.git-color-2.git-commit-dot {
    border-color: #f9826c;
    color: #f9826c;
}

.git-color-3.git-commit-dot {
    border-color: #a855f7;
    color: #a855f7;
}

.git-color-0.git-connection-top,
.git-color-0.git-connection-bottom,
.git-color-0.git-branch-line,
.git-color-0.git-curve-connection,
.git-color-0.git-diagonal-connection,
.git-color-0.git-merge-connection {
    border-color: #00d4aa;
    color: #00d4aa;
}

.git-color-1.git-connection-top,
.git-color-1.git-connection-bottom,
.git-color-1.git-branch-line,
.git-color-1.git-curve-connection,
.git-color-1.git-diagonal-connection,
.git-color-1.git-merge-connection {
    border-color: #1f7ce8;
    color: #1f7ce8;
}

.git-color-2.git-connection-top,
.git-color-2.git-connection-bottom,
.git-color-2.git-branch-line,
.git-color-2.git-curve-connection,
.git-color-2.git-diagonal-connection,
.git-color-2.git-merge-connection {
    border-color: #f9826c;
    color: #f9826c;
}

.git-color-3.git-connection-top,
.git-color-3.git-connection-bottom,
.git-color-3.git-branch-line,
.git-color-3.git-curve-connection,
.git-color-3.git-diagonal-connection,
.git-color-3.git-merge-connection {
    border-color: #a855f7;
    color: #a855f7;
}

/* === AMÉLIORATIONS POUR LES MERGES === */
.git-merge-dot {
    border-radius: 50%;
    min-width: 10px;
    min-height: 10px;
    font-size: 6px;
    border: 3px solid;
    background-color: @git_bg_primary;
}

.git-merge-dot.git-color-0 {
    border-color: #00d4aa;
    color: #00d4aa;
}

.git-merge-dot.git-color-1 {
    border-color: #1f7ce8;
    color: #1f7ce8;
}

.git-merge-dot.git-color-2 {
    border-color: #f9826c;
    color: #f9826c;
}

.git-merge-dot.git-color-3 {
    border-color: #a855f7;
    color: #a855f7;
}

/* === LABELS DE BRANCHE/TAG === */
.git-label-branch {
    background-color: #1f7ce8;
    color: white;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
    margin-right: 4px;
}

.git-label-remote {
    background-color: #00d4aa;
    color: white;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
    margin-right: 4px;
}

.git-label-tag {
    background-color: #f9826c;
    color: white;
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: 3px;
    margin-right: 4px;
}

/* === TEXTE DU COMMIT === */
.git-message {
    color: @git_fg_primary;
    font-size: 12px;
    font-weight: 400;
}

.git-author {
    color: @git_fg_secondary;
    font-size: 11px;
}

.git-hash {
    color: @git_accent;
    font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
    font-size: 11px;
}

.git-date {
    color: @git_fg_secondary;
    font-size: 11px;
}

.git-commit-icon {
    color: @git_fg_secondary;
    margin-right: 8px;
}

.git-commit-icon.git-branch-master {
    color: #f9826c;
}

.git-commit-icon.git-branch-develop {
    color: #4ec9b0;
}

.git-commit-icon.git-branch-feature {
    color: #569cd6;
}

.git-commit-icon.git-branch-hotfix {
    color: #f44747;
}

.git-commit-icon.git-branch-release {
    color: #dcdcaa;
}

.git-commit-icon.git-branch-remote {
    color: #c586c0;
}

.git-commit-icon.git-branch-1 {
    color: #f9826c;
}

.git-commit-icon.git-branch-2 {
    color: #4ec9b0;
}

.git-commit-icon.git-branch-3 {
    color: #569cd6;
}

.git-commit-icon.git-branch-4 {
    color: #dcdcaa;
}

.git-commit-icon.git-branch-5 {
    color: #c586c0;
}

.git-commit-message {
    font-size: 13px;
    color: @git_fg_primary;
    font-weight: 400;
}

.git-commit-hash {
    font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
    font-size: 11px;
    color: @git_accent;
}

.git-commit-date {
    font-size: 11px;
    color: @git_fg_secondary;
}

.git-author-name {
    font-size: 11px;
    color: @git_fg_secondary;
}

.git-avatar {
    border-radius: 50%;
    color: @git_fg_dim;
}

/* === REFS DANS LE GIT GRAPH === */
.git-ref-branch {
    font-size: 10px;
    background-color: #4ec9b0;
    color: @git_bg_primary;
    padding: 2px 6px;
    border-radius: 3px;
    margin: 0 3px;
    font-weight: 500;
    border: 1px solid #4ec9b0;
}

.git-ref-remote {
    font-size: 10px;
    background-color: #569cd6;
    color: white;
    padding: 2px 6px;
    border-radius: 3px;
    margin: 0 3px;
    font-weight: 500;
    border: 1px solid #569cd6;
}

.git-ref-tag {
    font-size: 10px;
    background-color: #dcdcaa;
    color: @git_bg_primary;
    padding: 2px 6px;
    border-radius: 3px;
    margin: 0 3px;
    font-weight: 500;
    border: 1px solid #dcdcaa;
}

/* === PANNEAU DE DÉTAILS === */
.commit-details-panel {
    background-color: @git_bg_secondary;
    border-left: 1px solid @git_border;
}

.detail-tabs {
    background-color: @git_bg_tertiary;
    border-bottom: 1px solid @git_border;
    padding: 0;
    min-height: 32px;
}

.detail-tab {
    background-color: transparent;
    border: none;
    border-radius: 0;
    padding: 6px 12px;
    margin: 0;
    color: @git_fg_secondary;
    font-weight: 400;
    font-size: 12px;
    border-right: 1px solid @git_border;
}

.detail-tab:hover {
    background-color: @git_bg_secondary;
    color: @git_fg_primary;
}

.detail-tab:checked {
    background-color: @git_bg_primary;
    color: @git_fg_primary;
    border-bottom: 2px solid @git_accent;
}

/* === HEADER DU REPO === */
.repo-header {
    background-color: @git_bg_secondary;
    border-bottom: 1px solid @git_border;
    padding: 8px 12px;
}

.branch-name {
    font-weight: 600;
    color: @git_fg_primary;
    font-size: 13px;
}

/* === LISTE DES COMMITS STYLE GIT === */
.commit-paned {
    background-color: @git_bg_primary;
}

.commit-list {
    background-color: @git_bg_primary;
    border: none;
    margin: 0;
}

.commit-list row {
    border-bottom: 1px solid @git_border;
    padding: 0;
    min-height: 22px;
}

.commit-list row:hover {
    background-color: @git_bg_tertiary;
}

.commit-list row:selected {
    background-color: @git_selection;
    color: @git_fg_primary;
}

.commit-hash {
    font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
    font-size: 11px;
    color: @git_accent;
    min-width: 60px;
}

.commit-summary {
    font-size: 13px;
    color: @git_fg_primary;
    font-weight: 400;
}

.commit-author {
    font-size: 11px;
    color: @git_fg_secondary;
}

.commit-ref {
    font-size: 10px;
    background-color: @git_success;
    color: @git_bg_primary;
    padding: 1px 4px;
    border-radius: 2px;
    margin: 0 2px;
}

/* === DÉTAILS DU COMMIT === */
.commit-detail {
    background-color: @git_bg_primary;
    border: none;
    margin: 0;
    padding: 8px;
    font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
    font-size: 12px;
    line-height: 1.3;
    color: @git_fg_primary;
}

/* === BOUTONS STYLE GIT === */
button.suggested-action {
    background-color: @git_accent;
    color: white;
    border: 1px solid @git_accent;
    border-radius: 2px;
    padding: 4px 8px;
    font-size: 11px;
    font-weight: 400;
}

button.suggested-action:hover {
    background-color: @git_accent_hover;
    border-color: @git_accent_hover;
}

/* === MENU BUTTONS === */
.menu-button {
    padding: 4px 8px;
    border-radius: 2px;
    font-size: 13px;
}

.menu-button:hover {
    background-color: @git_bg_tertiary;
}

/* === ENTRÉES DE RECHERCHE === */
entry {
    background-color: @git_bg_tertiary;
    border: 1px solid @git_border;
    border-radius: 2px;
    color: @git_fg_primary;
    padding: 4px 8px;
    font-size: 13px;
}

entry:focus {
    border-color: @git_accent;
    box-shadow: 0 0 0 1px @git_accent;
}

entry placeholder {
    color: @git_fg_dim;
}

/* === LABELS === */
.dim-label {
    color: @git_fg_dim;
    font-size: 11px;
}

.placeholder-text {
    color: @git_fg_dim;
    font-size: 13px;
}

/* === SÉPARATEURS === */
separator {
    background-color: @git_border;
    min-width: 1px;
    min-height: 1px;
}

/* === SCROLLBARS === */
scrollbar {
    background-color: transparent;
    min-width: 14px;
}

scrollbar slider {
    background-color: @git_fg_dim;
    border-radius: 7px;
    min-width: 6px;
    min-height: 20px;
    margin: 4px;
}

scrollbar slider:hover {
    background-color: @git_fg_secondary;
}

/* === FOCUS === */
*:focus {
    outline: 1px solid @git_accent;
    outline-offset: -1px;
}

button:focus,
entry:focus {
    outline: none;
    box-shadow: 0 0 0 1px @git_accent;
}
`
}
