use crate::api::FileMeta;
use anyhow::{Context, anyhow};
use chrono::Local;
use clap::Parser;
use fxhash::{FxHashMap, FxHashSet};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::{env, fs};

mod api;
mod local_path;

#[derive(Parser)]
#[command(name = "bunnysync")]
#[command(arg_required_else_help = true)]
#[command(about = "Sync your files to bunny cdn storage zone")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(
    long_about = "bunnysync is a tool for synchronizing files to bunny cdn storage zones

bunnysync can sync to subtrees of your storage zone, the entire storage zone, or selectively skip
parts of the tree. It can easily deploy a static site with a single command.

bunnysync refuses to sync if it looks like there's already an active sync job to the storage
zone. It places a lockfile into the storage zone during the sync to have rudimentary concurrency
control.

bunnysync aims to make the local_path and the path within the storage zone exactly equal. It will sync
HTML at the end, to ensure other assets like CSS are already updated by the time they sync."
)]
struct Args {
    /// Which bunny cdn endpoint to use
    #[arg(short, long, default_value = "storage.bunnycdn.com")]
    endpoint: String,
    /// Password for the storage zone - looked up in environment variable BUNNYSYNC_KEY if not present
    #[arg(short, long)]
    access_key: Option<String>,
    /// Local directory to put in the storage zone
    #[arg(name = "local_path", required = true, num_args = 1)]
    local_path: String,
    /// Which storage zone to sync to
    #[arg(name = "storage_zone", required = true, num_args = 1)]
    storage_zone: String,
    /// Path inside the storage zone to sync to, path to a directory
    #[arg(short, long, default_value = "/")]
    path: String,
    /// Don't actually sync, just show what would change
    #[arg(long, default_value_t = false)]
    dry_run: bool,
    /// Force a sync despite a hanging lock file
    #[arg(short, long, default_value_t = false)]
    force: bool,
    /// Filename to use for the lockfile. bunnysync will not sync if this file exists in the destination.
    #[arg(long, default_value = ".bunnysync.lock")]
    lockfile: String,
    /// Do not delete anything in the storage zone paths that start with this prefix (can pass multiple times)
    #[arg(short, long)]
    ignore: Vec<String>,
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn must_remove<'a>(
    local_files: &'a FxHashMap<String, PathBuf>,
    remote_files: &'a FxHashMap<String, FileMeta>,
    ignored_prefix: &[String],
) -> FxHashSet<&'a str> {
    remote_files
        .keys()
        .filter(|p| !local_files.contains_key(p.as_str()))
        .filter(|p| !ignored_prefix.iter().any(|prefix| p.starts_with(prefix)))
        .map(|s| s.as_str())
        .collect()
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let Args {
        endpoint,
        access_key,
        local_path,
        storage_zone,
        path,
        dry_run,
        force,
        lockfile,
        ignore,
        verbose,
    } = args;

    let access_key = access_key
        .or_else(|| env::var("BUNNYSYNC_KEY").ok())
        .context("No API key provided with --access-key or BUNNYSYNC_KEY")?;

    let local_path = if local_path.ends_with("/") {
        local_path
    } else {
        format!("{local_path}/")
    };
    let path = if path.ends_with("/") {
        path
    } else {
        format!("{path}/")
    };
    let client = api::ApiClient::new(
        access_key.as_str(),
        endpoint.as_str(),
        storage_zone.as_str(),
        dry_run,
        verbose,
    );
    let remote_content = client.list_files(path.as_str())?;

    if let Ok(sync_time) = client.read_file(lockfile.as_str()) {
        eprintln!("WARNING: Remote is locked since {sync_time}");
        if !force {
            return Err(anyhow!("Dangling lock in {lockfile} prevents sync"));
        }
    }

    let now = Local::now();
    let ts = now.to_rfc3339();
    client.put_file(lockfile.as_str(), ts.bytes().collect(), Some("text/plain"))?;

    let local = local_path::files_by_remote_name(local_path.as_str(), path.as_str())?;
    let remove = must_remove(&local, &remote_content, &ignore);
    let mut remote_paths_ordered: Vec<_> = local.keys().map(|path| path.as_str()).collect();

    remote_paths_ordered.sort_by_key(|path| path.ends_with(".html") || path.ends_with(".htm"));
    // TODO: Consider using async client for this loop, the concurrency limits on the API are generous
    for remote_path in remote_paths_ordered {
        if let Some(physical_path) = local.get(remote_path) {
            let content = fs::read(physical_path)?;
            let mime_type = infer::get_from_path(physical_path)?.map(|t| t.mime_type());
            let digest = Sha256::digest(&content);
            if let Some(on_remote) = remote_content.get(remote_path) {
                if Some(digest.as_slice()) != on_remote.checksum.as_ref().map(|arr| arr.as_slice())
                {
                    client.put_file(remote_path, content, mime_type)?;
                } else if dry_run || verbose {
                    println!("{remote_path}: unchanged");
                }
            } else {
                client.put_file(remote_path, content, mime_type)?;
            }
        }
    }
    for remote_path in remove {
        client.delete_file(remote_path)?;
    }

    client.delete_file(lockfile.as_str())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_all_but_ignored_on_empty_local() {
        let local = FxHashMap::default();
        let mut remote = FxHashMap::default();
        remote.insert(
            "eugene/index.html".to_string(),
            FileMeta {
                checksum: Some([0; 32]),
            },
        );
        remote.insert(
            "blog/index.html".to_string(),
            FileMeta {
                checksum: Some([0; 32]),
            },
        );
        let remove = must_remove(&local, &remote, &["eugene".to_string()]);
        assert_eq!(remove.len(), 1);
        assert!(remove.contains("blog/index.html"));
    }

    #[test]
    fn remove_nothing_when_equal() {
        let mut remote = FxHashMap::default();
        remote.insert(
            "eugene/index.html".to_string(),
            FileMeta {
                checksum: Some([0; 32]),
            },
        );
        remote.insert(
            "blog/index.html".to_string(),
            FileMeta {
                checksum: Some([0; 32]),
            },
        );
        let mut local = FxHashMap::default();
        local.insert("eugene/index.html".to_string(), PathBuf::new());
        local.insert("blog/index.html".to_string(), PathBuf::new());
        assert!(must_remove(&local, &remote, &[]).is_empty());
    }

    #[test]
    fn remove_all_when_not_ignored() {
        let mut remote = FxHashMap::default();
        remote.insert(
            "eugene/index.html".to_string(),
            FileMeta {
                checksum: Some([0; 32]),
            },
        );
        remote.insert(
            "blog/index.html".to_string(),
            FileMeta {
                checksum: Some([0; 32]),
            },
        );
        let local = FxHashMap::default();
        assert_eq!(must_remove(&local, &remote, &[]).len(), 2);
    }
}
