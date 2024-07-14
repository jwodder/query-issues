use anyhow::{bail, Context};
use std::io::ErrorKind;
use std::path::Path;
use std::process::{Command, Stdio};

fn main() -> anyhow::Result<()> {
    if let Some(work_tree) = get_work_tree()? {
        println!("cargo::rerun-if-changed={work_tree}/.git/HEAD");
        println!("cargo::rerun-if-changed={work_tree}/.git/refs");
        if let Some(commit) = get_commit_hash(&work_tree)? {
            println!("cargo::rustc-env=GIT_COMMIT={commit}");
        }
    } else {
        println!("cargo::rerun-if-changed=build.rs");
    }
    Ok(())
}

fn get_work_tree() -> anyhow::Result<Option<String>> {
    match Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .stderr(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => {
            let mut work_tree = String::from_utf8(output.stdout)
                .context("`git rev-parse --show-toplevel` output was not UTF-8")?;
            if work_tree.ends_with('\n') {
                work_tree.pop();
                #[cfg(windows)]
                if work_tree.ends_with('\r') {
                    // Although Git on Windows (at least under GitHub Actions)
                    // seems to use LF as the newline sequence in its output,
                    // we should still take care to strip final CR on Windows
                    // if it ever shows up.  As Windows doesn't allow CR in
                    // file names, a CR here will always be part of a line
                    // ending.
                    work_tree.pop();
                }
            }
            Ok(Some(work_tree))
        }
        Ok(_) => Ok(None), // We are not in a Git repository
        Err(e) if e.kind() == ErrorKind::NotFound => {
            // Git doesn't seem to be installed, so assume we're not in a Git
            // repository
            Ok(None)
        }
        Err(e) => Err(e).context("failed to run `git rev-parse --show-toplevel`"),
    }
}

fn get_commit_hash<P: AsRef<Path>>(work_tree: P) -> anyhow::Result<Option<String>> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--short")
        .arg("HEAD")
        .current_dir(work_tree.as_ref())
        .output()
        .context("failed to run `git rev-parse --short HEAD`")?;
    if !output.status.success() {
        bail!(
            "`git rev-parse --short HEAD` command was not successful: {}",
            output.status
        );
    }
    let revision = std::str::from_utf8(&output.stdout)
        .context("`git rev-parse --short HEAD` output was not UTF-8")?
        .trim()
        .to_owned();
    Ok(Some(revision))
}
