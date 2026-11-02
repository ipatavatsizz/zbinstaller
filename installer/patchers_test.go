package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPatchSteamConfigsPatchesExistingAppsEntry(t *testing.T) {
	steamRoot := t.TempDir()
	configDir := filepath.Join(steamRoot, "userdata", "123", "config")
	if err := os.MkdirAll(configDir, 0755); err != nil {
		t.Fatal(err)
	}
	configPath := filepath.Join(configDir, "localconfig.vdf")
	input := `"UserLocalConfigStore"
{
	"Software"
	{
		"Valve"
		{
			"Steam"
			{
				"apps"
				{
					"108600"
					{
						"LaunchOptions"		"-debug"
					}
				}
			}
		}
	}
}`
	if err := os.WriteFile(configPath, []byte(input), 0644); err != nil {
		t.Fatal(err)
	}
	oldPath := STEAM_PATH
	STEAM_PATH = steamRoot
	t.Cleanup(func() { STEAM_PATH = oldPath })

	if err := PatchSteamConfigs(); err != nil {
		t.Fatal(err)
	}
	output, err := os.ReadFile(configPath)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(output), ZB_FLAGS) {
		t.Fatalf("patched config does not contain %q:\n%s", ZB_FLAGS, output)
	}
}

func TestPatchProjectZomboidBatAddsAndPreservesOptions(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "ProjectZomboid64.bat")
	if err := os.WriteFile(path, []byte("@echo off\r\nset PATH=%PATH%\r\n"), 0644); err != nil {
		t.Fatal(err)
	}
	oldPath := PZ_PATH
	PZ_PATH = dir
	t.Cleanup(func() { PZ_PATH = oldPath })

	if err := PatchProjectZomboidBat(); err != nil {
		t.Fatal(err)
	}
	output, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	text := string(output)
	if !strings.Contains(text, "SET _JAVA_OPTIONS="+ZB_FLAGS) || !strings.Contains(text, "set PATH=%PATH%") {
		t.Fatalf("unexpected batch output: %q", text)
	}
	if _, err := os.Stat(path + ".backup"); err != nil {
		t.Fatalf("backup was not created: %v", err)
	}
}

func TestPatchProjectZomboidBatIsIdempotent(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "ProjectZomboid64.bat")
	input := "SET _JAVA_OPTIONS=" + ZB_FLAGS + " -Xmx4G\r\n"
	if err := os.WriteFile(path, []byte(input), 0644); err != nil {
		t.Fatal(err)
	}
	oldPath := PZ_PATH
	PZ_PATH = dir
	t.Cleanup(func() { PZ_PATH = oldPath })

	if err := PatchProjectZomboidBat(); err != nil {
		t.Fatal(err)
	}
	first, _ := os.ReadFile(path)
	if err := PatchProjectZomboidBat(); err != nil {
		t.Fatal(err)
	}
	second, _ := os.ReadFile(path)
	if string(first) != string(second) {
		t.Fatalf("second patch changed the file: first=%q second=%q", first, second)
	}
}

func TestPatchProjectZomboidJsonAddsVMArg(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "ProjectZomboid64.json")
	if err := os.WriteFile(path, []byte(`{"vmArgs":["-Xmx4G"],"windows":{"vmArgs":["-Xms2G"]}}`), 0644); err != nil {
		t.Fatal(err)
	}
	oldPath := PZ_PATH
	PZ_PATH = dir
	t.Cleanup(func() { PZ_PATH = oldPath })

	if err := PatchProjectZomboidJson(); err != nil {
		t.Fatal(err)
	}
	output, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var parsed map[string]interface{}
	if err := json.Unmarshal(output, &parsed); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(output), ZB_FLAGS) {
		t.Fatalf("missing native launch flag: %s", output)
	}
	if strings.Contains(string(output), `"-agentlib:zbNative","-Xms2G"`) {
		t.Fatal("nested Windows vmArgs was modified")
	}
	if _, err := os.Stat(path + ".backup"); err != nil {
		t.Fatalf("backup was not created: %v", err)
	}

}

func TestPatchRootVMArgsOnly(t *testing.T) {
	root := map[string]interface{}{
		"vmArgs": []interface{}{"-Xmx3072m"},
		"windows": map[string]interface{}{
			"10.0.17134": map[string]interface{}{
				"vmArgs": []interface{}{"-XX:+UseZGC"},
			},
		},
	}
	changed, err := patchRootVMArgs(root)
	if err != nil || !changed {
		t.Fatalf("changed=%v err=%v", changed, err)
	}
	if got := root["vmArgs"].([]interface{})[0]; got != ZB_FLAGS {
		t.Fatalf("root vmArgs[0] = %v", got)
	}
	nested := root["windows"].(map[string]interface{})["10.0.17134"].(map[string]interface{})["vmArgs"].([]interface{})
	if len(nested) != 1 || nested[0] != "-XX:+UseZGC" {
		t.Fatalf("nested vmArgs changed: %v", nested)
	}
}

func TestPatchProjectZomboidJsonRejectsInvalidJSON(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "ProjectZomboid64.json"), []byte("{"), 0644); err != nil {
		t.Fatal(err)
	}
	oldPath := PZ_PATH
	PZ_PATH = dir
	t.Cleanup(func() { PZ_PATH = oldPath })
	if err := PatchProjectZomboidJson(); err == nil {
		t.Fatal("expected invalid JSON error")
	}
}

func TestPatchSteamConfigTextPreservesUnchangedBytes(t *testing.T) {
	input := []byte(`"UserLocalConfigStore"
{
	"Software"
	{
		"Valve"
		{
			"Steam"
			{
				"apps"
				{
					"108600"
					{
						"LaunchOptions"		"-Xmx16G -- "
					}
				}
			}
		}
	}
	"OtherStore"
	{
		"apps"
		{
			"108600"
			{
				"LaunchOptions"		"-other-option --"
			}
		}
	}
}`)
	output, found, changed, err := patchSteamConfigText(input)
	if err != nil || !found || !changed {
		t.Fatalf("patch result found=%v changed=%v err=%v", found, changed, err)
	}
	text := string(output)
	if !strings.Contains(text, ZB_FLAGS+" -Xmx16G --") {
		t.Fatalf("patched output = %q", output)
	}
	if !strings.Contains(text, `"LaunchOptions"		"-other-option --"`) {
		t.Fatal("unrelated apps section was changed")
	}
}

func TestAtomicWriteFileReplacesContents(t *testing.T) {
	path := filepath.Join(t.TempDir(), "atomic.txt")
	if err := atomicWriteFile(path, []byte("new"), 0644); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	if string(data) != "new" {
		t.Fatalf("contents = %q, want %q", data, "new")
	}
}

func TestGetConfigPaths(t *testing.T) {
	root := t.TempDir()
	config := filepath.Join(root, "userdata", "42", "config")
	if err := os.MkdirAll(config, 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(config, "localconfig.vdf"), []byte("{}"), 0644); err != nil {
		t.Fatal(err)
	}
	oldPath := STEAM_PATH
	STEAM_PATH = root
	t.Cleanup(func() { STEAM_PATH = oldPath })
	paths := GetConfigPaths()
	if len(paths) != 1 || paths[0] != filepath.Join(config, "localconfig.vdf") {
		t.Fatalf("config paths = %v", paths)
	}
}
