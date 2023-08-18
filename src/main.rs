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
struct DriveItemResp {
    id: String,
    name: String,
    folder: Option<Value>,
    children: Option<Vec<DriveItemResp>>,
}

fn visit<'a>(
    client: &'a reqwest::Client,
    base_url: &'a str,
    id: &'a str,
    prefix: &'a str,
) -> BoxFuture<'a, MyResult> {
    async move {
        let resp: DriveItemResp = reqwest::get(format!(
            "{}/items/{}/?expand=children(select=id,name,folder,children)",
            &base_url, &id
        ))
        .await?
        .json()
        .await?;

        if resp.folder.is_some() {
            let new_prefix = format!("{}{}/", prefix, resp.name);

            let children = &resp.children.unwrap();

            let mut tasks: FuturesUnordered<_> = children
                .iter()
                .filter_map(|child| {
                    if child.folder.is_some() {
                        Some(visit(client, base_url, &child.id, &new_prefix))
                    } else {
                        println!("{}{}", &new_prefix, child.name);
                        None
                    }
                })
                .collect();

            while let Some(ret) = tasks.next().await {
                ret?;
            }
        } else {
            println!("{}{}", &prefix, resp.name);
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
