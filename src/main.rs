use std::{fs::{self, create_dir_all, File}, io::{self, BufWriter, Write}, path::Path, sync::Arc};
use serde_json::Value;
use reqwest::{self, StatusCode};
use tokio::{self, sync::Semaphore, task};

#[tokio::main]
async fn main() {
    let mut fp = String::new();
    let mut url = String::new();

    println!("Fingerprint File (e.g. fingerprint.json): ");
    io::stdin().read_line(&mut fp).unwrap();

    println!("Asset URL: (e.g. https://game-assets.brawlstarsgame.com): ");
    io::stdin().read_line(&mut url).unwrap();

    url = url.trim().to_string();

    if !url.ends_with('/') {
        url.push('/');
    }

    download(fp, url, 10).await;
}


async fn download_file(client: &reqwest::Client, url: String, file_path: String, file_name: String) {
    let path = Path::new(&file_path);

    if path.exists() {
        println!("{} exists.", file_path);
        return;
    }

    match client.get(&url).send().await {
        Ok(response) => {
            match response.status() {
                StatusCode::OK => {
                    println!("Downloaded {}: Status {}", file_name, response.status());
                    if let Some(parent) = path.parent() {
                        create_dir_all(parent).unwrap();
                    }
                    let file = File::create(path).unwrap();
                    let mut writer = BufWriter::new(file);
                    let content = response.bytes().await.unwrap();
                    writer.write_all(&content).unwrap();
                    writer.flush().unwrap();
                },
                _ => {
                    println!("ERROR on {}: Status {}", file_name, response.status());
                }
            }
        },
        Err(err) => {
            println!("{} {}", file_name, err);
        }
    }
}

async fn download(fp: String, url: String, threads: usize) {
    let contents = fs::read_to_string(fp.trim()).unwrap();
    let fp_json: Value = serde_json::from_str(&contents).unwrap();
    let sha = fp_json["sha"].as_str().unwrap().trim_matches('"').to_string();
    println!("SHA: {}", sha);

    let mut tasks = Vec::new();
    let client = reqwest::Client::new();
    let semaphore = Arc::new(Semaphore::new(threads));

    for file in fp_json["files"].as_array().unwrap() {
        let file_name = file["file"].as_str().unwrap().trim_matches('"').to_string();

        let full_url = format!("{url}{sha}/{file_name}");
        let full_path = format!("{sha}/{file_name}");

        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let client_clone = client.clone();

        let task = task::spawn(async move {
            download_file(&client_clone, full_url, full_path, file_name).await;
            drop(permit);
        });

        tasks.push(task);
    }

    for task in tasks {
        let _ = task.await;
    }
}