FROM python:3.11-slim

RUN pip install flask requests

WORKDIR /app
COPY common ./common
COPY llm_comparison/service/models.json ./models.json
COPY llm_comparison/service/service.py ./service.py

EXPOSE 5000

CMD ["python", "-u", "service.py"]
