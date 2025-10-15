-- Create vcards table
CREATE TABLE IF NOT EXISTS vcards (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    mobile TEXT,
    work TEXT,
    email TEXT,
    company TEXT,
    role TEXT,
    street TEXT,
    city TEXT,
    state TEXT,
    website TEXT,
    color TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create index on created_at for faster queries
CREATE INDEX IF NOT EXISTS idx_vcards_created_at ON vcards(created_at);

-- Create index on email for potential lookups
CREATE INDEX IF NOT EXISTS idx_vcards_email ON vcards(email);
