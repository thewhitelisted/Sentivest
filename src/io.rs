// imports
use chrono::Utc;
use chrono::Duration;
use std::error::Error;
use time::OffsetDateTime;
use yahoo_finance_api as yahoo;
use scraper::{Html, Selector};
use regex::Regex;

// Get the CIK (Central Index Key) for a given stock ticker
pub fn get_cik(ticker: &str) -> Result<String, Box<dyn Error>> {
    // Read the embedded JSON file
    let json_data = include_str!("company_tickers.json");
    
    println!("Reading CIK data from local file");

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(json_data)?;
    let ticker_upper = ticker.to_uppercase();

    // Iterate over all entries in the JSON object
    if let Some(obj) = json.as_object() {
        for (_, company) in obj {
            // Check if this entry matches our ticker
            if let Some(ticker_value) = company.get("ticker") {
                if ticker_value.as_str().unwrap_or("").to_uppercase() == ticker_upper {
                    // Extract CIK (key is "cikstr" in the JSON)
                    if let Some(cik_num) = company.get("cik_str").and_then(|v| v.as_u64()) {
                        // Format CIK to 10 digits with leading zeros
                        return Ok(format!("{:010}", cik_num));
                    }
                }
            }
        }
    }

    Err(format!("CIK not found for ticker: {}", ticker).into())
}

// Fetch SEC filings for a given CIK
pub async fn fetch_sec_filings(cik: &str) -> Result<serde_json::Value, Box<dyn Error>> {
    let url = format!("https://data.sec.gov/api/xbrl/companyfacts/CIK{}.json", cik);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("User-Agent", "optimizeme/1.0 (jleechris06@gmail.com)")
        .send()
        .await?
        .text()
        .await?;
    
    serde_json::from_str(&response).map_err(|e| e.into())
}

// Get historical stock price data for a given ticker
async fn get_stock_history(ticker: &str, days: i64) -> Result<yahoo::YResponse, Box<dyn Error>> {
    let provider = yahoo::YahooConnector::new()?;

    let end = Utc::now();
    let start = end - Duration::days(days);

    println!("Fetching {} days of history for {}", days, ticker);

    let start_odt = OffsetDateTime::from_unix_timestamp(start.timestamp())?;
    let end_odt = OffsetDateTime::from_unix_timestamp(end.timestamp())?;

    let response = provider
        .get_quote_history(ticker, start_odt, end_odt)
        .await?;

    // Ensure we have valid data
    if response.quotes().is_err() {
        return Err("Failed to get quotes from Yahoo response".into());
    }

    Ok(response)
}

// Get the latest stock quote for a given ticker
async fn get_latest_quote(ticker: &str) -> Result<yahoo::YResponse, Box<dyn Error>> {
    let provider = yahoo::YahooConnector::new()?;
    println!("Fetching latest quote for {}", ticker);
    provider.get_latest_quotes(ticker, "1d").await.map_err(|e| e.into())
}

// Parse JSON from SEC filings to extract financial data
pub fn parse_json(json: &serde_json::Value) -> Vec<Option<f64>> {
    let mut revenue_data = Vec::with_capacity(2);
    let mut debt_equity = Vec::with_capacity(2);
    
    // extract revenue data from past 5 years to calculate growth rate
    let mut last_year = 0.0;
    let mut second_last_year = 0.0;
    
    if let Some(revenues) = json
        .get("facts")
        .and_then(|f| f.get("us-gaap"))
        .and_then(|g| g.get("Revenues"))
        .and_then(|r| r.get("units"))
        .and_then(|u| u.get("USD"))
        .and_then(|data| data.as_array())
    {
        // filter all non-yearly reports
        let mut yearly_reports: Vec<_> = revenues
            .iter()
            .filter(|r| r.get("fp").and_then(|fp| fp.as_str()) == Some("FY"))
            .collect();
            
        // reports come in chronological order, get the last two indices
        yearly_reports.reverse();
        if yearly_reports.len() >= 2 {
            // date should be later than 2021
            if let Some(year) = yearly_reports[0]
                .get("end")
                .and_then(|date| date.as_str())
                .and_then(|date| date.split('-').next())
                .and_then(|year| year.parse::<i32>().ok())
            {
                if year >= 2022 {
                    last_year = yearly_reports[0].get("val").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    second_last_year = yearly_reports[1].get("val").and_then(|v| v.as_f64()).unwrap_or(0.0);
                } else {
                    println!("Not enough data to calculate growth rate");
                }
            }
        } else {
            println!("Not enough data to calculate growth rate");
        }
    }

    // Check if we have valid revenue data
    if last_year > 0.0 && second_last_year > 0.0 {
        revenue_data.push(Some(
            ((last_year - second_last_year) / second_last_year) * 100.0,
        ));
    } else {
        revenue_data.push(None);
    }

    // find debt equity ratio
    // term debt / total shareholders equity
    let mut debt = 0.0;
    let mut equity = 0.0;

    // Get debt data
    if let Some(debts) = json
        .get("facts")
        .and_then(|f| f.get("us-gaap"))
        .and_then(|g| g.get("LongTermDebtNoncurrent"))
        .and_then(|d| d.get("units"))
        .and_then(|u| u.get("USD"))
        .and_then(|data| data.as_array())
    {
        let mut debt_reports: Vec<_> = debts
            .iter()
            .filter(|r| r.get("fp").and_then(|fp| fp.as_str()) == Some("FY"))
            .collect();
            
        // Get the most recent report
        debt_reports.reverse();
        if !debt_reports.is_empty() {
            debt = debt_reports[0].get("val").and_then(|v| v.as_f64()).unwrap_or(0.0);
            // push the last debt date
            if let Some(date) = debt_reports[0].get("end").and_then(|e| e.as_str()) {
                debt_equity.push(date.to_string());
            }
        }
    }

    // Get equity data
    if let Some(equities) = json
        .get("facts")
        .and_then(|f| f.get("us-gaap"))
        .and_then(|g| g.get("StockholdersEquity"))
        .and_then(|d| d.get("units"))
        .and_then(|u| u.get("USD"))
        .and_then(|data| data.as_array())
    {
        let mut equity_reports: Vec<_> = equities
            .iter()
            .filter(|r| r.get("fp").and_then(|fp| fp.as_str()) == Some("FY"))
            .collect();
            
        // Get the most recent report
        equity_reports.reverse();
        if !equity_reports.is_empty() {
            equity = equity_reports[0].get("val").and_then(|v| v.as_f64()).unwrap_or(0.0);
            // push the last equity date
            if let Some(date) = equity_reports[0].get("end").and_then(|e| e.as_str()) {
                debt_equity.push(date.to_string());
            }
        }
    }

    // Check if debt/equity ratio is valid
    if debt_equity.len() == 2 && debt_equity[0] == debt_equity[1] && equity > 0.0 {
        println!("Debt: {}, Equity: {}", debt, equity);
        println!("years: {:?} and {:?}", debt_equity[0], debt_equity[1]);
        revenue_data.push(Some(debt / equity));
    } else {
        revenue_data.push(None);
    }

    revenue_data
}

