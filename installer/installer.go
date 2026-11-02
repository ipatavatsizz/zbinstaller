package main

import (
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/app"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
)

type Branch string

const (
	BranchLTS  Branch = "3619862853"
	BranchBeta Branch = "3812624292"
)

type Status int

const (
	StatusIdle Status = iota
	StatusWaitingSubscription
	StatusInstalling
	StatusInstallFailed
	StatusInstallSucceeded
)

type installState struct {
	status  Status
	message string
}

var (
	currentInstallState = installState{status: StatusIdle, message: "Ready. Select an installation option."}
	installStateMu      sync.RWMutex
	installLog          []string
	installLogMu        sync.RWMutex
)

func updateInstallState(status Status, message string) {
	installStateMu.Lock()
	currentInstallState = installState{status: status, message: message}
	installStateMu.Unlock()
	recordInstallLog(message)
}

func readInstallState() installState {
	installStateMu.RLock()
	defer installStateMu.RUnlock()
	return currentInstallState
}

func recordInstallLog(message string) {
	entry := fmt.Sprintf("[%s] %s", time.Now().Format("15:04:05"), message)
	installLogMu.Lock()
	installLog = append(installLog, entry)
	installLogMu.Unlock()
}

func readInstallLog() string {
	installLogMu.RLock()
	defer installLogMu.RUnlock()
	return strings.Join(installLog, "\n")
}

func clearInstallLog() {
	installLogMu.Lock()
	installLog = nil
	installLogMu.Unlock()
}

const (
	VERSION  = "4.5"
	PZ_APP   = "108600"
	ZB_FLAGS = "-agentlib:zbNative"
)

var (
	STEAM_PATH = GetSteamPath()
	PZ_PATH    = findProjectZomboidPath(STEAM_PATH)
	ZB_FILES   = []string{"zbNative.dll", "ZombieBuddy.jar"}
)

func existingDirectory(path string) bool {
	if path == "" || !filepath.IsAbs(path) {
		return false
	}

	info, err := os.Stat(path)
	return err == nil && info.IsDir()
}

