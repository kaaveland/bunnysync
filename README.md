Syncing files to BunnyCDN
==

`thumper` is a self-contained binary that can place files from a local folder into a folder in a BunnyCDN Storage Zone.

I made this tool to facilitate quick and easy sync from multiple repositories/static sites that I run, into the same BunnyCDN Storage Zone. It is written in Rust to make use of the excellent [ubi](https://github.com/houseabsolute/ubi) installer, so that I can easily install it on all the machines and systems I use with [mise](https://mise.jdx.dev/).

## Features

- Checksumming to send only files that differ between source and destination
- Deleting files that are present in destination but not source
- Skip deleting in subtrees to easily facilitate many sites in different trees
- Rudimentary concurrency control by placing a lockfile in the storage zone to prevent concurrent deploys
- Dry runs and verbose output
- Concurrent requests to bunny.net API for both file listing and uploads
- Syncs html files last, so that other assets are present before they change

## Getting `thumper`

The recommended method of installing is to use [mise](https://mise.jdx.dev/):

```shell
mise use ubi:kaaveland/thumper@latest
```

This downloads the latest release for your platform from the [releases](https://github.com/kaaveland/thumper/releases) page, which you can also do manually.

There's a docker image available at [ghcr](https://ghcr.io/kaaveland/thumper).

It is also possible to install `thumper` with `cargo install thumper`.

## Usage

The [documentation](https://kaveland.no/thumper/) has a guide for configuring a storage zone and setting up GitHub Workflows to deploy a static site.

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

thumper is available under the MIT license, and contributions are welcome. Feel free to open an issue so we can have discussion before adding new code.

## Planned work

- Concurrent uploads ✅
- Progress indicator ✅
- Cleaning up empty folders in the target 🤔
- Add subcommand to purge pull zone ✅
- Add subcommand to purge url ✅
- Keyring integration 🤔
