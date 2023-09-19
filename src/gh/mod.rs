use serde::{Serialize, Deserialize};
use std::io::Write;
use reqwest::{Client, header::{HeaderMap, HeaderValue}};
use crate::{info, warn};

#[derive(Serialize, Deserialize)]
struct Repo {
    archive_url: String,
    default_branch: String,
}

pub async fn get_personal_repositories_urls(
    access_token: &String,
) -> Result<Vec<String>, String> {
    let url = "https://api.github.com/user/repos?per_page=100&type=owner&page=1&sort=updated";

    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", access_token)).expect(""),
    );
    headers.insert(
        "Accept",
        HeaderValue::from_str("application/vnd.github+json").expect(""),
    );
    headers.insert(
        "User-Agent",
        HeaderValue::from_str("github-backup").expect(""),
    );

    let client = Client::new();

    let response = match client.get(url).headers(headers).send().await {
        Ok(response) => response,
        Err(e) => return Err(format!("{}", e)),
    };

    let response = match response.text().await {
        Ok(response) => response,
        Err(e) => return Err(format!("{}", e)),
    };

    
    let result: Vec<Repo> = serde_json::from_str(&response).unwrap();
    
    let mut urls = Vec::new();

    for repo in result {
        let url = repo.archive_url;
        let branch = repo.default_branch;

        let url = url.replace("{archive_format}", "zipball");
        let url = url.replace("{/ref}",  format!("/{}", branch).as_str());

        urls.push(url);
    }

    info!("Found {} repositories", urls.len());

    Ok(urls)
}

pub async fn download_to_backup(url: String, access_token: &String, output: &String) -> Result<(), String> {
    info!("Downloading {}", url);
    
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", access_token)).expect(""),
    );
    headers.insert(
        "Accept",
        HeaderValue::from_str("application/vnd.github+json").expect(""),
    );
    headers.insert(
        "User-Agent",
        HeaderValue::from_str("github-backup").expect(""),
    );

    let client = Client::new();

    let response = match client.get(url).headers(headers).send().await {
        Ok(response) => response,
        Err(e) => return Err(format!("{}", e)),
    };

    let url = response.url().to_string();

    // get the file name from content-disposition
    let file_name = match response.headers().get("content-disposition") {
        Some(content_disposition) => {
            let content_disposition = content_disposition.to_str().unwrap();
            let content_disposition = content_disposition.replace("attachment; filename=", "");
            let content_disposition = content_disposition.replace("\"", "");
            content_disposition
        },
        None => {
            warn!("Repository is likely empty, skipping");

            return Ok(())
        }
    };

    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(e) => return Err(format!("{}", e)),
    };

    let response = match response.bytes().await {
        Ok(response) => response,
        Err(e) => return Err(format!("{}", e)),
    };

    let mut path = std::path::PathBuf::from(output);
    path.push(file_name);

    let mut file = match std::fs::File::create(path) {
        Ok(file) => file,
        Err(e) => return Err(format!("{}", e)),
    };

    match file.write_all(&response) {
        Ok(_) => (),
        Err(e) => return Err(format!("{}", e)),
    };
    
    Ok(())
}