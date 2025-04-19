Syncing files to BunnyCDN
==

`bunnysync` is a self-contained binary that can place files from a local folder into a folder in a BunnyCDN Storage Zone.

I made this tool to facilitate quick and easy sync from multiple repositories/static sites that I run, into the same BunnyCDN Storage Zone. It is written in Rust to make use of the excellent [ubi](https://github.com/houseabsolute/ubi) installer, so that I can easily install it on all the machines and systems I use with [mise](https://mise.jdx.dev/).

## Features

- Checksumming to send only files that differ between source and destination
- Deleting files that are present in destination but not source
- Skip deleting in subtrees to easily facilitate many sites in different trees
- Rudimentary concurrency control by placing a lockfile in the storage zone to prevent concurrent deploys
- Dry runs and verbose output
- Concurrent requests to bunny.net API for both file listing and uploads
- Syncs html files last, so that other assets are present before they change

## Usage

`bunnysync` can authenticate using the password to a storage zone, find it in FTP & API Access in the dashboard, under your storage zone. The password can be passed in the `BUNNYSYNC_KEY` environment variable, or on the command line with `--access-key`.

Place each file from the local `~/projects/blog/public` folder into the root of the storage zone named `eugene-docs`:

```shell
bunnysync sync ~/projects/blog/public eugene-docs
```

Place each file from the local `~/projects/eugene/eugene/docs/book` folder into the eugene/ folder on the storage zone named `eugene-docs`:

```shell
bunnysync sync ~/projects/blog/public eugene-docs --path eugene
```

Place each file from the local `~/projects/blog/public` folder into the root of the storage zone named `eugene-docs` but do not delete anything under `eugene/`:

```shell
bunnysync sync ~/projects/blog/public eugene-docs --ignore eugene
```

For more, see `bunnysync --help`:

```
bunnysync is a tool for synchronizing files to bunny cdn storage zones

bunnysync can sync to subtrees of your storage zone, the entire storage zone, or selectively skip
parts of the tree. It can easily deploy a static site with a single command.

bunnysync refuses to sync if it looks like there's already an active sync job to the storage
zone. It places a lockfile into the storage zone during the sync to have rudimentary concurrency
control.

bunnysync aims to make the local_path and the path within the storage zone exactly equal. It will sync
HTML at the end, to ensure other assets like CSS are already updated by the time they sync.

Usage: bunnysync <COMMAND>

Commands:
  sync         Sync a local folder to a path within a bunny.net Storage Zone
  completions  Provide shell completions
  purge-url    Purge a URL from the bunny.net cache
  purge-zone   Purge an entire pull zone from bunny.net cache
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

```

## Development

Run tests:

```shell
cargo test
```

Format:

```shell
cargo fmt
```

Lint:
```shell
cargo clippy && cargo check
```

## Contributions & License

bunnysync is available under the MIT license, and contributions are welcome. Feel free to open an issue so we can have discussion before adding new code.

## Planned work

- Concurrent uploads ✅
- Progress indicator ✅
- Cleaning up empty folders in the target 🤔
- Add subcommand to purge pull zone ✅
- Add subcommand to purge url ✅