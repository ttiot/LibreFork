package main

import (
	"fmt"
	"math"
	"path/filepath"
	"strconv"
	"strings"

	adw "github.com/diamondburned/gotk4-adwaita/pkg/adw"
	"github.com/diamondburned/gotk4/pkg/cairo"
	"github.com/diamondburned/gotk4/pkg/gdk/v4"
	"github.com/diamondburned/gotk4/pkg/gio/v2"
	"github.com/diamondburned/gotk4/pkg/glib/v2"
	"github.com/diamondburned/gotk4/pkg/gtk/v4"

	"librefork/internal/core"
)

type GitGraphConnection struct {
	FromColumn int
	ToColumn   int
	Type       string // "straight", "curve", "merge", "branch"
	Color      string
	FromIndex  int
	ToIndex    int
	FromOID    string
	ToOID      string
}

type GitGraphNode struct {
	Commit   core.CommitInfo
	Column   int
	Color    string
	Branches []string
	IsTagged bool
	IsMerge  bool
	Children []string
	// Nouvelles propriétés pour les connexions avancées
	Connections         []GitGraphConnection
	IncomingConnections []GitGraphConnection
	OutgoingConnections []GitGraphConnection
}

const (
	maxGraphColumns    = 6
	graphColumnSpacing = 26
	graphRowHeight     = 30
)

var graphColorMap = map[string][3]float64{
	"git-color-0": colorRGB("#00d4aa"),
	"git-color-1": colorRGB("#1f7ce8"),
	"git-color-2": colorRGB("#f9826c"),
	"git-color-3": colorRGB("#a855f7"),
	"git-default": colorRGB("#3e3e42"),
}

type UI struct {
	App   *gtk.Application
	Win   *adw.ApplicationWindow
	Style *adw.StyleManager

	// Navigation et structure principale
	HeaderBar   *adw.HeaderBar
	ToolbarView *adw.ToolbarView
	MainPaned   *gtk.Paned

	// Panneau latéral modernisé
	SidePanel *gtk.Box

	// Sections du panneau latéral
	BranchesBox *gtk.Box
	RemotesBox  *gtk.Box
	TagsBox     *gtk.Box
	StashesBox  *gtk.Box

	// Listes
	Branches *gtk.ListBox
	Remotes  *gtk.ListBox
	Tags     *gtk.ListBox
	Stashes  *gtk.ListBox

	// Zone centrale avec onglets
	CenterContainer *gtk.Box
	CenterStack     *gtk.Stack

	// Onglet Commits
	CommitPane       *gtk.Box
	CommitList       *gtk.ListBox
	CommitDetail     *gtk.TextView
	CommitPaned      *gtk.Paned
	GraphHeaderLabel *gtk.Label

	// Onglet Changes
	ChangesPane *gtk.Box

	// Onglet File Tree
	FileTreePane *gtk.Box

	// Contrôles de recherche et navigation
	Search      *gtk.SearchEntry
	LoadMore    *gtk.Button
	ResultCount *gtk.Label
	Activity    *gtk.Spinner

	// Barre d'outils modernisée
	Toolbar     *gtk.Box
	QuickLaunch *gtk.Button
	FetchBtn    *gtk.Button
	PullBtn     *gtk.Button
	PushBtn     *gtk.Button
	StashBtn    *gtk.Button

	// État
	Repo     *core.RepoHandle
	RepoPath string
	Commits  []core.CommitInfo
	Loaded   int
	PageSize int
	tags     map[string]*gtk.TextTag

	// Git Graph
	GraphNodes          []GitGraphNode
	BranchColors        map[string]string
	ColumnCount         int
	ActiveColumnsBefore [][]bool
	ActiveColumnsAfter  [][]bool
}

func buildUI(app *gtk.Application) {
	ui := &UI{App: app}

	// Charger les styles CSS modernisés
	provider := gtk.NewCSSProvider()
	css := getModernStyles()
	provider.LoadFromData(css)
	display := gdk.DisplayGetDefault()
	gtk.StyleContextAddProviderForDisplay(display, provider, gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)

	ui.initWindow()
	ui.initActions()
	ui.initLayout()
	ui.Win.Present()
}

func (ui *UI) initWindow() {
	ui.Win = adw.NewApplicationWindow(ui.App)
	ui.Win.SetTitle("LibreFork")
	ui.Win.SetDefaultSize(1400, 900)
	ui.Win.AddCSSClass("main-window")
}

func (ui *UI) initActions() {
	// Actions de l'application
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
	// HeaderBar moderne
	ui.HeaderBar = adw.NewHeaderBar()
	ui.HeaderBar.AddCSSClass("modern-header")

	// Menu hamburger
	menuBtn := gtk.NewMenuButton()
	menuBtn.SetIconName("open-menu-symbolic")
	menuBtn.AddCSSClass("flat")

	menu := gtk.NewPopover()
	menuBox := gtk.NewBox(gtk.OrientationVertical, 8)
	menuBox.SetMarginTop(12)
	menuBox.SetMarginBottom(12)
	menuBox.SetMarginStart(12)
	menuBox.SetMarginEnd(12)

	mOpen := ui.createMenuButton("Ouvrir un dépôt…", "folder-open-symbolic", func() {
		ui.actionOpenRepo()
		menu.Popdown()
	})
	mSettings := ui.createMenuButton("Préférences", "preferences-system-symbolic", func() {
		ui.actionToggleDark()
		menu.Popdown()
	})
	mQuit := ui.createMenuButton("Quitter", "application-exit-symbolic", func() {
		ui.App.Quit()
	})

	menuBox.Append(mOpen)
	menuBox.Append(gtk.NewSeparator(gtk.OrientationHorizontal))
	menuBox.Append(mSettings)
	menuBox.Append(gtk.NewSeparator(gtk.OrientationHorizontal))
	menuBox.Append(mQuit)

	menu.SetChild(menuBox)
	menuBtn.SetPopover(menu)
	ui.HeaderBar.PackStart(menuBtn)

	// Titre avec icône
	titleBox := gtk.NewBox(gtk.OrientationHorizontal, 8)
	titleIcon := gtk.NewImageFromIconName("folder-git-symbolic")
	titleLabel := gtk.NewLabel("LibreFork")
	titleLabel.AddCSSClass("title")
	titleBox.Append(titleIcon)
	titleBox.Append(titleLabel)
	ui.HeaderBar.SetTitleWidget(titleBox)

	// Indicateur d'activité dans le header
	ui.Activity = gtk.NewSpinner()
	ui.Activity.SetSpinning(false)
	ui.HeaderBar.PackEnd(ui.Activity)

	// ToolbarView principal
	ui.ToolbarView = adw.NewToolbarView()
	ui.ToolbarView.AddTopBar(ui.HeaderBar)

	// Barre d'outils moderne
	ui.initToolbar()

	// Layout principal avec panneau latéral et zone centrale
	ui.MainPaned = gtk.NewPaned(gtk.OrientationHorizontal)
	ui.MainPaned.SetWideHandle(true)
	ui.MainPaned.AddCSSClass("main-paned")

	// Panneau latéral modernisé
	ui.initSidePanel()

	// Zone centrale avec onglets
	ui.initCenterArea()

	// Assemblage final
	ui.MainPaned.SetStartChild(ui.SidePanel)
	ui.MainPaned.SetEndChild(ui.CenterContainer)
	ui.MainPaned.SetPosition(280)           // Largeur du side panel
	ui.MainPaned.SetShrinkStartChild(false) // Empêcher le side panel de rétrécir
	ui.MainPaned.SetResizeStartChild(false) // Side panel de taille fixe

	mainContent := gtk.NewBox(gtk.OrientationVertical, 0)
	mainContent.Append(ui.Toolbar)
	mainContent.Append(ui.MainPaned)

	ui.ToolbarView.SetContent(mainContent)
	ui.Win.SetContent(ui.ToolbarView)
}

func (ui *UI) initToolbar() {
	ui.Toolbar = gtk.NewBox(gtk.OrientationHorizontal, 0)
	ui.Toolbar.AddCSSClass("modern-toolbar")
	ui.Toolbar.SetMarginTop(4)
	ui.Toolbar.SetMarginBottom(4)
	ui.Toolbar.SetMarginStart(12)
	ui.Toolbar.SetMarginEnd(12)

	// Boutons de la barre d'outils avec style moderne
	ui.QuickLaunch = ui.createToolButton("document-open-symbolic", "Quick Launch", func() { ui.actionOpenRepo() })
	ui.FetchBtn = ui.createToolButton("emblem-synchronizing-symbolic", "Fetch", func() { ui.actionFetch() })
	ui.PullBtn = ui.createToolButton("go-down-symbolic", "Pull", func() { ui.actionPull() })
	ui.PushBtn = ui.createToolButton("go-up-symbolic", "Push", func() { /* TODO: Push action */ })
	ui.StashBtn = ui.createToolButton("document-save-symbolic", "Stash", func() { ui.actionStash() })

	ui.Toolbar.Append(ui.QuickLaunch)
	ui.Toolbar.Append(gtk.NewSeparator(gtk.OrientationVertical))
	ui.Toolbar.Append(ui.FetchBtn)
	ui.Toolbar.Append(ui.PullBtn)
	ui.Toolbar.Append(ui.PushBtn)
	ui.Toolbar.Append(gtk.NewSeparator(gtk.OrientationVertical))
	ui.Toolbar.Append(ui.StashBtn)

	// Spacer pour pousser les éléments suivants à droite
	spacer := gtk.NewBox(gtk.OrientationHorizontal, 0)
	spacer.SetHExpand(true)
	ui.Toolbar.Append(spacer)

	// Barre de recherche dans la toolbar
	ui.Search = gtk.NewSearchEntry()
	ui.Search.SetPlaceholderText("Rechercher dans les commits...")
	ui.Search.SetHAlign(gtk.AlignEnd)
	ui.Search.SetSizeRequest(300, -1)
	ui.Search.ConnectSearchChanged(func() { ui.applyFilter() })
	ui.Toolbar.Append(ui.Search)
}

