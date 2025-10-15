use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tower_sessions::Session;

// Session key for storing user ID
pub const USER_ID_KEY: &str = "user_id";
pub const USERNAME_KEY: &str = "username";
pub const IS_ADMIN_KEY: &str = "is_admin";

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub is_admin: bool,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        UserInfo {
            id: user.id,
            username: user.username,
            is_admin: user.is_admin,
        }
    }
}

// Check if user is authenticated
pub async fn get_current_user(session: &Session) -> Option<UserInfo> {
    let user_id: Option<i64> = session.get(USER_ID_KEY).await.ok().flatten();
    let username: Option<String> = session.get(USERNAME_KEY).await.ok().flatten();
    let is_admin: Option<bool> = session.get(IS_ADMIN_KEY).await.ok().flatten();

    match (user_id, username, is_admin) {
        (Some(id), Some(username), Some(is_admin)) => Some(UserInfo {
            id,
            username,
            is_admin,
        }),
        _ => None,
    }
}

// Verify password
pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

// Hash password
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, 12)
}

// Authenticate user
pub async fn authenticate_user(
    pool: &SqlitePool,
    username: &str,
    password: &str,
) -> Result<User, String> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, is_admin FROM users WHERE username = ?"
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    match user {
        Some(user) => {
            if verify_password(password, &user.password_hash) {
                Ok(user)
            } else {
                Err("Invalid username or password".to_string())
            }
        }
        None => Err("Invalid username or password".to_string()),
    }
}

// Set session for authenticated user
pub async fn set_user_session(session: &Session, user: &User) -> Result<(), tower_sessions::session::Error> {
    session.insert(USER_ID_KEY, user.id).await?;
    session.insert(USERNAME_KEY, user.username.clone()).await?;
    session.insert(IS_ADMIN_KEY, user.is_admin).await?;
    Ok(())
}

// Clear session (logout)
pub async fn clear_session(session: &Session) {
    let _ = session.delete().await;
}
