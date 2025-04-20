# Deploying a static site with GitHub Actions

## Setting up BunnyCDN storage zone

From the [Storage](https://dash.bunny.net/storage) page at BunnyCDN, click Add Storage Zone. Give your storage zone a memorable name and choose a tier and main storage region. If you choose a different region than "Europe (Falkenstein)", you will need to provide `--endpoint` to `thumper sync` later. Set up your desired amount of Geo Replication and click "Add Storage Zone."

Now you should have an empty Storage Zone. Navigate to the "FTP & API Access" pane. The Hostname that is listed corresponds to the `--endpoint` parameter to `thumper sync`. The password corresponds to the `THUMPER_KEY` environment variable or `--access-key` command line parameter. Put it somewhere safe, like a GitHub Actions secret, available under https://github.com/YOURNAME/YOURPROJECT/settings/secrets/actions.

Optionally, if you want to purge the cache when you deploy a new version of your site, you will need your BunnyCDN API key. You will find it under your [profile page](https://dash.bunny.net/account/api-key). Create a secret for that too. This is `THUMPER_API_KEY`. This makes for 2 secrets:

- `THUMPER_KEY` is the password for your storage zone.
- `THUMPER_API_KEY` is the bunny.net API key for your account.

## Setting up a GitHub Workflow

This documentation site is deployed to a BunnyCDN storage site with a GitHub workflow. The workflow looks like this:

```yaml
{{#include ../../.github/workflows/deploy_docs.yml}}
```

The first few steps in the `deploy_docs` job are all about producing the static site files. The repository is cloned. Then we use `jdx/mise-action` to install `mdbook` which builds the static site, and `thumper` from `mise.toml` at the root of the repository:

```toml
{{#include ../../mise.toml}}
```

The `thumper sync` command uses `--path thumper` to place the static site at `thumper/` in the storage zone. This is not necessary if you want to sync your site to the root of the storage zone instead, but this documentation site shares the domain with a few others. You may have to provide `--endpoint` to `thumper sync`. 

Purging the cache with `thumper purge-zone` is optional. You can force a cache-refresh immediately with this approach, making your new content available faster. This enables you to set a very high Cache Expiration Time. You can also use `thumper purge-url` with a `*` wildcard at the end of your URL, to purge only parts of your page.

## Setting up a BunnyCDN Pull Zone

Once you've verified that you can sync to your storage zone, you can configure a BunnyCDN pull zone to make your content available to the world. Here's the [official guide](https://support.bunny.net/hc/en-us/articles/8561433879964-How-to-access-and-deliver-files-from-Bunny-Storage).