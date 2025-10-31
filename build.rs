use std::fs;
use std::path::Path;
use std::process::Command;

fn read_git_head_ref() -> Option<String> {
    // Try to read .git/HEAD and resolve the ref
    let git_head = Path::new(".git/HEAD");
    if !git_head.exists() {
        return None;
    }

    let head = fs::read_to_string(git_head).ok()?;
    let head = head.trim();
    if head.starts_with("ref: ") {
        let ref_path = head.trim_start_matches("ref: ").trim();
        let ref_file = Path::new(".git").join(ref_path);
        if ref_file.exists() {
            if let Ok(hash) = fs::read_to_string(ref_file) {
                return Some(hash.trim().to_string());
            }
        }

        // Fallback: try packed-refs (simple scan)
        let packed = Path::new(".git/packed-refs");
        if packed.exists() {
            if let Ok(lines) = fs::read_to_string(packed) {
                for line in lines.lines() {
                    if line.starts_with('#') || line.trim().is_empty() {
                        continue;
                    }
                    if let Some((hash, refname)) = line.split_once(' ') {
                        if refname.trim() == ref_path {
                            return Some(hash.trim().to_string());
                        }
                    }
                }
            }
        }
    } else {
        // HEAD contains raw commit hash (detached HEAD)
        return Some(head.to_string());
    }

    None
}

fn main() {
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(o.stdout)
            } else {
                None
            }
        })
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            // Try to read full hash from .git and shorten to 7 chars
            read_git_head_ref().and_then(|full| {
                if full.len() >= 7 {
                    Some(full[..7].to_string())
                } else {
                    Some(full)
                }
            })
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_HASH_SHORT={}", git_hash);
}
