FROM python:3.11-slim

RUN useradd -m -u 10001 appuser

WORKDIR /app

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      build-essential gcc gfortran libopenblas-dev libblas-dev ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Copy requirements and PRINT IT (this is the key)
COPY requirements.txt /app/requirements.txt

RUN set -eux; \
    echo "=== /app/requirements.txt ==="; \
    ls -lah /app/requirements.txt; \
    sed -n '1,200p' /app/requirements.txt; \
    python -V; \
    python -m pip -V; \
    python -m pip install --no-cache-dir --upgrade pip setuptools wheel; \
    python -m pip install --no-cache-dir -r /app/requirements.txt -vv; \
    python -c "import numpy as np; print('NUMPY OK', np.__version__)"

COPY main.py /app/main.py
COPY static /app/static

RUN chown -R appuser:appuser /app

USER appuser

EXPOSE 8080
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "8080"]