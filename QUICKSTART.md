# Quick Start Guide

## Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Run Application
```bash
cd vcard-qr-generator
cargo run --release
```

## Access
Open: http://127.0.0.1:3000

## How It Works

### Architecture
```
User Form → Axum Server → Generate vCard → Create QR → Return Base64 PNG
```

### Flow
1. User fills contact form
2. JavaScript sends POST to `/api/generate`
3. Rust generates vCard string (RFC 2426)
4. QR code created from vCard
5. Image converted to PNG + base64
6. Frontend displays + download option

### Key Files
- `src/main.rs` - Server + QR logic (170 lines)
- `static/index.html` - UI (210 lines)
- `Cargo.toml` - Dependencies (9 crates)

### Customize
- **Port**: Line 146 in main.rs
- **Colors**: HTML color picker or via API
- **Format**: Change ImageFormat::Png to Jpeg/Webp
- **Size**: Modify qr_image render scale

### Extend
- Add logo overlay (image crate)
- SVG/EPS export (svg crate)
- Database storage (sqlx)
- Authentication (JWT)
