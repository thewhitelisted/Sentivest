use std::error::Error;
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::env;

mod io;

fn analyze_sentiment(text: &str) -> PyResult<Vec<f64>> {
    unsafe {
        env::set_var("PYTHONPATH", "./src");
    }
    pyo3::prepare_freethreaded_python();
    Python::with_gil(|py| {
        let sentiment_module = PyModule::import(py, "finbert")?;
        let sentiment_class = sentiment_module.getattr("FinBERTSentiment")?.call0()?;
        let sentiment_result = sentiment_class.getattr("analyze_sentiment")?.call1((text,))?;

        // Ensure we received a Python list
        let sentiment_list = sentiment_result.downcast::<PyList>()?;

        // Extract each float value manually
        let rust_vec: Vec<f64> = sentiment_list
            .iter()
            .map(|item| item.extract::<f64>())
            .collect::<PyResult<Vec<f64>>>()?;

        Ok(rust_vec)
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // todo: add impl for user portfolios
    // todo: from then filter any etfs or mutual funds before checking for sec filings
    //       - after getting sec data we run sentiment analysis on everything
    let tickers = vec!["MSFT", "GOOGL", "TSLA"];

    let mut company_datas = Vec::new();
    let mut articles: Vec<Vec<String>> = Vec::new();
    let mut sentiments: Vec<Vec<Vec<f64>>> = Vec::new();

    for ticker in &tickers {
        println!("\n=== Processing {} ===", ticker);

        // Get CIK from local JSON
        let cik = match io::get_cik(ticker) {
            Ok(cik) => cik,
            Err(e) => {
                println!("Error getting CIK for {}: {}", ticker, e);
                continue;
            }
        };

        println!("CIK for {}: {}", ticker, cik);

        // fetch comapnhy data
        let company_data = io::fetch_sec_filings(&cik).await?;
        let data = io::parse_json(&company_data);
        company_datas.push(company_data);

        println!("Company data: {:?}", data);

        // scrape news
        let articles_data = io::scrape_news(ticker).await?;
        articles.push(articles_data.clone());

        // analyze sentiment
        let mut sentiment_data = Vec::new();
        for article in &articles_data {
            let sentiment = analyze_sentiment(article)?;
            sentiment_data.push(sentiment);
        }
        sentiments.push(sentiment_data);
    }
    println!("sentiments: {:?}", sentiments);
    Ok(())
}
