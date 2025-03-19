use std::error::Error;

mod io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // todo: add impl for user portfolios
    // todo: from then filter any etfs or mutual funds before checking for sec filings
    //       - after getting sec data we run sentiment analysis on everything
    let tickers = vec!["MSFT", "GOOGL", "TSLA"];

    let mut company_datas = Vec::new();

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
    }
    Ok(())
}
