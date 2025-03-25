fn parse_growth_rate(data: Option<f64>) -> Vec<f64> {
    match data {
        Some(value) if value < 0.0 => vec![0.2, 0.8, 0.0],
        Some(value) if (0.0..=0.1).contains(&value) || value >= 0.5 => vec![0.0, 0.8, 0.2],
        Some(_) => vec![0.0, 1.0, 0.0],
        None => vec![0.0, 0.0, 0.0],
    }
}

fn parse_debt_equity(data: Option<f64>) -> Vec<f64> {
    match data {
        Some(value) if (1.0..=1.5).contains(&value) => vec![0.0, 0.8, 0.2],
        Some(value) if value < 1.0 => vec![0.0, 1.0, 0.0],
        Some(_) => vec![0.2, 0.8, 0.0],
        None => vec![0.0, 0.0, 0.0],
    }
}

pub fn analyze_fiancials(datas: Vec<Vec<Option<f64>>>) -> Vec<Vec<Vec<f64>>> {
    datas.into_iter()
        .map(|data| {
            if data.len() < 2 {
                return Vec::new();
            }
            vec![parse_growth_rate(data[0]), parse_debt_equity(data[1])]
        })
        .collect()
}

pub fn aggregate_sentiment(sentiments: Vec<Vec<f64>>) -> Vec<f64> {
    if sentiments.is_empty() {
        return Vec::new();
    }
    
    let len = sentiments.len() as f64;
    let mut result = vec![0.0; 3];
    
    for sentiment in sentiments {
        if sentiment.len() >= 3 {
            result[0] += sentiment[0];
            result[1] += sentiment[1];
            result[2] += sentiment[2];
        }
    }
    
    result.iter_mut().for_each(|val| *val /= len);
    result
}

pub fn sentiment_returns(sentiments: Vec<Vec<f64>>) -> Vec<f64> {
    sentiments.iter()
        .map(|sentiment| {
            if sentiment.len() < 3 {
                return 0.0;
            }
            let (bad, neutral, good) = (sentiment[0], sentiment[1], sentiment[2]);
            let total = bad + neutral + good;
            if total == 0.0 { 0.0 } else { (good - bad) / total }
        })
        .collect()
}

pub fn get_pviews(sentiment_returns: Vec<f64>) -> Vec<Vec<f64>> {
    let len = sentiment_returns.len();
    let mut p_values = Vec::with_capacity(len);
    
    for i in 0..len {
        let mut row = vec![0.0; len];
        row[i] = 1.0; // Set diagonal to 1.0
        
        // Set certain elements to -1.0 efficiently
        for j in (i+2)..len {
            row[j] = -1.0;
        }
        
        p_values.push(row);
    }
    
    p_values
}

pub fn get_qviews(sentiment_returns: Vec<f64>) -> Vec<f64> {
    if sentiment_returns.is_empty() {
        return Vec::new();
    }
    
    let mut q_values = Vec::with_capacity(sentiment_returns.len());
    q_values.push(sentiment_returns[0]);
    
    sentiment_returns.windows(2)
        .for_each(|window| {
            q_values.push(window[1] - window[0]);
        });
    
    q_values
}