//go:build windows

package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

func projectZomboidLauncherPath(root string) string {
	return filepath.Join(root, "ProjectZomboid64.bat")
}

func PatchProjectZomboidBat() error {
	if PZ_PATH == "" {
		return fmt.Errorf("Project Zomboid installation path could not be found")
	}
	batPath := filepath.Join(PZ_PATH, "ProjectZomboid64.bat")
	data, err := os.ReadFile(batPath)
	if err != nil {
		return fmt.Errorf("could not read %s: %w", batPath, err)
	}
	backupPath := batPath + ".backup"
	if _, err := os.Stat(backupPath); os.IsNotExist(err) {
		if err := os.WriteFile(backupPath, data, 0644); err != nil {
			return fmt.Errorf("could not create backup %s: %w", backupPath, err)
		}
	}
	newline := "\n"
	if strings.Contains(string(data), "\r\n") {
		newline = "\r\n"
	}
	lines := strings.Split(strings.ReplaceAll(string(data), "\r\n", "\n"), "\n")
	modified := false
	found := false
	for i, line := range lines {
		if strings.HasPrefix(strings.ToUpper(strings.TrimSpace(line)), "SET _JAVA_OPTIONS=") {
			found = true
			parts := strings.SplitN(line, "=", 2)
			if len(parts) != 2 {
				return fmt.Errorf("invalid _JAVA_OPTIONS assignment in %s at line %d", batPath, i+1)
			}
			if !strings.Contains(parts[1], ZB_FLAGS) {
				lines[i] = parts[0] + "=" + ZB_FLAGS + " " + strings.TrimSpace(parts[1])
				modified = true
			}
		}
	}
	if !found {
		lines = append([]string{"SET _JAVA_OPTIONS=" + ZB_FLAGS}, lines...)
		modified = true
	}
	if modified {
		if err := atomicWriteFile(batPath, []byte(strings.Join(lines, newline)), 0644); err != nil {
			return fmt.Errorf("could not update %s: %w", batPath, err)
		}
	}
	return nil
}
