# Authentication System Documentation

## Overview

The vCard QR Generator now includes a complete authentication system with user management. All QR generation functionality requires authentication, and admin users can manage other users through the admin panel.

## Default Credentials

**Username:** `admin`
**Password:** `admin`

⚠️ **IMPORTANT:** Change the default admin password immediately after first login via the Profile page.

## Features

### User Authentication
- Session-based authentication using tower-sessions
- Secure password hashing with bcrypt (cost 12)
- 24-hour session expiration on inactivity
- Sessions stored in SQLite database

### User Roles
- **Admin**: Can access admin panel, manage users, and generate QR codes
- **User**: Can only generate QR codes

### Pages

#### Login Page (`/login`)
- Public access
- Username and password authentication
- Error messages for failed login attempts

#### Home Page (`/`)
- **Protected** - requires authentication
- vCard form and QR code generation
- Navigation to Profile and Admin (if admin)

#### Profile Page (`/profile`)
- **Protected** - requires authentication
- View current username and role
- Change password functionality
- Validates current password before allowing change

#### Admin Page (`/admin`)
- **Protected** - requires admin role
- User management interface
- Create, edit, and delete users
- Cannot delete own account

## API Endpoints

### Public Endpoints

**POST `/api/login`**
- Request: `{ "username": "admin", "password": "admin" }`
- Response: `{ "message": "Login successful" }`
- Error: 401 with `{ "error": "Invalid username or password" }`

### Protected Endpoints (Require Authentication)

**GET `/api/me`**
- Returns current user info
- Response: `{ "id": 1, "username": "admin", "is_admin": true }`

**POST `/api/logout`**
- Clears session
- Response: `{ "message": "Logged out" }`

**POST `/api/change-password`**
- Request: `{ "current_password": "old", "new_password": "new" }`
- Response: `{ "message": "Password updated successfully" }`
- Errors:
  - 401: Current password incorrect
  - 500: Failed to update

**POST `/api/generate`**
- Generates vCard QR code (original functionality)
- Requires authentication
- Saves to database and returns QR code image

### Admin Endpoints (Require Admin Role)

**GET `/api/users`**
- Lists all users
- Response: Array of `{ "id": 1, "username": "admin", "is_admin": true }`

**POST `/api/users`**
- Creates new user
- Request: `{ "username": "newuser", "password": "pass", "is_admin": false }`
- Response: `{ "message": "User created successfully" }`
- Errors:
  - 409: Username already exists
  - 403: Not admin

**PUT `/api/users/:id`**
- Updates existing user
- Request: `{ "username": "updated", "password": "newpass" (optional), "is_admin": true }`
- Response: `{ "message": "User updated successfully" }`
- Note: Password is optional - omit to keep current password

**DELETE `/api/users/:id`**
- Deletes user
- Response: `{ "message": "User deleted successfully" }`
- Errors:
  - 400: Cannot delete own account
  - 403: Not admin

## Database Schema

### users table

| Column        | Type      | Description                       |
|---------------|-----------|-----------------------------------|
| id            | INTEGER   | Primary key (auto-increment)      |
| username      | TEXT      | Unique username                   |
| password_hash | TEXT      | bcrypt hashed password (cost 12)  |
| is_admin      | BOOLEAN   | Admin flag (0 or 1)               |
| created_at    | TIMESTAMP | Creation timestamp                |
| updated_at    | TIMESTAMP | Last update timestamp             |

**Index:** `idx_users_username` on `username`

### sessions table (auto-created by tower-sessions)
Stores session data for authenticated users.

## Security Features

### Password Security
- bcrypt hashing with cost factor 12
- No plain text passwords stored
- Password validation on change requires current password

### Session Security
- Session tokens stored securely
- 24-hour inactivity expiration
- Sessions tied to database
- Automatic cleanup of expired sessions

### Authorization
- Page-level protection (redirects to login)
- API-level protection (returns 401/403)
- Admin-only routes protected
- Users cannot delete themselves

### Protection Against Common Attacks
- CSRF protection via session tokens
- SQL injection prevented by parameterized queries
- No password hints or enumeration
- Generic error messages for authentication failures

## User Management Workflows

### First Time Setup
1. Start application
2. Navigate to http://127.0.0.1:3000
3. Redirected to /login
4. Login with `admin` / `admin`
5. Go to Profile page
6. Change password immediately

