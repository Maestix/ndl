use reqwest::{Client, header};
use scraper::{Html, Selector};
use std::{env, fs, sync::Arc};
use regex::Regex;
use tokio::{task, sync::Semaphore};

#[tokio::main]
async fn main() {
    // Get the gallery URL as a command line argument
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Please provide a valid nhentai gallery URL.");
        return;
    }

    let url = &args[1];
    let semaphore = Arc::new(Semaphore::new(12));
    // Create the client
    let client = Arc::new(Client::new());
    // Fetch the HTML of the gallery
    let html = match fetch_html(url).await {
        Ok(html) => html,
        Err(e) => {
            eprintln!("Failed to fetch HTML: {}", e);
            return;
        }
    };

    // Extract the number of pages (just for logging purposes)
    let num_pages = match extract_num_pages(&html) {
        Some(pages) => pages,
        None => {
            eprintln!("Unable to extract the number of pages.");
            return;
        }
    };
    println!("Number of pages: {}", num_pages);

    // Extract the image URLs from the HTML
    let image_urls = extract_image_urls(&html);

    // Fix URLs
    let fixed_urls: Vec<String> = image_urls
        .into_iter()
        .filter_map(|url| fix_url(&url))
        .collect();

    // Print fixed URLs
    for fixed_url in &fixed_urls {
        println!("{}", fixed_url);
    }

    // Download all images concurrently
    let handles: Vec<_> = fixed_urls.into_iter().map(|image_url| {
        let client_clone = Arc::clone(&client);
        let semaphore = Arc::clone(&semaphore);
        task::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            download_image(&client_clone, &image_url).await;
        })
    }).collect();

    // Await all download tasks
    for handle in handles {
        handle.await.unwrap();
    }
}

async fn fetch_html(url: &str) -> Result<String, reqwest::Error> {
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    Ok(body)
}

fn extract_num_pages(html: &str) -> Option<u32> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("div.thumb-container a.gallerythumb img").unwrap();

    Some(document.select(&selector).count() as u32)
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
    // Regex to match the URL and transform it
    let re = Regex::new(r"https://t(\d+)\.nhentai\.net/galleries/(\d+)/(\d+)t\.(webp|jpg|png)").unwrap();
    re.captures(url).map(|caps| {
        format!(
            "https://i{}.nhentai.net/galleries/{}/{}.{}",
            &caps[1], &caps[2], &caps[3], &caps[4]
        )
    })
}

async fn download_image(client: &Client, image_url: &str) {
    println!("Downloading image from: {}", image_url);

    // Perform GET request to download the image
    let response = client
        .get(image_url)
        .header(header::USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:134.0) Gecko/20100101 Firefox/134.0")
        .header(header::REFERER, "https://nhentai.net/")
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

                let filename = image_url.split('/').last().unwrap_or("image.webp");
                if let Err(e) = fs::write(filename, content) {
                    eprintln!("Failed to save image: {}", e);
                } else {
                    println!("Downloaded image: {}", filename);
                }
            } else {
                eprintln!("Failed to download image from: {} - Status: {}", image_url, response.status());
            }
        }
        Err(e) => {
            eprintln!("Error downloading image from: {} - Error: {}", image_url, e);
        }
    }
}
