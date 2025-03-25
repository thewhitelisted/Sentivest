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
        
        # Truncate extremely long texts to prevent memory issues
        max_length = 512
        truncated_text = text[:50000] if len(text) > 50000 else text
        
        inputs = self.tokenizer(truncated_text, return_tensors="pt", truncation=True, padding=True, max_length=max_length)
        
        with torch.no_grad():
            logits = self.model(**inputs).logits
        
        probabilities = torch.nn.functional.softmax(logits, dim=-1)
        
        # Convert probabilities to a list
        return probabilities[0].tolist()  # Returns [negative, neutral, positive] scores

if __name__ == "__main__":
    finbert = FinBERTSentiment()
    text = "I love this! I love this so so so so soso sos so sos osos sosos much!!!!! I love everything so much~!!!!! I love this so much! WOWOWOWOWOWOWIE EPICCC I LOVE THIS SO MUCH"
    print(finbert.analyze_sentiment(text))  # [0.0002, 0.0001, 0.9997]