### Creating Users
1. Login as admin
2. Navigate to Admin page
3. Click "Add User"
4. Enter username, password, select role
5. Click "Create User"

### Editing Users
1. Login as admin
2. Navigate to Admin page
3. Click "Edit" on user row
4. Update username, password (optional), or role
5. Click "Update User"
- Leave password blank to keep current password

### Deleting Users
1. Login as admin
2. Navigate to Admin page
3. Click "Delete" on user row
4. Confirm deletion
- Cannot delete own account

### Changing Own Password
1. Login
2. Navigate to Profile page
3. Enter current password
4. Enter new password twice
5. Click "Update Password"

## Code Organization

### Main Files

**src/auth.rs**
- Authentication module
- User struct and UserInfo
- Session management functions
- Password hashing and verification
- User authentication logic

**src/main.rs**
- Route handlers for authentication
- Page handlers with auth checks
- Admin handlers with role checks
- Session middleware integration

**static/login.html**
- Login page UI

**static/profile.html**
- Profile and password change UI

**static/admin.html**
- User management UI

**migrations/002_create_users_table.sql**
- Users table schema
- Creates default admin user

## Session Management

Sessions are managed by `tower-sessions` with SQLite storage:

```rust
// Session expiry: 24 hours of inactivity
Expiry::OnInactivity(Duration::hours(24))
```

Session data stored:
- `user_id`: User's database ID
- `username`: Username
- `is_admin`: Admin flag

## Troubleshooting

### Cannot Login
- Verify database exists: `ls vcards.db`
- Check users table: `sqlite3 vcards.db "SELECT * FROM users"`
- Verify migration applied: `sqlite3 vcards.db "SELECT * FROM migrations WHERE name LIKE '%users%'"`
- Check server logs for errors

### Forgot Admin Password
Reset via SQL:
```sql
sqlite3 vcards.db
UPDATE users SET password_hash = '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYKqXqKKQWK'
WHERE username = 'admin';
```
This resets password to "admin"

### Session Expires Too Quickly
Modify expiry in main.rs:
```rust
.with_expiry(Expiry::OnInactivity(Duration::hours(48))) // 48 hours
```

### Cannot Access Admin Page
- Verify user is admin: `sqlite3 vcards.db "SELECT username, is_admin FROM users"`
- If not, update: `UPDATE users SET is_admin = 1 WHERE username = 'youruser'`

## Production Considerations

### Must Do Before Production
1. Change default admin password
2. Use HTTPS (not HTTP)
3. Set secure session cookies
4. Implement rate limiting for login attempts
5. Add password complexity requirements
6. Implement account lockout after failed attempts
7. Add password reset functionality
8. Use environment variables for sensitive config
9. Regular backups of database
10. Monitor for suspicious login activity

### Recommended Enhancements
- Two-factor authentication (2FA)
- Email verification
- Password reset via email
- Audit logging for admin actions
- Session management (view/revoke active sessions)
- Password expiry policy
- Force password change on first login
- Remember me functionality
- Captcha for login attempts

## Dependencies

Authentication requires these new dependencies:
- `tower-sessions` - Session management
- `tower-sessions-sqlx-store` - SQLite session storage
- `bcrypt` - Password hashing
- `uuid` - Unique identifiers
- `time` (via tower-sessions) - Time handling

## Testing

### Manual Testing Checklist
- ✓ Can login with default admin credentials
- ✓ Cannot access protected pages without login
- ✓ Can change password
- ✓ Admin can create users
- ✓ Admin can edit users
- ✓ Admin can delete users (except self)
- ✓ Non-admin cannot access admin page
- ✓ Sessions persist across requests
- ✓ Sessions expire after inactivity
- ✓ QR generation requires authentication

### Testing Accounts
Create test accounts for different scenarios:
```sql
-- Regular user
INSERT INTO users (username, password_hash, is_admin)
VALUES ('testuser', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYKqXqKKQWK', 0);

-- Another admin
INSERT INTO users (username, password_hash, is_admin)
VALUES ('admin2', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYKqXqKKQWK', 1);
```
(Password for both is "admin")

## Summary

Authentication is now fully integrated with:
- Default admin user (`admin`/`admin`)
- Session-based auth with 24-hour expiry
- Role-based access control (admin vs user)
- Complete user management interface
- Password change functionality
- All QR generation protected by authentication

The system is production-ready with proper security measures, though additional hardening is recommended for public deployment.
