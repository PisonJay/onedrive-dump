use base64::Engine;
use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use futures::{future::BoxFuture, FutureExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;

type MyResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(clap::Parser)]
struct Args {
    #[arg(short, long)]
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileInfo {
    hashes: HashInfo,

    #[serde(rename = "mimeType")]
    mime_type: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct HashInfo {
    #[serde(rename = "sha1Hash")]
    sha1_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DriveItemResp {
    id: String,
    name: String,
    folder: Option<Value>,
    children: Option<Vec<DriveItemResp>>,
    file: Option<FileInfo>,
    // #[serde(rename = "@content.downloadUrl")]
    // download_url: Option<String>,
}

fn print_aria2_entry<'a>(base_url: &'a str, prefix: &'a str, entry: &'a DriveItemResp) {
    let url = format!("{}/items/{}/content", base_url, &entry.id);
    let path = format!("{}{}", prefix, &entry.name);
    let sha1_hash = &entry.file.as_ref().unwrap().hashes.sha1_hash;
    println!(
        "
{}
    out={}
    checksum=sha-1={}
",
        url, path, sha1_hash
    );
}

fn visit<'a>(
    client: &'a reqwest::Client,
    base_url: &'a str,
    id: &'a str,
    prefix: &'a str,
) -> BoxFuture<'a, MyResult> {
    async move {
        let resp: DriveItemResp = reqwest::get(format!(
            "{}/items/{}/?expand=children(select=id,name,children,file,folder)",
            &base_url, &id
        ))
        .await?
        .json()
        .await?;

        if resp.folder.is_some() {
            let new_prefix = format!("{}{}/", prefix, resp.name);

            let children = resp.children.as_ref().unwrap();

            let mut tasks: FuturesUnordered<_> = children
                .iter()
                .filter_map(|child| {
                    if child.folder.is_some() {
                        Some(visit(client, base_url, &child.id, &new_prefix))
                    } else {
                        print_aria2_entry(base_url, &new_prefix, child);
                        None
                    }
                })
                .collect();

            while let Some(ret) = tasks.next().await {
                ret?;
            }
        } else {
            print_aria2_entry(base_url, prefix, &resp);
        }

        Ok(())
    }
    .boxed()
}

#[tokio::main]
async fn main() -> MyResult {
    let args = Args::parse();
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&args.url);

    let base_url = format!("https://api.onedrive.com/v1.0/shares/u!{}", &encoded);

    let client = reqwest::Client::new();

    let resp: DriveItemResp = client
        .get(format!("{}/driveItem?select=id,name", base_url))
        .send()
        .await?
        .json()
        .await?;

    visit(&client, &base_url, &resp.id, "").await?;

    Ok(())
}