func openDebugWindow(a fyne.App, onClosed func()) fyne.Window {
	window := a.NewWindow("ZombieBuddy Installer Diagnostics")
	window.Resize(fyne.NewSize(760, 560))
	window.CenterOnScreen()
	window.Canvas().SetOnTypedKey(func(key *fyne.KeyEvent) {
		if key.Name == fyne.KeyEscape {
			window.Close()
		}
	})

	status := widget.NewLabel("")
	status.Wrapping = fyne.TextWrapWord
	log := widget.NewLabel("")
	log.Wrapping = fyne.TextWrapWord
	log.TextStyle = fyne.TextStyle{Monospace: true}
	log.Alignment = fyne.TextAlignLeading

	appendLog := func(title string, value interface{}) {
		recordInstallLog(fmt.Sprintf("%s: %v", title, value))
		log.SetText(readInstallLog())
	}

	refresh := func() {
		state := readInstallState()
		status.SetText(fmt.Sprintf("Status: %s\nMessage: %s\n\nSteam: %s\nProject Zomboid: %s",
			debugStatusName(state.status),
			state.message,
			debugPathState(STEAM_PATH),
			debugPathState(PZ_PATH),
		))
	}

	run := func(name string, action func() interface{}) {
		appendLog(name, "Running...")
		go func() {
			result := action()
			fyne.Do(func() {
				appendLog(name, result)
				refresh()
			})
		}()
	}

	configs := widget.NewLabel("No config paths loaded.")
	libraries := widget.NewLabel("No Steam libraries loaded.")
	files := widget.NewLabel("Launcher files: not checked")
	refreshDetails := func() {
		refresh()
		paths := GetConfigPaths()
		if len(paths) == 0 {
			configs.SetText("Steam configs: none found")
		} else {
			configs.SetText("Steam configs:\n" + strings.Join(paths, "\n"))
		}
		libraryPaths, err := steamLibraryPaths(STEAM_PATH)
		if err != nil {
			libraries.SetText("Steam libraries: " + err.Error())
		} else {
			libraries.SetText("Steam libraries:\n" + strings.Join(libraryPaths, "\n"))
		}
		steamState := "not running"
		if running, err := steamProcessRunning(); err != nil {
			steamState = "check failed: " + err.Error()
		} else if running {
			steamState = "running"
		}
		jsonPath := filepath.Join(PZ_PATH, "ProjectZomboid64.json")
		batPath := filepath.Join(PZ_PATH, "ProjectZomboid64.bat")
		files.SetText(fmt.Sprintf(
			"Launcher files:\nProjectZomboid64.json: %s\nProjectZomboid64.bat: %s\nzbNative.dll: %s\nZombieBuddy.jar: %s",
			debugFileState(jsonPath),
			debugFileState(batPath),
			debugFileState(filepath.Join(PZ_PATH, "zbNative.dll")),
			debugFileState(filepath.Join(PZ_PATH, "ZombieBuddy.jar")),
		))
		appendLog("Diagnostics refreshed", fmt.Sprintf(
			"Steam process=%s; localconfig files=%d; libraries=%d; LTS=%s; Beta=%s",
			steamState,
			len(paths),
			len(libraryPaths),
			GetZBBranchVersion(BranchLTS),
			GetZBBranchVersion(BranchBeta),
		))
	}

	refreshButton := widget.NewButton("Refresh Diagnostics", refreshDetails)
	configButton := widget.NewButton("Patch Steam Launch Options", func() {
		run("Patch Steam Launch Options", func() interface{} {
			if err := PatchSteamConfigs(); err != nil {
				return "ERROR: " + err.Error()
			}
			return "Completed successfully."
		})
	})
	jsonButton := widget.NewButton("Patch PZ JSON", func() {
		run("Patch ProjectZomboid64.json", func() interface{} {
			if err := PatchProjectZomboidJson(); err != nil {
				return "ERROR: " + err.Error()
			}
			return "Completed successfully."
		})
	})
	batButton := widget.NewButton("Patch PZ Batch", func() {
		run("Patch ProjectZomboid64.bat", func() interface{} {
			if err := PatchProjectZomboidBat(); err != nil {
				return "ERROR: " + err.Error()
			}
			return "Completed successfully."
		})
	})
	clearButton := widget.NewButton("Clear Log", func() {
		clearInstallLog()
		log.SetText("")
	})

	actions := container.NewGridWithColumns(2, refreshButton, configButton, jsonButton, batButton, clearButton)
	details := container.NewVBox(status, widget.NewSeparator(), configs, widget.NewSeparator(), libraries, widget.NewSeparator(), files)
	logScroll := container.NewVScroll(log)
	logScroll.SetMinSize(fyne.NewSize(720, 190))
	content := container.NewBorder(details, actions, nil, nil, logScroll)
	window.SetContent(content)
	refreshDetails()
	log.SetText(readInstallLog())
	stopRefresh := make(chan struct{})
	go func() {
		ticker := time.NewTicker(200 * time.Millisecond)
		defer ticker.Stop()
		for {
			select {
			case <-ticker.C:
				fyne.Do(func() {
					state := readInstallState()
					status.SetText(fmt.Sprintf("Status: %s\nMessage: %s\n\nSteam: %s\nProject Zomboid: %s",
						debugStatusName(state.status),
						state.message,
						debugPathState(STEAM_PATH),
						debugPathState(PZ_PATH),
					))
					log.SetText(readInstallLog())
					log.Refresh()
				})
			case <-stopRefresh:
				return
			}
		}
	}()
	window.SetOnClosed(func() {
		close(stopRefresh)
		if onClosed != nil {
			onClosed()
		}
	})
	return window
}

func debugStatusName(status Status) string {
	switch status {
	case StatusWaitingSubscription:
		return "Waiting for Workshop subscription"
	case StatusInstalling:
		return "Installing"
	case StatusInstallFailed:
		return "Installation failed"
	case StatusInstallSucceeded:
		return "Installation succeeded"
	default:
		return "Idle"
	}
}

func debugPathState(path string) string {
	if existingDirectory(path) {
		return path + " (available)"
	}
	if path == "" {
		return "(not detected)"
	}
	return path + " (missing or invalid)"
}