// Scrape news articles about a stock
pub async fn scrape_news(ticker: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let search_url = format!("https://www.google.com/search?q={}+stock+news&tbm=nws", ticker);
    
    // Fetch search results
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()?;
        
    let search_response = client
        .get(&search_url)
        .send()
        .await?
        .text()
        .await?;

    let search_doc = Html::parse_document(&search_response);
    
    // Extracting URLs from Google's result page
    let link_selector = Selector::parse("a")?;
    let url_pattern = Regex::new(r"/url\?q=(https://[^&]+)&")?;

    let mut article_texts = Vec::with_capacity(5);
    let mut urls = Vec::with_capacity(5);

    // Extract URLs first
    for element in search_doc.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            if let Some(captures) = url_pattern.captures(href) {
                if let Some(link_match) = captures.get(1) {
                    let link = link_match.as_str();
                    urls.push(link.to_string());
                    
                    if urls.len() >= 5 {
                        break;
                    }
                }
            }
        }
    }
    
    // Now scrape each article
    for url in urls {
        println!("found element");
        match scrape_article_text(&url).await {
            Ok(text) => article_texts.push(text),
            Err(_) => article_texts.push("Failed to scrape".to_string())
        }
    }
    
    Ok(article_texts)
}

// Scrape the text content from an article
async fn scrape_article_text(url: &str) -> Result<String, Box<dyn Error>> {
    let response = reqwest::get(url).await?.text().await?;
    let document = Html::parse_document(&response);

    // Extracts paragraph text
    let article_selector = Selector::parse("p")?;  
    
    let text = document
        .select(&article_selector)
        .map(|el| el.text().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    
    println!("done scraping article");
    Ok(text)
}

// Get market weights for a list of tickers
pub async fn get_market_weights(tickers: Vec<&str>) -> Result<Vec<f64>, Box<dyn Error>> {
    let mut market_weights = Vec::with_capacity(tickers.len());
    
    for ticker in tickers {
        let response = get_latest_quote(ticker).await?;
        if let Ok(quotes) = response.quotes() {
            if let Some(quote) = quotes.first() {
                market_weights.push(quote.close);
            } else {
                market_weights.push(0.0);
            }
        } else {
            market_weights.push(0.0);
        }
    }

    // Normalize weights
    let total: f64 = market_weights.iter().sum();
    if total > 0.0 {
        let inv_total = 1.0 / total;
        for weight in market_weights.iter_mut() {
            *weight *= inv_total;
        }
    }
    
    Ok(market_weights)
}

// Get covariance matrix for a list of tickers
pub async fn get_covariance_matrix(tickers: Vec<&str>) -> Result<Vec<Vec<f64>>, Box<dyn Error>> {
    let n = tickers.len();
    let mut prices = Vec::with_capacity(n);
    
    // Fetch historical prices for each ticker
    for ticker in tickers {
        let response = get_stock_history(ticker, 365).await?;
        if let Ok(quotes) = response.quotes() {
            prices.push(quotes.iter().map(|quote| quote.close).collect::<Vec<f64>>());
        } else {
            return Err("Failed to get quotes for covariance calculation".into());
        }
    }

    // Calculate means
    let mut means = Vec::with_capacity(n);
    for price_series in &prices {
        let sum: f64 = price_series.iter().sum();
        means.push(sum / price_series.len() as f64);
    }
    
    // Calculate covariance matrix
    let mut covariance_matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in i..n {  // Use symmetry to reduce calculations
            let price_count = prices[i].len().min(prices[j].len());
            let mut cov = 0.0;
            
            for k in 0..price_count {
                cov += (prices[i][k] - means[i]) * (prices[j][k] - means[j]);
            }
            
            let val = cov / (price_count as f64 - 1.0);
            covariance_matrix[i][j] = val;
            covariance_matrix[j][i] = val;  // Symmetric matrix
        }
    }

    Ok(covariance_matrix)
}

// Create an uncertainty matrix for a list of tickers
pub fn get_uncertainty_matrix(tickers: Vec<&str>) -> Vec<Vec<f64>> {
    let n = tickers.len();
    let mut uncertainty_matrix = vec![vec![0.0; n]; n];
    
    // Only the diagonal elements are non-zero
    for i in 0..n {
        uncertainty_matrix[i][i] = 0.01;
    }

    uncertainty_matrix
}