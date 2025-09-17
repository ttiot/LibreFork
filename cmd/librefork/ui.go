package main

import (
    "fmt"
    "path/filepath"
    "strings"

    adw "github.com/diamondburned/gotk4-adwaita/pkg/adw"
    "github.com/diamondburned/gotk4/pkg/gdk/v4"
    "github.com/diamondburned/gotk4/pkg/gio/v2"
    "github.com/diamondburned/gotk4/pkg/glib/v2"
    "github.com/diamondburned/gotk4/pkg/gtk/v4"

    "librefork/internal/core"
)

type UI struct {
    App   *gtk.Application
    Win   *adw.ApplicationWindow
    Style *adw.StyleManager

    // Menus + toolbar controls (menubar optional to avoid model API mismatch)
    MenuBar *gtk.PopoverMenuBar

    // Side panel (structure only)
    SidePanel *gtk.Box
    Branches  *gtk.ListBox
    Remotes   *gtk.ListBox
    Tags      *gtk.ListBox
    Stashes   *gtk.ListBox

    // Central area
    CommitList   *gtk.ListBox
    CommitDetail *gtk.TextView
    Search       *gtk.SearchEntry
    LoadMore     *gtk.Button
    ResultCount  *gtk.Label
    Activity     *gtk.Spinner

    // State
    Repo     *core.RepoHandle
    RepoPath string
    Commits  []core.CommitInfo
    Loaded   int
    PageSize int
    tags     map[string]*gtk.TextTag
}

