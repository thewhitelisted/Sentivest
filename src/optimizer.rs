fn parse_growth_rate(data: Option<f64>) -> Vec<f64> {
    if let Some(data) = data {
        if data < 0.0 {
            vec![0.2, 0.8, 0.0]
        } else if data <= 0.1 && data >= 0.5 {
            vec![0.0, 0.8, 0.2]
        } else {
            vec![0.0, 1.0, 0.0]
        }
    } else {
        vec![0.0, 0.0, 0.0]
    }
}

fn parse_debt_equity(data: Option<f64>) -> Vec<f64> {
    if let Some(data) = data {
        if data < 1.0 {
            vec![0.0, 1.0, 0.0]
        } else if data >= 1.0 && data <= 1.5 {
            vec![0.0, 0.8, 0.2]
        } else {
            vec![0.2, 0.8, 0.0]
        }
    } else {
        vec![0.0, 0.0, 0.0]
    }
}

pub fn analyze_fiancials(datas: Vec<Vec<Option<f64>>>) -> Vec<Vec<Vec<f64>>> {
    let mut result = Vec::new();
    for data in datas {
        let growth_rate = data[0];
        let debt_equity = data[1];
        let growth_matrix = parse_growth_rate(growth_rate);
        let debt_matrix = parse_debt_equity(debt_equity);
        result.push(vec![growth_matrix, debt_matrix]);
    }
    result
}

pub fn aggregate_sentiment(sentiments: Vec<Vec<f64>>) -> Vec<f64> {
    // average sentiment for each article
    let mut result = Vec::new();
    let mut good = 0.0;
    let mut neutral = 0.0;
    let mut bad = 0.0;
    for sentiment in &sentiments {
        bad += sentiment[0];
        neutral += sentiment[1];
        good += sentiment[2];
    }
    result.push(bad / sentiments.len() as f64);
    result.push(neutral / sentiments.len() as f64);
    result.push(good / sentiments.len() as f64);
    result
}

pub fn sentiment_returns(sentiments: Vec<Vec<f64>>) -> Vec<f64> {
    let mut final_sentiments = Vec::new();
    for i in 0..sentiments.len() {
        let bad = sentiments[i][0];
        let neutral = sentiments[i][1];
        let good = sentiments[i][2];
        let sentiment = (good - bad)/(good + bad + neutral);
        final_sentiments.push(sentiment);
    }
    final_sentiments
}