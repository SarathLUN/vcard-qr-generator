use axum::{
    extract::{Json, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use image::{ImageBuffer, Luma, DynamicImage, ImageFormat};
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, migrate::MigrateDatabase, Sqlite};
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

async fn generate_qr(
    State(pool): State<SqlitePool>,
    Json(data): Json<VCardData>,
) -> Result<Json<QrResponse>, StatusCode> {
    // Save to database
    let result = sqlx::query(
        r#"
        INSERT INTO vcards (first_name, last_name, mobile, work, email, company, role, street, city, state, website, color)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(&data.first_name)
    .bind(&data.last_name)
    .bind(&data.mobile)
    .bind(&data.work)
    .bind(&data.email)
    .bind(&data.company)
    .bind(&data.role)
    .bind(&data.street)
    .bind(&data.city)
    .bind(&data.state)
    .bind(&data.website)
    .bind(&data.color)
    .execute(&pool)
    .await;

    if let Err(e) = result {
        eprintln!("Database error: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

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

async fn run_migrations(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // Create migrations table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // List of migrations to apply
    let migrations = vec![
        ("001_create_vcards_table", include_str!("../migrations/001_create_vcards_table.sql")),
    ];

    for (name, sql) in migrations {
        // Check if migration already applied
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM migrations WHERE name = ?")
            .bind(name)
            .fetch_one(pool)
            .await?;

        if exists == 0 {
            println!("Running migration: {}", name);

            // Execute migration SQL
            sqlx::raw_sql(sql).execute(pool).await?;

            // Record migration as applied
            sqlx::query("INSERT INTO migrations (name) VALUES (?)")
                .bind(name)
                .execute(pool)
                .await?;

            println!("✓ Migration {} applied", name);
        } else {
            println!("→ Migration {} already applied", name);
        }
    }

    Ok(())
}

async fn init_database() -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let db_url = "sqlite://vcards.db";

    // Create database if it doesn't exist
    if !Sqlite::database_exists(db_url).await.unwrap_or(false) {
        println!("Creating database...");
        Sqlite::create_database(db_url).await?;
        println!("✓ Database created");
    }

    // Connect to database
    let pool = SqlitePool::connect(db_url).await?;
    println!("✓ Connected to database");

    // Run migrations
    run_migrations(&pool).await?;

    Ok(pool)
}

#[tokio::main]
async fn main() {
    // Initialize database
    let pool = init_database().await.expect("Failed to initialize database");

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/api/generate", post(generate_qr))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}
