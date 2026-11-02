package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/andygrunwald/vdf"
	"golang.org/x/sys/windows"
)

func PatchProjectZomboidBat() error {
	if PZ_PATH == "" {
		return fmt.Errorf("Project Zomboid installation path could not be found")
	}

	batPath := filepath.Join(PZ_PATH, "ProjectZomboid64.bat")

	data, err := os.ReadFile(batPath)
	if err != nil {
		return fmt.Errorf("could not read %s: %w", batPath, err)
	}

	content := string(data)

	backupPath := batPath + ".backup"
	if _, err := os.Stat(backupPath); os.IsNotExist(err) {
		if err := os.WriteFile(backupPath, data, 0644); err != nil {
			return fmt.Errorf("could not create backup %s: %w", backupPath, err)
		}
	}

	newline := "\n"
	if strings.Contains(content, "\r\n") {
		newline = "\r\n"
	}
	lines := strings.Split(strings.ReplaceAll(content, "\r\n", "\n"), "\n")
	modified := false
	foundJavaOptions := false

	for i, line := range lines {
		trimmed := strings.TrimSpace(line)

		if strings.HasPrefix(strings.ToUpper(trimmed), "SET _JAVA_OPTIONS=") {
			foundJavaOptions = true
			parts := strings.SplitN(line, "=", 2)
			if len(parts) != 2 {
				return fmt.Errorf("invalid _JAVA_OPTIONS assignment in %s at line %d", batPath, i+1)
			}
			if !strings.Contains(parts[1], ZB_FLAGS) {
				val := strings.TrimSpace(parts[1])
				if val == "" {
					line = parts[0] + "=" + ZB_FLAGS
				} else {
					line = parts[0] + "=" + ZB_FLAGS + " " + val
				}
				modified = true
			}
		}
		lines[i] = line
	}

	if !foundJavaOptions {
		insertAt := 0
		for insertAt < len(lines) && strings.HasPrefix(strings.TrimSpace(lines[insertAt]), "@") {
			insertAt++
		}
		lines = append(lines, "")
		copy(lines[insertAt+1:], lines[insertAt:])
		lines[insertAt] = "SET _JAVA_OPTIONS=" + ZB_FLAGS
		modified = true
	}

	if modified {
		if err := atomicWriteFile(batPath, []byte(strings.Join(lines, newline)), 0644); err != nil {
			return fmt.Errorf("could not update %s: %w", batPath, err)
		}
	}
	return nil
}

func PatchProjectZomboidJson() error {
	if PZ_PATH == "" {
		return fmt.Errorf("Project Zomboid installation path could not be found")
	}

	jsonPath := filepath.Join(PZ_PATH, "ProjectZomboid64.json")
	if _, err := os.Stat(jsonPath); err != nil {
		found := false
		walkErr := filepath.Walk(PZ_PATH, func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if !info.IsDir() && strings.EqualFold(info.Name(), "ProjectZomboid64.json") {
				jsonPath = path
				found = true
				return filepath.SkipAll
			}
			return nil
		})
		if walkErr != nil && walkErr != filepath.SkipAll {
			return fmt.Errorf("could not search for ProjectZomboid64.json: %w", walkErr)
		}
		if !found {
			return fmt.Errorf("could not find ProjectZomboid64.json under %s", PZ_PATH)
		}
	}

	data, err := os.ReadFile(jsonPath)
	if err != nil {
		return fmt.Errorf("could not read %s: %w", jsonPath, err)
	}

	backupPath := jsonPath + ".backup"
	if _, err := os.Stat(backupPath); os.IsNotExist(err) {
		if err := os.WriteFile(backupPath, data, 0644); err != nil {
			return fmt.Errorf("could not create backup %s: %w", backupPath, err)
		}
	}

	var raw interface{}
	if err := json.Unmarshal(data, &raw); err != nil {
		return fmt.Errorf("could not parse %s: %w", jsonPath, err)
	}

	root, ok := raw.(map[string]interface{})
	if !ok {
		return fmt.Errorf("expected %s to contain a JSON object", jsonPath)
	}
	modified, err := patchRootVMArgs(root)
	if err != nil {
		return fmt.Errorf("could not patch root vmArgs in %s: %w", jsonPath, err)
	}

	if modified {
		out, err := json.MarshalIndent(raw, "", "\t")
		if err != nil {
			return fmt.Errorf("could not serialize %s: %w", jsonPath, err)
		}
		if err := atomicWriteFile(jsonPath, out, 0644); err != nil {
			return fmt.Errorf("could not update %s: %w", jsonPath, err)
		}
	}
	return nil
}

