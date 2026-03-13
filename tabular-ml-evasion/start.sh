#!/bin/bash

docker compose up --build -d

PORT=$(docker compose port web 8080 | cut -d: -f2)

echo ""
echo "🚀 Challenge running!"
echo ""
echo "Open in browser:"
echo "http://localhost:$PORT"
echo ""