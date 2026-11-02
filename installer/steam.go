package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/andygrunwald/vdf"
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

func waitForZBBranch(branch Branch, timeout time.Duration, onProgress func(time.Duration)) (string, error) {
	started := time.Now()
	ticker := time.NewTicker(500 * time.Millisecond)
	defer ticker.Stop()

	for {
		if path := findWorkshopPath(STEAM_PATH, branch); path != "" {
			ready := true
			for _, file := range ZB_FILES {
				info, err := os.Stat(filepath.Join(path, "mods", "ZombieBuddy", "libs", file))
				if err != nil || info.IsDir() || info.Size() == 0 {
					ready = false
					break
				}
			}
			if ready {
				return path, nil
			}
		}

		elapsed := time.Since(started).Round(time.Second)
		if onProgress != nil {
			onProgress(elapsed)
		}
		if time.Since(started) >= timeout {
			return "", fmt.Errorf("ZombieBuddy Workshop item did not finish downloading within %s", timeout.Round(time.Second))
		}
		<-ticker.C
	}
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

func GetSteamPath() string {
	path, err := detectSteamPath()
	if err != nil {
		return ""
	}

	path = filepath.Clean(strings.TrimSpace(path))
	if !existingDirectory(path) {
		return ""
	}
	return path
}

func steamRootHasMetadata(path string) bool {
	for _, candidate := range []string{
		filepath.Join(path, "steamapps"),
		filepath.Join(path, "steam.sh"),
		filepath.Join(path, "steam"),
	} {
		if _, err := os.Stat(candidate); err == nil {
			return true
		}
	}
	return false
}

func KillSteam() error {
	return killSteamProcess()
}

func StartSteam() error {
	return startSteamProcess()
}

func OpenSteamWorkshopPage(branch Branch) error {
	return openSteamWorkshopPage(branch)
}

func steamProcessRunning() (bool, error) {
	return steamProcessIsRunning()
}
