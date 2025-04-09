use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::{header, Client};
use scraper::{Html, Selector};
use std::{env, path::Path, sync::Arc};
use tokio::{fs as tokio_fs, sync::Semaphore, task};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Please provide a valid nhentai gallery URL.");
        return;
    }

    let mut url = args[1].clone();
    url = url.replace("https://nhentai.net", "https://nhentai.to");

    let semaphore = Arc::new(Semaphore::new(12));

    let client = create_client();

    let html = match fetch_html(client.clone(), &url).await {
        Ok(html) => html,
        Err(e) => {
            eprintln!("Failed to fetch HTML: {}", e);
            return;
        }
    };

    let manga_title = match extract_manga_title(&html) {
        Some(title) => title,
        None => {
            eprintln!("Unable to extract manga title.");
            return;
        }
    };
    println!("Manga Title: {}", manga_title);

    let folder_name = sanitize_windows_filename(&manga_title);

    let folder_path = Path::new(&folder_name);
    if !folder_path.exists() {
        if let Err(e) = tokio_fs::create_dir_all(&folder_path).await {
            eprintln!("Failed to create directory: {}", e);
            return;
        }
    }

    let image_urls = extract_image_urls(&html);

    let fixed_urls: Vec<String> = image_urls.iter().filter_map(|url| fix_url(&url)).collect();

    let pb = Arc::new(ProgressBar::new(fixed_urls.len() as u64));
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})",
            )
            .unwrap()
            .progress_chars("#>-"),
    );

    let handles: Vec<_> = fixed_urls
        .into_iter()
        .map(|image_url| {
            let client_clone = client.clone();
            let semaphore = Arc::clone(&semaphore);
            let pb = Arc::clone(&pb);
            let folder_name = folder_name.clone();
            task::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                download_image(&client_clone, &image_url, &folder_name).await;
                pb.inc(1);
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }
    pb.finish_with_message("Download complete!");
}

fn create_client() -> Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36",
        ),
    );
    Client::builder()
        .default_headers(headers)
        .build()
        .expect("Failed to create reqwest client")
}

async fn fetch_html(client: Client, url: &str) -> Result<String, reqwest::Error> {
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    Ok(body)
}

fn extract_manga_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("div#info-block h1").unwrap();
    document
        .select(&selector)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
}

fn extract_image_urls(html: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("div.thumb-container a.gallerythumb img").unwrap();
    let mut image_urls = Vec::new();
    for element in document.select(&selector) {
        if let Some(data_src) = element.value().attr("data-src") {
            image_urls.push(data_src.to_string());
        }
    }
    image_urls
}

fn fix_url(url: &str) -> Option<String> {
    let re = Regex::new(r"(https://[^/]+/galleries/\d+/)(\d+)t(\.(webp|jpg|png))$").unwrap();
    re.captures(url)
        .map(|caps| format!("{}{}{}", &caps[1], &caps[2], &caps[3]))
}

fn sanitize_windows_filename(name: &str) -> String {
    let re = Regex::new(r#"[<>:"/\\|?*]"#).unwrap();

    let sanitized = re.replace_all(name, "_").to_string();

    if sanitized.is_empty() {
        return "invalid_filename".to_string();
    }

    sanitized
}

async fn download_image(client: &Client, image_url: &str, folder_name: &str) {
    let response = client
        .get(image_url)
        .header(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:134.0) Gecko/20100101 Firefox/135.0",
            ),
        )
        .header(
            header::REFERER,
            header::HeaderValue::from_static("https://nhentai.to/"),
        )
        .send()
        .await;

    match response {
        Ok(response) => {
            if response.status().is_success() {
                let content = match response.bytes().await {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        eprintln!("Error reading image data from: {}", image_url);
                        return;
                    }
                };
                let raw_filename = image_url.split('/').last().unwrap_or("image.webp");
                let filename = sanitize_windows_filename(raw_filename);
                let file_path = Path::new(folder_name).join(filename);
                if let Err(e) = tokio_fs::write(&file_path, content).await {
                    eprintln!("Failed to save image {}: {}", file_path.display(), e);
                }
            } else {
                eprintln!(
                    "Failed to download image from: {} - Status: {}",
                    image_url,
                    response.status()
                );
            }
        }
        Err(e) => {
            eprintln!("Error downloading image from: {} - Error: {}", image_url, e);
        }
    }
}
