-- Add user roles support
-- Add role column to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS role VARCHAR(20) DEFAULT 'user';

-- Add check constraint for role values
ALTER TABLE users DROP CONSTRAINT IF EXISTS check_user_role;
ALTER TABLE users ADD CONSTRAINT check_user_role CHECK (role IN ('admin', 'user'));

-- Update existing admin user to have admin role
UPDATE users SET role = 'admin' WHERE username = 'admin';