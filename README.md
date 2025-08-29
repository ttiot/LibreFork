# LibreFork

Un **client Git** léger et rapide pour **Linux**, inspiré par Fork.
Stack : **Rust + GTK4 + libadwaita + libgit2**.

## État
MVP en cours : ouverture d’un dépôt, affichage des commits, détail d’un commit.

## Construire (Debian/Ubuntu)
```bash
sudo apt install -y build-essential pkg-config libgtk-4-dev libadwaita-1-dev libgit2-dev
curl https://sh.rustup.rs -sSf | sh   # si Rust n'est pas installé
cd LibreFork
cargo run -p librefork-ui -- --repo /chemin/vers/mon/repo
```

## Roadmap (extrait)
- [x] Squelette Rust + GTK4
- [x] Liste de commits (revwalk topo + temps)
- [x] Détails d’un commit sélectionné
- [x] Diff viewer unifié → side-by-side
- [x] Staging par fichier, puis par hunk
- [ ] Fetch/Pull/Push (progress UI)
- [ ] Rebase interactif (pick/squash/fixup)
- [ ] Stash manager
- [ ] Worktrees/Submodules
- [ ] Flatpak

## Licence
MPL-2.0
