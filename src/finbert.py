from transformers import AutoModelForSequenceClassification, AutoTokenizer
import torch

class FinBERTSentiment:
    def __init__(self):
        model_name = "ProsusAI/finbert"
        self.tokenizer = AutoTokenizer.from_pretrained(model_name)
        self.model = AutoModelForSequenceClassification.from_pretrained(model_name)

    def analyze_sentiment(self, text):
        if len(text) < 150:
            return [0.0, 0.0, 0.0]
        inputs = self.tokenizer(text, return_tensors="pt", truncation=True, padding=True)
        with torch.no_grad():
            logits = self.model(**inputs).logits
        probabilities = torch.nn.functional.softmax(logits, dim=-1)
        
        # Convert probabilities to a list instead of a dictionary
        return probabilities[0].tolist()  # Returns [negative, neutral, positive] scores
