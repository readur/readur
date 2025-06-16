-- Update existing file paths to use the new structured directory layout
-- This migration moves file paths from ./uploads/filename to ./uploads/documents/filename

UPDATE documents 
SET file_path = CASE 
    -- Update file paths that start with ./uploads/ but don't already have /documents/
    WHEN file_path LIKE './uploads/%' AND file_path NOT LIKE './uploads/documents/%' THEN 
        REPLACE(file_path, './uploads/', './uploads/documents/')
    -- Update file paths that start with uploads/ but don't already have /documents/
    WHEN file_path LIKE 'uploads/%' AND file_path NOT LIKE 'uploads/documents/%' THEN 
        REPLACE(file_path, 'uploads/', 'uploads/documents/')
    ELSE file_path
END
WHERE 
    (file_path LIKE './uploads/%' AND file_path NOT LIKE './uploads/documents/%')
    OR 
    (file_path LIKE 'uploads/%' AND file_path NOT LIKE 'uploads/documents/%');