from transformers import AutoModelForSequenceClassification, AutoTokenizer
import torch
import yfinance as yf
from GoogleNews import GoogleNews
from newspaper import Article
import datetime
from pypfopt.expected_returns import mean_historical_return
from pypfopt.risk_models import CovarianceShrinkage
from pypfopt.efficient_frontier import EfficientFrontier

SOURCE_WEIGHTS = {
    "bloomberg.com": 1.0,
    "cnbc.com": 0.9,
    "reuters.com": 0.9,
    "marketwatch.com": 0.8,
    "forbes.com": 0.7,
    "twitter.com": 0.5,
    "reddit.com": 0.4,
    "seekingalpha.com": 0.6
}

model_name = "yiyanghkust/finbert-tone"
tokenizer = AutoTokenizer.from_pretrained(model_name)
model = AutoModelForSequenceClassification.from_pretrained(model_name)

def get_sentiment(text):
    inputs = tokenizer(text, return_tensors="pt", truncation=True, padding=True, max_length=512)
    outputs = model(**inputs)
    probs = torch.nn.functional.softmax(outputs.logits, dim=-1)
    return {"positive": probs[0][0].item(), "neutral": probs[0][1].item(), "negative": probs[0][2].item()}

def get_news(ticker, num_pages=3):
    googlenews = GoogleNews(lang='en', region='US')
    googlenews.search(ticker)
    
    results = []
    for page in range(1, num_pages + 1):
        googlenews.get_page(page)
        results.extend(googlenews.results())

    return results

def extract_article(url):
    try:
        article = Article(url)
        article.download()
        article.parse()
        return article.text
    except:
        return None

def get_full_news_articles(ticker, num_articles=5):
    news_results = get_news(ticker)
    articles = []
    
    for article in news_results[:num_articles]:
        text = extract_article(article["link"])
        articles.append({"title": article["title"], "text": text})
    
    return articles

def analyze_sentiment(articles):
    scores = []
    for article in articles:
        sentiment = get_sentiment(article["text"])
        scores.append(sentiment)
    return scores


def recency_weight(article_date):
    days_old = (datetime.datetime.now() - article_date).days
    return max(0.5, 1 - (days_old / 30))  # Min weight of 0.5 for articles older than 30 days

def weighted_sentiment(article, sentiment):
    source = article.get("source", "").lower()
    source_wt = SOURCE_WEIGHTS.get(source, 0.5)  # Default to 0.5 if unknown source
    recency_wt = recency_weight(article["date"]) if "date" in article else 0.5  # Default recency weight if date is missing

    weight = source_wt * recency_wt
    
    return {
        "positive": sentiment["positive"] * weight,
        "neutral": sentiment["neutral"] * weight,
        "negative": sentiment["negative"] * weight
    }

def aggregate_sentiment(stock_articles):
    total_weight = 0
    final_score = {"positive": 0, "neutral": 0, "negative": 0}

    for article in stock_articles:
        if article["text"]:
            sentiment = get_sentiment(article["text"])  # FinBERT Sentiment
        else:
            sentiment = get_sentiment(article["title"])  # Use title if text is not available
        weighted_score = weighted_sentiment(article, sentiment)
        
        weight = sum(weighted_score.values())  # Total weight
        total_weight += weight
        
        for key in final_score:
            final_score[key] += weighted_score[key]

    # Normalize by total weight
    if total_weight > 0:
        for key in final_score:
            final_score[key] /= total_weight

    return final_score


# Define tickers (example stocks)
tickers = ["AAPL", "GOOGL", "MSFT", "AMZN", "TSLA"]

# Download historical adjusted closing prices (1 year)
data = yf.download(tickers, period="1y").get("Adj Close", yf.download(tickers, period="1y").get("Close"))

# Compute expected annualized returns
mu = mean_historical_return(data)

# Compute shrinkage covariance matrix (Ledoit-Wolf)
S = CovarianceShrinkage(data).ledoit_wolf()

ef = EfficientFrontier(mu, S)
weights = ef.max_sharpe()  # Maximize Sharpe ratio
cleaned_weights = ef.clean_weights()  # Clean small weights
print(cleaned_weights)  # Shows stock allocations
print(ef.portfolio_performance(verbose=True));

ef = EfficientFrontier(mu, S)
weights_low_risk = ef.min_volatility()  # Low-risk allocation
cleaned_weights = ef.clean_weights()
print(cleaned_weights)
print(ef.portfolio_performance(verbose=True));