# vCard QR Code Generator - Rust Clone

A full-stack Rust application that generates QR codes from vCard contact information.

## Features

- ✅ Generate vCard QR codes with contact details
- ✅ Support for multiple contact fields (name, phone, email, address, company)
- ✅ Custom QR code colors
- ✅ Download QR codes as PNG
- ✅ Responsive web interface
- ✅ Real-time QR generation

## Tech Stack

- **Backend**: Axum (async Rust web framework)
- **QR Generation**: qrcode crate
- **Image Processing**: image crate
- **Frontend**: Vanilla HTML/CSS/JavaScript

## Installation

1. Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2. Clone/navigate to project:
```bash
cd vcard-qr-generator
```

3. Build and run:
```bash
cargo build --release
cargo run --release
```

4. Open browser:
```
http://127.0.0.1:3000
```

## Usage

1. Fill in contact information:
   - Required: First Name, Last Name
   - Optional: Phone numbers, email, company, address, website
   
2. Choose QR code color (default: black)

3. Click "Generate QR Code"

4. Download the generated QR code

## API Endpoint

**POST** `/api/generate`

Request body:
```json
{
  "first_name": "John",
  "last_name": "Doe",
  "mobile": "+1234567890",
  "email": "john@example.com",
  "company": "Tech Corp",
  "role": "Software Engineer",
  "website": "https://johndoe.com",
  "color": "#000000"
}
```

Response:
```json
{
  "image": "data:image/png;base64,..."
}
```

## Project Structure

```
vcard-qr-generator/
├── Cargo.toml          # Dependencies
├── src/
│   └── main.rs         # Backend server
└── static/
    └── index.html      # Frontend UI
```

## Key Components

### Backend (main.rs)
- **generate_vcard()**: Creates vCard 3.0 format string
- **generate_qr()**: Generates QR code with color support
- **serve_index()**: Serves HTML frontend

### Frontend (index.html)
- Responsive form for contact data
- Color picker for QR customization
- Real-time QR generation
- Download functionality

## Dependencies

```toml
axum = "0.7"              # Web framework
tokio = "1"               # Async runtime
qrcode = "0.14"           # QR code generation
image = "0.25"            # Image processing
base64 = "0.22"           # Base64 encoding
serde = "1"               # Serialization
```

## Development

Run in dev mode:
```bash
cargo run
```

Build optimized:
```bash
cargo build --release
./target/release/vcard-qr-generator
```

## License

MIT
