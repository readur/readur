UPDATE settings SET 
    ocr_page_segmentation_mode = 3,
    ocr_engine_mode = 3,
    ocr_min_confidence = 30.0,
    ocr_dpi = 300,
    ocr_enhance_contrast = true,
    ocr_remove_noise = true,
    ocr_detect_orientation = true
WHERE ocr_page_segmentation_mode IS NULL;