# Binary Deployment Guide

This guide covers building and deploying the vCard QR Generator as a standalone binary to production servers.

## Prerequisites

### Development Machine
- Rust 1.70+ (install from https://rustup.rs)
- SQLite development libraries
- Git

### Production Server
- Linux server (Ubuntu/Debian recommended)
- SQLite runtime libraries
- Systemd (for service management)
- 512MB RAM minimum (1GB recommended)
- 100MB disk space

## Building the Binary

### Option 1: Build on Development Machine

#### Install Rust and Dependencies

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y build-essential libsqlite3-dev pkg-config

# macOS
brew install sqlite3

# Fedora/RHEL
sudo dnf install -y gcc sqlite-devel
```

#### Build Release Binary

```bash
# Clone repository (if not already)
git clone <repository-url>
cd vcard-qr-generator

# Build optimized release binary
cargo build --release

# Binary location
ls -lh target/release/vcard-qr-generator
```

The release binary will be at `target/release/vcard-qr-generator` (~15-20MB).

### Option 2: Cross-Compile for Linux (from macOS/Windows)

```bash
# Install cross-compilation target
rustup target add x86_64-unknown-linux-gnu

# Ubuntu/Debian - install cross-compiler
sudo apt-get install gcc-x86_64-linux-gnu

# macOS - install cross-compiler
brew install FiloSottile/musl-cross/musl-cross

# Build for Linux
cargo build --release --target x86_64-unknown-linux-gnu
```

### Option 3: Build Statically Linked Binary (Portable)

For maximum portability, build a statically linked binary:

```bash
# Install musl target
rustup target add x86_64-unknown-linux-musl

# Ubuntu/Debian
sudo apt-get install musl-tools

# Build static binary
cargo build --release --target x86_64-unknown-linux-musl

# Verify it's static
ldd target/x86_64-unknown-linux-musl/release/vcard-qr-generator
# Should output: "not a dynamic executable"
```

### Optimize Binary Size (Optional)

Add to `Cargo.toml`:

```toml
[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Strip symbols
```

Then rebuild:

```bash
cargo build --release
```

## Preparing for Production

### 1. Create Deployment Package

```bash
# Create deployment directory
mkdir -p vcard-qr-deploy
cd vcard-qr-deploy

# Copy binary
cp ../target/release/vcard-qr-generator .

# Copy static files and migrations
cp -r ../static .
cp -r ../migrations .

# Create data directory
mkdir -p data

# Create archive
cd ..
tar -czf vcard-qr-generator.tar.gz vcard-qr-deploy/
```

### 2. Transfer to Production Server

```bash
# Using scp
scp vcard-qr-generator.tar.gz user@production-server:/tmp/

# Or using rsync (better for updates)
rsync -avz vcard-qr-deploy/ user@production-server:/opt/vcard-qr/
```

## Production Server Setup

### 1. Install Runtime Dependencies

```bash
# SSH into production server
ssh user@production-server

# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y libsqlite3-0 ca-certificates curl

# Fedora/RHEL
sudo dnf install -y sqlite-libs ca-certificates curl

# Alpine Linux
sudo apk add --no-cache sqlite-libs ca-certificates libgcc
```

### 2. Setup Application User

```bash
# Create dedicated user (no login shell for security)
sudo useradd -r -s /bin/false -d /opt/vcard-qr vcard-qr

# Create application directory
sudo mkdir -p /opt/vcard-qr/{data,logs}
sudo chown -R vcard-qr:vcard-qr /opt/vcard-qr
```

### 3. Extract and Setup

```bash
# Extract archive
cd /opt/vcard-qr
sudo tar -xzf /tmp/vcard-qr-generator.tar.gz --strip-components=1

# Set permissions
sudo chown -R vcard-qr:vcard-qr /opt/vcard-qr
sudo chmod +x /opt/vcard-qr/vcard-qr-generator

# Create environment file
sudo tee /opt/vcard-qr/.env > /dev/null <<'EOF'
HOST=0.0.0.0
PORT=3000
DATABASE_PATH=/opt/vcard-qr/data/vcards.db
SESSION_EXPIRY_HOURS=24
RUST_LOG=info
EOF

sudo chown vcard-qr:vcard-qr /opt/vcard-qr/.env
sudo chmod 600 /opt/vcard-qr/.env
```

### 4. Create Systemd Service

```bash
# Create service file
sudo tee /etc/systemd/system/vcard-qr.service > /dev/null <<'EOF'
[Unit]
Description=vCard QR Generator
After=network.target
Documentation=https://github.com/your-org/vcard-qr-generator

[Service]
Type=simple
User=vcard-qr
Group=vcard-qr
WorkingDirectory=/opt/vcard-qr
EnvironmentFile=/opt/vcard-qr/.env
ExecStart=/opt/vcard-qr/vcard-qr-generator
Restart=on-failure
RestartSec=5s
StandardOutput=append:/opt/vcard-qr/logs/output.log
StandardError=append:/opt/vcard-qr/logs/error.log

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/vcard-qr/data /opt/vcard-qr/logs
CapabilityBoundingSet=

# Resource limits
LimitNOFILE=65536
TasksMax=512

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable vcard-qr

# Start service
sudo systemctl start vcard-qr

# Check status
sudo systemctl status vcard-qr
```

### 5. Setup Log Rotation

```bash
# Create logrotate configuration
sudo tee /etc/logrotate.d/vcard-qr > /dev/null <<'EOF'
/opt/vcard-qr/logs/*.log {
    daily
    rotate 14
    compress
    delaycompress
    missingok
    notifempty
    create 0640 vcard-qr vcard-qr
    sharedscripts
    postrotate
        systemctl reload vcard-qr > /dev/null 2>&1 || true
    endscript
}
EOF
```

## Reverse Proxy Setup

### Option 1: Nginx

```bash
# Install nginx
sudo apt-get install -y nginx

# Create site configuration
sudo tee /etc/nginx/sites-available/vcard-qr > /dev/null <<'EOF'
server {
    listen 80;
    server_name yourdomain.com;

    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name yourdomain.com;

    # SSL certificates (use certbot/letsencrypt)
    ssl_certificate /etc/letsencrypt/live/yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/yourdomain.com/privkey.pem;

    # SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;

    # Proxy settings
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api:10m rate=10r/s;
    location /api/ {
        limit_req zone=api burst=20 nodelay;
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Access and error logs
    access_log /var/log/nginx/vcard-qr-access.log;
    error_log /var/log/nginx/vcard-qr-error.log;
}
EOF

# Enable site
sudo ln -s /etc/nginx/sites-available/vcard-qr /etc/nginx/sites-enabled/

# Test configuration
sudo nginx -t

# Restart nginx
sudo systemctl restart nginx
```

### Option 2: Caddy (Automatic HTTPS)

```bash
# Install Caddy
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install caddy

# Create Caddyfile
sudo tee /etc/caddy/Caddyfile > /dev/null <<'EOF'
yourdomain.com {
    reverse_proxy localhost:3000

    # Rate limiting
    rate_limit {
        zone api {
            key {remote_host}
            events 100
            window 1m
        }
    }

    # Security headers
    header {
        Strict-Transport-Security "max-age=31536000;"
        X-Frame-Options "SAMEORIGIN"
        X-Content-Type-Options "nosniff"
    }

    # Logging
    log {
        output file /var/log/caddy/vcard-qr.log
        format json
    }
}
EOF

# Restart Caddy
sudo systemctl restart caddy
```

## Firewall Configuration

```bash
# Using UFW (Ubuntu)
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 22/tcp  # SSH
sudo ufw enable

# Using firewalld (RHEL/Fedora)
sudo firewall-cmd --permanent --add-service=http
sudo firewall-cmd --permanent --add-service=https
sudo firewall-cmd --permanent --add-service=ssh
sudo firewall-cmd --reload
```

## Monitoring and Management

### View Logs

```bash
# Service logs
sudo journalctl -u vcard-qr -f

# Application logs
sudo tail -f /opt/vcard-qr/logs/output.log
sudo tail -f /opt/vcard-qr/logs/error.log

# Last 100 lines
sudo journalctl -u vcard-qr -n 100
```

### Service Management

```bash
# Start service
sudo systemctl start vcard-qr

# Stop service
sudo systemctl stop vcard-qr

# Restart service
sudo systemctl restart vcard-qr

# Reload configuration (if supported)
sudo systemctl reload vcard-qr

# Check status
sudo systemctl status vcard-qr

# Check if enabled on boot
sudo systemctl is-enabled vcard-qr
```

### Check Application Health

```bash
# Test endpoint
curl -f http://localhost:3000/login

# Test with headers
curl -I http://localhost:3000/login

# Test login API
curl -X POST http://localhost:3000/api/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin"}'
```

## Backup and Restore

### Backup

```bash
# Create backup script
sudo tee /opt/vcard-qr/backup.sh > /dev/null <<'EOF'
#!/bin/bash
BACKUP_DIR="/opt/vcard-qr/backups"
DATE=$(date +%Y%m%d_%H%M%S)
DB_PATH="/opt/vcard-qr/data/vcards.db"

mkdir -p "$BACKUP_DIR"

# Backup database
sqlite3 "$DB_PATH" ".backup '$BACKUP_DIR/vcards_$DATE.db'"

# Compress old backups (older than 1 day)
find "$BACKUP_DIR" -name "*.db" -mtime +1 -exec gzip {} \;

# Delete backups older than 30 days
find "$BACKUP_DIR" -name "*.db.gz" -mtime +30 -delete

echo "Backup completed: vcards_$DATE.db"
EOF

sudo chmod +x /opt/vcard-qr/backup.sh
sudo chown vcard-qr:vcard-qr /opt/vcard-qr/backup.sh

# Setup cron job for daily backups
sudo crontab -u vcard-qr -e
# Add this line:
# 0 2 * * * /opt/vcard-qr/backup.sh >> /opt/vcard-qr/logs/backup.log 2>&1
```

### Restore

```bash
# Stop service
sudo systemctl stop vcard-qr

# Restore database
sudo -u vcard-qr cp /opt/vcard-qr/backups/vcards_YYYYMMDD_HHMMSS.db /opt/vcard-qr/data/vcards.db

# Or if compressed
sudo -u vcard-qr gunzip -c /opt/vcard-qr/backups/vcards_YYYYMMDD_HHMMSS.db.gz > /opt/vcard-qr/data/vcards.db

# Start service
sudo systemctl start vcard-qr
```

## Updating the Application

### 1. Build New Version

```bash
# On development machine
git pull origin main
cargo build --release
tar -czf vcard-qr-generator-v2.tar.gz target/release/vcard-qr-generator static/ migrations/
```

### 2. Deploy Update

```bash
# Transfer to server
scp vcard-qr-generator-v2.tar.gz user@production-server:/tmp/

# On production server
sudo systemctl stop vcard-qr

# Backup current binary
sudo cp /opt/vcard-qr/vcard-qr-generator /opt/vcard-qr/vcard-qr-generator.backup

# Extract new version
cd /opt/vcard-qr
sudo tar -xzf /tmp/vcard-qr-generator-v2.tar.gz --strip-components=1

# Set permissions
sudo chown -R vcard-qr:vcard-qr /opt/vcard-qr
sudo chmod +x /opt/vcard-qr/vcard-qr-generator

# Start service
sudo systemctl start vcard-qr

# Check status
sudo systemctl status vcard-qr

# Rollback if needed
# sudo cp /opt/vcard-qr/vcard-qr-generator.backup /opt/vcard-qr/vcard-qr-generator
# sudo systemctl restart vcard-qr
```

### 3. Zero-Downtime Deployment (Advanced)

```bash
# Use blue-green deployment or rolling updates
# Run two instances on different ports with load balancer
```

## Performance Tuning

### System Settings

```bash
# Increase file descriptor limits
sudo tee -a /etc/security/limits.conf > /dev/null <<'EOF'
vcard-qr soft nofile 65536
vcard-qr hard nofile 65536
EOF

# Optimize kernel parameters
sudo tee -a /etc/sysctl.conf > /dev/null <<'EOF'
# TCP tuning
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 8192
net.ipv4.ip_local_port_range = 1024 65535
EOF

sudo sysctl -p
```

### Database Optimization

```bash
# SQLite PRAGMA settings (add to migrations or startup)
sqlite3 /opt/vcard-qr/data/vcards.db <<'EOF'
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA cache_size=-64000;
PRAGMA temp_store=MEMORY;
PRAGMA mmap_size=30000000000;
EOF
```

## Security Hardening

### 1. Change Default Password

```bash
# First login and change via web interface at /profile
# Or via database:
sqlite3 /opt/vcard-qr/data/vcards.db
> UPDATE users SET password_hash = '$2b$12$NEW_HASH' WHERE username = 'admin';
```

### 2. Setup Fail2Ban

```bash
# Install fail2ban
sudo apt-get install -y fail2ban

# Create filter for vcard-qr
sudo tee /etc/fail2ban/filter.d/vcard-qr.conf > /dev/null <<'EOF'
[Definition]
failregex = .*"error":".*".*from <HOST>
ignoreregex =
EOF

# Create jail
sudo tee /etc/fail2ban/jail.d/vcard-qr.conf > /dev/null <<'EOF'
[vcard-qr]
enabled = true
filter = vcard-qr
logpath = /opt/vcard-qr/logs/output.log
maxretry = 5
bantime = 3600
findtime = 600
EOF

sudo systemctl restart fail2ban
```

## Troubleshooting

### Service Won't Start

```bash
# Check logs
sudo journalctl -u vcard-qr -n 50

# Check binary permissions
ls -la /opt/vcard-qr/vcard-qr-generator

# Check dependencies
ldd /opt/vcard-qr/vcard-qr-generator

# Test manually
sudo -u vcard-qr /opt/vcard-qr/vcard-qr-generator
```

### Database Errors

```bash
# Check database permissions
ls -la /opt/vcard-qr/data/vcards.db

# Check database integrity
sqlite3 /opt/vcard-qr/data/vcards.db "PRAGMA integrity_check;"

# Reset migrations (dangerous - only for development)
# sqlite3 /opt/vcard-qr/data/vcards.db "DELETE FROM migrations;"
```

### High Memory Usage

```bash
# Check memory
ps aux | grep vcard-qr-generator

# Add memory limit to systemd service
sudo systemctl edit vcard-qr
# Add:
# [Service]
# MemoryMax=512M
# MemoryHigh=384M
```

### Connection Issues

```bash
# Check if service is listening
sudo netstat -tulpn | grep 3000
# or
sudo ss -tulpn | grep 3000

# Check from localhost
curl -v http://localhost:3000/login

# Check from external
curl -v http://your-server-ip:3000/login
```

## Production Checklist

Before going live:

- [ ] Change default admin password
- [ ] Configure SSL/HTTPS
- [ ] Setup firewall rules
- [ ] Configure backup automation
- [ ] Setup log rotation
- [ ] Configure monitoring/alerts
- [ ] Test disaster recovery
- [ ] Document server access procedures
- [ ] Setup reverse proxy with rate limiting
- [ ] Verify database backups are working
- [ ] Test application updates procedure
- [ ] Review security settings
- [ ] Setup fail2ban or similar
- [ ] Configure appropriate session timeout
- [ ] Test health checks
- [ ] Verify systemd service auto-starts on reboot

## Support

For issues:
- Check logs: `sudo journalctl -u vcard-qr -f`
- Check application logs: `/opt/vcard-qr/logs/`
- Verify database: `sqlite3 /opt/vcard-qr/data/vcards.db`
- Test manually: `sudo -u vcard-qr /opt/vcard-qr/vcard-qr-generator`
