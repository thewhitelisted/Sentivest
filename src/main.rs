// imports
use yahoo_finance_api as yahoo;
use std::error::Error;
use serde_json;
use chrono::{Utc, Duration};
use time::OffsetDateTime;
use reqwest;
use scraper::{Html, Selector};

async fn fetch_sec_filings(cik: &str, form_type: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let url = format!(
        "https://data.sec.gov/submissions/CIK{}.json",
        cik
    );

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "optimizeme/1.0 (jleechris06@gmail.com)")
        .send()
        .await?
        .text()
        .await?;
    let json: serde_json::Value = serde_json::from_str(&response)?;

    let mut filings_urls = Vec::new();

    // Extract filings
    if let Some(filings) = json["filings"]["recent"]["form"].as_array() {
        for (i, form) in filings.iter().enumerate() {
            if let Some(form_str) = form.as_str() {
                if form_str == form_type {
                    if let Some(accession_number) = json["filings"]["recent"]["accessionNumber"][i].as_str() {
                        // Proper SEC EDGAR URL format: CIK/AccessionNumber-no-dashes/AccessionNumber-with-dashes.txt
                        let acc_no_dashes = accession_number.replace("-", "");
                        let filing_url = format!(
                            "https://www.sec.gov/Archives/edgar/data/{}/{}/{}.txt",
                            cik,
                            acc_no_dashes,
                            accession_number
                        );
                        filings_urls.push(filing_url);
                    }
                }
            }
        }
    }
    
    Ok(filings_urls)
}

