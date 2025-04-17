use crate::api::StorageZoneClient;
use crate::cli::{Action, Cli, SyncArgs};
use crate::planning::{Execution, SyncAction, SyncPlan, plan_execution, plan_sync};
use anyhow::{Context, anyhow};
use chrono::Local;
use clap::{CommandFactory, Parser};
use clap_complete::Shell::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_complete::generate;
use fxhash::FxHashMap;
use std::{env, fs, io};

mod api;
mod cli;
mod local_path;
mod planning;

fn execute_sync(
    verbose: bool,
    dry_run: bool,
    job: &[SyncPlan],
    client: &StorageZoneClient,
    lockfile: &str
) -> anyhow::Result<()> {
    // TODO: Consider introducing concurrency in this loop, the rate limit is generous
    for action in job {
        let Execution { remote, action } = plan_execution(action, fs::read)?;
        if verbose || dry_run {
            let event = match &action {
                SyncAction::Put { .. } => "put",
                SyncAction::Ignore => "unchanged",
                SyncAction::Delete => "delete",
            };
            println!("{remote}: {event}");
        }
        if !dry_run {
            match action {
                SyncAction::Put { content, mime_type } => {
                    client.put_file(remote, content, mime_type)?;
                }
                SyncAction::Delete if remote != lockfile => {
                    client.delete_file(remote)?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn take_lock(client: &StorageZoneClient, lockfile: &str, force: bool) -> anyhow::Result<()> {
    if let Ok(sync_time) = client.read_file(lockfile) {
        eprintln!("WARNING: Remote is locked since {sync_time}");
        if !force {
            return Err(anyhow!("Dangling lock in {lockfile} prevents sync"));
        }
    }
    let now = Local::now();
    let ts = now.to_rfc3339();
    client.put_file(lockfile, ts.bytes().collect(), Some("text/plain"))
}

fn remove_lock(client: &StorageZoneClient, lockfile: &str) -> anyhow::Result<()> {
    client.delete_file(lockfile)
}

struct SyncJob {
    client: StorageZoneClient,
    path: String,
    local_path: String,
}

fn normalize_path(mut path: String) -> String {
    if path.ends_with("/") {
        path
    } else {
        path.push('/');
        path
    }
}

fn init_sync(
    access_key: Option<String>,
    local_path: String,
    path: String,
    storage_zone: String,
    endpoint: String,
) -> anyhow::Result<SyncJob> {
    let access_key = access_key
        .or_else(|| env::var("BUNNYSYNC_KEY").ok())
        .context("No API key provided with --access-key or BUNNYSYNC_KEY")?;
    let client = StorageZoneClient::new(access_key, endpoint, storage_zone);

    Ok(SyncJob {
        client,
        path: normalize_path(path),
        local_path: normalize_path(local_path),
    })
}

fn do_sync(args: SyncArgs) -> anyhow::Result<()> {
    let SyncArgs {
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

    let SyncJob {
        client,
        path,
        local_path,
    } = init_sync(access_key, local_path, path, storage_zone, endpoint)?;
    if !dry_run {
        take_lock(&client, lockfile.as_str(), force)?;
    }
    let local = local_path::files_by_remote_name(local_path.as_str(), path.as_str())?;
    // TODO: Consider introducing concurrency in this, it seems listing files is slow
    let remote = client.list_files(path.as_str(), &ignore)?;
    let job = plan_sync(&local, &remote, &ignore);
    execute_sync(verbose, dry_run, &job, &client, lockfile.as_str())?;
    if !dry_run {
        remove_lock(&client, lockfile.as_str())?;
    }
    Ok(())
}

fn use_api_key(api_key: Option<String>) -> anyhow::Result<String> {
    api_key
        .or_else(|| env::var("BUNNYSYNC_API_KEY").ok())
        .context("No API key provided with --api-key or BUNNYSYNC_API_KEY")
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    match args.command {
        Action::Sync { args } => do_sync(args),
        Action::Completions { shell } => {
            let sh = match shell.as_str() {
                "bash" => Ok(Bash),
                "zsh" => Ok(Zsh),
                "fish" => Ok(Fish),
                "pwsh" | "powershell" => Ok(PowerShell),
                "elvish" => Ok(Elvish),
                _ => Err(anyhow!("Unsupported shell: {shell}")),
            }?;
            let mut com = Cli::command();
            generate(sh, &mut com, "bunnysync", &mut io::stdout());
            Ok(())
        }
        Action::PurgeUrl { url, api_key } => {
            let key = use_api_key(api_key)?;
            let client = reqwest::blocking::Client::new();
            let encoded = urlencoding::encode(url.as_str());
            let response = client
                .post("https://api.bunny.net/purge")
                .query(&[("url", encoded.as_ref())])
                .header("AccessKey", key.as_str())
                .send()?;
            Ok(response
                .error_for_status()
                .map(|_| println!("Purged {url}"))?)
        }
        Action::PurgeZone {
            pullzone,
            api_key,
            cache_tag,
        } => {
            let key = use_api_key(api_key)?;
            let client = reqwest::blocking::Client::new();
            let request = client
                .post(format!(
                    "https://api.bunny.net/pullzone/{pullzone}/purgeCache"
                ))
                .header("AccessKey", key);
            let response = if let Some(tag) = cache_tag {
                let mut form = FxHashMap::default();
                form.insert("CacheTag", tag);
                request.form(&form).send()
            } else {
                request.send()
            }?;
            Ok(response
                .error_for_status()
                .map(|_| println!("Purged {pullzone}"))?)
        }
    }
}
