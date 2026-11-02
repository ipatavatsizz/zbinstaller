package main

import (
	"fmt"
	"os"
	"path/filepath"
)

func InstallZB(branch Branch) (err error) {
	updateInstallState(StatusInstalling, "Step 1/8: Locating the selected Workshop branch...")
	if !existingDirectory(STEAM_PATH) {
		updateInstallState(StatusInstallFailed, "Installation failed: Steam path was not found.")
		return fmt.Errorf("could not find the Steam installation")
	}
	if !existingDirectory(PZ_PATH) {
		updateInstallState(StatusInstallFailed, "Installation failed: Project Zomboid path was not found.")
		return fmt.Errorf("could not find the Project Zomboid installation")
	}
	snapshot, err := createInstallSnapshot(append([]string{
		filepath.Join(PZ_PATH, ZB_FILES[0]),
		filepath.Join(PZ_PATH, ZB_FILES[1]),
		filepath.Join(PZ_PATH, "ProjectZomboid64.json"),
		filepath.Join(PZ_PATH, "ProjectZomboid64.bat"),
	}, GetConfigPaths()...))
	if err != nil {
		updateInstallState(StatusInstallFailed, "Installation failed: could not prepare a rollback snapshot.")
		return fmt.Errorf("could not prepare rollback snapshot: %w", err)
	}
	defer func() {
		if err != nil {
			if rollbackErr := snapshot.restore(); rollbackErr != nil {
				err = fmt.Errorf("%w; rollback failed: %v", err, rollbackErr)
				updateInstallState(StatusInstallFailed, "Installation failed and automatic rollback was incomplete.")
			}
		}
	}()

	bpath, err := GetZBBranchPath(branch)
	if err != nil {
		updateInstallState(StatusWaitingSubscription, "ZombieBuddy is not subscribed in Steam Workshop.")
		return fmt.Errorf("could not find ZombieBuddy %s branch", branch)
	}

	// Steam must be closed before its launch configuration is modified.
	updateInstallState(StatusInstalling, "Step 2/8: Closing Steam...")
	if err := KillSteam(); err != nil {
		updateInstallState(StatusInstallFailed, "Installation failed while closing Steam.")
		return fmt.Errorf("could not close Steam: %w", err)
	}

	// Copy the mod libraries into the Project Zomboid installation directory.
	for i, file := range ZB_FILES {
		updateInstallState(StatusInstalling, fmt.Sprintf("Step %d/8: Copying %s...", i+3, file))
		src := filepath.Join(bpath, "mods", "ZombieBuddy", "libs", file)
		dst := filepath.Join(PZ_PATH, file)

		if err := CopyFile(src, dst); err != nil {
			updateInstallState(StatusInstallFailed, "Installation failed while copying "+file+".")
			return fmt.Errorf("failed to copy %s: %w", file, err)
		}
	}

	updateInstallState(StatusInstalling, "Step 5/8: Patching Steam config...")
	if err := PatchSteamConfigs(); err != nil {
		updateInstallState(StatusInstallFailed, "Installation failed while patching Steam config.")
		return fmt.Errorf("could not update Steam launch options: %w", err)
	}

	updateInstallState(StatusInstalling, "Step 6/8: Patching ProjectZomboid64.json...")
	if err := PatchProjectZomboidJson(); err != nil {
		updateInstallState(StatusInstallFailed, "Installation failed while patching ProjectZomboid64.json.")
		return fmt.Errorf("could not update the normal launcher: %w", err)
	}

	updateInstallState(StatusInstalling, "Step 7/8: Patching ProjectZomboid64.bat...")
	if err := PatchProjectZomboidBat(); err != nil {
		updateInstallState(StatusInstallFailed, "Installation failed while patching ProjectZomboid64.bat.")
		return fmt.Errorf("could not update the alternate launcher: %w", err)
	}

	updateInstallState(StatusInstallSucceeded, "Installation completed successfully.")
	return nil
}

func CopyFile(src string, dst string) error {
	data, err := os.ReadFile(src)
	if err != nil {
		return err
	}
	return atomicWriteFile(dst, data, 0644)
}

type installSnapshot struct {
	files   map[string][]byte
	missing map[string]bool
}

func createInstallSnapshot(paths []string) (*installSnapshot, error) {
	snapshot := &installSnapshot{files: make(map[string][]byte), missing: make(map[string]bool)}
	for _, path := range paths {
		if _, seen := snapshot.files[path]; seen {
			continue
		}
		data, err := os.ReadFile(path)
		if err == nil {
			snapshot.files[path] = data
			continue
		}
		if os.IsNotExist(err) {
			snapshot.missing[path] = true
			continue
		}
		return nil, fmt.Errorf("could not snapshot %s: %w", path, err)
	}
	return snapshot, nil
}

func (s *installSnapshot) restore() error {
	for path, data := range s.files {
		if err := atomicWriteFile(path, data, 0644); err != nil {
			return fmt.Errorf("could not restore %s: %w", path, err)
		}
	}
	for path := range s.missing {
		if _, err := os.Stat(path); err == nil {
			if err := os.Remove(path); err != nil {
				return fmt.Errorf("could not remove new file %s: %w", path, err)
			}
		} else if !os.IsNotExist(err) {
			return fmt.Errorf("could not inspect %s: %w", path, err)
		}
	}
	return nil
}
