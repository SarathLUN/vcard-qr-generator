mod auth;

use axum::{
    extract::{Json, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post, put},
    Router,
};
use image::{ImageBuffer, Luma, DynamicImage, ImageFormat};
use qrcode::QrCode;
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, migrate::MigrateDatabase, Sqlite};
use std::io::Cursor;
use tower_http::services::ServeDir;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use tower_sessions::Session;

use auth::{User, UserInfo, authenticate_user, set_user_session, clear_session, get_current_user, hash_password};

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

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
    password: String,
    is_admin: bool,
}

#[derive(Deserialize)]
struct UpdateUserRequest {
    username: String,
    password: Option<String>,
    is_admin: bool,
}

#[derive(Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
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

// Authentication handlers
async fn login_handler(
    State(pool): State<SqlitePool>,
    session: Session,
    Json(req): Json<LoginRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    match authenticate_user(&pool, &req.username, &req.password).await {
        Ok(user) => {
            set_user_session(&session, &user).await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Session error".to_string() })))?;

            Ok(Json(MessageResponse {
                message: "Login successful".to_string(),
            }))
        }
        Err(e) => Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: e }))),
    }
}

async fn logout_handler(session: Session) -> impl IntoResponse {
    clear_session(&session).await;
    Json(MessageResponse {
        message: "Logged out".to_string(),
    })
}

async fn me_handler(session: Session) -> Result<Json<UserInfo>, StatusCode> {
    match get_current_user(&session).await {
        Some(user) => Ok(Json(user)),
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

async fn change_password_handler(
    State(pool): State<SqlitePool>,
    session: Session,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user_info = get_current_user(&session).await
        .ok_or((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Not authenticated".to_string() })))?;

    // Get full user from database
    let user: User = sqlx::query_as("SELECT id, username, password_hash, is_admin FROM users WHERE id = ?")
        .bind(user_info.id)
        .fetch_one(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Database error".to_string() })))?;

    // Verify current password
    if !auth::verify_password(&req.current_password, &user.password_hash) {
        return Err((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Current password is incorrect".to_string() })));
    }

    // Hash new password
    let new_hash = hash_password(&req.new_password)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Failed to hash password".to_string() })))?;

    // Update password
    sqlx::query("UPDATE users SET password_hash = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&new_hash)
        .bind(user.id)
        .execute(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Failed to update password".to_string() })))?;

    Ok(Json(MessageResponse {
        message: "Password updated successfully".to_string(),
    }))
}

// Admin handlers
async fn get_users_handler(
    State(pool): State<SqlitePool>,
    session: Session,
) -> Result<Json<Vec<UserInfo>>, (StatusCode, Json<ErrorResponse>)> {
    let user = get_current_user(&session).await
        .ok_or((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Not authenticated".to_string() })))?;

    if !user.is_admin {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "Admin access required".to_string() })));
    }

    // Fetch users with created_at for display

    #[derive(Serialize, sqlx::FromRow)]
    struct UserWithDate {
        id: i64,
        username: String,
        is_admin: bool,
        created_at: String,
    }

    let users_with_dates: Vec<UserWithDate> = sqlx::query_as("SELECT id, username, is_admin, created_at FROM users ORDER BY id")
        .fetch_all(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Database error".to_string() })))?;

    Ok(Json(users_with_dates.into_iter().map(|u| UserInfo { id: u.id, username: u.username, is_admin: u.is_admin }).collect()))
}

async fn create_user_handler(
    State(pool): State<SqlitePool>,
    session: Session,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user = get_current_user(&session).await
        .ok_or((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Not authenticated".to_string() })))?;

    if !user.is_admin {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "Admin access required".to_string() })));
    }

    let password_hash = hash_password(&req.password)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Failed to hash password".to_string() })))?;

    sqlx::query("INSERT INTO users (username, password_hash, is_admin) VALUES (?, ?, ?)")
        .bind(&req.username)
        .bind(&password_hash)
        .bind(req.is_admin)
        .execute(&pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                (StatusCode::CONFLICT, Json(ErrorResponse { error: "Username already exists".to_string() }))
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Database error".to_string() }))
            }
        })?;

    Ok(Json(MessageResponse {
        message: "User created successfully".to_string(),
    }))
}

async fn update_user_handler(
    State(pool): State<SqlitePool>,
    session: Session,
    Path(user_id): Path<i64>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user = get_current_user(&session).await
        .ok_or((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Not authenticated".to_string() })))?;

    if !user.is_admin {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "Admin access required".to_string() })));
    }

    // Update username and admin status
    sqlx::query("UPDATE users SET username = ?, is_admin = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&req.username)
        .bind(req.is_admin)
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Failed to update user".to_string() })))?;

    // Update password if provided
    if let Some(password) = req.password {
        if !password.is_empty() {
            let password_hash = hash_password(&password)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Failed to hash password".to_string() })))?;

            sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
                .bind(&password_hash)
                .bind(user_id)
                .execute(&pool)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Failed to update password".to_string() })))?;
        }
    }

    Ok(Json(MessageResponse {
        message: "User updated successfully".to_string(),
    }))
}

async fn delete_user_handler(
    State(pool): State<SqlitePool>,
    session: Session,
    Path(user_id): Path<i64>,
) -> Result<Json<MessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let user = get_current_user(&session).await
        .ok_or((StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Not authenticated".to_string() })))?;

    if !user.is_admin {
        return Err((StatusCode::FORBIDDEN, Json(ErrorResponse { error: "Admin access required".to_string() })));
    }

    // Prevent deleting own account
    if user.id == user_id {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Cannot delete your own account".to_string() })));
    }

    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&pool)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "Failed to delete user".to_string() })))?;

    Ok(Json(MessageResponse {
        message: "User deleted successfully".to_string(),
    }))
}