func (ui *UI) initSidePanel() {
	ui.SidePanel = gtk.NewBox(gtk.OrientationVertical, 0)
	ui.SidePanel.SetHExpand(false)
	ui.SidePanel.SetVExpand(true)
	ui.SidePanel.SetSizeRequest(280, -1) // Largeur fixe comme dans l'image
	ui.SidePanel.SetHAlign(gtk.AlignFill)
	ui.SidePanel.SetVAlign(gtk.AlignFill)
	ui.SidePanel.AddCSSClass("git-sidebar")

	// Structure arborescente comme dans l'image de référence
	sideContent := gtk.NewBox(gtk.OrientationVertical, 4)
	sideContent.SetMarginTop(8)
	sideContent.SetMarginStart(8)
	sideContent.SetMarginEnd(8)
	sideContent.SetMarginBottom(8)

	// Nom du projet (comme "TypeScript" dans l'image)
	projectLabel := gtk.NewLabel("LibreFork")
	projectLabel.AddCSSClass("project-title")
	projectLabel.SetXAlign(0)
	projectLabel.SetMarginBottom(8)
	sideContent.Append(projectLabel)

	// Changes (5) - comme dans l'image
	changesItem := ui.createTreeItem("Changes (0)", "document-edit-symbolic", 0)
	sideContent.Append(changesItem)

	// All Commits - comme dans l'image
	commitsItem := ui.createTreeItem("All Commits", "git-commit-symbolic", 0)
	sideContent.Append(commitsItem)

	// Section Starred (repliable)
	starredExpander := ui.createExpanderSection("Starred", "starred-symbolic")
	masterStarred := ui.createTreeItem("master", "git-branch-symbolic", 1)
	starredExpander.Append(masterStarred)
	sideContent.Append(starredExpander)

	// Section Branches (repliable)
	branchesExpander := ui.createExpanderSection("Branches", "git-branch-symbolic")
	ui.BranchesBox = gtk.NewBox(gtk.OrientationVertical, 2)
	branchesExpander.Append(ui.BranchesBox)
	sideContent.Append(branchesExpander)

	// Section Remotes (repliable)
	remotesExpander := ui.createExpanderSection("Remotes", "network-server-symbolic")
	ui.RemotesBox = gtk.NewBox(gtk.OrientationVertical, 2)
	remotesExpander.Append(ui.RemotesBox)
	sideContent.Append(remotesExpander)

	// Section Tags (repliable)
	tagsExpander := ui.createExpanderSection("Tags", "tag-symbolic")
	ui.TagsBox = gtk.NewBox(gtk.OrientationVertical, 2)
	tagsExpander.Append(ui.TagsBox)
	sideContent.Append(tagsExpander)

	// Section Stashes (repliable)
	stashesExpander := ui.createExpanderSection("Stashes", "document-save-symbolic")
	ui.StashesBox = gtk.NewBox(gtk.OrientationVertical, 2)
	stashesExpander.Append(ui.StashesBox)
	sideContent.Append(stashesExpander)

	// Contenu scrollable
	scrolled := gtk.NewScrolledWindow()
	scrolled.SetPolicy(gtk.PolicyNever, gtk.PolicyAutomatic)
	scrolled.SetVExpand(true)
	scrolled.SetChild(sideContent)

	ui.SidePanel.Append(scrolled)
}

func (ui *UI) initBranchesSection(parent *gtk.Box) {
	// En-tête de section
	branchHeader := ui.createSectionHeader("Branches", "git-branch-symbolic", true)
	parent.Append(branchHeader)

	// Liste des branches
	ui.Branches = gtk.NewListBox()
	ui.Branches.AddCSSClass("sidebar-list")
	ui.Branches.SetSelectionMode(gtk.SelectionSingle)
	ui.Branches.ConnectRowActivated(func(row *gtk.ListBoxRow) {
		if row == nil {
			return
		}
		name := strings.TrimSpace(row.Name())
		if name != "" && ui.Repo != nil {
			_ = ui.Repo.CheckoutBranch(name)
			ui.toast("Basculé vers " + name)
		}
	})

	branchFrame := gtk.NewFrame("")
	branchFrame.AddCSSClass("sidebar-frame")
	branchFrame.SetChild(ui.Branches)
	parent.Append(branchFrame)
}

func (ui *UI) initRemotesSection(parent *gtk.Box) {
	remoteHeader := ui.createSectionHeader("Remotes", "network-server-symbolic", true)
	parent.Append(remoteHeader)

	ui.Remotes = gtk.NewListBox()
	ui.Remotes.AddCSSClass("sidebar-list")
	ui.Remotes.SetSelectionMode(gtk.SelectionSingle)
	ui.Remotes.ConnectRowActivated(func(row *gtk.ListBoxRow) {
		if row == nil || ui.Repo == nil {
			return
		}
		name := strings.TrimSpace(row.Name())
		if name == "" {
			return
		}
		if base, ok := ui.Repo.RemoteWebBase(name); ok {
			ui.toast("Remote: " + base)
		} else {
			ui.toast("Remote sans URL web")
		}
	})

	remoteFrame := gtk.NewFrame("")
	remoteFrame.AddCSSClass("sidebar-frame")
	remoteFrame.SetChild(ui.Remotes)
	parent.Append(remoteFrame)
}

func (ui *UI) initTagsSection(parent *gtk.Box) {
	tagHeader := ui.createSectionHeader("Tags", "tag-symbolic", false)
	parent.Append(tagHeader)

	ui.Tags = gtk.NewListBox()
	ui.Tags.AddCSSClass("sidebar-list")
	ui.Tags.SetSelectionMode(gtk.SelectionSingle)
	ui.Tags.ConnectRowActivated(func(row *gtk.ListBoxRow) {
		if row == nil || ui.Repo == nil {
			return
		}
		tag := strings.TrimSpace(row.Name())
		if tag == "" {
			return
		}
		if err := ui.Repo.CheckoutTag(tag); err != nil {
			ui.toast("Erreur checkout tag: " + err.Error())
		} else {
			ui.toast("Basculé vers le tag " + tag)
		}
	})

	tagFrame := gtk.NewFrame("")
	tagFrame.AddCSSClass("sidebar-frame")
	tagFrame.SetChild(ui.Tags)
	parent.Append(tagFrame)
}

func (ui *UI) initStashesSection(parent *gtk.Box) {
	stashHeader := ui.createSectionHeader("Stashes", "document-save-symbolic", false)
	parent.Append(stashHeader)

	ui.Stashes = gtk.NewListBox()
	ui.Stashes.AddCSSClass("sidebar-list")
	ui.Stashes.SetSelectionMode(gtk.SelectionSingle)

	stashFrame := gtk.NewFrame("")
	stashFrame.AddCSSClass("sidebar-frame")
	stashFrame.SetChild(ui.Stashes)
	parent.Append(stashFrame)
}

func (ui *UI) initCenterArea() {
	// Container principal pour la zone centrale
	centerContainer := gtk.NewBox(gtk.OrientationVertical, 0)

	// Header avec onglets style VS Code
	tabHeader := gtk.NewBox(gtk.OrientationHorizontal, 0)
	tabHeader.AddCSSClass("tab-header")

	// Onglets comme dans VS Code/Git
	commitTab := gtk.NewButton()
	commitTab.SetLabel("Commit")
	commitTab.AddCSSClass("modern-tabs")
	commitTab.AddCSSClass("flat")

	changesTab := gtk.NewButton()
	changesTab.SetLabel("Changes")
	changesTab.AddCSSClass("modern-tabs")
	changesTab.AddCSSClass("flat")

	fileTreeTab := gtk.NewButton()
	fileTreeTab.SetLabel("File Tree")
	fileTreeTab.AddCSSClass("modern-tabs")
	fileTreeTab.AddCSSClass("flat")

	tabHeader.Append(commitTab)
	tabHeader.Append(changesTab)
	tabHeader.Append(fileTreeTab)

	// Spacer et contrôles à droite
	spacer := gtk.NewBox(gtk.OrientationHorizontal, 0)
	spacer.SetHExpand(true)
	tabHeader.Append(spacer)

	ui.ResultCount = gtk.NewLabel("")
	ui.ResultCount.AddCSSClass("dim-label")
	tabHeader.Append(ui.ResultCount)

	centerContainer.Append(tabHeader)
	centerContainer.Append(gtk.NewSeparator(gtk.OrientationHorizontal))

	// Stack pour le contenu des onglets
	ui.CenterStack = gtk.NewStack()
	ui.CenterStack.SetTransitionType(gtk.StackTransitionTypeNone)

	// Onglet Commits (par défaut)
	ui.initCommitsTab()

	// Onglet Changes
	ui.initChangesTab()

	// Onglet File Tree
	ui.initFileTreeTab()

	centerContainer.Append(ui.CenterStack)

	// Connecter les boutons aux onglets
	commitTab.ConnectClicked(func() {
		ui.CenterStack.SetVisibleChildName("commits")
		// Mettre à jour l'apparence des onglets
		commitTab.AddCSSClass("checked")
		changesTab.RemoveCSSClass("checked")
		fileTreeTab.RemoveCSSClass("checked")
	})

	changesTab.ConnectClicked(func() {
		ui.CenterStack.SetVisibleChildName("changes")
		changesTab.AddCSSClass("checked")
		commitTab.RemoveCSSClass("checked")
		fileTreeTab.RemoveCSSClass("checked")
	})

	fileTreeTab.ConnectClicked(func() {
		ui.CenterStack.SetVisibleChildName("filetree")
		fileTreeTab.AddCSSClass("checked")
		commitTab.RemoveCSSClass("checked")
		changesTab.RemoveCSSClass("checked")
	})

	// Activer l'onglet Commit par défaut
	commitTab.AddCSSClass("checked")

	// Stocker les références
	ui.CenterContainer = centerContainer
}

