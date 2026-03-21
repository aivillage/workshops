FROM python:3.11-slim

WORKDIR /app

RUN pip install --no-cache-dir \
    flask==3.1.* \
    transformers>=4.49.0 \
    numpy==2.1.* \
    accelerate>=1.13.0


RUN pip install --no-cache-dir \
    torch==2.5.* --index-url https://download.pytorch.org/whl/cpu

COPY app.py .
COPY static/ static/
COPY gemma-3-270m/ gemma-3-270m/

ENV MODEL_PATH=/app/gemma-3-270m
ENV PORT=8080

EXPOSE 8080

CMD ["python", "app.py"]