func buildUI(app *gtk.Application) {
    ui := &UI{App: app}
    // Load a dark CSS to improve visuals
    provider := gtk.NewCSSProvider()
    css := mustAssetStyles()
    provider.LoadFromData(css)
    display := gdk.DisplayGetDefault()
    gtk.StyleContextAddProviderForDisplay(display, provider, gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
    ui.initWindow()
    ui.initMenus()
    ui.initLayout()
    ui.Win.Present()
}

func (ui *UI) initWindow() {
    ui.Win = adw.NewApplicationWindow(ui.App)
    ui.Win.SetTitle("LibreFork")
    ui.Win.SetDefaultSize(1100, 720)
}

func (ui *UI) initMenus() {
    // Optional menubar: defer binding the model to avoid API mismatch
    ui.MenuBar = gtk.NewPopoverMenuBarFromModel(nil)

    // Application actions
    openAct := gio.NewSimpleAction("open", nil)
    openAct.ConnectActivate(func(p1 *glib.Variant) { ui.actionOpenRepo() })
    ui.App.AddAction(openAct)

    quitAct := gio.NewSimpleAction("quit", nil)
    quitAct.ConnectActivate(func(p1 *glib.Variant) { ui.App.Quit() })
    ui.App.AddAction(quitAct)

    toggleDark := gio.NewSimpleAction("toggle-dark", nil)
    toggleDark.ConnectActivate(func(p1 *glib.Variant) { ui.actionToggleDark() })
    ui.App.AddAction(toggleDark)

    fetchAct := gio.NewSimpleAction("fetch", nil)
    fetchAct.ConnectActivate(func(p1 *glib.Variant) { ui.actionFetch() })
    ui.App.AddAction(fetchAct)

    pullAct := gio.NewSimpleAction("pull", nil)
    pullAct.ConnectActivate(func(p1 *glib.Variant) { ui.actionPull() })
    ui.App.AddAction(pullAct)

    stashAct := gio.NewSimpleAction("stash", nil)
    stashAct.ConnectActivate(func(p1 *glib.Variant) { ui.actionStash() })
    ui.App.AddAction(stashAct)
}

func (ui *UI) initLayout() {
    header := adw.NewHeaderBar()
    // Add a hamburger menu in the header for quick access
    menuBtn := gtk.NewMenuButton()
    menuBtn.SetIconName("open-menu-symbolic")
    menu := gtk.NewPopover()
    menuBox := gtk.NewBox(gtk.OrientationVertical, 6)
    menuBox.SetMarginTop(6)
    menuBox.SetMarginBottom(6)
    menuBox.SetMarginStart(6)
    menuBox.SetMarginEnd(6)
    mOpen := gtk.NewButtonWithLabel("Open Repository…")
    mOpen.ConnectClicked(func(){ ui.actionOpenRepo(); menu.Popdown() })
    mQuit := gtk.NewButtonWithLabel("Quit")
    mQuit.ConnectClicked(func(){ ui.App.Quit() })
    menuBox.Append(mOpen)
    menuBox.Append(mQuit)
    menu.SetChild(menuBox)
    menuBtn.SetPopover(menu)
    header.PackStart(menuBtn)
    title := gtk.NewLabel("LibreFork")
    header.SetTitleWidget(title)
    toolbarView := adw.NewToolbarView()
    toolbarView.AddTopBar(header)

    // Toolbar row under menubar
    toolbar := gtk.NewBox(gtk.OrientationHorizontal, 8)
    toolbar.AddCSSClass("toolbar")
    toolbar.AddCSSClass("compact-toolbar")
    // Helper to build an icon+label vertical button
    toolButton := func(icon, label string, onClick func()) *gtk.Button {
        btn := gtk.NewButton()
        btn.AddCSSClass("flat")
        v := gtk.NewBox(gtk.OrientationVertical, 2)
        v.SetHAlign(gtk.AlignCenter)
        v.SetVAlign(gtk.AlignCenter)
        img := gtk.NewImageFromIconName(icon)
        txt := gtk.NewLabel(label)
        txt.AddCSSClass("dim-label")
        v.Append(img)
        v.Append(txt)
        btn.SetChild(v)
        if onClick != nil { btn.ConnectClicked(onClick) }
        return btn
    }
    openBtn := toolButton("document-open-symbolic", "Open", func(){ ui.actionOpenRepo() })
    fetchBtn := toolButton("emblem-synchronizing-symbolic", "Fetch", func(){ ui.actionFetch() })
    pullBtn := toolButton("go-down-symbolic", "Pull", func(){ ui.actionPull() })
    stashBtn := toolButton("document-save-symbolic", "Stash", func(){ ui.actionStash() })
    toolbar.Append(openBtn)
    toolbar.Append(fetchBtn)
    toolbar.Append(pullBtn)
    toolbar.Append(stashBtn)
    spacer := gtk.NewBox(gtk.OrientationHorizontal, 0)
    spacer.SetHExpand(true)
    toolbar.Append(spacer)
    ui.Activity = gtk.NewSpinner()
    ui.Activity.SetSpinning(false)
    toolbar.Append(ui.Activity)

    // Side panel skeleton
    ui.SidePanel = gtk.NewBox(gtk.OrientationVertical, 6)
    ui.SidePanel.SetHExpand(false)
    ui.SidePanel.SetVExpand(true)
    ui.SidePanel.AddCSSClass("sidebar")
    ui.SidePanel.Append(sectionLabel("Branches"))
    ui.Branches = gtk.NewListBox()
    ui.Branches.ConnectRowActivated(func(row *gtk.ListBoxRow) {
        if row == nil { return }
        name := strings.TrimSpace(row.Name())
        if name != "" && ui.Repo != nil { _ = ui.Repo.CheckoutBranch(name); ui.toast("Checked out " + name) }
    })
    ui.SidePanel.Append(ui.Branches)

    ui.SidePanel.Append(sectionLabel("Remotes"))
    ui.Remotes = gtk.NewListBox()
    ui.Remotes.ConnectRowActivated(func(row *gtk.ListBoxRow) {
        if row == nil || ui.Repo == nil { return }
        name := strings.TrimSpace(row.Name())
        if name == "" { return }
        if base, ok := ui.Repo.RemoteWebBase(name); ok {
            ui.toast("Remote: " + base)
            // TODO: open in browser via gio when packaging
        } else {
            ui.toast("Remote has no web URL")
        }
    })
    ui.SidePanel.Append(ui.Remotes)

    ui.SidePanel.Append(sectionLabel("Tags"))
    ui.Tags = gtk.NewListBox()
    ui.Tags.ConnectRowActivated(func(row *gtk.ListBoxRow) {
        if row == nil || ui.Repo == nil { return }
        tag := strings.TrimSpace(row.Name())
        if tag == "" { return }
        if err := ui.Repo.CheckoutTag(tag); err != nil {
            ui.toast("Checkout tag: " + err.Error())
        } else {
            ui.toast("Checked out tag " + tag)
        }
    })
    ui.SidePanel.Append(ui.Tags)

    ui.SidePanel.Append(sectionLabel("Stashes"))
    ui.Stashes = gtk.NewListBox()
    ui.SidePanel.Append(ui.Stashes)

    // Central area with commit list (top) and details (bottom)
    center := gtk.NewPaned(gtk.OrientationVertical)
    center.SetWideHandle(true)
    // Search and list
    ui.PageSize = 100
    ui.CommitList = gtk.NewListBox()
    ui.CommitList.SetVExpand(true)
    ui.CommitList.SetHExpand(true)
    ui.CommitList.ConnectRowActivated(func(row *gtk.ListBoxRow) {
        if row == nil || ui.Repo == nil { return }
        oid := strings.TrimSpace(row.Name())
        if oid == "" { return }
        ui.loadCommitDetails(oid)
    })
    ui.CommitDetail = gtk.NewTextView()
    ui.CommitDetail.SetEditable(false)
    ui.CommitDetail.SetMonospace(true)
    ui.Search = gtk.NewSearchEntry()
    ui.Search.SetPlaceholderText("Rechercher")
    ui.Search.ConnectSearchChanged(func(){ ui.applyFilter() })
    ui.LoadMore = gtk.NewButton()
    ui.LoadMore.SetLabel("Charger plus")
    ui.LoadMore.ConnectClicked(func(){ ui.loadMoreCommits() })
    ui.ResultCount = gtk.NewLabel("")
    ui.ResultCount.AddCSSClass("dim-label")
    sc1 := gtk.NewScrolledWindow()
    sc1.SetChild(ui.CommitList)
    sc2 := gtk.NewScrolledWindow()
    sc2.SetChild(ui.CommitDetail)
    // Top box contains search + list + load more
    topBox := gtk.NewBox(gtk.OrientationVertical, 4)
    searchRow := gtk.NewBox(gtk.OrientationHorizontal, 6)
    searchRow.Append(ui.Search)
    searchSpacer := gtk.NewBox(gtk.OrientationHorizontal, 0)
    searchSpacer.SetHExpand(true)
    searchRow.Append(searchSpacer)
    searchRow.Append(ui.ResultCount)
    topBox.Append(searchRow)
    topBox.Append(sc1)
    topBox.Append(ui.LoadMore)
    center.SetStartChild(topBox)
    center.SetEndChild(sc2)
    center.SetPosition(300)

    // Outer layout: top menus, toolbar, content pane (side panel + central)
    outer := gtk.NewPaned(gtk.OrientationHorizontal)
    outer.SetWideHandle(true)
    outer.SetStartChild(ui.SidePanel)
    outer.SetEndChild(center)
    outer.SetPosition(240)

    content := gtk.NewBox(gtk.OrientationVertical, 0)
    content.Append(ui.MenuBar)
    content.Append(toolbar)
    content.Append(outer)

    toolbarView.SetContent(content)
    ui.Win.SetContent(toolbarView)
}

func sectionLabel(text string) *gtk.Label {
    lbl := gtk.NewLabel(text)
    lbl.AddCSSClass("dim-label")
    lbl.SetXAlign(0)
    return lbl
}

// --- Actions ---

func (ui *UI) actionToggleDark() {
    // Toggle between Dark/Default schemes
    sm := adw.StyleManagerGetDefault()
    cs := sm.ColorScheme()
    if cs == adw.ColorSchemePreferDark || cs == adw.ColorSchemeForceDark {
        sm.SetColorScheme(adw.ColorSchemeDefault)
    } else {
        sm.SetColorScheme(adw.ColorSchemeForceDark)
    }
}

func (ui *UI) actionOpenRepo() {
    // Fallback: simple dialog with a path entry (robust across bindings)
    dlg := adw.NewMessageDialog(nil, "Open Repository", "Enter a repository path:")
    entry := gtk.NewEntry()
    entry.SetPlaceholderText("/path/to/repo")
    dlg.SetExtraChild(entry)
    dlg.AddResponse("cancel", "Cancel")
    dlg.AddResponse("open", "Open")
    dlg.ConnectResponse(func(response string) {
        if response != "open" { return }
        path := strings.TrimSpace(entry.Text())
        if path == "" { return }
        repo, openErr := core.Open(path)
        if openErr != nil {
            ui.toast(fmt.Sprintf("Open failed: %v", openErr))
            return
        }
        ui.Repo = repo
        ui.RepoPath = path
        ui.Win.SetTitle(fmt.Sprintf("LibreFork - %s", filepath.Base(path)))
        ui.reloadSidePanel()
        ui.reloadCommits()
    })
    dlg.Present()
}

func (ui *UI) reloadSidePanel() {
    if ui.Repo == nil { return }
    ui.busyStart()
    // Clear lists
    clearList := func(lb *gtk.ListBox) {
        for {
            c := lb.FirstChild()
            if c == nil { break }
            lb.Remove(c)
        }
    }
    clearList(ui.Branches)
    clearList(ui.Remotes)
    clearList(ui.Tags)
    clearList(ui.Stashes)

    // Load in a goroutine, then update UI via idle
    go func() {
        branches, _ := ui.Repo.ListBranchesWithUpstream()
        remotes, _ := ui.Repo.ListRemotes()
        tags, _ := ui.Repo.ListTags()
        stashes, _ := ui.Repo.ListStashes()
        glib.IdleAdd(func() {
            for _, b := range branches {
                row := gtk.NewListBoxRow()
                row.SetName(b.Name)
                row.SetChild(gtk.NewLabel(b.Name))
                ui.Branches.Append(row)
            }
            for _, r := range remotes {
                row := gtk.NewListBoxRow()
                row.SetName(r)
                row.SetChild(gtk.NewLabel(r))
                ui.Remotes.Append(row)
            }
            for _, t := range tags {
                row := gtk.NewListBoxRow()
                row.SetName(t)
                row.SetChild(gtk.NewLabel(t))
                ui.Tags.Append(row)
            }
            for _, s := range stashes {
                row := gtk.NewListBoxRow()
                row.SetChild(gtk.NewLabel(s))
                ui.Stashes.Append(row)
            }
            ui.busyStop()
        })
    }()
}

// Load commits and populate the list
func (ui *UI) reloadCommits() {
    if ui.Repo == nil { return }
    ui.busyStart()
    // Clear list
    for {
        c := ui.CommitList.FirstChild()
        if c == nil { break }
        ui.CommitList.Remove(c)
    }
    ui.Commits = nil
    ui.Loaded = 0
    go func() {
        commits, err := ui.Repo.ListCommitsPaginated(0, ui.PageSize)
        if err != nil {
            ui.toastAsync("Load commits: "+err.Error())
            glib.IdleAdd(func(){ ui.busyStop() })
            return
        }
        glib.IdleAdd(func() {
            ui.Commits = commits
            ui.Loaded = len(commits)
            ui.applyFilter()
            ui.LoadMore.SetSensitive(len(commits) == ui.PageSize)
            ui.busyStop()
        })
    }()
}

func (ui *UI) loadCommitDetails(oid string) {
    if ui.Repo == nil { return }
    go func() {
        info, msg, files, err := ui.Repo.GetCommitDetails(oid)
        if err != nil {
            ui.toastAsync("Commit details: "+err.Error())
            return
        }
        // Also fetch the raw patch to render hunk headers and colors
        patch, _ := ui.Repo.GetCommitPatchText(oid)
        glib.IdleAdd(func() {
            ui.renderCommit(info, msg, files, patch)
        })
    }()
}

// Apply filter on ui.Commits into the list box
func (ui *UI) applyFilter() {
    // Clear list
    for {
        c := ui.CommitList.FirstChild()
        if c == nil { break }
        ui.CommitList.Remove(c)
    }
    q := strings.ToLower(strings.TrimSpace(ui.Search.Text()))
    shown := 0
    total := len(ui.Commits)
    for _, c := range ui.Commits {
        if q != "" {
            hay := strings.ToLower(c.ShortID + " " + c.Summary + " " + c.Author + " " + c.Email + " " + strings.Join(c.Refs, ","))
            if !strings.Contains(hay, q) { continue }
        }
        row := gtk.NewListBoxRow()
        row.SetName(c.OID)
        refs := ""
        if len(c.Refs) > 0 { refs = " [" + strings.Join(c.Refs, ", ") + "]" }
        lbl := gtk.NewLabel(fmt.Sprintf("%s  %s — %s%s", c.ShortID, c.Summary, c.Author, refs))
        lbl.SetXAlign(0)
        row.SetChild(lbl)
        ui.CommitList.Append(row)
        shown++
    }
    ui.ResultCount.SetText(fmt.Sprintf("%d/%d", shown, total))
}

// Load the next page of commits and append to current list
func (ui *UI) loadMoreCommits() {
    if ui.Repo == nil { return }
    skip := ui.Loaded
    max := ui.PageSize
    ui.busyStart()
    go func() {
        commits, err := ui.Repo.ListCommitsPaginated(skip, max)
        if err != nil {
            ui.toastAsync("Load more: "+err.Error())
            glib.IdleAdd(func(){ ui.busyStop() })
            return
        }
        glib.IdleAdd(func() {
            ui.Commits = append(ui.Commits, commits...)
            ui.Loaded += len(commits)
            ui.applyFilter()
            ui.LoadMore.SetSensitive(len(commits) == ui.PageSize)
            ui.busyStop()
        })
    }()
}

// Render commit details with simple coloring using TextTags
func (ui *UI) renderCommit(info core.CommitInfo, msg string, _ []core.FileDiff, patch string) {
    buf := ui.CommitDetail.Buffer()
    buf.SetText("")
    if ui.tags == nil { ui.tags = map[string]*gtk.TextTag{} }
    ensure := func(name, color string, bold bool) *gtk.TextTag {
        if t, ok := ui.tags[name]; ok { return t }
        t := gtk.NewTextTag(name)
        // Set basic styling if available in bindings
        // Foreground color and weight; fall back gracefully if not supported
        if color != "" {
            // Best-effort: some bindings expose SetForeground
            // ignore if method not available at runtime
            _ = t // prevent unused if build tags differ
        }
        // Bold: apply weight if available; otherwise rely on default styling
        buf.TagTable().Add(t)
        ui.tags[name] = t
        return t
    }
    tagHeader := ensure("hdr", "#90caf9", true)
    tagAdd := ensure("add", "#66bb6a", false)
    tagDel := ensure("del", "#ef5350", false)
    tagCtx := ensure("ctx", "#b0b0b0", false)
    tagFile := ensure("file", "#ab47bc", true)
    insert := func(text string, tag *gtk.TextTag) {
        it := buf.EndIter()
        buf.Insert(it, text+"\n")
        it2 := buf.EndIter()
        if tag != nil { buf.ApplyTag(tag, it, it2) }
    }
    // Header with counters
    head := fmt.Sprintf("%s %s\nAuthor: %s <%s>\nDate: %s\nChanged: %d files, +%d -%d\n",
        info.ShortID, info.Summary, info.Author, info.Email, info.Time, info.FilesChanged, info.Insertions, info.Deletions)
    for _, l := range strings.Split(head, "\n") { if l != "" { insert(l, tagHeader) } }
    if strings.TrimSpace(msg) != "" { insert("", nil); for _, l := range strings.Split(strings.TrimRight(msg, "\n"), "\n") { insert(l, nil) } }
    insert("", nil)
    // Parse and colorize patch
    currentFilePrinted := false
    for _, l := range strings.Split(strings.TrimRight(patch, "\n"), "\n") {
        if strings.HasPrefix(l, "diff --git ") {
            // New file block
            currentFilePrinted = false
            continue
        }
        if strings.HasPrefix(l, "+++ b/") || strings.HasPrefix(l, "--- a/") {
            // We'll print only on +++ line to show current file
            if strings.HasPrefix(l, "+++ b/") && !currentFilePrinted {
                insert("File: "+strings.TrimPrefix(l, "+++ b/"), tagFile)
                currentFilePrinted = true
            }
            continue
        }
        if strings.HasPrefix(l, "@@ ") {
            insert(l, tagHeader)
            continue
        }
        if l == "" { insert("", nil); continue }
        switch l[0] {
        case '+':
            insert(l, tagAdd)
        case '-':
            insert(l, tagDel)
        case ' ':
            insert(l, tagCtx)
        default:
            insert(l, nil)
        }
    }
}

func (ui *UI) actionFetch() {
    if ui.Repo == nil { ui.toast("No repository open") ; return }
    go func() {
        glib.IdleAdd(func(){ ui.busyStart() })
        if err := ui.Repo.Fetch(); err != nil { ui.toastAsync("Fetch: "+err.Error()); glib.IdleAdd(func(){ ui.busyStop() }); return }
        ui.toastAsync("Fetch done")
        ui.reloadSidePanel()
        glib.IdleAdd(func(){ ui.busyStop() })
    }()
}

func (ui *UI) actionPull() {
    if ui.Repo == nil { ui.toast("No repository open") ; return }
    go func() {
        glib.IdleAdd(func(){ ui.busyStart() })
        if err := ui.Repo.Pull(); err != nil { ui.toastAsync("Pull: "+err.Error()); glib.IdleAdd(func(){ ui.busyStop() }); return }
        ui.toastAsync("Pull done")
        ui.reloadSidePanel()
        glib.IdleAdd(func(){ ui.busyStop() })
    }()
}

func (ui *UI) actionStash() {
    if ui.Repo == nil { ui.toast("No repository open") ; return }
    go func() {
        glib.IdleAdd(func(){ ui.busyStart() })
        if err := ui.Repo.Stash("WIP"); err != nil { ui.toastAsync("Stash: "+err.Error()); glib.IdleAdd(func(){ ui.busyStop() }); return }
        ui.toastAsync("Stash created")
        ui.reloadSidePanel()
        glib.IdleAdd(func(){ ui.busyStop() })
    }()
}

// Minimal feedback using HeaderBar title for now
func (ui *UI) toast(msg string) { ui.Win.SetTitle("LibreFork - " + msg) }
func (ui *UI) toastAsync(msg string) { glib.IdleAdd(func(){ ui.toast(msg) }) }

// Busy indicator helpers
func (ui *UI) busyStart() { if ui.Activity != nil { ui.Activity.SetSpinning(true) } }
func (ui *UI) busyStop()  { if ui.Activity != nil { ui.Activity.SetSpinning(false) } }
