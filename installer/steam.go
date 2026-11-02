package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/andygrunwald/vdf"
	"golang.org/x/sys/windows/registry"
)

func steamLibraryPaths(steamPath string) ([]string, error) {
	if !existingDirectory(steamPath) {
		return nil, fmt.Errorf("Steam directory does not exist: %s", steamPath)
	}

	paths := []string{filepath.Clean(steamPath)}
	indexPath := filepath.Join(steamPath, "steamapps", "libraryfolders.vdf")
	file, err := os.Open(indexPath)
	if err != nil {
		if os.IsNotExist(err) {
			return paths, nil
		}
		return nil, fmt.Errorf("could not open %s: %w", indexPath, err)
	}
	defer file.Close()

	parsed, err := vdf.NewParser(file).Parse()
	if err != nil {
		return nil, fmt.Errorf("could not parse %s: %w", indexPath, err)
	}
	collectSteamLibraryPaths(parsed, &paths)
	return uniquePaths(paths), nil
}

func collectSteamLibraryPaths(node interface{}, paths *[]string) {
	switch value := node.(type) {
	case map[string]interface{}:
		for key, child := range value {
			if strings.EqualFold(key, "path") {
				if path, ok := child.(string); ok && filepath.IsAbs(path) {
					*paths = append(*paths, filepath.Clean(path))
				}
			}
			collectSteamLibraryPaths(child, paths)
		}
	case map[interface{}]interface{}:
		for key, child := range value {
			if strings.EqualFold(fmt.Sprint(key), "path") {
				if path, ok := child.(string); ok && filepath.IsAbs(path) {
					*paths = append(*paths, filepath.Clean(path))
				}
			}
			collectSteamLibraryPaths(child, paths)
		}
	}
}

func uniquePaths(paths []string) []string {
	seen := make(map[string]struct{}, len(paths))
	result := make([]string, 0, len(paths))
	for _, path := range paths {
		key := strings.ToLower(filepath.Clean(path))
		if _, exists := seen[key]; exists {
			continue
		}
		seen[key] = struct{}{}
		result = append(result, filepath.Clean(path))
	}
	return result
}

func findProjectZomboidPath(steamPath string) string {
	libraries, err := steamLibraryPaths(steamPath)
	if err != nil {
		return ""
	}
	for _, library := range libraries {
		manifestPath := filepath.Join(library, "steamapps", "appmanifest_"+PZ_APP+".acf")
		if installDir, ok := readManifestInstallDir(manifestPath); ok {
			path := filepath.Join(library, "steamapps", "common", installDir)
			if existingDirectory(path) {
				return path
			}
		}
	}
	for _, library := range libraries {
		path := filepath.Join(library, "steamapps", "common", "ProjectZomboid")
		if existingDirectory(path) {
			return path
		}
	}
	return ""
}

func findWorkshopPath(steamPath string, modID Branch) string {
	libraries, err := steamLibraryPaths(steamPath)
	if err != nil {
		return ""
	}
	for _, library := range libraries {
		path := filepath.Join(library, "steamapps", "workshop", "content", PZ_APP, string(modID))
		if existingDirectory(path) {
			return path
		}
	}
	return ""
}

func readManifestInstallDir(path string) (string, bool) {
	file, err := os.Open(path)
	if err != nil {
		return "", false
	}
	defer file.Close()
	parsed, err := vdf.NewParser(file).Parse()
	if err != nil {
		return "", false
	}
	var installDir string
	var visit func(interface{})
	visit = func(node interface{}) {
		switch value := node.(type) {
		case map[string]interface{}:
			for key, child := range value {
				if strings.EqualFold(key, "installdir") {
					installDir, _ = child.(string)
				}
				visit(child)
			}
		case map[interface{}]interface{}:
			for key, child := range value {
				if strings.EqualFold(fmt.Sprint(key), "installdir") {
					installDir, _ = child.(string)
				}
				visit(child)
			}
		}
	}
	visit(parsed)
	return strings.TrimSpace(installDir), installDir != ""
}

func GetZBBranchPath(branchType Branch) (string, error) {
	if path := findWorkshopPath(STEAM_PATH, branchType); path != "" {
		return path, nil
	}
	return "", fmt.Errorf("ZombieBuddy Workshop branch is not subscribed: %s", branchType)
}

func GetZBBranchVersion(branchType Branch) string {
	path, err := GetZBBranchPath(branchType)
	if err != nil {
		return "(Not subscribed)"
	}

	versionFile := filepath.Join(path, "mods", "ZombieBuddy", "java", "VERSION")
	if _, err := os.Stat(versionFile); err != nil {
		return "Version not found"
	}

	version, err := os.ReadFile(versionFile)
	if err != nil {
		return "Version not found"
	}

	return strings.TrimSpace(string(version))
}

func FindSteam() (uint64, error) {
	k, err := registry.OpenKey(registry.CURRENT_USER, `Software\Valve\Steam\ActiveProcess`, registry.QUERY_VALUE)
	if err != nil {
		return 0, err
	}
	defer k.Close()

	pid, _, err := k.GetIntegerValue("pid")
	if err != nil {
		return 0, err
	}

	return pid, nil
}

func GetSteamPath() string {
	k, err := registry.OpenKey(registry.CURRENT_USER, `Software\Valve\Steam`, registry.QUERY_VALUE)
	if err != nil {
		return ""
	}
	defer k.Close()

	path, _, err := k.GetStringValue("SteamPath")
	if err != nil {
		return ""
	}

	path = filepath.Clean(strings.TrimSpace(path))
	if !existingDirectory(path) {
		return ""
	}
	return path
}

func KillSteam() error {
	running, err := steamProcessRunning()
	if err != nil {
		return fmt.Errorf("could not check whether Steam is running: %w", err)
	}
	if !running {
		return nil
	}

	pid, err := FindSteam()
	if err != nil {
		return fmt.Errorf("could not determine whether Steam is running: %w", err)
	}
	if pid == 0 {
		return nil
	}

	p, err := os.FindProcess(int(pid))
	if err != nil {
		return fmt.Errorf("could not open Steam process %d: %w", pid, err)
	}

	if err := p.Kill(); err != nil {
		return fmt.Errorf("could not close Steam (process %d): %w", pid, err)
	}

	running, err = steamProcessRunning()
	if err != nil {
		return fmt.Errorf("could not verify that Steam closed: %w", err)
	}
	if !running {
		return nil
	}

	return fmt.Errorf("Steam process %d is still running", pid)
}

func steamProcessRunning() (bool, error) {
	output, err := exec.Command("tasklist", "/FI", "IMAGENAME eq steam.exe", "/NH").Output()
	if err != nil {
		return false, err
	}

	line := strings.ToLower(string(output))
	if !strings.Contains(line, "steam.exe") {
		return false, nil
	}

	// A localized tasklist response can contain informational text, so require
	// a numeric PID field rather than relying only on the process name.
	fields := strings.Fields(line)
	for _, field := range fields {
		if _, err := strconv.ParseUint(field, 10, 64); err == nil {
			return true, nil
		}
	}
	return false, nil
}