func debugFileState(path string) string {
	if path == "" {
		return "not detected"
	}
	info, err := os.Stat(path)
	if err != nil {
		if os.IsNotExist(err) {
			return "missing"
		}
		return "error: " + err.Error()
	}
	if info.IsDir() {
		return "invalid (directory)"
	}
	return fmt.Sprintf("available (%d bytes)", info.Size())
}

func main() {
	icon, err := fyne.LoadResourceFromPath("./icon.png")
	if err != nil {
		fmt.Printf("[!] Failed to load icon: %v\n", err)
	}

	app := app.New()
	app.SetIcon(icon)
	var dwindow fyne.Window
	window := app.NewWindow("ZombieBuddy Installer " + VERSION)
	window.Resize(fyne.NewSize(400, 200))
	window.CenterOnScreen()
	window.SetOnClosed(func() {
		if dwindow != nil {
			dwindow.Close()
		}
	})

	window.Canvas().SetOnTypedKey(func(key *fyne.KeyEvent) {
		if key.Name == fyne.KeyEscape {
			app.Quit()
		}
		if key.Name == fyne.KeyF5 {
			if dwindow != nil {
				return
			}
			dwindow = openDebugWindow(app, func() { dwindow = nil })
			dwindow.Show()
		}
	})

	mlayout := container.New(layout.NewCustomPaddedVBoxLayout(12))

	logo := canvas.NewImageFromResource(icon)
	logo.FillMode = canvas.ImageFillContain
	logo.SetMinSize(fyne.NewSquareSize(140))

	mlayout.Add(logo)

	if runtime.GOOS != "windows" {
		mlayout.Add(widget.NewLabel("Sorry! This installer only works on Windows systems."))
		mlayout.Add(widget.NewButton("Exit", func() {
			app.Quit()
		}))
	}

	statusLayout := container.NewCenter()

	slabel := widget.NewLabel(readInstallState().message)
	var ltsButton *widget.Button
	var betaButton *widget.Button
	installBranch := func(branch Branch) {
		updateInstallState(StatusInstalling, "Starting installation...")
		slabel.SetText("Starting installation...")
		slabel.Importance = widget.MediumImportance
		slabel.Refresh()
		ltsButton.Disable()
		betaButton.Disable()
		result := make(chan error, 1)
		done := make(chan struct{})
		go func() {
			result <- InstallZB(branch)
			close(done)
			fyne.Do(func() {
				err := <-result
				state := readInstallState()
				slabel.SetText(state.message)
				slabel.Importance = widget.SuccessImportance
				if err != nil {
					slabel.Importance = widget.DangerImportance
					dialog.ShowError(err, window)
				} else {
					dialog.ShowInformation("Installation complete", "ZombieBuddy was installed and configured successfully.", window)
				}
				slabel.Refresh()
				ltsButton.Enable()
				betaButton.Enable()
			})
		}()
		go func() {
			ticker := time.NewTicker(100 * time.Millisecond)
			defer ticker.Stop()
			for {
				select {
				case <-done:
					return
				case <-ticker.C:
					fyne.Do(func() {
						state := readInstallState()
						slabel.SetText(state.message)
						slabel.Refresh()
					})
				}
			}
		}()
	}

	statusLayout.Add(slabel)

	branchLayout := container.NewGridWithColumns(2)

	ltsButton = widget.NewButton("Install LTS "+GetZBBranchVersion(BranchLTS), func() {
		installBranch(BranchLTS)
	})
	betaButton = widget.NewButton("Install Beta "+GetZBBranchVersion(BranchBeta), func() {
		installBranch(BranchBeta)
	})
	branchLayout.Add(ltsButton)

	branchLayout.Add(betaButton)

	mlayout.Add(statusLayout)
	mlayout.Add(branchLayout)
	githubURL, _ := url.Parse("https://github.com/zed-0xff/ZombieBuddy")
	websiteURL, _ := url.Parse("https://ipatavatsizz.github.io/zbinstaller")
	links := container.NewCenter(container.NewHBox(
		widget.NewLabel("Created by"),
		widget.NewHyperlink("zed-0xff", githubURL),
		widget.NewLabel("  •  "),
		widget.NewHyperlink("ZombieBuddy Installer", websiteURL),
	))
	mlayout.Add(links)

	window.SetContent(mlayout)
	window.ShowAndRun()
}
