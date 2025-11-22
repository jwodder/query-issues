fn main() -> anyhow::Result<()> {
    if let Some(work_tree) = githash::get_work_tree()? {
        println!("cargo::rerun-if-changed={work_tree}/.git/HEAD");
        println!("cargo::rerun-if-changed={work_tree}/.git/refs");
        if let Some(commit) = githash::get_commit_hash(&work_tree)? {
            println!("cargo::rustc-env=GIT_COMMIT={commit}");
        }
    } else {
        println!("cargo::rerun-if-changed=build.rs");
    }
    Ok(())
}
