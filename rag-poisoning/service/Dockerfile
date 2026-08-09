FROM python:3.11-slim

RUN pip install flask requests

WORKDIR /app
COPY common ./common
COPY rag-poisoning/service/service.py ./service.py

EXPOSE 5000

CMD ["python", "-u", "service.py"]
