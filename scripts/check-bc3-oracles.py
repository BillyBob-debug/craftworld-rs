"""Authored synthetic BC3 checks with installed Pillow; no original inputs or output files.

Microsoft D3D10 BC3 specification plus D3D9 Textures with Alpha Channels specify
four RGB colors regardless of endpoint order. Toolkit d6a2ef80 DDSReader instead
uses BC1's order branch; the Java guard records that known external-oracle conflict.
Pillow 12.3.0 independently corroborates reversed/equal color and alpha behavior.
Its RGB565 endpoint expansion differs from our Toolkit-compatible nearest convention;
this script is not a full-corpus pixel oracle. No third-party decoder copied.
"""
import io
import struct
from PIL import Image, __version__


def recipe(alpha0, alpha1, color0, color1):
    b = bytearray(144)
    b[:4] = b"DDS "
    for p, v in [(4, 124), (8, 0x81007), (12, 4), (16, 4), (20, 16),
                 (28, 1), (76, 32), (80, 4), (108, 0x1000)]:
        struct.pack_into("<I", b, p, v)
    b[84:88] = b"DXT5"
    b[128:130] = bytes([alpha0, alpha1])
    indices = sum((i % 8) << (3 * i) for i in range(16))
    b[130:136] = indices.to_bytes(6, "little")
    struct.pack_into("<HHI", b, 136, color0, color1, 0xe4e4e4e4)
    return bytes(b)


def main():
    modes = [(255, 0, [255, 0, 218, 182, 145, 109, 72, 36]),
             (0, 255, [0, 255, 51, 102, 153, 204, 0, 255]),
             (17, 17, [17, 17, 17, 17, 17, 17, 0, 255]),
             (23, 11, [23, 11, 21, 19, 17, 16, 14, 12])]
    for a, b, alphas in modes:
        with Image.open(io.BytesIO(recipe(a, b, 0x07e0, 0xf800))) as im:
            for i in range(16):
                colors = [(0, 255, 0), (255, 0, 0), (85, 170, 0), (170, 85, 0)]
                assert im.getpixel((i % 4, i // 4)) == colors[i % 4] + (alphas[i % 8],)
        with Image.open(io.BytesIO(recipe(a, b, 0x07e0, 0x07e0))) as im:
            for i in range(16):
                assert im.getpixel((i % 4, i // 4)) == (0, 255, 0, alphas[i % 8])
    with Image.open(io.BytesIO(recipe(255, 0, 0x18e3, 0))) as im:
        assert im.getpixel((0, 0)) == (24, 28, 24, 255)
    print(f"Pillow {__version__}: authored BC3 alpha modes and reversed/equal colors pass; endpoint convention difference recorded")


if __name__ == "__main__":
    main()
