//go:build windows

package main

import (
	"encoding/csv"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"
	"time"

	"golang.org/x/sys/windows/registry"
)

func detectSteamPath() (string, error) {
	type registryLocation struct {
		root   registry.Key
		access uint32
	}

	locations := []registryLocation{
		{registry.CURRENT_USER, registry.QUERY_VALUE},
		{registry.CURRENT_USER, registry.QUERY_VALUE | registry.WOW64_32KEY},
		{registry.CURRENT_USER, registry.QUERY_VALUE | registry.WOW64_64KEY},
		{registry.LOCAL_MACHINE, registry.QUERY_VALUE},
		{registry.LOCAL_MACHINE, registry.QUERY_VALUE | registry.WOW64_32KEY},
		{registry.LOCAL_MACHINE, registry.QUERY_VALUE | registry.WOW64_64KEY},
	}
	const keyPath = `Software\Valve\Steam`
	const missingPath = "could not find Steam path in registry"

	for _, location := range locations {
		k, err := registry.OpenKey(location.root, keyPath, location.access)
		if err != nil {
			continue
		}

		for _, valueName := range []string{"SteamPath", "InstallPath"} {
			path, _, valueErr := k.GetStringValue(valueName)
			if valueErr == nil && strings.TrimSpace(path) != "" {
				k.Close()
				return filepath.Clean(strings.TrimSpace(path)), nil
			}
		}
		k.Close()
	}

	return "", fmt.Errorf(missingPath)
}

func detectSteamAppInstallPath(appID string) (string, error) {
	keyPath := `Software\Microsoft\Windows\CurrentVersion\Uninstall\Steam App ` + appID
	type registryLocation struct {
		root   registry.Key
		access uint32
	}

	locations := []registryLocation{
		{registry.CURRENT_USER, registry.QUERY_VALUE},
		{registry.CURRENT_USER, registry.QUERY_VALUE | registry.WOW64_32KEY},
		{registry.CURRENT_USER, registry.QUERY_VALUE | registry.WOW64_64KEY},
		{registry.LOCAL_MACHINE, registry.QUERY_VALUE},
		{registry.LOCAL_MACHINE, registry.QUERY_VALUE | registry.WOW64_32KEY},
		{registry.LOCAL_MACHINE, registry.QUERY_VALUE | registry.WOW64_64KEY},
	}

	for _, location := range locations {
		k, err := registry.OpenKey(location.root, keyPath, location.access)
		if err != nil {
			continue
		}
		installLocation, _, valueErr := k.GetStringValue("InstallLocation")
		k.Close()
		if valueErr != nil || installLocation == "" {
			continue
		}

		path := filepath.Clean(installLocation)
		if _, err := os.Stat(path); err == nil {
			return path, nil
		}
	}

	return "", fmt.Errorf("could not find Steam app %s in registry", appID)
}

func FindSteam() (uint64, error) {
	k, err := registry.OpenKey(registry.CURRENT_USER, `Software\Valve\Steam\ActiveProcess`, registry.QUERY_VALUE)
	if err != nil {
		return 0, err
	}
	defer k.Close()
	pid, _, err := k.GetIntegerValue("pid")
	return pid, err
}

func steamProcessIDs() ([]int, error) {
	command := exec.Command("tasklist", "/FI", "IMAGENAME eq steam.exe", "/FO", "CSV", "/NH")
	command.SysProcAttr = &syscall.SysProcAttr{HideWindow: true}
	output, err := command.Output()
	if err != nil {
		return nil, err
	}
	reader := csv.NewReader(strings.NewReader(string(output)))
	var pids []int
	for {
		record, err := reader.Read()
		if err == io.EOF {
			break
		}
		if err != nil {
			return nil, fmt.Errorf("could not parse Steam process list: %w", err)
		}
		if len(record) < 2 || !strings.EqualFold(strings.TrimSpace(record[0]), "steam.exe") {
			continue
		}
		pid, err := strconv.Atoi(strings.TrimSpace(record[1]))
		if err == nil && pid > 0 {
			pids = append(pids, pid)
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
	remaining, _ := steamProcessIDs()
	return fmt.Errorf("Steam processes are still running: %v", remaining)
}

func steamExecutable() (string, error) {
	if !existingDirectory(STEAM_PATH) {
		return "", fmt.Errorf("could not find the Steam installation")
	}
	path := filepath.Join(STEAM_PATH, "steam.exe")
	info, err := os.Stat(path)
	if err != nil {
		return "", fmt.Errorf("could not find Steam executable %s: %w", path, err)
	}
	if info.IsDir() {
		return "", fmt.Errorf("Steam executable path is a directory: %s", path)
	}
	return path, nil
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
	return []string{"zbNative.dll", "ZombieBuddy.jar"}
}