// VCard generation handler (requires auth)
async fn generate_qr(
    State(pool): State<SqlitePool>,
    session: Session,
    Json(data): Json<VCardData>,
) -> Result<Json<QrResponse>, StatusCode> {
    // Check authentication
    if get_current_user(&session).await.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

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

// Page handlers
async fn serve_index(session: Session) -> Response {
    if get_current_user(&session).await.is_none() {
        return Redirect::to("/login").into_response();
    }
    let html = include_str!("../static/index.html");
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], html).into_response()
}

async fn serve_login() -> Response {
    let html = include_str!("../static/login.html");
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], html).into_response()
}

async fn serve_profile(session: Session) -> Response {
    if get_current_user(&session).await.is_none() {
        return Redirect::to("/login").into_response();
    }
    let html = include_str!("../static/profile.html");
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], html).into_response()
}

async fn serve_admin(session: Session) -> Response {
    match get_current_user(&session).await {
        Some(user) if user.is_admin => {
            let html = include_str!("../static/admin.html");
            (StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], html).into_response()
        }
        Some(_) => (StatusCode::FORBIDDEN, "Admin access required").into_response(),
        None => Redirect::to("/login").into_response(),
    }
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
        ("002_create_users_table", include_str!("../migrations/002_create_users_table.sql")),
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
    // Get database path from environment variable or use default
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "vcards.db".to_string());
    let db_url = format!("sqlite://{}", db_path);

    // Create database if it doesn't exist
    if !Sqlite::database_exists(&db_url).await.unwrap_or(false) {
        println!("Creating database at {}...", db_path);
        Sqlite::create_database(&db_url).await?;
        println!("✓ Database created");
    }

    // Connect to database
    let pool = SqlitePool::connect(&db_url).await?;
    println!("✓ Connected to database at {}", db_path);

    // Run migrations
    run_migrations(&pool).await?;

    Ok(pool)
}

#[tokio::main]
async fn main() {
    // Initialize database
    let pool = init_database().await.expect("Failed to initialize database");

    // Create session store
    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await.expect("Failed to migrate session store");

    // Get session expiry from environment variable (default 24 hours)
    let session_hours = std::env::var("SESSION_EXPIRY_HOURS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(24);

    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(tower_sessions::cookie::time::Duration::hours(session_hours)));

    let app = Router::new()
        // Public routes
        .route("/login", get(serve_login))
        // Protected routes
        .route("/", get(serve_index))
        .route("/profile", get(serve_profile))
        .route("/admin", get(serve_admin))
        // API routes
        .route("/api/login", post(login_handler))
        .route("/api/logout", post(logout_handler))
        .route("/api/me", get(me_handler))
        .route("/api/change-password", post(change_password_handler))
        .route("/api/generate", post(generate_qr))
        // Admin API routes
        .route("/api/users", get(get_users_handler).post(create_user_handler))
        .route("/api/users/:id", put(update_user_handler).delete(delete_user_handler))
        .nest_service("/static", ServeDir::new("static"))
        .layer(session_layer)
        .with_state(pool);

    // Get bind address from environment variable or use default
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(3000);
    let bind_addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap();

    println!("Server running on http://{}", bind_addr);
    println!("Default admin credentials: username=admin, password=admin");
    println!("Database path: {}", std::env::var("DATABASE_PATH").unwrap_or_else(|_| "vcards.db".to_string()));

    axum::serve(listener, app).await.unwrap();
}
