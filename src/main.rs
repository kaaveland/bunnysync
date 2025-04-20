use crate::api::StorageZoneClient;
use crate::cli::{Action, Cli, SyncArgs};
use crate::planning::{Execution, SyncAction, SyncPlan, plan_execution, plan_sync};
use anyhow::{Context, anyhow};
use chrono::Local;
use clap::{CommandFactory, Parser};
use clap_complete::Shell::{Bash, Elvish, Fish, PowerShell, Zsh};
use clap_complete::generate;
use crossbeam::channel::unbounded;
use fxhash::FxHashMap;
use std::{env, fs, io, thread};

mod api;
mod cli;
mod local_path;
mod planning;

fn execute_job(
    client: &StorageZoneClient,
    job: SyncPlan,
    dry_run: bool,
    lockfile: &str,
) -> anyhow::Result<(String, &'static str)> {
    let Execution { remote, action } = plan_execution(&job, fs::read)?;

    let event = match &action {
        SyncAction::Put { .. } => "put",
        SyncAction::Ignore => "unchanged",
        SyncAction::Delete => "delete",
    };
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

    Ok((remote.to_string(), event))
}

fn execute_sync(
    verbose: bool,
    dry_run: bool,
    job: Vec<SyncPlan>,
    client: &StorageZoneClient,
    lockfile: &str,
    concurrency: usize,
) -> anyhow::Result<()> {
    let (send_work, receive_work) = unbounded();
    let (send_result, receive_result) = unbounded();
    let expected = job.len();

    thread::scope(move |scope| {
        for action in job {
            send_work.send(action)?;
        }

        for _ in 0..concurrency {
            let receive_work = receive_work.clone();
            let send_result = send_result.clone();

            scope.spawn(move || {
                while let Ok(action) = receive_work.recv() {
                    let r = execute_job(client, action, dry_run, lockfile);
                    send_result.send(r)?;
                }
                Ok::<(), anyhow::Error>(())
            });
        }

        for _ in 0..expected {
            let (remote, event) = receive_result.recv()??;
            if verbose || dry_run {
                println!("{remote}: {event}");
            }
        }

        drop(send_work);

        Ok::<_, anyhow::Error>(())
    })
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
        .or_else(|| env::var("THUMPER_KEY").ok())
        .context("No API key provided with --access-key or THUMPER_KEY")?;
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
        concurrency,
    } = args;

    let concurrency = concurrency.unwrap_or_else(num_cpus::get);

    let SyncJob {
        client,
        path,
        local_path,
    } = init_sync(access_key, local_path, path, storage_zone, endpoint)?;
    if !dry_run {
        take_lock(&client, lockfile.as_str(), force)?;
    }
    let local = local_path::files_by_remote_name(local_path.as_str(), path.as_str())?;
    let remote = client.list_files(path.as_str(), &ignore, concurrency)?;
    let job = plan_sync(&local, &remote, &ignore);
    execute_sync(
        verbose,
        dry_run,
        job,
        &client,
        lockfile.as_str(),
        concurrency,
    )?;
    if !dry_run {
        remove_lock(&client, lockfile.as_str())?;
    }
    Ok(())
}

fn use_api_key(api_key: Option<String>) -> anyhow::Result<String> {
    api_key
        .or_else(|| env::var("THUMPER_API_KEY").ok())
        .context("No API key provided with --api-key or thumper_API_KEY")
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
            generate(sh, &mut com, "thumper", &mut io::stdout());
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
