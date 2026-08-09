FROM python:3.9-slim

# Install aiosmtpd
RUN pip install aiosmtpd requests

# Create a non-root user for security (optional, but good practice)
WORKDIR /app
COPY common ./common
COPY email-indirect/service/service.py ./service.py

# Expose the port we defined in the script
EXPOSE 2525

# Run unbuffered to see logs in K8s immediately
CMD ["python", "-u", "service.py"]