async fn scrape_filing(url: &str) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let response = client.get(url)
        .header("User-Agent", "optimizeme/1.0 (jleechris06@gmail.com)")
        .send()
        .await?
        .text()
        .await?;
    
    // First, look for <DOCUMENT> sections which contain the actual documents
    let mut extracted_text = String::new();
    
    // Find the 10-K document specifically (could be multiple documents in the filing)
    if let Some(start_idx) = response.find("<TYPE>10-K") {
        // Find the start of this document
        if let Some(doc_start) = response[start_idx..].find("<TEXT>") {
            let doc_start_idx = start_idx + doc_start + 6; // +6 to skip "<TEXT>"
            
            // Find the end of this document
            if let Some(doc_end) = response[doc_start_idx..].find("</TEXT>") {
                let doc_content = &response[doc_start_idx..(doc_start_idx + doc_end)];
                
                // Check if it's HTML content
                if doc_content.contains("<html>") || doc_content.contains("<HTML>") {
                    // Parse as HTML
                    let document = Html::parse_document(doc_content);
                    
                    // Extract text from body
                    if let Ok(body_selector) = Selector::parse("body") {
                        for body in document.select(&body_selector) {
                            for text_node in body.text() {
                                let trimmed = text_node.trim();
                                if !trimmed.is_empty() {
                                    extracted_text.push_str(trimmed);
                                    extracted_text.push_str("\n");
                                }
                            }
                        }
                    }
                    
                    // If body parsing failed, try to get text from all elements
                    if extracted_text.trim().is_empty() {
                        // Get text from various content elements
                        for selector_str in ["p", "div", "td", "li", "span", "h1", "h2", "h3", "h4"] {
                            if let Ok(selector) = Selector::parse(selector_str) {
                                for element in document.select(&selector) {
                                    for text_node in element.text() {
                                        let trimmed = text_node.trim();
                                        if !trimmed.is_empty() {
                                            extracted_text.push_str(trimmed);
                                            extracted_text.push_str("\n");
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // Treat as plain text - just return the document content
                    extracted_text = doc_content.to_string();
                }
            }
        }
    }
    
    // If we couldn't extract anything meaningful using the methods above
    if extracted_text.trim().is_empty() {
        // Alternative approach: Look for the main document after the header section
        if let Some(idx) = response.find("</SEC-HEADER>") {
            extracted_text = response[idx + 13..].to_string(); // +13 to skip "</SEC-HEADER>"
        }
    }
    
    // Clean up the text (remove excessive whitespace, etc.)
    let cleaned_text = extracted_text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    
    if cleaned_text.trim().is_empty() {
        // Last resort: return a portion of the original response
        Ok(response.chars().take(10000).collect())
    } else {
        Ok(cleaned_text)
    }
}

fn get_cik(ticker: &str) -> Result<String, Box<dyn Error>> {
    // Read the embedded JSON file
    let json_data = include_str!("company_tickers.json");
    
    println!("Reading CIK data from local file");
    
    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(json_data)?;

    // Iterate over all entries in the JSON object
    if let Some(obj) = json.as_object() {
        for (_, company) in obj {
            // Check if this entry matches our ticker
            if let Some(ticker_value) = company.get("ticker") {
                if ticker_value.as_str().unwrap_or("").to_uppercase() == ticker.to_uppercase() {
                    // Extract CIK (key is "cikstr" in the JSON)
                    if let Some(cik_value) = company.get("cik_str") {
                        if let Some(cik_num) = cik_value.as_u64() {
                            // Format CIK to 10 digits with leading zeros
                            return Ok(format!("{:010}", cik_num));
                        }
                    }
                }
            }
        }
    }
    
    Err(format!("CIK not found for ticker: {}", ticker).into())
}

async fn get_stock_history(ticker: &str, days: i64) -> Result<yahoo::YResponse, Box<dyn Error>> {
    let provider = yahoo::YahooConnector::new()?;
    
    let end = Utc::now();
    let start = end - Duration::days(days);
    
    println!("Fetching {} days of history for {}", days, ticker);
    
    let start_odt = OffsetDateTime::from_unix_timestamp(start.timestamp())?;
    let end_odt = OffsetDateTime::from_unix_timestamp(end.timestamp())?;
    
    let response = provider.get_quote_history(ticker, start_odt, end_odt).await?;
    
    // Ensure we have valid data
    if response.quotes().is_err() {
        return Err("Failed to get quotes from Yahoo response".into());
    }
    
    Ok(response)
}

async fn get_latest_quote(ticker: &str) -> Result<yahoo::YResponse, Box<dyn Error>> {
    let provider = yahoo::YahooConnector::new()?;
    
    println!("Fetching latest quote for {}", ticker);
    
    // Get the latest quotes
    let response = provider.get_latest_quotes(ticker, "1d").await?;
    
    Ok(response)
}

fn analyze_stock_data(history: &yahoo::YResponse) -> Result<String, Box<dyn Error>> {
    let quotes = history.quotes()?;
    
    if quotes.is_empty() {
        return Ok("No historical data available for analysis".to_string());
    }
    
    // Calculate some basic metrics
    let latest_close = quotes.last().unwrap().close;
    let earliest_close = quotes.first().unwrap().close;
    let percent_change = (latest_close - earliest_close) / earliest_close * 100.0;
    
    // Find highest and lowest prices
    let mut highest = quotes[0].high;
    let mut lowest = quotes[0].low;
    let mut total_volume = 0;
    
    for quote in quotes.iter() {
        if quote.high > highest {
            highest = quote.high;
        }
        if quote.low < lowest {
            lowest = quote.low;
        }
        total_volume += quote.volume;
    }
    
    // Average volume
    let avg_volume = total_volume as f64 / quotes.len() as f64;
    
    // Format the analysis
    let analysis = format!(
        "Analysis over {} days:\n\
         - Starting price: ${:.2}\n\
         - Latest price: ${:.2}\n\
         - Change: {:.2}%\n\
         - Highest price: ${:.2}\n\
         - Lowest price: ${:.2}\n\
         - Average daily volume: {:.0}",
        quotes.len(),
        earliest_close,
        latest_close,
        percent_change,
        highest,
        lowest,
        avg_volume
    );
    
    Ok(analysis)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let tickers = vec!["MSFT", "GOOGL", "TSLA"];
    use std::fs;
    use std::path::Path;
    
    // Create a directory to store the filings if it doesn't exist
    let filings_dir = "sec_filings";
    if !Path::new(filings_dir).exists() {
        fs::create_dir(filings_dir)?;
    }

    for ticker in &tickers {
        println!("\n=== Processing {} ===", ticker);

        // Get CIK from local JSON
        let cik = match get_cik(ticker) {
            Ok(cik) => cik,
            Err(e) => {
                println!("Error getting CIK for {}: {}", ticker, e);
                continue;
            }
        };

        println!("CIK for {}: {}", ticker, cik);

        // Fetch latest 10-K filing
        let filings = fetch_sec_filings(&cik, "10-K").await?;
        if let Some(filing_url) = filings.first() {
            println!("Fetching 10-K for {}: {}", ticker, filing_url);

            // Scrape filing content
            let extracted_text = scrape_filing(filing_url).await?;
            
            // Save the full content to a file
            let file_path = format!("{}/{}_10K.txt", filings_dir, ticker);
            fs::write(&file_path, &extracted_text)?;
            println!("Saved full 10-K content ({} bytes) to {}", extracted_text.len(), file_path);
            
            // Print preview of content
            println!("Extracted SEC Filing Content (preview):\n{}", 
                &extracted_text.chars().take(1000).collect::<String>()); 
        }
    }

    Ok(())
}
