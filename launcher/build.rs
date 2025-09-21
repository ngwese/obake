use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell cargo to rerun this build script if the git repository changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");

    let version = get_version();

    // Write the version to a file that can be included at compile time
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("version.rs");
    fs::write(
        &dest_path,
        format!("pub const VERSION: &str = \"{}\";", version),
    )
    .expect("Failed to write version file");
}

fn get_version() -> String {
    let repo = match git2::Repository::discover(".") {
        Ok(repo) => repo,
        Err(_) => {
            // If we can't open the git repo, return a fallback version
            return "unknown".to_string();
        }
    };

    // Check if the working directory is dirty
    let is_dirty = is_working_dir_dirty(&repo);

    // Get the current commit
    let head = match repo.head() {
        Ok(head) => head,
        Err(_) => return "unknown".to_string(),
    };

    let commit = match head.peel_to_commit() {
        Ok(commit) => commit,
        Err(_) => return "unknown".to_string(),
    };

    let short_sha = &commit.id().to_string()[..8];

    // Try to find the most recent tag
    let mut tags = Vec::new();
    let _ = repo.tag_foreach(|_id, name| {
        if let Ok(name) = std::str::from_utf8(name) {
            if name.starts_with("refs/tags/") {
                let tag_name = &name[10..]; // Remove "refs/tags/" prefix
                if let Ok(tag_ref) = repo.find_reference(name) {
                    if let Ok(tag_commit) = tag_ref.peel_to_commit() {
                        tags.push((tag_name.to_string(), tag_commit.id()));
                    }
                }
            }
        }
        true
    });

    // Find the most recent tag that points to the current commit or an ancestor
    let mut best_tag = None;
    for (tag_name, tag_commit_id) in tags {
        if tag_commit_id == commit.id() {
            // Current commit is tagged
            best_tag = Some(tag_name);
            break;
        } else if let Ok(merge_base) = repo.merge_base(commit.id(), tag_commit_id) {
            if merge_base == tag_commit_id {
                // Tag points to an ancestor of current commit
                if best_tag.is_none() {
                    best_tag = Some(tag_name);
                }
            }
        }
    }

    match best_tag {
        Some(tag) => {
            if is_dirty {
                format!("{}-dirty", tag)
            } else {
                tag
            }
        }
        None => {
            if is_dirty {
                format!("{}-dirty", short_sha)
            } else {
                short_sha.to_string()
            }
        }
    }
}

fn is_working_dir_dirty(repo: &git2::Repository) -> bool {
    // Check if there are any uncommitted changes
    if let Ok(index) = repo.index() {
        if let Ok(diff) = repo.diff_index_to_workdir(Some(&index), None) {
            return diff.deltas().len() > 0;
        }
    }
    false
}
