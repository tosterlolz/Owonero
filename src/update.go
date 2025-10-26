package main

import (
	"archive/zip"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"time"
)

type GitHubRelease struct {
	TagName string        `json:"tag_name"`
	Assets  []GitHubAsset `json:"assets"`
}

type GitHubAsset struct {
	Name               string `json:"name"`
	BrowserDownloadURL string `json:"browser_download_url"`
}

func checkForUpdates() {
	// Create HTTP client with timeout
	client := &http.Client{
		Timeout: 10 * time.Second,
	}

	resp, err := client.Get("https://api.github.com/repos/tosterlolz/Owonero/releases/latest")
	if err != nil {
		fmt.Printf("\033[33mFailed to check for updates: %v\033[0m\n", err)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		fmt.Printf("\033[33mUpdate check failed: HTTP %d\033[0m\n", resp.StatusCode)
		return
	}

	var release GitHubRelease
	if err := json.NewDecoder(resp.Body).Decode(&release); err != nil {
		fmt.Printf("\033[33mFailed to parse update info: %v\033[0m\n", err)
		return
	}

	latestVer := strings.TrimPrefix(release.TagName, "v")
	if latestVer == ver {
		fmt.Printf("\033[32mYou are running the latest version (%s)\033[0m\n", ver)
		return
	}

	// Check if latest version is actually newer
	if isVersionNewer(latestVer, ver) {
		fmt.Printf("\033[33mNew version available: %s (current: %s)\033[0m\n", latestVer, ver)
		fmt.Printf("\033[36mDownloading update...\033[0m\n")
		downloadAndInstallUpdate(client, release)
	} else {
		fmt.Printf("\033[32mYou are running the latest version (%s)\033[0m\n", ver)
	}
}

func isVersionNewer(latest, current string) bool {
	// Simple version comparison (assumes semantic versioning)
	latestParts := strings.Split(latest, ".")
	currentParts := strings.Split(current, ".")

	for i := 0; i < len(latestParts) && i < len(currentParts); i++ {
		latestNum, err1 := strconv.Atoi(latestParts[i])
		currentNum, err2 := strconv.Atoi(currentParts[i])
		if err1 != nil || err2 != nil {
			return false
		}
		if latestNum > currentNum {
			return true
		}
		if latestNum < currentNum {
			return false
		}
	}
	return len(latestParts) > len(currentParts)
}

func downloadAndInstallUpdate(client *http.Client, release GitHubRelease) {
	// Determine asset name
	osName := runtime.GOOS
	arch := runtime.GOARCH
	var assetName string
	if osName == "windows" {
		assetName = fmt.Sprintf("owonero-%s-%s.zip", osName, arch)
	} else {
		assetName = fmt.Sprintf("owonero-%s-%s.zip", osName, arch)
	}

	var downloadURL string
	for _, asset := range release.Assets {
		if asset.Name == assetName {
			downloadURL = asset.BrowserDownloadURL
			break
		}
	}

	if downloadURL == "" {
		fmt.Printf("\033[31mNo suitable update found for %s/%s\033[0m\n", osName, arch)
		return
	}

	// Download the update
	resp, err := client.Get(downloadURL)
	if err != nil {
		fmt.Printf("\033[31mFailed to download update: %v\033[0m\n", err)
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		fmt.Printf("\033[31mDownload failed: HTTP %d\033[0m\n", resp.StatusCode)
		return
	}

	// Get current executable path
	execPath, err := os.Executable()
	if err != nil {
		fmt.Printf("\033[31mFailed to get executable path: %v\033[0m\n", err)
		return
	}

	// Create backup
	backupPath := execPath + ".backup"
	if err := os.Rename(execPath, backupPath); err != nil {
		fmt.Printf("\033[31mFailed to create backup: %v\033[0m\n", err)
		return
	}

	// Download to temp zip file first
	tempZipPath := execPath + ".tmp.zip"
	out, err := os.Create(tempZipPath)
	if err != nil {
		fmt.Printf("\033[31mFailed to create temp zip file: %v\033[0m\n", err)
		os.Rename(backupPath, execPath) // restore
		return
	}
	defer out.Close()

	if _, err := io.Copy(out, resp.Body); err != nil {
		fmt.Printf("\033[31mFailed to write update zip: %v\033[0m\n", err)
		os.Remove(tempZipPath)
		os.Rename(backupPath, execPath) // restore
		return
	}
	out.Close()

	// Extract the zip file
	fmt.Printf("\033[36mExtracting update...\033[0m\n")
	if err := extractZip(tempZipPath, filepath.Dir(execPath)); err != nil {
		fmt.Printf("\033[31mFailed to extract update: %v\033[0m\n", err)
		os.Remove(tempZipPath)
		os.Rename(backupPath, execPath) // restore
		return
	}

	// Clean up zip file
	os.Remove(tempZipPath)

	// Make executable on Unix
	if osName != "windows" {
		if err := os.Chmod(execPath, 0755); err != nil {
			fmt.Printf("\033[31mFailed to make executable: %v\033[0m\n", err)
			os.Rename(backupPath, execPath) // restore
			return
		}
	}

	// Clean up backup
	os.Remove(backupPath)

	fmt.Printf("\033[32mUpdate installed successfully! Please restart the application.\033[0m\n")
	os.Exit(0)
}

func extractZip(zipPath, destDir string) error {
	r, err := zip.OpenReader(zipPath)
	if err != nil {
		return err
	}
	defer r.Close()

	for _, f := range r.File {
		fpath := filepath.Join(destDir, f.Name)
		if !strings.HasPrefix(fpath, filepath.Clean(destDir)+string(os.PathSeparator)) {
			return fmt.Errorf("illegal file path: %s", fpath)
		}

		if f.FileInfo().IsDir() {
			os.MkdirAll(fpath, os.ModePerm)
			continue
		}

		if err = os.MkdirAll(filepath.Dir(fpath), os.ModePerm); err != nil {
			return err
		}

		outFile, err := os.OpenFile(fpath, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, f.Mode())
		if err != nil {
			return err
		}

		rc, err := f.Open()
		if err != nil {
			outFile.Close()
			return err
		}

		_, err = io.Copy(outFile, rc)
		outFile.Close()
		rc.Close()

		if err != nil {
			return err
		}
	}
	return nil
}