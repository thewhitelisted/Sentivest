use std::error::Error;
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::env;

mod io;
mod optimizer;
mod litterman;

fn analyze_sentiment(text: &str) -> PyResult<Vec<f64>> {
    unsafe {
        env::set_var("PYTHONPATH", "./src");
    }
    
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let sentiment_module = PyModule::import(py, "finbert")?;
        let sentiment_class = sentiment_module.getattr("FinBERTSentiment")?.call0()?;
        let sentiment_result = sentiment_class.getattr("analyze_sentiment")?.call1((text,))?;

        // Convert Python list to Rust Vec<f64>
        sentiment_result.downcast::<PyList>()?.extract()
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let tickers = vec!["TSLA", "AAPL", "MSFT", "GOOGL", "AMZN"];
    let mut company_datas = Vec::with_capacity(tickers.len());
    let mut articles = Vec::with_capacity(tickers.len());
    let mut sentiments = Vec::with_capacity(tickers.len());
    let mut valid_tickers = Vec::with_capacity(tickers.len());

    // Process each ticker
    for ticker in &tickers {
        println!("\n=== Processing {} ===", ticker);

        // Get CIK from local JSON
        let cik = match io::get_cik(ticker) {
            Ok(cik) => {
                println!("CIK for {}: {}", ticker, cik);
                cik
            },
            Err(e) => {
                println!("Error getting CIK for {}: {}", ticker, e);
                continue;
            }
        };

        // Fetch company data
        match io::fetch_sec_filings(&cik).await {
            Ok(company_data) => {
                company_datas.push(io::parse_json(&company_data));
                println!("Company data claimed");
                
                // Scrape news articles
                match io::scrape_news(ticker).await {
                    Ok(articles_data) => {
                        articles.push(articles_data.clone());
                        println!("Articles scraped");
                        
                        // Analyze sentiment
                        match articles_data
                            .iter()
                            .map(|article| {
                                let sentiment = analyze_sentiment(article)?;
                                println!("Sentiment done");
                                Ok(sentiment)
                            })
                            .collect::<Result<Vec<Vec<f64>>, PyErr>>() {
                                Ok(sentiment_data) => {
                                    sentiments.push(sentiment_data);
                                    println!("Sentiments analyzed");
                                    valid_tickers.push(*ticker);
                                },
                                Err(e) => {
                                    println!("Error analyzing sentiments: {:?}", e);
                                }
                            }
                    },
                    Err(e) => println!("Error scraping news: {}", e)
                }
            },
            Err(e) => println!("Error fetching SEC filings: {}", e)
        }
    }

    // Analyze financial data and merge with sentiment data
    let analyzed_financials = optimizer::analyze_fiancials(company_datas);
    for (i, fin) in analyzed_financials.iter().enumerate() {
        for data in fin {
            if i < sentiments.len() {
                sentiments[i].push(data.clone());
            }
        }
    }

    // Aggregate sentiments
    let agg_sentiments: Vec<_> = sentiments
        .into_iter()
        .map(optimizer::aggregate_sentiment)
        .collect();
    println!("Done aggregating sentiments");

    // Calculate returns and sort
    let returns = optimizer::sentiment_returns(agg_sentiments);
    
    // Track ordering during sort for consistent indexing
    let mut indexed_returns: Vec<(usize, &f64)> = returns.iter().enumerate().collect();
    indexed_returns.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));
    
    // Update tickers to match new ordering
    let sorted_tickers: Vec<_> = indexed_returns
        .iter()
        .map(|(i, _)| valid_tickers[*i])
        .collect();

    // Extract sorted values
    let values: Vec<f64> = indexed_returns
        .iter()
        .map(|(_, v)| **v)
        .collect();
                                        
    // Generate views
    let p_values = optimizer::get_pviews(values.clone());
    let q_values = optimizer::get_qviews(values);

    // Get market data
    let market_weights = io::get_market_weights(sorted_tickers.clone()).await?;
    let sigma = io::get_covariance_matrix(sorted_tickers.clone()).await?;
    let omega = io::get_uncertainty_matrix(sorted_tickers.clone());

    // Run Black-Litterman model
    let tau = 0.025;
    let posterior_mean = litterman::black_litterman(
        &sigma, 
        &market_weights, 
        tau, 
        &p_values, 
        &q_values, 
        &omega
    );

    println!("Posterior mean: {:?}", posterior_mean);

    let updated_weights = litterman::mvo(&sigma, posterior_mean);
    for i in 0..sorted_tickers.len() {
        println!("{}: {:.2}%", sorted_tickers[i], updated_weights[i] * 100.0);
    }
    
    Ok(())
}
