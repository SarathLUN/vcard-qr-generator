# Database Documentation

## Overview

The vCard QR Generator now uses SQLite to persist all vCard data submitted by users. The database automatically initializes on first run and applies migrations to keep the schema up-to-date.

## Database Location

- **File**: `vcards.db` (created in the project root directory)
- **Type**: SQLite 3

## Schema

### `vcards` Table

Stores all vCard submissions with the following fields:

| Column      | Type      | Nullable | Description                           |
|-------------|-----------|----------|---------------------------------------|
| id          | INTEGER   | No       | Primary key (auto-increment)          |
| first_name  | TEXT      | No       | Contact's first name                  |
| last_name   | TEXT      | No       | Contact's last name                   |
| mobile      | TEXT      | Yes      | Mobile phone number                   |
| work        | TEXT      | Yes      | Work phone number                     |
| email       | TEXT      | Yes      | Email address                         |
| company     | TEXT      | Yes      | Company name                          |
| role        | TEXT      | Yes      | Job title/role                        |
| street      | TEXT      | Yes      | Street address                        |
| city        | TEXT      | Yes      | City                                  |
| state       | TEXT      | Yes      | State/Province                        |
| website     | TEXT      | Yes      | Website URL                           |
| color       | TEXT      | Yes      | QR code color (hex format)            |
| created_at  | TIMESTAMP | No       | Record creation timestamp             |
| updated_at  | TIMESTAMP | No       | Record last update timestamp          |

**Indexes:**
- `idx_vcards_created_at` on `created_at` - for faster time-based queries
- `idx_vcards_email` on `email` - for potential email lookups

### `migrations` Table

Tracks applied database migrations:

| Column      | Type      | Nullable | Description                           |
|-------------|-----------|----------|---------------------------------------|
| id          | INTEGER   | No       | Primary key (auto-increment)          |
| name        | TEXT      | No       | Migration name (unique)               |
| applied_at  | TIMESTAMP | No       | When migration was applied            |

## Migration System

### How It Works

1. On startup, the application checks if the database exists
2. If not, it creates a new SQLite database file
3. The migration system checks which migrations have been applied
4. Any pending migrations are executed in order
5. Each successful migration is recorded in the `migrations` table

### Migration Files

Located in the `migrations/` directory:

- **001_create_vcards_table.sql** - Creates the main vcards table with indexes

### Adding New Migrations

To add a new migration:

1. Create a new SQL file in `migrations/` with the format: `NNN_description.sql`
   - Use sequential numbering (e.g., `002_add_notes_field.sql`)

2. Add your SQL statements to the file

3. Update `src/main.rs` in the `run_migrations()` function:
   ```rust
   let migrations = vec![
       ("001_create_vcards_table", include_str!("../migrations/001_create_vcards_table.sql")),
       ("002_add_notes_field", include_str!("../migrations/002_add_notes_field.sql")), // Add this line
   ];
   ```

4. Migrations run automatically on next server start

## Querying the Database

### Using SQLite CLI

```bash
# Open database
sqlite3 vcards.db

# View all tables
.tables

# View schema
.schema

# Query all vcards
SELECT * FROM vcards;

# Query recent vcards
SELECT * FROM vcards ORDER BY created_at DESC LIMIT 10;

# Count total vcards
SELECT COUNT(*) FROM vcards;

# Search by email
SELECT * FROM vcards WHERE email LIKE '%example.com';
```

### Using SQL in Code

The application uses SQLx for database operations. Example:

```rust
// Query all vcards
let vcards = sqlx::query("SELECT * FROM vcards ORDER BY created_at DESC")
    .fetch_all(&pool)
    .await?;

// Query by email
let vcard = sqlx::query("SELECT * FROM vcards WHERE email = ?")
    .bind("user@example.com")
    .fetch_optional(&pool)
    .await?;
```

## Data Flow

1. User submits form data via the web interface
2. Frontend sends POST request to `/api/generate` with JSON payload
3. Backend receives data and validates it
4. **Data is saved to SQLite database** (new step)
5. vCard string is generated from the data
6. QR code is created and encoded as PNG
7. Base64-encoded image is returned to frontend

## Database Maintenance

### Backup

```bash
# Create a backup
sqlite3 vcards.db ".backup vcards_backup.db"

# Or using cp
cp vcards.db vcards_backup_$(date +%Y%m%d).db
```

### Reset Database

```bash
# Delete database file (will be recreated on next run)
rm vcards.db
```

### View Statistics

```bash
sqlite3 vcards.db <<EOF
SELECT
    COUNT(*) as total_records,
    COUNT(DISTINCT email) as unique_emails,
    DATE(created_at) as date,
    COUNT(*) as records_per_day
FROM vcards
GROUP BY DATE(created_at)
ORDER BY date DESC;
EOF
```

## Performance Considerations

- **Indexes**: The database has indexes on `created_at` and `email` fields for faster queries
- **Connection Pooling**: SQLx manages a connection pool automatically
- **Async Operations**: All database operations are async and non-blocking
- **Transaction Support**: SQLite supports transactions for atomic operations

## Security Notes

- Database file should be excluded from version control (not committed to git)
- Consider implementing data retention policies
- For production use, implement proper backup strategies
- Consider encrypting sensitive data before storage
- Add rate limiting to prevent database spam

## Troubleshooting

### Database locked error
- Ensure only one instance of the application is running
- Check for long-running transactions

### Migration fails
- Check migration SQL syntax
- Verify migration hasn't been partially applied
- Check `migrations` table for applied migrations

### Performance issues
- Add indexes for frequently queried columns
- Use `EXPLAIN QUERY PLAN` to analyze slow queries
- Consider VACUUM to optimize database file