func patchRootVMArgs(root map[string]interface{}) (bool, error) {
	value, exists := root["vmArgs"]
	if !exists {
		return false, fmt.Errorf("root vmArgs array was not found")
	}
	args, ok := value.([]interface{})
	if !ok {
		return false, fmt.Errorf("root vmArgs is not an array")
	}
	for _, item := range args {
		if argument, ok := item.(string); ok && argument == ZB_FLAGS {
			return false, nil
		}
	}
	root["vmArgs"] = append([]interface{}{ZB_FLAGS}, args...)
	return true, nil
}

func PatchSteamConfigs() error {
	paths := GetConfigPaths()
	if len(paths) == 0 {
		return fmt.Errorf("could not find any Steam user configuration")
	}

	patchedConfigs := 0
	for _, configPath := range paths {
		input, err := os.ReadFile(configPath)
		if err != nil {
			return fmt.Errorf("could not read %s: %w", configPath, err)
		}
		file, err := os.Open(configPath)
		if err != nil {
			return fmt.Errorf("could not open %s: %w", configPath, err)
		}
		if _, err := vdf.NewParser(file).Parse(); err != nil {
			file.Close()
			return fmt.Errorf("could not parse %s: %w", configPath, err)
		}
		file.Close()

		output, foundApp, changed, err := patchSteamConfigText(input)
		if err != nil {
			return fmt.Errorf("could not patch %s: %w", configPath, err)
		}
		if !foundApp {
			continue
		}
		patchedConfigs++

		backupPath := configPath + ".backup"
		if _, err := os.Stat(backupPath); os.IsNotExist(err) {
			inputData, err := os.ReadFile(configPath)
			if err != nil {
				return fmt.Errorf("could not read %s: %w", configPath, err)
			}
			if err := os.WriteFile(backupPath, inputData, 0644); err != nil {
				return fmt.Errorf("could not create backup %s: %w", backupPath, err)
			}
		}

		if changed {
			if err := atomicWriteFile(configPath, output, 0644); err != nil {
				return fmt.Errorf("could not update %s: %w", configPath, err)
			}
		}
	}

	if patchedConfigs == 0 {
		return fmt.Errorf("Project Zomboid app %s was not found in any Steam user configuration", PZ_APP)
	}
	return nil
}

