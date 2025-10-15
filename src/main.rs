use axum::{
    extract::Json,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use image::{ImageBuffer, Luma, DynamicImage, ImageFormat};
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use tower_http::services::ServeDir;

#[derive(Deserialize)]
struct VCardData {
    first_name: String,
    last_name: String,
    mobile: Option<String>,
    work: Option<String>,
    email: Option<String>,
    company: Option<String>,
    role: Option<String>,
    street: Option<String>,
    city: Option<String>,
    state: Option<String>,
    website: Option<String>,
    color: Option<String>,
}

#[derive(Serialize)]
struct QrResponse {
    image: String, // base64 encoded
}

fn generate_vcard(data: &VCardData) -> String {
    let mut vcard = String::from("BEGIN:VCARD\nVERSION:3.0\n");
    
    // Name
    vcard.push_str(&format!("FN:{} {}\n", data.first_name, data.last_name));
    vcard.push_str(&format!("N:{};{};;;\n", data.last_name, data.first_name));
    
    // Phone numbers
    if let Some(mobile) = &data.mobile {
        if !mobile.is_empty() {
            vcard.push_str(&format!("TEL;TYPE=CELL:{}\n", mobile));
        }
    }
    if let Some(work) = &data.work {
        if !work.is_empty() {
            vcard.push_str(&format!("TEL;TYPE=WORK:{}\n", work));
        }
    }
    
    // Email
    if let Some(email) = &data.email {
        if !email.is_empty() {
            vcard.push_str(&format!("EMAIL:{}\n", email));
        }
    }
    
    // Organization
    if let Some(company) = &data.company {
        if !company.is_empty() {
            vcard.push_str(&format!("ORG:{}\n", company));
        }
    }
    if let Some(role) = &data.role {
        if !role.is_empty() {
            vcard.push_str(&format!("TITLE:{}\n", role));
        }
    }
    
    // Address
    let has_address = data.street.as_ref().map_or(false, |s| !s.is_empty())
        || data.city.as_ref().map_or(false, |s| !s.is_empty())
        || data.state.as_ref().map_or(false, |s| !s.is_empty());

    if has_address {
        vcard.push_str(&format!("ADR;TYPE=WORK:;;{};{};{};;;\n",
            data.street.as_ref().unwrap_or(&String::new()),
            data.city.as_ref().unwrap_or(&String::new()),
            data.state.as_ref().unwrap_or(&String::new())
        ));
    }
    
    // Website
    if let Some(website) = &data.website {
        if !website.is_empty() {
            vcard.push_str(&format!("URL:{}\n", website));
        }
    }
    
    vcard.push_str("END:VCARD");
    vcard
}

fn parse_color(color_str: &str) -> (u8, u8, u8) {
    let hex = color_str.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        (r, g, b)
    } else {
        (0, 0, 0)
    }
}

async fn generate_qr(Json(data): Json<VCardData>) -> Result<Json<QrResponse>, StatusCode> {
    let vcard_content = generate_vcard(&data);
    
    let code = QrCode::new(vcard_content.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let qr_image = code.render::<Luma<u8>>().build();
    
    // Convert to colored image if color is specified
    let dynamic_img = if let Some(color_str) = &data.color {
        let (r, g, b) = parse_color(color_str);
        let width = qr_image.width();
        let height = qr_image.height();
        let rgb_img = ImageBuffer::from_fn(width, height, |x, y| {
            let pixel = qr_image.get_pixel(x, y);
            if pixel[0] == 0 {
                image::Rgb([r, g, b])
            } else {
                image::Rgb([255, 255, 255])
            }
        });
        DynamicImage::ImageRgb8(rgb_img)
    } else {
        DynamicImage::ImageLuma8(qr_image)
    };
    
    // Encode to PNG
    let mut buffer = Cursor::new(Vec::new());
    dynamic_img.write_to(&mut buffer, ImageFormat::Png)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let base64_img = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, buffer.into_inner());
    
    Ok(Json(QrResponse {
        image: format!("data:image/png;base64,{}", base64_img),
    }))
}

async fn serve_index() -> Response {
    let html = include_str!("../static/index.html");
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], html).into_response()
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/generate", post(generate_qr))
        .nest_service("/static", ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    
    println!("Server running on http://127.0.0.1:3000");
    
    axum::serve(listener, app).await.unwrap();
}
