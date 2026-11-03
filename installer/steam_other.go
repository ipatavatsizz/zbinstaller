//go:build !windows

package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"time"
)

func detectSteamPath() (string, error) {
	for _, path := range steamPathCandidates() {
		if existingDirectory(path) && steamRootHasMetadata(path) {
			return path, nil
		}
	}
	return "", fmt.Errorf("could not find a Steam installation")
}

func detectSteamAppInstallPath(appID string) (string, error) {
	for _, library := range steamPathCandidates() {
		manifest := filepath.Join(library, "steamapps", "appmanifest_"+appID+".acf")
		if installDir, ok := readManifestInstallDir(manifest); ok {
			path := filepath.Join(library, "steamapps", "common", installDir)
			if existingDirectory(path) {
				return path, nil
			}
		}
	}
	return "", fmt.Errorf("could not find Steam app %s", appID)
}

func steamPathCandidates() []string {
	home, err := os.UserHomeDir()
	if err != nil {
		return nil
	}
	if runtime.GOOS == "darwin" {
		return []string{
			filepath.Join(home, "Library", "Application Support", "Steam"),
		}
	}
	return []string{
		filepath.Join(home, ".steam", "steam"),
		filepath.Join(home, ".local", "share", "Steam"),
		filepath.Join(home, ".local", "share", "steam"),
		filepath.Join(home, ".var", "app", "com.valvesoftware.Steam", "data", "Steam"),
		filepath.Join(home, "snap", "steam", "common", ".steam", "steam"),
	}
}

func fileExists(path string) bool {
	info, err := os.Stat(path)
	return err == nil && !info.IsDir()
}

func FindSteam() (uint64, error) {
	pids, err := steamProcessIDs()
	if err != nil || len(pids) == 0 {
		return 0, fmt.Errorf("Steam is not running")
	}
	return uint64(pids[0]), nil
}

func steamProcessIDs() ([]int, error) {
	names := []string{"steam", "Steam"}
	seen := map[int]bool{}
	var pids []int
	for _, name := range names {
		output, err := exec.Command("pgrep", "-x", name).Output()
		if err != nil {
			if _, ok := err.(*exec.ExitError); ok {
				continue
			}
			return nil, err
		}
		for _, line := range strings.Split(strings.TrimSpace(string(output)), "\n") {
			pid, err := strconv.Atoi(strings.TrimSpace(line))
			if err == nil && pid > 0 && !seen[pid] {
				seen[pid] = true
				pids = append(pids, pid)
			}
		}
	}
	return pids, nil
}

func steamProcessIsRunning() (bool, error) {
	pids, err := steamProcessIDs()
	return len(pids) > 0, err
}

func killSteamProcess() error {
	pids, err := steamProcessIDs()
	if err != nil {
		return fmt.Errorf("could not check whether Steam is running: %w", err)
	}
	for _, pid := range pids {
		process, err := os.FindProcess(pid)
		if err != nil {
			return fmt.Errorf("could not open Steam process %d: %w", pid, err)
		}
		if err := process.Kill(); err != nil {
			return fmt.Errorf("could not close Steam process %d: %w", pid, err)
		}
	}
	for attempt := 0; attempt < 50; attempt++ {
		time.Sleep(100 * time.Millisecond)
		remaining, err := steamProcessIDs()
		if err != nil {
			return fmt.Errorf("could not verify that Steam closed: %w", err)
		}
		if len(remaining) == 0 {
			return nil
		}
	}
	return fmt.Errorf("Steam processes are still running")
}

func steamExecutable() (string, error) {
	roots := steamPathCandidates()
	if runtime.GOOS == "darwin" {
		roots = append(roots, "/Applications/Steam.app", filepath.Join(mustHomeDir(), "Applications", "Steam.app"))
	}
	for _, root := range roots {
		candidates := []string{filepath.Join(root, "steam")}
		if runtime.GOOS == "darwin" {
			candidates = []string{
				filepath.Join(root, "Steam.app", "Contents", "MacOS", "steam_osx"),
				filepath.Join(root, "Steam.app", "Contents", "MacOS", "steam"),
			}
		}
		for _, path := range candidates {
			if fileExists(path) {
				return path, nil
			}
		}
	}
	return "", fmt.Errorf("could not find Steam executable")
}

func mustHomeDir() string {
	home, _ := os.UserHomeDir()
	return home
}

func startSteamProcess() error {
	path, err := steamExecutable()
	if err != nil {
		return err
	}
	if err := exec.Command(path).Start(); err != nil {
		return fmt.Errorf("could not start Steam: %w", err)
	}
	return nil
}

func openSteamWorkshopPage(branch Branch) error {
	path, err := steamExecutable()
	if err != nil {
		return err
	}
	if err := exec.Command(path, "steam://url/CommunityFilePage/"+string(branch)).Start(); err != nil {
		return fmt.Errorf("could not open Steam Workshop page: %w", err)
	}
	return nil
}

func platformZBFiles() []string {
	if runtime.GOOS == "darwin" {
		return []string{"libzbNative.dylib", "ZombieBuddy.jar"}
	}
	return []string{"libzbNative.so", "ZombieBuddy.jar"}
}

func platformZBFlags() string {
	return "-javaagent:ZombieBuddy.jar --"
}
