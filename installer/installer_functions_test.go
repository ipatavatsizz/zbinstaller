package main

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestInstallStateRoundTrip(t *testing.T) {
	updateInstallState(StatusInstalling, "Testing")
	state := readInstallState()
	if state.status != StatusInstalling || state.message != "Testing" {
		t.Fatalf("state = %+v", state)
	}
}

func TestExistingDirectory(t *testing.T) {
	dir := t.TempDir()
	if !existingDirectory(dir) {
		t.Fatal("existing directory was rejected")
	}
	if existingDirectory(filepath.Join(dir, "missing")) {
		t.Fatal("missing directory was accepted")
	}
	if existingDirectory("relative") {
		t.Fatal("relative directory was accepted")
	}
}

func TestCopyFile(t *testing.T) {
	dir := t.TempDir()
	src := filepath.Join(dir, "source")
	dst := filepath.Join(dir, "nested", "destination")
	if err := os.WriteFile(src, []byte("content"), 0644); err != nil {
		t.Fatal(err)
	}
	if err := os.Mkdir(filepath.Dir(dst), 0755); err != nil {
		t.Fatal(err)
	}
	if err := CopyFile(src, dst); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(dst)
	if string(data) != "content" {
		t.Fatalf("copied contents = %q", data)
	}
}

func TestInstallSnapshotRestore(t *testing.T) {
	dir := t.TempDir()
	existing := filepath.Join(dir, "existing")
	created := filepath.Join(dir, "created")
	if err := os.WriteFile(existing, []byte("before"), 0644); err != nil {
		t.Fatal(err)
	}
	snapshot, err := createInstallSnapshot([]string{existing, created, existing})
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(existing, []byte("changed"), 0644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(created, []byte("new"), 0644); err != nil {
		t.Fatal(err)
	}
	if err := snapshot.restore(); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(existing)
	if string(data) != "before" {
		t.Fatalf("restored contents = %q", data)
	}
	if _, err := os.Stat(created); !os.IsNotExist(err) {
		t.Fatal("new file was not removed during rollback")
	}
}

func TestUniquePaths(t *testing.T) {
	paths := uniquePaths([]string{`C:\Steam`, `c:\steam`, `C:\Other`})
	if !reflect.DeepEqual(paths, []string{`C:\Steam`, `C:\Other`}) {
		t.Fatalf("unique paths = %v", paths)
	}
}

func TestCollectSteamLibraryPaths(t *testing.T) {
	data := map[string]interface{}{
		"libraryfolders": map[string]interface{}{
			"1": map[string]interface{}{"path": `D:\SteamLibrary`},
			"2": map[string]interface{}{"path": "relative"},
		},
	}
	paths := []string{`C:\Steam`}
	collectSteamLibraryPaths(data, &paths)
	if !reflect.DeepEqual(paths, []string{`C:\Steam`, `D:\SteamLibrary`}) {
		t.Fatalf("library paths = %v", paths)
	}
}

func TestSteamLibraryPathsMissingIndex(t *testing.T) {
	root := t.TempDir()
	paths, err := steamLibraryPaths(root)
	if err != nil {
		t.Fatal(err)
	}
	if len(paths) != 1 || paths[0] != root {
		t.Fatalf("paths = %v", paths)
	}
}

func TestSteamLibraryPathsAndManifest(t *testing.T) {
	root := t.TempDir()
	if err := os.MkdirAll(filepath.Join(root, "steamapps"), 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.MkdirAll(filepath.Join(root, "steamapps", "common", "ProjectZomboid"), 0755); err != nil {
		t.Fatal(err)
	}
	manifest := `"AppState"
{
	"installdir"		"ProjectZomboid"
}`
	manifestPath := filepath.Join(root, "steamapps", "appmanifest_"+PZ_APP+".acf")
	if err := os.WriteFile(manifestPath, []byte(manifest), 0644); err != nil {
		t.Fatal(err)
	}
	if got := findProjectZomboidPath(root); got != filepath.Join(root, "steamapps", "common", "ProjectZomboid") {
		t.Fatalf("Project Zomboid path = %q", got)
	}
}

func TestFindWorkshopPathAndBranchVersion(t *testing.T) {
	root := t.TempDir()
	branch := filepath.Join(root, "steamapps", "workshop", "content", PZ_APP, string(BranchLTS))
	version := filepath.Join(branch, "mods", "ZombieBuddy", "java")
	if err := os.MkdirAll(version, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(version, "VERSION"), []byte("  1.2.3 \n"), 0644); err != nil {
		t.Fatal(err)
	}
	if got := findWorkshopPath(root, BranchLTS); got != branch {
		t.Fatalf("workshop path = %q", got)
	}
	oldPath := STEAM_PATH
	STEAM_PATH = root
	t.Cleanup(func() { STEAM_PATH = oldPath })
	got, err := GetZBBranchPath(BranchLTS)
	if err != nil || got != branch {
		t.Fatalf("branch path = %q, err = %v", got, err)
	}
	if got := GetZBBranchVersion(BranchLTS); got != "1.2.3" {
		t.Fatalf("branch version = %q", got)
	}
	if got := GetZBBranchVersion(BranchBeta); got != "(Not subscribed)" {
		t.Fatalf("missing branch version = %q", got)
	}
}

func TestReadManifestInstallDir(t *testing.T) {
	path := filepath.Join(t.TempDir(), "manifest.acf")
	if err := os.WriteFile(path, []byte(`"AppState" { "installdir" "ProjectZomboid" }`), 0644); err != nil {
		t.Fatal(err)
	}
	got, ok := readManifestInstallDir(path)
	if !ok || got != "ProjectZomboid" {
		t.Fatalf("manifest = %q, %v", got, ok)
	}
}

func TestSteamProcessRunning(t *testing.T) {
	if _, err := steamProcessRunning(); err != nil {
		t.Fatal(err)
	}
}

func TestSteamRegistryAndProcessFunctions(t *testing.T) {
	if path := GetSteamPath(); path != "" && !filepath.IsAbs(path) {
		t.Fatalf("Steam path is not absolute: %q", path)
	}
	if pid, err := FindSteam(); err == nil && pid == 0 {
		t.Fatal("Steam registry returned an invalid zero PID")
	}
	if _, err := detectSteamPath(); err == nil {
		// A successful registry lookup must return a usable absolute path.
		path, lookupErr := detectSteamPath()
		if lookupErr != nil || !filepath.IsAbs(path) {
			t.Fatalf("detected Steam path = %q, err = %v", path, lookupErr)
		}
	}
	if _, err := detectSteamAppInstallPath("installer-test-app-does-not-exist"); err == nil {
		t.Fatal("nonexistent Steam app unexpectedly resolved")
	}
}

func TestKillSteamWhenAlreadyClosed(t *testing.T) {
	running, err := steamProcessRunning()
	if err != nil {
		t.Fatal(err)
	}
	if running {
		t.Skip("Steam is running; test will not terminate a user process")
	}
	if err := KillSteam(); err != nil {
		t.Fatal(err)
	}
}

func TestInstallZBReportsMissingSteam(t *testing.T) {
	oldSteam, oldPZ := STEAM_PATH, PZ_PATH
	STEAM_PATH, PZ_PATH = "", ""
	t.Cleanup(func() {
		STEAM_PATH, PZ_PATH = oldSteam, oldPZ
	})
	if err := InstallZB(BranchLTS); err == nil || !strings.Contains(err.Error(), "Steam") {
		t.Fatalf("error = %v", err)
	}
	state := readInstallState()
	if state.status != StatusInstallFailed {
		t.Fatalf("state after failed install = %+v", state)
	}
}