func patchSteamConfigText(input []byte) ([]byte, bool, bool, error) {
	content := string(input)
	newline := "\n"
	if strings.Contains(content, "\r\n") {
		newline = "\r\n"
	}
	lines := strings.SplitAfter(content, newline)
	launchLines := []int{}
	stack := []string{}
	pendingKey := ""
	foundApp := false

	for i, raw := range lines {
		line := strings.TrimSuffix(strings.TrimSuffix(raw, "\n"), "\r")
		trimmed := strings.TrimSpace(line)
		if trimmed == "}" {
			if len(stack) > 0 {
				stack = stack[:len(stack)-1]
			}
			pendingKey = ""
			continue
		}
		if trimmed == "{" {
			if pendingKey != "" {
				stack = append(stack, pendingKey)
				if hasVDFPath(stack, "Software", "Valve", "Steam", "apps", PZ_APP) {
					foundApp = true
				}
				pendingKey = ""
			}
			continue
		}

		key, hasKey := vdfKeyName(trimmed)
		if !hasKey {
			pendingKey = ""
			continue
		}
		if key == "LaunchOptions" && hasVDFPath(stack, "Software", "Valve", "Steam", "apps", PZ_APP) {
			launchLines = append(launchLines, i)
		}
		if isVDFKeyOnly(trimmed, key) {
			pendingKey = key
		} else {
			pendingKey = ""
		}
	}

	if !foundApp {
		return input, false, false, nil
	}
	if len(launchLines) == 0 {
		return input, true, false, fmt.Errorf("Project Zomboid app has no LaunchOptions entry")
	}
	changed := false
	for _, lineIndex := range launchLines {
		line := strings.TrimSuffix(strings.TrimSuffix(lines[lineIndex], "\n"), "\r")
		value := vdfValue(line)
		if strings.Contains(value, ZB_FLAGS) {
			continue
		}
		desired := ZB_FLAGS
		if strings.TrimSpace(value) != "" {
			desired += " " + strings.TrimSpace(value)
		}
		indent := line[:len(line)-len(strings.TrimLeft(line, " \t"))]
		ending := strings.TrimPrefix(lines[lineIndex], line)
		lines[lineIndex] = indent + "\"LaunchOptions\"\t\t\"" + desired + "\"" + ending
		changed = true
	}
	return []byte(strings.Join(lines, "")), true, changed, nil
}

func vdfKeyName(line string) (string, bool) {
	if !strings.HasPrefix(line, `"`) {
		return "", false
	}
	end := strings.Index(line[1:], `"`)
	if end < 0 {
		return "", false
	}
	return line[1 : end+1], true
}

func isVDFKeyOnly(line, key string) bool {
	return strings.TrimSpace(line[len(key)+2:]) == ""
}

func hasVDFPath(stack []string, expected ...string) bool {
	if len(stack) < len(expected) {
		return false
	}
	offset := len(stack) - len(expected)
	for i := range expected {
		if !strings.EqualFold(stack[offset+i], expected[i]) {
			return false
		}
	}
	return true
}

func vdfValue(line string) string {
	parts := strings.Split(line, `"`)
	if len(parts) < 4 {
		return ""
	}
	return parts[3]
}

func atomicWriteFile(path string, data []byte, mode os.FileMode) error {
	temp, err := os.CreateTemp(filepath.Dir(path), filepath.Base(path)+".tmp-*")
	if err != nil {
		return err
	}
	tempPath := temp.Name()
	defer os.Remove(tempPath)
	if err := temp.Chmod(mode); err != nil {
		temp.Close()
		return err
	}
	if _, err := temp.Write(data); err != nil {
		temp.Close()
		return err
	}
	if err := temp.Sync(); err != nil {
		temp.Close()
		return err
	}
	if err := temp.Close(); err != nil {
		return err
	}
	oldName, err := windows.UTF16PtrFromString(tempPath)
	if err != nil {
		return err
	}
	newName, err := windows.UTF16PtrFromString(path)
	if err != nil {
		return err
	}
	return windows.MoveFileEx(oldName, newName, windows.MOVEFILE_REPLACE_EXISTING|windows.MOVEFILE_WRITE_THROUGH)
}

func GetConfigPaths() []string {
	if !existingDirectory(STEAM_PATH) {
		return nil
	}

	var paths []string
	userdataDir := filepath.Join(STEAM_PATH, "userdata")

	entries, err := os.ReadDir(userdataDir)
	if err != nil {
		return paths
	}

	for _, entry := range entries {
		if entry.IsDir() {
			configPath := filepath.Join(userdataDir, entry.Name(), "config", "localconfig.vdf")

			if _, err := os.Stat(configPath); err == nil {
				paths = append(paths, configPath)
			}
		}
	}

	return paths
}
