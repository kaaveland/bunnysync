use anyhow::anyhow;
use fxhash::FxHashMap;
use reqwest::blocking::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FileInfo {
    pub path: String,
    pub object_name: String,
    pub checksum: Option<String>,
    pub is_directory: bool,
}

#[derive(Debug)]
pub struct FileMeta {
    pub checksum: Option<[u8; 32]>,
}

pub struct ApiClient<'a> {
    client: Client,
    access_key: &'a str,
    endpoint: &'a str,
    storage_zone: &'a str,
    dry_run: bool,
    verbose: bool,
}

impl<'a> ApiClient<'a> {
    pub fn new(
        access_key: &'a str,
        endpoint: &'a str,
        storage_zone: &'a str,
        dry_run: bool,
        verbose: bool,
    ) -> Self {
        ApiClient {
            client: Client::new(),
            access_key,
            endpoint,
            storage_zone,
            dry_run,
            verbose,
        }
    }

    pub fn read_file(&self, path: &str) -> anyhow::Result<String> {
        let response = self
            .client
            .get(self.url_for(path))
            .header("AccessKey", self.access_key)
            .send()?;
        if response.status().is_success() {
            Ok(response.text()?)
        } else {
            Err(anyhow!("Unable to read: {:?}", response.status()))
        }
    }

    fn url_for(&self, path: &str) -> String {
        format!("https://{}/{}/{path}", self.endpoint, self.storage_zone)
    }

    fn discover_files(&self, path: &str) -> anyhow::Result<Vec<FileInfo>> {
        let response = self
            .client
            .get(self.url_for(path))
            .header("AccessKey", self.access_key)
            .send()?;
        let mut files: Vec<FileInfo> = response.json()?;
        let mut extra = vec![];
        for dir in files
            .iter()
            .filter(|fi| fi.is_directory)
            .collect::<Vec<_>>()
        {
            extra.extend(
                self.discover_files(
                    format!("{path}{}/", dir.object_name)
                        .as_str()
                        .trim_start_matches("/"),
                )?,
            );
        }
        files.extend(extra);
        files.retain(|fi| !fi.is_directory);
        Ok(files)
    }

    pub fn list_files(&self, path: &str) -> anyhow::Result<FxHashMap<String, FileMeta>> {
        let files = self.discover_files(path)?;
        let mut files_by_name = FxHashMap::default();
        let trim_prefix = format!("/{}/", self.storage_zone);
        for fi in files {
            let checksum = fi
                .checksum
                .map(|hex_checksum| {
                    let mut checksum = [0; 32];
                    hex::decode_to_slice(hex_checksum.as_bytes(), &mut checksum)?;
                    Ok::<[u8; 32], anyhow::Error>(checksum)
                })
                .transpose()?;
            files_by_name.insert(
                format!(
                    "{}{}",
                    fi.path.trim_start_matches(trim_prefix.as_str()),
                    fi.object_name
                ),
                FileMeta { checksum },
            );
        }
        Ok(files_by_name)
    }

    pub fn put_file(
        &'a self,
        path: &str,
        body: Vec<u8>,
        content_type: Option<&str>,
    ) -> anyhow::Result<()> {
        if self.verbose || self.dry_run {
            println!("{path}: put");
        }

        if !self.dry_run {
            let url = self.url_for(path);

            let response = self
                .client
                .put(url)
                .header("AccessKey", self.access_key)
                .header(
                    "Content-Type",
                    content_type.unwrap_or("application/octet-stream"),
                )
                .body(body)
                .send()?;

            if response.status().is_success() {
                Ok(())
            } else {
                Err(anyhow!("Request failed: {:?}", response.status()))
            }
        } else {
            Ok(())
        }
    }

    pub fn delete_file(&self, path: &str) -> anyhow::Result<()> {
        if self.verbose || self.dry_run {
            println!("{path}: delete");
        }

        if !self.dry_run {
            let response = self
                .client
                .delete(self.url_for(path))
                .header("AccessKey", self.access_key)
                .send()?;
            Ok(response.error_for_status().map(|_| ())?)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EX: &str = "{
    \"StorageZoneName\": \"eugene-docs\",
    \"Path\": \"/eugene-docs/\",
    \"ObjectName\": \"404.html\",
    \"Length\": 9665,
    \"LastChanged\": \"2025-04-15T16:52:33.824\",
    \"ArrayNumber\": 2,
    \"IsDirectory\": false,
    \"ContentType\": \"\",
    \"DateCreated\": \"2025-04-15T16:52:33.824\",
    \"Checksum\": \"FD9495967478FCD8B9FB08F70EAF2806BD50F4AB2261BE16A9BEAA542C37A441\",
    \"ReplicatedZones\": \"SE,UK,LA,SG,BR,NY\"
  }";

    #[test]
    fn test_parse() {
        let _: FileInfo = serde_json::from_str(EX).unwrap();
    }
}
