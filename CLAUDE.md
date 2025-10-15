# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A full-stack Rust application that generates vCard QR codes. The backend uses Axum for the web server, and the frontend is a single HTML file with vanilla JavaScript. The app accepts contact information, generates a vCard 3.0 format string, creates a QR code (with optional color customization), and returns it as a base64-encoded PNG.

## Build and Run Commands

```bash
# Development mode (with debug symbols, faster compilation)
cargo run

# Release mode (optimized, slower compilation but faster runtime)
cargo build --release
cargo run --release

# Build only (without running)
cargo build

# Check code without building
cargo check
```

The server runs on `http://127.0.0.1:3000` by default.

## Project Structure

- `main.rs` - Single-file backend containing the entire server logic (178 lines)
- `index.html` or `static/index.html` - Frontend UI (currently at root, served via `include_str!`)
- `Cargo.toml` - Dependency configuration

Note: The HTML file is currently at the root as `index.html` but is loaded via `include_str!("../static/index.html")` in main.rs:160, so it should be moved to a `static/` directory for the code to work correctly.

## Architecture

### Request Flow
1. User submits form → JavaScript POST to `/api/generate`
2. Axum handler `generate_qr()` deserializes JSON to `VCardData` struct
3. `generate_vcard()` builds vCard 3.0 string from contact fields
4. QR code generated using `qrcode` crate
5. Image colorized (if color specified) using `image` crate
6. PNG encoded and base64-wrapped as data URI
7. Frontend displays and provides download

### Key Functions in main.rs

- `generate_vcard(&VCardData) -> String` (lines 38-107): Constructs vCard 3.0 format string with conditional field inclusion
- `parse_color(&str) -> (u8, u8, u8)` (lines 109-119): Converts hex color strings to RGB tuples
- `generate_qr(Json<VCardData>) -> Result<Json<QrResponse>, StatusCode>` (lines 121-157): Main API handler that orchestrates QR generation
- `serve_index()` (lines 159-162): Serves the embedded HTML file

### API Contract

**POST /api/generate**
- Request: JSON with contact fields (first_name, last_name required; mobile, work, fax, email, company, role, street, city, state, zip, country, website, color optional)
- Response: `{ "image": "data:image/png;base64,..." }`
- Error: Returns 500 StatusCode on QR generation or image encoding failure

### vCard Format

Generates vCard 3.0 (RFC 2426) with:
- FN (formatted name) and N (structured name)
- TEL fields with TYPE=CELL/WORK/FAX
- EMAIL, ORG, TITLE, URL
- ADR with TYPE=WORK (format: ;;street;city;state;zip;country)

Fields are only included if non-empty strings are provided.

## Dependencies

- `axum` 0.7 - Web framework with extractors and routing
- `tokio` 1.0 - Async runtime (features = ["full"])
- `tower-http` 0.5 - Middleware (uses ServeDir, CORS features)
- `qrcode` 0.14 - QR code generation
- `image` 0.25 - Image manipulation and format encoding
- `base64` 0.22 - Base64 encoding
- `serde` + `serde_json` 1.0 - JSON serialization

## Customization Points

- **Port**: Change `127.0.0.1:3000` in main.rs:171
- **QR size**: Modify `.render()` call in generate_qr() to add `.max_dimensions()` or `.min_dimensions()`
- **Image format**: Change `ImageFormat::Png` to `ImageFormat::Jpeg` or others in main.rs:149
- **Color logic**: Modify colorization in main.rs:130-145 (currently inverts: black pixels → color, white pixels → white)
