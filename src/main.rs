use std::{fs::{self, create_dir_all, File}, io::{self, BufWriter, Write}, path::Path, sync::Arc};
use serde_json::Value;
use reqwest::{self, StatusCode};
use tokio::{self, sync::Semaphore, task};

#[tokio::main]
async fn main() {
    let (fp, url, is_file) = get_config();
    download(fp, url, is_file, 10).await;
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

                    create_dir_all(path.parent().unwrap()).unwrap();
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

async fn download(fp: String, url: String, is_file: bool, threads: usize) {

    let contents = if is_file {
        fs::read_to_string(fp.trim()).unwrap()
    } else {
        reqwest::get(format!("{url}{fp}/fingerprint.json")).await.unwrap().text().await.unwrap()
    };

    let fp_json: Value = serde_json::from_str(&contents).unwrap();
    let sha = fp_json["sha"].as_str().unwrap().trim_matches('"').to_string();
    println!("SHA: {}", sha);

    let mut tasks = Vec::new();
    let client = Arc::new(reqwest::Client::new());
    let semaphore = Arc::new(Semaphore::new(threads));

    for file in fp_json["files"].as_array().unwrap() {
        let file_name = file["file"].as_str().unwrap().trim_matches('"').to_string();

        let full_url = format!("{url}{sha}/{file_name}");
        let full_path = format!("{sha}/{file_name}");

        let semaphore_permit = semaphore.clone().acquire_owned().await.unwrap();
        let client_clone = Arc::clone(&client);

        let task = task::spawn(async move {
            download_file(&client_clone, full_url, full_path, file_name).await;
            drop(semaphore_permit);
        });

        tasks.push(task);
    }

    for task in tasks {
        task.await.unwrap();
    }
}

fn get_config() -> (String, String, bool) {
    let mut fp = String::new();
    let mut url = String::new();

    println!("Fingerprint File or SHA (e.g. fingerprint.json or 026b98730aac824ae292238be1176a927e139da8): ");
    io::stdin().read_line(&mut fp).unwrap();

    println!("Asset URL: (e.g. https://game-assets.brawlstarsgame.com): ");
    io::stdin().read_line(&mut url).unwrap();

    url = url.trim().to_string();
    fp = fp.trim().to_string();

    if !url.ends_with('/') {
        url.push('/');
    }

    if fp.contains(".") {
        return (fp, url, true);
    } else {
        return (fp, url, false);
    }

}