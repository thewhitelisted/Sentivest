from transformers import AutoModelForSequenceClassification, AutoTokenizer
import torch
import yfinance as yf
from GoogleNews import GoogleNews
from newspaper import Article

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

    print(len(news_results))
    
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

news_articles = get_full_news_articles("MSFT")
sentiment_results = analyze_sentiment(news_articles)
for s in sentiment_results:
    print(s)
