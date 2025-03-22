use std::error::Error;
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::env;

mod io;
mod optimizer;

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
    let tickers = vec!["TSLA"];

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
        company_datas.push(data);

        println!("Company data claimed");

        // scrape news
        let articles_data = io::scrape_news(ticker).await?;
        articles.push(articles_data.clone());

        println!("Articles scraped");

        // analyze sentiment
        let mut sentiment_data = Vec::new();
        for article in &articles_data {
            let sentiment = analyze_sentiment(article)?;
            println!("Sentiment done");
            sentiment_data.push(sentiment);
        }
        sentiments.push(sentiment_data);
        println!("Sentiments analyzed");
    }

    let analyzed_financials = optimizer::analyze_fiancials(company_datas);
    // add each sentiment in analyzed_financials to sentiments in each ticker
    for (i, fin) in analyzed_financials.iter().enumerate() {
        for data in fin {
            sentiments[i].push(data.clone());
        }
    }

    let mut agg_sentiments = Vec::new();

    for sentiment in sentiments {
        let agg_sentiment = optimizer::aggregate_sentiment(sentiment);
        agg_sentiments.push(agg_sentiment);
    }

    println!("done aggregating sentiments");

    let returns = optimizer::sentiment_returns(agg_sentiments);
    println!("Returns: {:?}", returns);
    Ok(())
}