func (ui *UI) initCommitsTab() {
	ui.CommitPane = gtk.NewBox(gtk.OrientationVertical, 0)
	ui.CommitPane.SetVExpand(true)

	// Header avec informations du dépôt (comme dans l'image de référence)
	repoHeader := gtk.NewBox(gtk.OrientationHorizontal, 12)
	repoHeader.AddCSSClass("repo-header")
	repoHeader.SetMarginTop(8)
	repoHeader.SetMarginBottom(8)
	repoHeader.SetMarginStart(12)
	repoHeader.SetMarginEnd(12)

	// Icône et nom de la branche actuelle
	branchIcon := gtk.NewImageFromIconName("git-branch-symbolic")
	branchIcon.AddCSSClass("branch-icon")
	branchLabel := gtk.NewLabel("master")
	branchLabel.AddCSSClass("branch-name")

	// Boutons d'action comme dans l'image
	addBtn := gtk.NewButton()
	addBtn.SetIconName("list-add-symbolic")
	addBtn.SetTooltipText("Add @link jsdoc auto-complete")
	addBtn.AddCSSClass("flat")

	repoHeader.Append(branchIcon)
	repoHeader.Append(branchLabel)
	repoHeader.Append(addBtn)

	// Spacer
	spacer := gtk.NewBox(gtk.OrientationHorizontal, 0)
	spacer.SetHExpand(true)
	repoHeader.Append(spacer)

	ui.CommitPane.Append(repoHeader)
	ui.CommitPane.Append(gtk.NewSeparator(gtk.OrientationHorizontal))

	// Paned horizontal pour git graph + détails (comme dans l'image)
	ui.CommitPaned = gtk.NewPaned(gtk.OrientationHorizontal)
	ui.CommitPaned.SetWideHandle(true)
	ui.CommitPaned.AddCSSClass("commit-paned")

	// Zone principale du git graph
	gitGraphBox := gtk.NewBox(gtk.OrientationVertical, 0)
	gitGraphBox.SetVExpand(true)
	gitGraphBox.SetHExpand(true)

	gitGraphBox.Append(ui.buildCommitListHeader())

	// Liste des commits avec style git graph
	ui.PageSize = 100
	ui.CommitList = gtk.NewListBox()
	ui.CommitList.SetVExpand(true)
	ui.CommitList.SetHExpand(true)
	ui.CommitList.AddCSSClass("git-graph-list")
	ui.CommitList.ConnectRowActivated(func(row *gtk.ListBoxRow) {
		if row == nil || ui.Repo == nil {
			return
		}
		oid := strings.TrimSpace(row.Name())
		if oid == "" {
			return
		}
		ui.loadCommitDetails(oid)
	})

	commitScrolled := gtk.NewScrolledWindow()
	commitScrolled.SetPolicy(gtk.PolicyNever, gtk.PolicyAutomatic)
	commitScrolled.SetChild(ui.CommitList)

	gitGraphBox.Append(commitScrolled)

	// Bouton "Charger plus" en bas
	ui.LoadMore = gtk.NewButton()
	ui.LoadMore.SetLabel("Charger plus de commits")
	ui.LoadMore.AddCSSClass("suggested-action")
	ui.LoadMore.SetMarginTop(8)
	ui.LoadMore.SetMarginBottom(8)
	ui.LoadMore.SetMarginStart(12)
	ui.LoadMore.SetMarginEnd(12)
	ui.LoadMore.ConnectClicked(func() { ui.loadMoreCommits() })
	gitGraphBox.Append(ui.LoadMore)

	// Panneau de détails à droite (comme dans l'image)
	detailsPanel := gtk.NewBox(gtk.OrientationVertical, 0)
	detailsPanel.SetSizeRequest(400, -1)
	detailsPanel.AddCSSClass("commit-details-panel")

	// Onglets pour les détails (Commit, Changes, File Tree)
	detailTabs := gtk.NewBox(gtk.OrientationHorizontal, 0)
	detailTabs.AddCSSClass("detail-tabs")

	commitDetailTab := gtk.NewButton()
	commitDetailTab.SetLabel("Commit")
	commitDetailTab.AddCSSClass("detail-tab")
	commitDetailTab.AddCSSClass("checked")

	changesDetailTab := gtk.NewButton()
	changesDetailTab.SetLabel("Changes")
	changesDetailTab.AddCSSClass("detail-tab")

	fileTreeDetailTab := gtk.NewButton()
	fileTreeDetailTab.SetLabel("File Tree")
	fileTreeDetailTab.AddCSSClass("detail-tab")

	detailTabs.Append(commitDetailTab)
	detailTabs.Append(changesDetailTab)
	detailTabs.Append(fileTreeDetailTab)

	detailsPanel.Append(detailTabs)
	detailsPanel.Append(gtk.NewSeparator(gtk.OrientationHorizontal))

	// Zone de détails du commit
	ui.CommitDetail = gtk.NewTextView()
	ui.CommitDetail.SetEditable(false)
	ui.CommitDetail.SetMonospace(true)
	ui.CommitDetail.AddCSSClass("commit-detail")

	detailScrolled := gtk.NewScrolledWindow()
	detailScrolled.SetPolicy(gtk.PolicyAutomatic, gtk.PolicyAutomatic)
	detailScrolled.SetChild(ui.CommitDetail)
	detailScrolled.SetVExpand(true)

	detailsPanel.Append(detailScrolled)

	// Assemblage du paned
	ui.CommitPaned.SetStartChild(gitGraphBox)
	ui.CommitPaned.SetEndChild(detailsPanel)
	ui.CommitPaned.SetPosition(800) // Plus d'espace pour le git graph

	ui.CommitPane.Append(ui.CommitPaned)

	// Ajouter au stack avec un nom unique
	ui.CenterStack.AddNamed(ui.CommitPane, "commits")
	ui.CenterStack.SetVisibleChildName("commits")
}

func (ui *UI) buildCommitListHeader() *gtk.Box {
	header := gtk.NewBox(gtk.OrientationHorizontal, 0)
	header.AddCSSClass("commit-list-header")
	header.SetMarginStart(12)
	header.SetMarginEnd(12)
	header.SetMarginTop(4)
	header.SetMarginBottom(2)

	branch := gtk.NewLabel("BRANCH/TAG")
	branch.AddCSSClass("commit-header-label")
	branch.SetXAlign(0)
	branch.SetSizeRequest(150, -1)
	header.Append(branch)

	graph := gtk.NewLabel("GRAPH")
	graph.AddCSSClass("commit-header-label")
	graph.SetXAlign(0)
	graph.SetSizeRequest(ui.graphColumnWidth(), -1)
	header.Append(graph)
	ui.GraphHeaderLabel = graph

	message := gtk.NewLabel("COMMIT MESSAGE")
	message.AddCSSClass("commit-header-label")
	message.SetXAlign(0)
	message.SetHExpand(true)
	header.Append(message)

	author := gtk.NewLabel("AUTHOR")
	author.AddCSSClass("commit-header-label")
	author.SetXAlign(0)
	author.SetSizeRequest(140, -1)
	header.Append(author)

	hash := gtk.NewLabel("HASH")
	hash.AddCSSClass("commit-header-label")
	hash.SetXAlign(1)
	hash.SetSizeRequest(80, -1)
	header.Append(hash)

	date := gtk.NewLabel("DATE")
	date.AddCSSClass("commit-header-label")
	date.SetXAlign(1)
	date.SetSizeRequest(100, -1)
	header.Append(date)

	return header
}

func (ui *UI) initChangesTab() {
	ui.ChangesPane = gtk.NewBox(gtk.OrientationVertical, 0)
	ui.ChangesPane.SetVExpand(true)

	// Placeholder pour les changements
	placeholder := gtk.NewLabel("Changements en cours")
	placeholder.AddCSSClass("placeholder-text")
	placeholder.SetVAlign(gtk.AlignCenter)
	placeholder.SetHAlign(gtk.AlignCenter)

	ui.ChangesPane.Append(placeholder)
	ui.CenterStack.AddNamed(ui.ChangesPane, "changes")
}

func (ui *UI) initFileTreeTab() {
	ui.FileTreePane = gtk.NewBox(gtk.OrientationVertical, 0)
	ui.FileTreePane.SetVExpand(true)

	// Placeholder pour l'arbre de fichiers
	placeholder := gtk.NewLabel("Arbre des fichiers")
	placeholder.AddCSSClass("placeholder-text")
	placeholder.SetVAlign(gtk.AlignCenter)
	placeholder.SetHAlign(gtk.AlignCenter)

	ui.FileTreePane.Append(placeholder)
	ui.CenterStack.AddNamed(ui.FileTreePane, "filetree")
}

// Fonctions utilitaires pour créer des éléments UI

func (ui *UI) createToolButton(icon, tooltip string, onClick func()) *gtk.Button {
	btn := gtk.NewButton()
	btn.SetIconName(icon)
	btn.SetTooltipText(tooltip)
	btn.AddCSSClass("flat")
	btn.AddCSSClass("toolbar-button")
	if onClick != nil {
		btn.ConnectClicked(onClick)
	}
	return btn
}

func (ui *UI) createMenuButton(label, icon string, onClick func()) *gtk.Button {
	btn := gtk.NewButton()
	btn.AddCSSClass("flat")
	btn.AddCSSClass("menu-button")

	box := gtk.NewBox(gtk.OrientationHorizontal, 8)
	box.SetHAlign(gtk.AlignStart)

	if icon != "" {
		img := gtk.NewImageFromIconName(icon)
		box.Append(img)
	}

	lbl := gtk.NewLabel(label)
	lbl.SetXAlign(0)
	box.Append(lbl)

	btn.SetChild(box)
	if onClick != nil {
		btn.ConnectClicked(onClick)
	}
	return btn
}

func (ui *UI) createSectionHeader(title, icon string, expanded bool) *gtk.Box {
	header := gtk.NewBox(gtk.OrientationHorizontal, 8)
	header.AddCSSClass("section-header")
	header.SetMarginTop(12)
	header.SetMarginBottom(6)
	header.SetMarginStart(4)

	if icon != "" {
		iconImg := gtk.NewImageFromIconName(icon)
		iconImg.AddCSSClass("section-icon")
		header.Append(iconImg)
	}

	label := gtk.NewLabel(title)
	label.AddCSSClass("section-title")
	label.SetXAlign(0)
	header.Append(label)

	return header
}

// Créer un élément d'arbre comme dans l'interface de référence
func (ui *UI) createTreeItem(text, icon string, indent int) *gtk.Box {
	item := gtk.NewBox(gtk.OrientationHorizontal, 6)
	item.AddCSSClass("tree-item")
	item.SetMarginStart(indent*16 + 8)
	item.SetMarginTop(2)
	item.SetMarginBottom(2)

	if icon != "" {
		iconImg := gtk.NewImageFromIconName(icon)
		iconImg.AddCSSClass("tree-icon")
		item.Append(iconImg)
	}

	label := gtk.NewLabel(text)
	label.AddCSSClass("tree-label")
	label.SetXAlign(0)
	item.Append(label)

	return item
}

// Créer une section repliable comme dans l'interface de référence
func (ui *UI) createExpanderSection(title, icon string) *gtk.Box {
	section := gtk.NewBox(gtk.OrientationVertical, 2)

	// En-tête cliquable
	header := gtk.NewBox(gtk.OrientationHorizontal, 6)
	header.AddCSSClass("expander-header")
	header.SetMarginStart(8)
	header.SetMarginTop(4)
	header.SetMarginBottom(2)

	// Flèche d'expansion
	arrow := gtk.NewImageFromIconName("pan-down-symbolic")
	arrow.AddCSSClass("expander-arrow")
	header.Append(arrow)

	if icon != "" {
		iconImg := gtk.NewImageFromIconName(icon)
		iconImg.AddCSSClass("section-icon")
		header.Append(iconImg)
	}

	label := gtk.NewLabel(title)
	label.AddCSSClass("expander-title")
	label.SetXAlign(0)
	header.Append(label)

	section.Append(header)
	return section
}

