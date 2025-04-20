# Introduction

`thumper` is a self-contained tool for deploying static assets to [BunnyCDN](https://bunny.net/) storage zones. It
is straightforward to integrate into CI/CD and deploys static web pages quickly.

## Features

- Syncs only files that have changed locally
- Delete files remotely that are missing locally
- Concurrency control to prevent concurrent deploys
- Concurrent file discovery and upload for quick deployments
- Syncs html files last
- Dry runs and verbose output
- Auto-complete

## Getting `thumper`

The recommended method of installing is to use [mise](https://mise.jdx.dev/):

```shell
mise use ubi:kaaveland/thumper@latest
```

This downloads the latest release for your platform from the [releases](https://github.com/kaaveland/thumper/releases) page, which you can also do manually.

It is also possible to install `thumper` with `cargo install thumper`.

## Usage

`thumper` needs to authenticate to `BunnyCDN` in order to work. The recommended approach is to make two environment variables available:

- `THUMPER_KEY` - this is the password to a `BunnyCDN` storage zone.
- `THUMPER_API_KEY` - this is the api key to `BunnyCDN`. It is only necessary if you want `thumper` to purge your pull zone. 

Make sure to keep these values secret, an attacker could do a lot of damage with them.

### `thumper`
```shell
{{#include help}}
```

### `thumper sync`
```shell
{{#include synchelp}}
```
