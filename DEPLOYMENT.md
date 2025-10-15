# Deployment Guide

This guide covers deploying the vCard QR Generator application to production using Docker.

## Prerequisites

- Docker Engine 20.10 or higher
- Docker Compose 2.0 or higher (for compose deployment)
- 512MB RAM minimum (1GB recommended)
- 1GB disk space for images and database

## Quick Start with Docker Compose

The easiest way to deploy is using Docker Compose:

```bash
# Build and start the service
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the service
docker-compose down

# Stop and remove volumes (WARNING: deletes all data)
docker-compose down -v
```

The application will be available at `http://localhost:3000`

## Building the Docker Image

### Build locally

```bash
docker build -t vcard-qr-generator:latest .
```

### Build for specific platform

```bash
# For ARM64 (Apple Silicon, AWS Graviton)
docker build --platform linux/arm64 -t vcard-qr-generator:latest .

# For AMD64 (Intel/AMD processors)
docker build --platform linux/amd64 -t vcard-qr-generator:latest .
```

## Running with Docker

### Basic run

```bash
docker run -d \
  --name vcard-qr-generator \
  -p 3000:3000 \
  -v vcard-data:/app/data \
  vcard-qr-generator:latest
```

### Run with custom configuration

```bash
docker run -d \
  --name vcard-qr-generator \
  -p 8080:8080 \
  -v $(pwd)/data:/app/data \
  -e PORT=8080 \
  -e DATABASE_PATH=/app/data/vcards.db \
  -e SESSION_EXPIRY_HOURS=48 \
  -e RUST_LOG=info \
  vcard-qr-generator:latest
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Server bind address (use 0.0.0.0 for Docker) |
| `PORT` | `3000` | Server port |
| `DATABASE_PATH` | `vcards.db` | Path to SQLite database file |
| `SESSION_EXPIRY_HOURS` | `24` | Session inactivity timeout in hours |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |

## Data Persistence

The application stores all data in a SQLite database. To persist data:

### Using named volumes (recommended)

```bash
docker volume create vcard-data
docker run -v vcard-data:/app/data vcard-qr-generator:latest
```

### Using bind mounts

```bash
mkdir -p ./data
docker run -v $(pwd)/data:/app/data vcard-qr-generator:latest
```

## Production Deployment

### 1. Update docker-compose.yml for production

```yaml
version: '3.8'

services:
  vcard-qr-generator:
    build:
      context: .
      dockerfile: Dockerfile
    image: vcard-qr-generator:latest
    container_name: vcard-qr-generator
    ports:
      - "3000:3000"
    volumes:
      - vcard-data:/app/data
    environment:
      - RUST_LOG=warn  # Reduce log verbosity
      - DATABASE_PATH=/app/data/vcards.db
      - SESSION_EXPIRY_HOURS=12
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/login"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 5s
    networks:
      - vcard-network

volumes:
  vcard-data:
    driver: local

networks:
  vcard-network:
    driver: bridge
```

### 2. Set up SSL/HTTPS with Nginx

Uncomment the nginx service in `docker-compose.yml` and create `nginx.conf`:

```nginx
events {
    worker_connections 1024;
}

http {
    upstream vcard_app {
        server vcard-qr-generator:3000;
    }

    server {
        listen 80;
        server_name your-domain.com;
        return 301 https://$server_name$request_uri;
    }

    server {
        listen 443 ssl http2;
        server_name your-domain.com;

        ssl_certificate /etc/nginx/ssl/cert.pem;
        ssl_certificate_key /etc/nginx/ssl/key.pem;

        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;

        location / {
            proxy_pass http://vcard_app;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }
    }
}
```

Place SSL certificates in `./ssl/` directory.

### 3. Security best practices

- **Change default admin password immediately** after first login
- Use strong passwords for all accounts
- Keep the Docker image updated with security patches
- Use a reverse proxy (nginx, traefik) for SSL termination
- Restrict network access using firewall rules
- Regularly backup the database volume
- Set appropriate resource limits in docker-compose.yml:

```yaml
services:
  vcard-qr-generator:
    # ... other config ...
    deploy:
      resources:
        limits:
          cpus: '1'
          memory: 512M
        reservations:
          cpus: '0.5'
          memory: 256M
```

## Backup and Restore

### Backup database

```bash
# Using docker-compose
docker-compose exec vcard-qr-generator sqlite3 /app/data/vcards.db ".backup /app/data/backup.db"
docker cp vcard-qr-generator:/app/data/backup.db ./backup-$(date +%Y%m%d).db

# Using named volume
docker run --rm -v vcard-data:/data -v $(pwd):/backup alpine cp /data/vcards.db /backup/backup.db
```

### Restore database

```bash
# Stop the service
docker-compose down

# Copy backup to volume
docker run --rm -v vcard-data:/data -v $(pwd):/backup alpine cp /backup/backup.db /data/vcards.db

# Start the service
docker-compose up -d
```

## Monitoring

### View logs

```bash
# All logs
docker-compose logs -f

# Last 100 lines
docker-compose logs --tail=100 -f

# Specific service
docker logs vcard-qr-generator -f
```

### Health check

```bash
# Check container status
docker ps

# Check health endpoint
curl -f http://localhost:3000/login

# Check from inside container
docker exec vcard-qr-generator curl -f http://localhost:3000/login
```

## Troubleshooting

### Container won't start

1. Check logs: `docker-compose logs`
2. Verify port 3000 is not in use: `netstat -tuln | grep 3000`
3. Ensure database directory has correct permissions

### Database permission errors

```bash
# Fix permissions on bind-mounted volume
sudo chown -R 1000:1000 ./data
```

### Can't login with admin credentials

```bash
# Reset admin password
docker exec -it vcard-qr-generator sqlite3 /app/data/vcards.db
sqlite> UPDATE users SET password_hash = '$2b$12$21yrV/a7WOeMgVekvZMgB.VaT/2HyYU3OBnfFpyFDaHH3ewoIlHKi' WHERE username = 'admin';
sqlite> .quit
```

This resets the admin password to `admin`.

### Out of memory errors

Increase Docker memory limits or add swap space:

```yaml
services:
  vcard-qr-generator:
    mem_limit: 1g
    mem_reservation: 512m
```

## Scaling Considerations

This application uses SQLite and is designed for single-instance deployment. For high-traffic scenarios:

1. Use a CDN for static assets
2. Implement rate limiting at the reverse proxy level
3. Consider migrating to PostgreSQL for better concurrency
4. Use Redis for session storage instead of SQLite

## Default Credentials

**Username:** `admin`
**Password:** `admin`

**⚠️ IMPORTANT:** Change the default password immediately after first login via the Profile page.

## Updating the Application

```bash
# Pull latest code
git pull

# Rebuild and restart
docker-compose build
docker-compose up -d

# Or with no downtime
docker-compose up -d --build --no-deps vcard-qr-generator
```

## Support

For issues and questions:
- Check the main README.md for feature documentation
- Review logs for error messages
- Ensure all environment variables are correctly set
- Verify network connectivity and firewall rules