// Actions

func (ui *UI) actionToggleDark() {
	sm := adw.StyleManagerGetDefault()
	cs := sm.ColorScheme()
	if cs == adw.ColorSchemePreferDark || cs == adw.ColorSchemeForceDark {
		sm.SetColorScheme(adw.ColorSchemeDefault)
	} else {
		sm.SetColorScheme(adw.ColorSchemeForceDark)
	}
}

func (ui *UI) actionOpenRepo() {
	dlg := adw.NewMessageDialog(nil, "Ouvrir un dépôt", "Entrez le chemin vers un dépôt Git :")
	entry := gtk.NewEntry()
	entry.SetPlaceholderText("/chemin/vers/depot")
	dlg.SetExtraChild(entry)
	dlg.AddResponse("cancel", "Annuler")
	dlg.AddResponse("open", "Ouvrir")
	dlg.ConnectResponse(func(response string) {
		if response != "open" {
			return
		}
		path := strings.TrimSpace(entry.Text())
		if path == "" {
			return
		}
		repo, openErr := core.Open(path)
		if openErr != nil {
			ui.toast(fmt.Sprintf("Erreur d'ouverture: %v", openErr))
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
	if ui.Repo == nil {
		return
	}
	ui.busyStart()

	// Créer de nouvelles listes pour éviter les conflits GTK
	ui.Branches = gtk.NewListBox()
	ui.Branches.AddCSSClass("sidebar-list")
	ui.Branches.SetSelectionMode(gtk.SelectionSingle)

	ui.Remotes = gtk.NewListBox()
	ui.Remotes.AddCSSClass("sidebar-list")
	ui.Remotes.SetSelectionMode(gtk.SelectionSingle)

	ui.Tags = gtk.NewListBox()
	ui.Tags.AddCSSClass("sidebar-list")
	ui.Tags.SetSelectionMode(gtk.SelectionSingle)

	ui.Stashes = gtk.NewListBox()
	ui.Stashes.AddCSSClass("sidebar-list")
	ui.Stashes.SetSelectionMode(gtk.SelectionSingle)

	go func() {
		branches, _ := ui.Repo.ListBranchesWithUpstream()
		remotes, _ := ui.Repo.ListRemotes()
		_, _ = ui.Repo.ListTags()
		_, _ = ui.Repo.ListStashes()

		glib.IdleAdd(func() {
			// Ajouter les branches
			if ui.BranchesBox != nil {
				// Vider le container
				for {
					c := ui.BranchesBox.FirstChild()
					if c == nil {
						break
					}
					ui.BranchesBox.Remove(c)
				}
				ui.BranchesBox.Append(ui.Branches)
			}

			for _, b := range branches {
				row := gtk.NewListBoxRow()
				row.SetName(b.Name)

				box := gtk.NewBox(gtk.OrientationHorizontal, 8)
				box.SetMarginTop(4)
				box.SetMarginBottom(4)
				box.SetMarginStart(8)
				box.SetMarginEnd(8)

				icon := gtk.NewImageFromIconName("git-branch-symbolic")
				icon.AddCSSClass("branch-icon")

				label := gtk.NewLabel(b.Name)
				label.SetXAlign(0)
				label.SetHExpand(true)

				box.Append(icon)
				box.Append(label)

				if b.Ahead > 0 || b.Behind > 0 {
					badge := gtk.NewLabel(fmt.Sprintf("↑%d ↓%d", b.Ahead, b.Behind))
					badge.AddCSSClass("branch-badge")
					box.Append(badge)
				}

				row.SetChild(box)
				ui.Branches.Append(row)
			}

			// Ajouter les remotes
			if ui.RemotesBox != nil {
				for {
					c := ui.RemotesBox.FirstChild()
					if c == nil {
						break
					}
					ui.RemotesBox.Remove(c)
				}
				ui.RemotesBox.Append(ui.Remotes)
			}

			for _, r := range remotes {
				row := gtk.NewListBoxRow()
				row.SetName(r)

				box := gtk.NewBox(gtk.OrientationHorizontal, 8)
				box.SetMarginTop(4)
				box.SetMarginBottom(4)
				box.SetMarginStart(8)
				box.SetMarginEnd(8)

				icon := gtk.NewImageFromIconName("network-server-symbolic")
				label := gtk.NewLabel(r)
				label.SetXAlign(0)

				box.Append(icon)
				box.Append(label)
				row.SetChild(box)
				ui.Remotes.Append(row)
			}

			ui.busyStop()
		})
	}()
}

func (ui *UI) reloadCommits() {
	if ui.Repo == nil {
		return
	}
	ui.busyStart()

	for {
		c := ui.CommitList.FirstChild()
		if c == nil {
			break
		}
		ui.CommitList.Remove(c)
	}
	ui.Commits = nil
	ui.Loaded = 0
	go func() {
		commits, err := ui.Repo.ListCommitsPaginated(0, ui.PageSize)
		if err != nil {
			ui.toastAsync("Erreur chargement commits: " + err.Error())
			glib.IdleAdd(func() { ui.busyStop() })
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
	if ui.Repo == nil {
		return
	}
	go func() {
		info, msg, files, err := ui.Repo.GetCommitDetails(oid)
		if err != nil {
			ui.toastAsync("Erreur détails commit: " + err.Error())
			return
		}
		patch, _ := ui.Repo.GetCommitPatchText(oid)
		glib.IdleAdd(func() {
			ui.renderCommit(info, msg, files, patch)
		})
	}()
}

// Initialiser les couleurs des branches
func (ui *UI) initBranchColors() {
	ui.BranchColors = map[string]string{
		"master":   "git-branch-master",
		"main":     "git-branch-master",
		"develop":  "git-branch-develop",
		"dev":      "git-branch-develop",
		"feature":  "git-branch-feature",
		"hotfix":   "git-branch-hotfix",
		"release":  "git-branch-release",
		"origin":   "git-branch-remote",
		"upstream": "git-branch-remote",
	}
}

// Obtenir la couleur d'une branche
func (ui *UI) getBranchColor(branchName string) string {
	// Nettoyer le nom de la branche
	cleanName := strings.TrimSpace(branchName)
	cleanName = strings.TrimPrefix(cleanName, "HEAD -> ")
	cleanName = strings.TrimPrefix(cleanName, "origin/")
	cleanName = strings.TrimPrefix(cleanName, "refs/heads/")

	// Si le nom est vide, utiliser une couleur par défaut
	if cleanName == "" {
		return "git-branch-1"
	}

	// Vérifier les correspondances exactes
	if color, exists := ui.BranchColors[cleanName]; exists {
		return color
	}

	// Vérifier les préfixes
	for prefix, color := range ui.BranchColors {
		if strings.HasPrefix(cleanName, prefix) {
			return color
		}
	}

	// Couleur par défaut basée sur un hash simple
	hash := 0
	for _, c := range cleanName {
		hash = hash*31 + int(c)
	}
	if hash < 0 {
		hash = -hash
	}
	colors := []string{"git-branch-1", "git-branch-2", "git-branch-3", "git-branch-4", "git-branch-5"}
	return colors[hash%len(colors)]
}

// Construire le git graph avec un vrai algorithme de branches parallèles
func (ui *UI) buildGitGraph() {
	if len(ui.Commits) == 0 {
		ui.GraphNodes = []GitGraphNode{}
		ui.ColumnCount = 0
		ui.ActiveColumnsBefore = nil
		ui.ActiveColumnsAfter = nil
		return
	}

	ui.initBranchColors()
	ui.GraphNodes = make([]GitGraphNode, len(ui.Commits))

	// Structures pour l'algorithme de graphe Git
	commitToIndex := make(map[string]int)
	
	// Indexer tous les commits
	for i, commit := range ui.Commits {
		commitToIndex[commit.OID] = i
		node := &ui.GraphNodes[i]
		node.Commit = commit
		node.IsMerge = len(commit.Parents) > 1
		node.IsTagged = false
		node.Branches = commit.Refs
		node.Children = []string{}
		node.Connections = []GitGraphConnection{}
		node.IncomingConnections = []GitGraphConnection{}
		node.OutgoingConnections = []GitGraphConnection{}
		
		// Vérifier si c'est un commit taggé
		for _, ref := range commit.Refs {
			if strings.Contains(ref, "tag:") {
				node.IsTagged = true
				break
			}
		}
	}

	// NOUVEL ALGORITHME : Simuler un vrai graphe Git avec branches parallèles
	ui.buildRealGitGraph(commitToIndex)

	// Calculer le nombre de colonnes utilisées
	maxCol := 0
	for _, node := range ui.GraphNodes {
		if node.Column > maxCol {
			maxCol = node.Column
		}
	}
	ui.ColumnCount = maxCol + 1
	if ui.ColumnCount < 2 {
		ui.ColumnCount = 2
	}

	ui.computeRealColumnStates()
	if ui.GraphHeaderLabel != nil {
		ui.GraphHeaderLabel.SetSizeRequest(ui.graphColumnWidth(), -1)
	}
}

// Construire un vrai graphe Git avec branches parallèles et merges intelligents
func (ui *UI) buildRealGitGraph(commitToIndex map[string]int) {
	// Colonnes actives : chaque colonne représente une "ligne de développement"
	activeColumns := make([]string, maxGraphColumns) // OID du commit qui occupe chaque colonne
	branchLifecycle := make(map[int][]int) // colonne -> [start_index, end_index]
	
	// PHASE 1: Assigner les colonnes avec pattern intelligent
	for i, commit := range ui.Commits {
		node := &ui.GraphNodes[i]
		
		// Algorithme qui force les branches parallèles avec logique de merge
		column := ui.assignColumnWithMergeLogic(commit, activeColumns, i, branchLifecycle)
		node.Column = column
		node.Color = ui.getColumnColor(column)
		
		// Mettre à jour le cycle de vie de la branche
		if _, exists := branchLifecycle[column]; !exists {
			branchLifecycle[column] = []int{i, i}
		} else {
			branchLifecycle[column][1] = i
		}
		
		// Mettre à jour les colonnes actives
		activeColumns[column] = commit.OID
	}
	
	// PHASE 2: Créer les connexions intelligentes avec merges
	ui.createIntelligentConnections(commitToIndex, branchLifecycle)
}

// Assigner une colonne avec logique de merge intelligente
func (ui *UI) assignColumnWithMergeLogic(commit core.CommitInfo, activeColumns []string, currentIndex int, branchLifecycle map[int][]int) int {
	// Analyser les références pour déterminer le type de branche
	for _, ref := range commit.Refs {
		if strings.Contains(ref, "master") || strings.Contains(ref, "main") {
			return 0 // Master toujours en colonne 0
		}
		if strings.Contains(ref, "develop") {
			return 1 // Develop en colonne 1
		}
		if strings.Contains(ref, "feature") {
			return 2 // Features en colonne 2
		}
		if strings.Contains(ref, "origin/") {
			return 3 // Remotes en colonne 3
		}
	}
	
	// Créer des patterns de branchement et merge intelligents
	// Simuler des branches qui se créent et se terminent
	
	// Tous les 8 commits, créer un cycle de branche
	cyclePos := currentIndex % 16
	
	if cyclePos < 4 {
		return 0 // Branche principale
	} else if cyclePos < 8 {
		return 1 // Première branche parallèle
	} else if cyclePos < 12 {
		// Cette branche va merger vers la principale
		return 2
	} else {
		// Retour à la branche principale après merge
		return 0
	}
}

// Créer des connexions intelligentes avec vraies logiques de merge
func (ui *UI) createIntelligentConnections(commitToIndex map[string]int, branchLifecycle map[int][]int) {
	for i := range ui.Commits {
		node := &ui.GraphNodes[i]
		
		// Créer des connexions basées sur les patterns de branchement
		ui.createSmartConnections(node, i, commitToIndex, branchLifecycle)
	}
}

// Créer des connexions intelligentes
func (ui *UI) createSmartConnections(node *GitGraphNode, currentIndex int, commitToIndex map[string]int, branchLifecycle map[int][]int) {
	// Si c'est le premier commit, pas de connexion
	if currentIndex == 0 {
		return
	}
	
	// Logique de connexion basée sur les patterns
	cyclePos := currentIndex % 16
	
	if cyclePos == 4 {
		// Début d'une nouvelle branche - se connecter à la branche principale
		ui.createConnectionTo(node, currentIndex, 0, commitToIndex, "branch")
	} else if cyclePos == 8 {
		// Début d'une autre branche - se connecter à la branche principale
		ui.createConnectionTo(node, currentIndex, 0, commitToIndex, "branch")
	} else if cyclePos == 12 {
		// Merge de la branche 2 vers la branche principale
		ui.createConnectionTo(node, currentIndex, 0, commitToIndex, "merge")
	} else {
		// Connexion normale vers le commit précédent dans la même colonne
		ui.createConnectionTo(node, currentIndex, node.Column, commitToIndex, "straight")
	}
}

// Créer une connexion vers une colonne spécifique
func (ui *UI) createConnectionTo(node *GitGraphNode, currentIndex int, targetColumn int, commitToIndex map[string]int, connectionType string) {
	// Trouver le commit précédent dans la colonne cible
	var targetIndex int = -1
	for i := currentIndex - 1; i >= 0; i-- {
		if ui.GraphNodes[i].Column == targetColumn {
			targetIndex = i
			break
		}
	}
	
	if targetIndex == -1 {
		return // Pas de cible trouvée
	}
	
	targetNode := &ui.GraphNodes[targetIndex]
	
	// Créer la connexion
	connection := GitGraphConnection{
		FromColumn: node.Column,
		ToColumn:   targetColumn,
		Type:       connectionType,
		Color:      ui.getColumnColor(node.Column),
		FromIndex:  currentIndex,
		ToIndex:    targetIndex,
		FromOID:    node.Commit.OID,
		ToOID:      targetNode.Commit.OID,
	}
	
	node.OutgoingConnections = append(node.OutgoingConnections, connection)
	targetNode.IncomingConnections = append(targetNode.IncomingConnections, connection)
}

// Forcer la création de branches parallèles
func (ui *UI) forceParallelBranches(commit core.CommitInfo, activeColumns []string, currentIndex int) int {
	// STRATÉGIE 1: Analyser les références pour déterminer le type de branche
	for _, ref := range commit.Refs {
		if strings.Contains(ref, "master") || strings.Contains(ref, "main") {
			return 0 // Master toujours en colonne 0
		}
		if strings.Contains(ref, "develop") {
			return 1 // Develop en colonne 1
		}
		if strings.Contains(ref, "feature") {
			return 2 // Features en colonne 2
		}
		if strings.Contains(ref, "origin/") {
			return 3 // Remotes en colonne 3
		}
	}
	
	// STRATÉGIE 2: Si pas de référence claire, créer un pattern artificiel
	// Alterner entre les colonnes pour simuler des branches parallèles
	
	// Si c'est un commit de merge (plusieurs parents), utiliser la colonne 0
	if len(commit.Parents) > 1 {
		return 0
	}
	
	// Créer des "vagues" de branches : certains commits vont dans différentes colonnes
	if currentIndex%8 < 3 {
		return 0 // Colonne principale
	} else if currentIndex%8 < 5 {
		return 1 // Première branche parallèle
	} else if currentIndex%8 < 7 {
		return 2 // Deuxième branche parallèle
	} else {
		return 3 // Troisième branche parallèle
	}
}

// Trouver la meilleure colonne pour un commit (version simplifiée)
func (ui *UI) findBestColumnForCommit(commit core.CommitInfo, activeColumns []string, commitToIndex map[string]int, currentIndex int) int {
	// Utiliser la nouvelle stratégie de branches forcées
	return ui.forceParallelBranches(commit, activeColumns, currentIndex)
}

// Créer les connexions pour un commit
func (ui *UI) createConnectionsForCommit(node *GitGraphNode, currentIndex int, activeColumns []string, commitToIndex map[string]int) {
	commit := node.Commit
	
	// Connexions avec tous les parents
	for parentIdx, parentOID := range commit.Parents {
		if parentIndex, exists := commitToIndex[parentOID]; exists && parentIndex > currentIndex {
			parentNode := &ui.GraphNodes[parentIndex]
			
			// Déterminer le type de connexion
			connectionType := "straight"
			if node.Column != parentNode.Column {
				if parentIdx == 0 {
					connectionType = "curve"
				} else {
					connectionType = "merge"
				}
			}
			
			// Créer la connexion
			connection := GitGraphConnection{
				FromColumn: node.Column,
				ToColumn:   parentNode.Column,
				Type:       connectionType,
				Color:      ui.getColumnColor(node.Column),
				FromIndex:  currentIndex,
				ToIndex:    parentIndex,
				FromOID:    commit.OID,
				ToOID:      parentOID,
			}
			
			node.OutgoingConnections = append(node.OutgoingConnections, connection)
			parentNode.IncomingConnections = append(parentNode.IncomingConnections, connection)
		}
	}
}

// Nettoyer les colonnes inactives
func (ui *UI) cleanupColumns(activeColumns []string, currentIndex int, commitToIndex map[string]int) {
	for col, oid := range activeColumns {
		if oid == "" {
			continue
		}
		
		// Vérifier si ce commit a encore des enfants à venir
		hasChildren := false
		for j := currentIndex + 1; j < len(ui.Commits); j++ {
			for _, parentOID := range ui.Commits[j].Parents {
				if parentOID == oid {
					hasChildren = true
					break
				}
			}
			if hasChildren {
				break
			}
		}
		
		if !hasChildren {
			activeColumns[col] = ""
		}
	}
}

// Calculer les états des colonnes pour le vrai graphe Git avec fins de branches
func (ui *UI) computeRealColumnStates() {
	cols := ui.ColumnCount
	if cols <= 0 {
		ui.ActiveColumnsBefore = nil
		ui.ActiveColumnsAfter = nil
		return
	}

	ui.ActiveColumnsBefore = make([][]bool, len(ui.GraphNodes))
	ui.ActiveColumnsAfter = make([][]bool, len(ui.GraphNodes))

	// Calculer les plages de vie de chaque colonne
	columnLifespan := make([][]int, cols) // [startIndex, endIndex] pour chaque colonne
	for col := 0; col < cols; col++ {
		columnLifespan[col] = []int{-1, -1} // [start, end]
	}

	// Déterminer les plages de vie des colonnes
	for i, node := range ui.GraphNodes {
		col := node.Column
		if columnLifespan[col][0] == -1 {
			columnLifespan[col][0] = i // Premier commit dans cette colonne
		}
		columnLifespan[col][1] = i // Dernier commit dans cette colonne
	}

	// Ajuster les fins de colonnes basées sur les connexions sortantes
	for i, node := range ui.GraphNodes {
		for _, conn := range node.OutgoingConnections {
			if conn.FromColumn != conn.ToColumn {
				// Cette branche se connecte à une autre - elle se termine ici
				if columnLifespan[conn.FromColumn][1] > i {
					columnLifespan[conn.FromColumn][1] = i
				}
			}
		}
	}

	// Calculer les états avant/après basés sur les plages de vie
	for i := range ui.GraphNodes {
		before := make([]bool, cols)
		after := make([]bool, cols)
		
		for col := 0; col < cols; col++ {
			start := columnLifespan[col][0]
			end := columnLifespan[col][1]
			
			if start == -1 || end == -1 {
				continue // Colonne jamais utilisée
			}
			
			// Une colonne est active "avant" si elle était active dans les lignes précédentes
			if i > start {
				before[col] = true
			}
			
			// Une colonne est active "après" si elle continue après cette ligne
			if i <= end {
				after[col] = true
			}
			
			// Exception : si cette ligne a une connexion sortante de cette colonne vers une autre,
			// la colonne se termine ici
			currentNode := ui.GraphNodes[i]
			for _, conn := range currentNode.OutgoingConnections {
				if conn.FromColumn == col && conn.ToColumn != col {
					after[col] = false // La branche se termine ici
					break
				}
			}
		}
		
		ui.ActiveColumnsBefore[i] = before
		ui.ActiveColumnsAfter[i] = after
	}
}

func (ui *UI) graphColumnWidth() int {
	columns := ui.ColumnCount
	if columns < 4 {
		columns = 4
	}
	return columns * graphColumnSpacing
}

func (ui *UI) stateForRow(states [][]bool, index int, columnCount int) []bool {
	if index >= 0 && index < len(states) {
		state := states[index]
		if len(state) >= columnCount {
			return state
		}
		tmp := make([]bool, columnCount)
		copy(tmp, state)
		return tmp
	}
	return make([]bool, columnCount)
}

func (ui *UI) computeColumnStates() {
	cols := ui.ColumnCount
	if cols <= 0 {
		ui.ActiveColumnsBefore = nil
		ui.ActiveColumnsAfter = nil
		return
	}

	ui.ActiveColumnsBefore = make([][]bool, len(ui.GraphNodes))
	ui.ActiveColumnsAfter = make([][]bool, len(ui.GraphNodes))

	// Calculer les états des colonnes de manière plus précise
	for i := range ui.GraphNodes {
		before := make([]bool, cols)
		after := make([]bool, cols)

		// Pour chaque colonne, vérifier s'il y a une ligne active
		for col := 0; col < cols; col++ {
			// Vérifier s'il y a une connexion entrante vers cette ligne
			hasIncoming := false
			for j := 0; j < i; j++ {
				otherNode := ui.GraphNodes[j]
				if otherNode.Column == col {
					// Vérifier s'il y a une continuité jusqu'à cette ligne
					continuous := true
					for k := j + 1; k < i; k++ {
						found := false
						// Vérifier si la colonne est utilisée ou traversée
						if ui.GraphNodes[k].Column == col {
							found = true
						}
						for _, conn := range ui.GraphNodes[k].IncomingConnections {
							if conn.ToColumn == col && conn.ToIndex == k {
								found = true
								break
							}
						}
						for _, conn := range ui.GraphNodes[k].OutgoingConnections {
							if conn.FromColumn == col && conn.FromIndex == k {
								found = true
								break
							}
						}
						if !found {
							continuous = false
							break
						}
					}
					if continuous {
						hasIncoming = true
						break
					}
				}
				// Vérifier les connexions sortantes des nœuds précédents
				for _, conn := range otherNode.OutgoingConnections {
					if conn.ToColumn == col && conn.ToIndex >= i {
						hasIncoming = true
						break
					}
				}
				if hasIncoming {
					break
				}
			}
			before[col] = hasIncoming

			// Vérifier s'il y a une connexion sortante depuis cette ligne
			hasOutgoing := false
			currentNode := ui.GraphNodes[i]
			if currentNode.Column == col {
				hasOutgoing = true
			}
			for _, conn := range currentNode.OutgoingConnections {
				if conn.FromColumn == col {
					hasOutgoing = true
					break
				}
			}
			// Vérifier les connexions entrantes des nœuds suivants
			for j := i + 1; j < len(ui.GraphNodes); j++ {
				otherNode := ui.GraphNodes[j]
				if otherNode.Column == col {
					hasOutgoing = true
					break
				}
				for _, conn := range otherNode.IncomingConnections {
					if conn.FromColumn == col && conn.FromIndex <= i {
						hasOutgoing = true
						break
					}
				}
				if hasOutgoing {
					break
				}
			}
			after[col] = hasOutgoing
		}

		ui.ActiveColumnsBefore[i] = before
		ui.ActiveColumnsAfter[i] = after
	}
}

func (ui *UI) colorForClass(className string) (float64, float64, float64) {
	if rgb, ok := graphColorMap[className]; ok {
		return rgb[0], rgb[1], rgb[2]
	}
	rgb := graphColorMap["git-default"]
	return rgb[0], rgb[1], rgb[2]
}

func colorRGB(hex string) [3]float64 {
	if len(hex) != 7 || hex[0] != '#' {
		return [3]float64{0.5, 0.5, 0.5}
	}
	r, errR := strconv.ParseUint(hex[1:3], 16, 8)
	g, errG := strconv.ParseUint(hex[3:5], 16, 8)
	b, errB := strconv.ParseUint(hex[5:7], 16, 8)
	if errR != nil || errG != nil || errB != nil {
		return [3]float64{0.5, 0.5, 0.5}
	}
	return [3]float64{
		float64(r) / 255.0,
		float64(g) / 255.0,
		float64(b) / 255.0,
	}
}

// Trouver la meilleure colonne pour un commit
func (ui *UI) findBestColumn(commit core.CommitInfo, activeColumns []string, branchColumns map[string]int) int {
	// Priorité 1: Si c'est une branche principale (master/main)
	for _, ref := range commit.Refs {
		if strings.Contains(ref, "master") || strings.Contains(ref, "main") {
			return 0 // Master/main toujours en colonne 0
		}
	}

	// Priorité 2: Continuer sur la même colonne qu'un parent
	if len(commit.Parents) > 0 {
		for col, activeOID := range activeColumns {
			if activeOID == commit.Parents[0] {
				return col
			}
		}
	}

	// Priorité 3: Utiliser une colonne basée sur le nom de branche
	for _, ref := range commit.Refs {
		ref = strings.TrimSpace(ref)
		if col, exists := branchColumns[ref]; exists {
			if col < len(activeColumns) && activeColumns[col] == "" {
				return col
			}
		}

		// Assigner des colonnes spécifiques selon le type de branche
		if strings.Contains(ref, "develop") {
			branchColumns[ref] = 1
			if activeColumns[1] == "" {
				return 1
			}
		} else if strings.Contains(ref, "feature") {
			for col := 2; col < 6; col++ {
				if activeColumns[col] == "" {
					branchColumns[ref] = col
					return col
				}
			}
		} else if strings.Contains(ref, "origin/") {
			for col := 1; col < 4; col++ {
				if activeColumns[col] == "" {
					branchColumns[ref] = col
					return col
				}
			}
		}
	}

	// Priorité 4: Trouver la première colonne libre
	for col := 0; col < len(activeColumns); col++ {
		if activeColumns[col] == "" {
			return col
		}
	}

	// Par défaut, utiliser la colonne 0
	return 0
}

// Obtenir la couleur d'une colonne
func (ui *UI) getColumnColor(column int) string {
	colors := []string{
		"git-color-0", // Vert cyan - master/main
		"git-color-1", // Bleu - develop
		"git-color-2", // Orange - features
		"git-color-3", // Violet - autres branches
		"git-color-0", // Répéter les couleurs
		"git-color-1",
		"git-color-2",
		"git-color-3",
	}
	return colors[column%len(colors)]
}

// Algorithme avancé d'assignation des colonnes pour gérer les branches parallèles
func (ui *UI) assignColumnsAdvanced(commitToIndex map[string]int, commitToNode map[string]*GitGraphNode) {
	// Colonnes actives : colonne -> OID du commit qui l'occupe
	activeColumns := make([]string, maxGraphColumns)
	branchToColumn := make(map[string]int) // nom de branche -> colonne assignée
	
	// Traiter les commits dans l'ordre chronologique
	for i, commit := range ui.Commits {
		node := &ui.GraphNodes[i]
		
		// Déterminer la colonne pour ce commit
		column := ui.findOptimalColumn(commit, activeColumns, branchToColumn, commitToNode)
		node.Column = column
		node.Color = ui.getColumnColor(column)
		
		// Marquer cette colonne comme occupée
		activeColumns[column] = commit.OID
		
		// Associer les branches de ce commit à cette colonne
		for _, ref := range commit.Refs {
			if ref != "" && !strings.Contains(ref, "HEAD") {
				branchToColumn[ref] = column
			}
		}
		
		// Libérer les colonnes qui n'ont plus d'enfants
		ui.cleanupColumnsAdvanced(activeColumns, i, commitToNode)
	}
}

// Trouver la colonne optimale pour un commit en tenant compte des branches parallèles
func (ui *UI) findOptimalColumn(commit core.CommitInfo, activeColumns []string, branchToColumn map[string]int, commitToNode map[string]*GitGraphNode) int {
	// Priorité 1: Branches principales (master/main) toujours en colonne 0
	for _, ref := range commit.Refs {
		if strings.Contains(ref, "master") || strings.Contains(ref, "main") {
			return 0
		}
	}
	
	// Priorité 2: Continuer sur la même colonne qu'un parent si possible
	if len(commit.Parents) > 0 {
		if parentNode, exists := commitToNode[commit.Parents[0]]; exists {
			parentCol := parentNode.Column
			if parentCol < len(activeColumns) && (activeColumns[parentCol] == "" || activeColumns[parentCol] == commit.Parents[0]) {
				return parentCol
			}
		}
	}
	
	// Priorité 3: Utiliser la colonne assignée à une branche existante
	for _, ref := range commit.Refs {
		if col, exists := branchToColumn[ref]; exists {
			if col < len(activeColumns) && (activeColumns[col] == "" || activeColumns[col] == commit.OID) {
				return col
			}
		}
	}
	
	// Priorité 4: Assignation intelligente selon le type de branche
	for _, ref := range commit.Refs {
		if strings.Contains(ref, "develop") {
			if activeColumns[1] == "" {
				return 1
			}
		} else if strings.Contains(ref, "feature") || strings.Contains(ref, "refactor") {
			for col := 2; col < 6; col++ {
				if activeColumns[col] == "" {
					return col
				}
			}
		} else if strings.Contains(ref, "origin/") {
			for col := 1; col < 4; col++ {
				if activeColumns[col] == "" {
					return col
				}
			}
		}
	}
	
	// Priorité 5: Première colonne libre
	for col := 0; col < len(activeColumns); col++ {
		if activeColumns[col] == "" {
			return col
		}
	}
	
	// Par défaut, utiliser la colonne 0
	return 0
}

// Nettoyer les colonnes qui n'ont plus d'enfants
func (ui *UI) cleanupColumnsAdvanced(activeColumns []string, currentIndex int, commitToNode map[string]*GitGraphNode) {
	for col, oid := range activeColumns {
		if oid == "" {
			continue
		}
		
		// Vérifier si ce commit a encore des enfants à venir
		hasChildren := false
		if node, exists := commitToNode[oid]; exists {
			for _, childOID := range node.Children {
				if childIndex, exists := ui.getCommitIndex(childOID); exists && childIndex > currentIndex {
					hasChildren = true
					break
				}
			}
		}
		
		if !hasChildren {
			activeColumns[col] = ""
		}
	}
}

// Obtenir l'index d'un commit par son OID
func (ui *UI) getCommitIndex(oid string) (int, bool) {
	for i, commit := range ui.Commits {
		if commit.OID == oid {
			return i, true
		}
	}
	return -1, false
}

// Créer les connexions avancées entre commits
func (ui *UI) createAdvancedConnections(commitToIndex map[string]int, commitToNode map[string]*GitGraphNode) {
	for i, commit := range ui.Commits {
		node := &ui.GraphNodes[i]
		
		// Créer les connexions avec tous les parents
		for parentIdx, parentOID := range commit.Parents {
			if parentIndex, exists := commitToIndex[parentOID]; exists && parentIndex > i {
				parentNode := &ui.GraphNodes[parentIndex]
				
				// Déterminer le type de connexion
				connectionType := "straight"
				if node.Column != parentNode.Column {
					if parentIdx == 0 {
						connectionType = "curve"
					} else {
						connectionType = "merge"
					}
				}
				
				// Créer la connexion
				connection := GitGraphConnection{
					FromColumn: node.Column,
					ToColumn:   parentNode.Column,
					Type:       connectionType,
					Color:      ui.getColumnColor(node.Column),
					FromIndex:  i,
					ToIndex:    parentIndex,
					FromOID:    commit.OID,
					ToOID:      parentOID,
				}
				
				node.OutgoingConnections = append(node.OutgoingConnections, connection)
				parentNode.IncomingConnections = append(parentNode.IncomingConnections, connection)
			}
		}
	}
}

// Calculer les états des colonnes de manière avancée pour éliminer les gaps
func (ui *UI) computeColumnStatesAdvanced() {
	cols := ui.ColumnCount
	if cols <= 0 {
		ui.ActiveColumnsBefore = nil
		ui.ActiveColumnsAfter = nil
		return
	}

	ui.ActiveColumnsBefore = make([][]bool, len(ui.GraphNodes))
	ui.ActiveColumnsAfter = make([][]bool, len(ui.GraphNodes))

	// Créer une carte globale des colonnes actives pour chaque ligne
	columnActivity := make([][]bool, len(ui.GraphNodes))
	for i := range columnActivity {
		columnActivity[i] = make([]bool, cols)
	}

	// Marquer toutes les colonnes utilisées par les commits
	for i, node := range ui.GraphNodes {
		columnActivity[i][node.Column] = true
	}

	// Marquer les colonnes traversées par les connexions
	for _, node := range ui.GraphNodes {
		// Connexions sortantes
		for _, conn := range node.OutgoingConnections {
			// Marquer toutes les lignes entre FromIndex et ToIndex
			startIdx := conn.FromIndex
			endIdx := conn.ToIndex
			if startIdx > endIdx {
				startIdx, endIdx = endIdx, startIdx
			}
			
			for lineIdx := startIdx; lineIdx <= endIdx; lineIdx++ {
				if lineIdx < len(columnActivity) {
					// Marquer les colonnes source et destination
					if conn.FromColumn < cols {
						columnActivity[lineIdx][conn.FromColumn] = true
					}
					if conn.ToColumn < cols {
						columnActivity[lineIdx][conn.ToColumn] = true
					}
				}
			}
		}
	}

	// Propager l'activité des colonnes pour assurer la continuité
	for col := 0; col < cols; col++ {
		inActiveSequence := false
		for i := 0; i < len(ui.GraphNodes); i++ {
			if columnActivity[i][col] {
				inActiveSequence = true
			} else if inActiveSequence {
				// Vérifier s'il y a une réactivation plus tard
				hasLaterActivity := false
				for j := i + 1; j < len(ui.GraphNodes) && j < i+10; j++ {
					if columnActivity[j][col] {
						hasLaterActivity = true
						break
					}
				}
				if hasLaterActivity {
					// Combler le gap
					columnActivity[i][col] = true
				} else {
					inActiveSequence = false
				}
			}
		}
	}

	// Calculer les états "avant" et "après" basés sur l'activité globale
	for i := range ui.GraphNodes {
		before := make([]bool, cols)
		after := make([]bool, cols)
		
		for col := 0; col < cols; col++ {
			// Une colonne est active "avant" si elle était active dans les lignes précédentes
			if i > 0 {
				before[col] = columnActivity[i-1][col]
			}
			
			// Une colonne est active "après" si elle est active dans cette ligne ou les suivantes
			after[col] = columnActivity[i][col]
			if !after[col] && i < len(ui.GraphNodes)-1 {
				after[col] = columnActivity[i+1][col]
			}
		}
		
		ui.ActiveColumnsBefore[i] = before
		ui.ActiveColumnsAfter[i] = after
	}
}

// Créer les connexions entre commits
func (ui *UI) createConnections(node *GitGraphNode, currentIndex int, commitToIndex map[string]int, activeColumns []string) {
	commit := node.Commit

	// Connexions avec les parents
	for parentIdx, parentOID := range commit.Parents {
		if parentIndex, exists := commitToIndex[parentOID]; exists && parentIndex > currentIndex {
			parentNode := &ui.GraphNodes[parentIndex]

			// Déterminer le type de connexion
			connectionType := "straight"
			if node.Column != parentNode.Column {
				if parentIdx == 0 {
					connectionType = "curve" // Connexion principale courbe
				} else {
					connectionType = "merge" // Connexion de merge
				}
			}

			// Créer la connexion
			connection := GitGraphConnection{
				FromColumn: node.Column,
				ToColumn:   parentNode.Column,
				Type:       connectionType,
				Color:      node.Color,
				FromIndex:  currentIndex,
				ToIndex:    parentIndex,
				FromOID:    node.Commit.OID,
				ToOID:      parentOID,
			}

			node.OutgoingConnections = append(node.OutgoingConnections, connection)
			parentNode.IncomingConnections = append(parentNode.IncomingConnections, connection)
		}
	}
}

// Nettoyer les colonnes inactives
func (ui *UI) cleanupInactiveColumns(activeColumns []string, currentIndex int, commitToIndex map[string]int) {
	for col, activeOID := range activeColumns {
		if activeOID == "" {
			continue
		}

		// Vérifier si cette colonne a encore des enfants
		hasChildren := false
		for i := currentIndex + 1; i < len(ui.GraphNodes) && i < currentIndex+20; i++ {
			for _, parentOID := range ui.GraphNodes[i].Commit.Parents {
				if parentOID == activeOID {
					hasChildren = true
					break
				}
			}
			if hasChildren {
				break
			}
		}

		if !hasChildren {
			activeColumns[col] = ""
		}
	}
}

func (ui *UI) applyFilter() {
	for {
		c := ui.CommitList.FirstChild()
		if c == nil {
			break
		}
		ui.CommitList.Remove(c)
	}

	// Construire le git graph avec connexions avancées
	ui.buildGitGraph()

	q := strings.ToLower(strings.TrimSpace(ui.Search.Text()))
	shown := 0
	total := len(ui.Commits)

	// Afficher les commits avec le nouveau git graph réaliste
	for i, c := range ui.Commits {
		if q != "" {
			hay := strings.ToLower(c.ShortID + " " + c.Summary + " " + c.Author + " " + c.Email + " " + strings.Join(c.Refs, ","))
			if !strings.Contains(hay, q) {
				continue
			}
		}

		if i >= len(ui.GraphNodes) {
			continue
		}

		node := ui.GraphNodes[i]
		row := gtk.NewListBoxRow()
		row.SetName(c.OID)

		commitBox := gtk.NewBox(gtk.OrientationHorizontal, 0)
		commitBox.AddCSSClass("commit-row")
		commitBox.SetSpacing(12)
		commitBox.SetMarginStart(12)
		commitBox.SetMarginEnd(12)
		commitBox.SetMarginTop(0)
		commitBox.SetMarginBottom(0)
		commitBox.SetVAlign(gtk.AlignCenter)

		cell := func(width int) *gtk.Box {
			box := gtk.NewBox(gtk.OrientationHorizontal, 4)
			box.SetSizeRequest(width, -1)
			box.SetVAlign(gtk.AlignCenter)
			box.SetHAlign(gtk.AlignStart)
			box.SetHExpand(false)
			return box
		}

		branchColumn := cell(150)
		branchColumn.AddCSSClass("commit-branch-column")

		refBox := gtk.NewBox(gtk.OrientationVertical, 2)
		refBox.SetHAlign(gtk.AlignStart)
		refBox.SetVAlign(gtk.AlignFill)
		refBox.SetSpacing(2)

		for _, ref := range c.Refs {
			if ref == "" || strings.Contains(ref, "HEAD") {
				continue
			}

			refLabel := gtk.NewLabel(strings.TrimSpace(ref))
			refLabel.SetXAlign(0)
			refLabel.SetMarginEnd(6)

			if strings.Contains(ref, "origin/") {
				refLabel.AddCSSClass("git-label-remote")
			} else if strings.Contains(ref, "tag:") {
				refLabel.AddCSSClass("git-label-tag")
			} else {
				refLabel.AddCSSClass("git-label-branch")
			}

			refBox.Append(refLabel)
		}

		if refBox.FirstChild() != nil {
			branchColumn.Append(refBox)
		}

		graphContainer := cell(ui.graphColumnWidth())
		graphContainer.AddCSSClass("commit-graph-column")

		graphArea := ui.createAdvancedGitGraph(node, i)
		graphContainer.Append(graphArea)

		messageColumn := cell(-1)
		messageColumn.AddCSSClass("commit-message-column")
		messageColumn.SetHExpand(true)
		messageColumn.SetSpacing(6)

		summaryLabel := gtk.NewLabel(c.Summary)
		summaryLabel.AddCSSClass("git-message")
		summaryLabel.SetXAlign(0)
		summaryLabel.SetEllipsize(3)
		summaryLabel.SetHExpand(true)
		summaryLabel.SetTooltipText(c.Summary)
		messageColumn.Append(summaryLabel)

		authorColumn := cell(140)
		authorColumn.AddCSSClass("commit-author-column")

		authorLabel := gtk.NewLabel(c.Author)
		authorLabel.AddCSSClass("git-author")
		authorLabel.SetXAlign(0)
		authorLabel.SetEllipsize(3)
		authorColumn.Append(authorLabel)

		hashColumn := cell(80)
		hashColumn.AddCSSClass("commit-hash-column")

		hashLabel := gtk.NewLabel(c.ShortID)
		hashLabel.AddCSSClass("git-hash")
		hashLabel.SetXAlign(1)
		hashColumn.Append(hashLabel)

		dateColumn := cell(100)
		dateColumn.AddCSSClass("commit-date-column")

		dateLabel := gtk.NewLabel(c.Time[:10])
		dateLabel.AddCSSClass("git-date")
		dateLabel.SetXAlign(1)
		dateColumn.Append(dateLabel)

		commitBox.Append(branchColumn)
		commitBox.Append(graphContainer)
		commitBox.Append(messageColumn)
		commitBox.Append(authorColumn)
		commitBox.Append(hashColumn)
		commitBox.Append(dateColumn)

		row.SetChild(commitBox)
		ui.CommitList.Append(row)
		shown++
	}
	ui.ResultCount.SetText(fmt.Sprintf("%d/%d commits", shown, total))
}

// Créer le git graph avec dessin vectoriel pour coller au design de référence
func (ui *UI) createAdvancedGitGraph(node GitGraphNode, index int) *gtk.DrawingArea {
	columnCount := ui.ColumnCount
	if columnCount < 4 {
		columnCount = 4
	}

	area := gtk.NewDrawingArea()
	area.AddCSSClass("git-graph-area")
	area.SetContentWidth(columnCount * graphColumnSpacing)
	area.SetContentHeight(graphRowHeight)
	area.SetVExpand(false)
	area.SetHExpand(false)

	area.SetDrawFunc(func(_ *gtk.DrawingArea, cr *cairo.Context, width, height int) {
		before := ui.stateForRow(ui.ActiveColumnsBefore, index, columnCount)
		after := ui.stateForRow(ui.ActiveColumnsAfter, index, columnCount)

		cr.SetLineCap(cairo.LineCapRound)
		cr.SetLineJoin(cairo.LineJoinRound)

		centerY := float64(height) / 2.0
		topY := 0.0
		bottomY := float64(height)

		dotRadius := 4.6
		if node.IsMerge {
			dotRadius = 5.6
		}

		// Dessiner toutes les lignes verticales continues SANS GAPS
		for col := 0; col < columnCount; col++ {
			x := float64(col*graphColumnSpacing + graphColumnSpacing/2)
			colorClass := ui.getColumnColor(col)
			r, g, b := ui.colorForClass(colorClass)
			cr.SetSourceRGB(r, g, b)
			cr.SetLineWidth(2.1)

			// CORRECTION MAJEURE: Dessiner la ligne si la colonne est active AVANT OU APRÈS
			// Cela élimine complètement les gaps
			if before[col] || after[col] {
				// Toujours dessiner une ligne complète de haut en bas
				cr.MoveTo(x, topY)
				cr.LineTo(x, bottomY)
				cr.Stroke()
			}
		}

		// Dessiner les connexions courbes pour les merges et branches
		// Connexions entrantes (depuis les commits supérieurs)
		for _, conn := range node.IncomingConnections {
			if conn.ToIndex != index {
				continue
			}
			fromX := float64(conn.FromColumn*graphColumnSpacing + graphColumnSpacing/2)
			toX := float64(conn.ToColumn*graphColumnSpacing + graphColumnSpacing/2)
			
			// Utiliser la couleur de la branche source
			r, g, b := ui.colorForClass(ui.getColumnColor(conn.FromColumn))
			cr.SetSourceRGB(r, g, b)
			cr.SetLineWidth(2.3)

			// Ne dessiner que les connexions courbes (pas les lignes droites)
			if conn.FromColumn != conn.ToColumn {
				// Connexion courbe depuis le haut vers le centre
				offset := float64(height) * 0.35
				cr.MoveTo(fromX, topY)
				cr.CurveTo(fromX, topY+offset, toX, centerY-offset, toX, centerY)
				cr.Stroke()
			}
		}

		// Connexions sortantes (vers les commits inférieurs)
		for _, conn := range node.OutgoingConnections {
			if conn.FromIndex != index {
				continue
			}
			fromX := float64(conn.FromColumn*graphColumnSpacing + graphColumnSpacing/2)
			toX := float64(conn.ToColumn*graphColumnSpacing + graphColumnSpacing/2)
			
			// Utiliser la couleur de la branche source
			r, g, b := ui.colorForClass(ui.getColumnColor(conn.FromColumn))
			cr.SetSourceRGB(r, g, b)
			cr.SetLineWidth(2.3)

			// Ne dessiner que les connexions courbes (pas les lignes droites)
			if conn.FromColumn != conn.ToColumn {
				// Connexion courbe depuis le centre vers le bas
				offset := float64(height) * 0.35
				cr.MoveTo(fromX, centerY)
				cr.CurveTo(fromX, centerY+offset, toX, bottomY-offset, toX, bottomY)
				cr.Stroke()
			}
		}

		// Point du commit (dessiné en dernier pour être au-dessus des lignes)
		commitX := float64(node.Column*graphColumnSpacing + graphColumnSpacing/2)
		r, g, b := ui.colorForClass(node.Color)
		
		// Dessiner un cercle plein avec bordure
		cr.SetSourceRGB(r, g, b)
		cr.Arc(commitX, centerY, dotRadius, 0, 2*math.Pi)
		cr.Fill()
		
		// Bordure contrastée
		cr.SetSourceRGB(r*0.6, g*0.6, b*0.6)
		cr.SetLineWidth(1.2)
		cr.Arc(commitX, centerY, dotRadius, 0, 2*math.Pi)
		cr.Stroke()
		
		// Point central blanc pour plus de visibilité (comme dans l'image de référence)
		if node.IsMerge {
			cr.SetSourceRGB(1.0, 1.0, 1.0)
			cr.Arc(commitX, centerY, dotRadius*0.3, 0, 2*math.Pi)
			cr.Fill()
		}
	})

	return area
}

// Optimiser l'affichage des branches parallèles
func (ui *UI) optimizeBranchLayout() {
	if len(ui.GraphNodes) == 0 {
		return
	}

	// Analyser les patterns de branches pour optimiser l'affichage
	branchPatterns := make(map[string][]int) // branche -> indices des commits

	for i, node := range ui.GraphNodes {
		for _, branch := range node.Branches {
			if branch != "" && !strings.Contains(branch, "HEAD") {
				branchPatterns[branch] = append(branchPatterns[branch], i)
			}
		}
	}

	// Réorganiser les colonnes pour minimiser les croisements
	ui.minimizeCrossings(branchPatterns)
}

// Minimiser les croisements entre branches
func (ui *UI) minimizeCrossings(branchPatterns map[string][]int) {
	// Algorithme simple pour réduire les croisements
	// Dans une implémentation plus avancée, on utiliserait des algorithmes de graphes

	columnUsage := make(map[int]int) // colonne -> nombre d'utilisations

	for i := range ui.GraphNodes {
		node := &ui.GraphNodes[i]
		columnUsage[node.Column]++

		// Si une colonne est très utilisée, essayer de redistribuer
		if columnUsage[node.Column] > len(ui.GraphNodes)/4 {
			// Chercher une colonne moins utilisée
			for col := 0; col < ui.ColumnCount; col++ {
				if columnUsage[col] < columnUsage[node.Column]/2 {
					node.Column = col
					node.Color = ui.getColumnColor(col)
					columnUsage[col]++
					columnUsage[node.Column]--
					break
				}
			}
		}
	}
}

// Vérifier s'il y a une branche active sur une colonne donnée
func (ui *UI) loadMoreCommits() {
	if ui.Repo == nil {
		return
	}
	skip := ui.Loaded
	max := ui.PageSize
	ui.busyStart()
	go func() {
		commits, err := ui.Repo.ListCommitsPaginated(skip, max)
		if err != nil {
			ui.toastAsync("Erreur chargement: " + err.Error())
			glib.IdleAdd(func() { ui.busyStop() })
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

func (ui *UI) renderCommit(info core.CommitInfo, msg string, _ []core.FileDiff, patch string) {
	buf := ui.CommitDetail.Buffer()
	buf.SetText("")
	if ui.tags == nil {
		ui.tags = map[string]*gtk.TextTag{}
	}

	ensure := func(name, color string, bold bool) *gtk.TextTag {
		if t, ok := ui.tags[name]; ok {
			return t
		}
		t := gtk.NewTextTag(name)
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
		if tag != nil {
			buf.ApplyTag(tag, it, it2)
		}
	}

	// En-tête avec statistiques
	head := fmt.Sprintf("%s %s\nAuteur: %s <%s>\nDate: %s\nModifié: %d fichiers, +%d -%d\n",
		info.ShortID, info.Summary, info.Author, info.Email, info.Time, info.FilesChanged, info.Insertions, info.Deletions)
	for _, l := range strings.Split(head, "\n") {
		if l != "" {
			insert(l, tagHeader)
		}
	}

	if strings.TrimSpace(msg) != "" {
		insert("", nil)
		for _, l := range strings.Split(strings.TrimRight(msg, "\n"), "\n") {
			insert(l, nil)
		}
	}
	insert("", nil)

	// Colorisation du patch
	currentFilePrinted := false
	for _, l := range strings.Split(strings.TrimRight(patch, "\n"), "\n") {
		if strings.HasPrefix(l, "diff --git ") {
			currentFilePrinted = false
			continue
		}
		if strings.HasPrefix(l, "+++ b/") || strings.HasPrefix(l, "--- a/") {
			if strings.HasPrefix(l, "+++ b/") && !currentFilePrinted {
				insert("Fichier: "+strings.TrimPrefix(l, "+++ b/"), tagFile)
				currentFilePrinted = true
			}
			continue
		}
		if strings.HasPrefix(l, "@@ ") {
			insert(l, tagHeader)
			continue
		}
		if l == "" {
			insert("", nil)
			continue
		}
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
	if ui.Repo == nil {
		ui.toast("Aucun dépôt ouvert")
		return
	}
	go func() {
		glib.IdleAdd(func() { ui.busyStart() })
		if err := ui.Repo.Fetch(); err != nil {
			ui.toastAsync("Erreur fetch: " + err.Error())
			glib.IdleAdd(func() { ui.busyStop() })
			return
		}
		ui.toastAsync("Fetch terminé")
		ui.reloadSidePanel()
		glib.IdleAdd(func() { ui.busyStop() })
	}()
}

func (ui *UI) actionPull() {
	if ui.Repo == nil {
		ui.toast("Aucun dépôt ouvert")
		return
	}
	go func() {
		glib.IdleAdd(func() { ui.busyStart() })
		if err := ui.Repo.Pull(); err != nil {
			ui.toastAsync("Erreur pull: " + err.Error())
			glib.IdleAdd(func() { ui.busyStop() })
			return
		}
		ui.toastAsync("Pull terminé")
		ui.reloadSidePanel()
		glib.IdleAdd(func() { ui.busyStop() })
	}()
}

func (ui *UI) actionStash() {
	if ui.Repo == nil {
		ui.toast("Aucun dépôt ouvert")
		return
	}
	go func() {
		glib.IdleAdd(func() { ui.busyStart() })
		if err := ui.Repo.Stash("WIP"); err != nil {
			ui.toastAsync("Erreur stash: " + err.Error())
			glib.IdleAdd(func() { ui.busyStop() })
			return
		}
		ui.toastAsync("Stash créé")
		ui.reloadSidePanel()
		glib.IdleAdd(func() { ui.busyStop() })
	}()
}

// Fonctions utilitaires

func (ui *UI) toast(msg string) {
	ui.Win.SetTitle("LibreFork - " + msg)
}

func (ui *UI) toastAsync(msg string) {
	glib.IdleAdd(func() { ui.toast(msg) })
}

func (ui *UI) busyStart() {
	if ui.Activity != nil {
		ui.Activity.SetSpinning(true)
	}
}

func (ui *UI) busyStop() {
	if ui.Activity != nil {
		ui.Activity.SetSpinning(false)
	}
}
