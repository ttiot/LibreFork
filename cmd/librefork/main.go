package main

import (
    "os"

    adw "github.com/diamondburned/gotk4-adwaita/pkg/adw"
    "github.com/diamondburned/gotk4/pkg/gtk/v4"
)

func main() {
    adw.Init()
    app := gtk.NewApplication("dev.librefork.app", 0)
    app.ConnectActivate(func() { buildUI(app) })
    os.Exit(int(app.Run(nil)))
}
