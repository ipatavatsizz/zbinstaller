//go:build !windows

package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

func projectZomboidLauncherPath(root string) string {
	for _, name := range []string{"ProjectZomboid64.sh", "ProjectZomboid64.command", "start-server.sh"} {
		path := filepath.Join(root, name)
		if info, err := os.Stat(path); err == nil && !info.IsDir() {
			return path
		}
	}
	return filepath.Join(root, "ProjectZomboid64.sh")
}

func PatchProjectZomboidBat() error {
	if PZ_PATH == "" {
		return fmt.Errorf("Project Zomboid installation path could not be found")
	}
	launcher := projectZomboidLauncherPath(PZ_PATH)
	if info, err := os.Stat(launcher); err != nil || info.IsDir() {
		launcher = ""
	}
	if launcher == "" {
		return fmt.Errorf("could not find a Linux/macOS Project Zomboid launcher under %s", PZ_PATH)
	}
	data, err := os.ReadFile(launcher)
	if err != nil {
		return fmt.Errorf("could not read %s: %w", launcher, err)
	}
	if strings.Contains(string(data), ZB_FLAGS) {
		return nil
	}
	backupPath := launcher + ".backup"
	if _, err := os.Stat(backupPath); os.IsNotExist(err) {
		if err := os.WriteFile(backupPath, data, 0755); err != nil {
			return fmt.Errorf("could not create backup %s: %w", backupPath, err)
		}
	}
	content := string(data)
	line := "export _JAVA_OPTIONS=\"" + ZB_FLAGS + " ${_JAVA_OPTIONS:-}\""
	content = line + "\n" + content
	if err := atomicWriteFile(launcher, []byte(content), 0755); err != nil {
		return fmt.Errorf("could not update %s: %w", launcher, err)
	}
	return nil
}
