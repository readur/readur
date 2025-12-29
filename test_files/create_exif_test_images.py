#!/usr/bin/env python3
"""
Generate test JPEG images with EXIF orientation tags for testing auto-rotation.

EXIF Orientation values:
1 = Normal (no rotation needed)
2 = Mirrored horizontally
3 = Rotated 180 degrees
4 = Mirrored vertically
5 = Mirrored horizontally then rotated 270 CW
6 = Rotated 90 CW
7 = Mirrored horizontally then rotated 90 CW
8 = Rotated 270 CW (90 CCW)
"""

import io
import struct
from PIL import Image


def create_asymmetric_image():
    """
    Create an asymmetric image so rotation/flip effects are visible.
    The image is 40x20 pixels with a distinctive pattern:
    - Red square in top-left corner
    - Blue strip on right edge
    - Green strip on bottom edge
    This makes it easy to verify orientation transformations.
    """
    img = Image.new("RGB", (40, 20), color=(255, 255, 255))

    # Red square in top-left (10x10)
    for x in range(10):
        for y in range(10):
            img.putpixel((x, y), (255, 0, 0))

    # Blue strip on right edge (last 5 columns)
    for x in range(35, 40):
        for y in range(20):
            img.putpixel((x, y), (0, 0, 255))

    # Green strip on bottom edge (last 5 rows, excluding blue area)
    for x in range(35):
        for y in range(15, 20):
            img.putpixel((x, y), (0, 255, 0))

    return img


def create_minimal_exif_with_orientation(orientation):
    """
    Create minimal EXIF data with just the orientation tag.

    EXIF structure:
    - APP1 marker (0xFFE1)
    - Length (2 bytes, big-endian)
    - "Exif\0\0" identifier
    - TIFF header
    - IFD0 with Orientation tag
    """
    # TIFF header (little-endian)
    tiff_header = b"II"  # Little-endian
    tiff_header += struct.pack("<H", 42)  # TIFF magic number
    tiff_header += struct.pack("<I", 8)   # Offset to first IFD

    # IFD0 with one entry (Orientation)
    ifd = struct.pack("<H", 1)  # Number of entries

    # Orientation tag entry:
    # Tag ID: 0x0112 (274)
    # Type: SHORT (3)
    # Count: 1
    # Value: orientation
    ifd += struct.pack("<H", 0x0112)  # Tag ID
    ifd += struct.pack("<H", 3)       # Type (SHORT)
    ifd += struct.pack("<I", 1)       # Count
    ifd += struct.pack("<HH", orientation, 0)  # Value (padded to 4 bytes)

    # Next IFD offset (0 = no more IFDs)
    ifd += struct.pack("<I", 0)

    # Combine TIFF data
    tiff_data = tiff_header + ifd

    # Build APP1 segment
    exif_header = b"Exif\x00\x00"
    app1_data = exif_header + tiff_data
    app1_length = len(app1_data) + 2  # +2 for length field itself

    app1_segment = b"\xff\xe1"  # APP1 marker
    app1_segment += struct.pack(">H", app1_length)
    app1_segment += app1_data

    return app1_segment


def embed_exif_in_jpeg(jpeg_bytes, exif_segment):
    """
    Embed EXIF data into a JPEG image.
    Insert APP1 segment right after SOI marker.
    """
    # JPEG must start with SOI (0xFFD8)
    if jpeg_bytes[:2] != b"\xff\xd8":
        raise ValueError("Not a valid JPEG file")

    # Insert EXIF APP1 right after SOI
    return jpeg_bytes[:2] + exif_segment + jpeg_bytes[2:]


def main():
    print("Creating test JPEG images with EXIF orientation tags...")

    # Create base image
    base_img = create_asymmetric_image()

    # Save base image to bytes (no EXIF)
    base_buffer = io.BytesIO()
    base_img.save(base_buffer, format="JPEG", quality=95)
    base_jpeg = base_buffer.getvalue()

    # Generate test images for each orientation value
    orientations = {
        1: "normal",
        2: "flip_horizontal",
        3: "rotate_180",
        4: "flip_vertical",
        5: "transpose",
        6: "rotate_90_cw",
        7: "transverse",
        8: "rotate_270_cw",
    }

    for orientation, name in orientations.items():
        exif_segment = create_minimal_exif_with_orientation(orientation)
        jpeg_with_exif = embed_exif_in_jpeg(base_jpeg, exif_segment)

        filename = f"exif_orientation_{orientation}_{name}.jpg"
        with open(filename, "wb") as f:
            f.write(jpeg_with_exif)
        print(f"  Created: {filename} ({len(jpeg_with_exif)} bytes)")

    # Also create a JPEG without any EXIF for testing fallback
    with open("exif_orientation_none.jpg", "wb") as f:
        f.write(base_jpeg)
    print(f"  Created: exif_orientation_none.jpg ({len(base_jpeg)} bytes)")

    print("\nDone! Created 9 test images.")


if __name__ == "__main__":
    main()
