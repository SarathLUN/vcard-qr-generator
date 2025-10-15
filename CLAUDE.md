# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A full-stack Rust application that generates vCard QR codes with SQLite persistence. The backend uses Axum for the web server, SQLx for database operations, and the frontend is a single HTML file with vanilla JavaScript. The app accepts contact information, saves it to a SQLite database, generates a vCard 3.0 format string, creates a QR code (with optional color customization), and returns it as a base64-encoded PNG.

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

- `src/main.rs` - Backend server with database integration (~270 lines)
- `static/index.html` - Frontend UI served via `include_str!`
- `Cargo.toml` - Dependency configuration
- `migrations/` - Database migration SQL files
  - `001_create_vcards_table.sql` - Initial table schema
- `vcards.db` - SQLite database (created on first run, not in version control)
- `DATABASE.md` - Comprehensive database documentation

## Architecture

### Request Flow
1. User submits form → JavaScript POST to `/api/generate`
2. Axum handler `generate_qr()` deserializes JSON to `VCardData` struct
3. **Data is saved to SQLite database via SQLx**
4. `generate_vcard()` builds vCard 3.0 string from contact fields
5. QR code generated using `qrcode` crate
6. Image colorized (if color specified) using `image` crate
7. PNG encoded and base64-wrapped as data URI
8. Frontend displays and provides download

### Key Functions in main.rs

- `generate_vcard(&VCardData) -> String`: Constructs vCard 3.0 format string with conditional field inclusion
- `parse_color(&str) -> (u8, u8, u8)`: Converts hex color strings to RGB tuples
- `generate_qr(State<SqlitePool>, Json<VCardData>) -> Result<Json<QrResponse>, StatusCode>`: Main API handler that saves to DB and orchestrates QR generation
- `serve_index()`: Serves the embedded HTML file
- `init_database() -> Result<SqlitePool, Error>`: Creates database if needed, connects, and returns connection pool
- `run_migrations(&SqlitePool) -> Result<(), Error>`: Applies pending SQL migrations from the migrations/ directory

### API Contract

**POST /api/generate**
- Request: JSON with contact fields (first_name, last_name required; mobile, work, email, company, role, street, city, state, website, color optional)
- Side effect: Saves vCard data to SQLite database
- Response: `{ "image": "data:image/png;base64,..." }`
- Error: Returns 500 StatusCode on database error, QR generation failure, or image encoding failure

### vCard Format

Generates vCard 3.0 (RFC 2426) with:
- FN (formatted name) and N (structured name)
- TEL fields with TYPE=CELL/WORK
- EMAIL, ORG, TITLE, URL
- ADR with TYPE=WORK (format: ;;street;city;state;;; - note: no zip/country fields)

Fields are only included if non-empty strings are provided.

### Database Schema

**vcards table:**
- Stores all submitted vCard data with timestamps
- Fields: id, first_name, last_name, mobile, work, email, company, role, street, city, state, website, color, created_at, updated_at
- Indexes on created_at and email for query performance

**migrations table:**
- Tracks applied database migrations
- Fields: id, name (unique), applied_at

See `DATABASE.md` for complete documentation.

## Dependencies

- `axum` 0.7 - Web framework with extractors and routing
- `tokio` 1.0 - Async runtime (features = ["full"])
- `tower-http` 0.5 - Middleware (uses ServeDir, CORS features)
- `sqlx` 0.8 - Async SQL toolkit with compile-time checked queries (features: runtime-tokio, sqlite)
- `chrono` 0.4 - Date and time library (with serde support)
- `qrcode` 0.14 - QR code generation
- `image` 0.25 - Image manipulation and format encoding
- `base64` 0.22 - Base64 encoding
- `serde` + `serde_json` 1.0 - JSON serialization

## Customization Points

- **Port**: Change `127.0.0.1:3000` in the main() function
- **Database location**: Change `"sqlite://vcards.db"` in init_database()
- **QR size**: Modify `.render()` call in generate_qr() to add `.max_dimensions()` or `.min_dimensions()`
- **Image format**: Change `ImageFormat::Png` to `ImageFormat::Jpeg` or others
- **Color logic**: Modify colorization in generate_qr() (currently inverts: black pixels → color, white pixels → white)

## Database Operations

### Migration Strategy

Migrations run automatically on application startup:
1. Database file created if it doesn't exist
2. Migrations table created/checked
3. Pending migrations applied in order
4. Each migration recorded to prevent re-application

To add a new migration:
1. Create `migrations/NNN_description.sql`
2. Add entry to the `migrations` vector in `run_migrations()`
3. Restart application to apply

### Querying Data

```bash
# Open database
sqlite3 vcards.db

# View all vcards
SELECT * FROM vcards ORDER BY created_at DESC;

# Count records
SELECT COUNT(*) FROM vcards;
```
