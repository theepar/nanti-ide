import math
from PIL import Image, ImageDraw, ImageFilter, ImageOps

def create_monochrome_logo(size=1024):
    scale = 2
    s = size * scale  # 2048 for high quality supersampling
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 1. Base Squircle (Pure deep black with subtle charcoal gradient)
    pad = int(s * 0.04)
    r = int(s * 0.22)
    box = [pad, pad, s - pad, s - pad]

    # Outer border / subtle edge rim (dark neutral gray)
    draw.rounded_rectangle(box, radius=r, fill=(12, 12, 12, 255), outline=(50, 50, 50, 255), width=int(s * 0.004))

    # Inner subtle rim
    inner_pad = pad + int(s * 0.003)
    inner_box = [inner_pad, inner_pad, s - inner_pad, s - inner_pad]
    draw.rounded_rectangle(inner_box, radius=r - int(s * 0.003), outline=(28, 28, 28, 255), width=int(s * 0.002))

    # 2. 'N' Glyph Coordinates (scaled)
    # Left pillar
    lp_x0 = int(s * 0.24)
    lp_x1 = int(s * 0.36)
    lp_y0 = int(s * 0.22)
    lp_y1 = int(s * 0.74)
    pillar_r = int(s * 0.03)

    # Right pillar
    rp_x0 = int(s * 0.64)
    rp_x1 = int(s * 0.76)
    rp_y0 = int(s * 0.22)
    rp_y1 = int(s * 0.61) # Cut short for the period dot

    # Draw left pillar with vertical metallic/silver gradient
    for y in range(lp_y0, lp_y1):
        frac = (y - lp_y0) / (lp_y1 - lp_y0)
        c = int(255 - frac * 85) # 255 down to 170
        draw.line([(lp_x0, y), (lp_x1, y)], fill=(c, c, c, 255), width=1)

    # Round caps for left pillar
    draw.rounded_rectangle([lp_x0, lp_y0, lp_x1, lp_y1], radius=pillar_r, outline=(255, 255, 255, 255), width=int(s * 0.003))

    # Draw right pillar with vertical gradient
    for y in range(rp_y0, rp_y1):
        frac = (y - rp_y0) / (rp_y1 - rp_y0)
        c = int(255 - frac * 85)
        draw.line([(rp_x0, y), (rp_x1, y)], fill=(c, c, c, 255), width=1)
    draw.rounded_rectangle([rp_x0, rp_y0, rp_x1, rp_y1], radius=pillar_r, outline=(255, 255, 255, 255), width=int(s * 0.003))

    # 3. High-Velocity Diagonal Slash (Chevron cut)
    diag_pts = [
        (int(s * 0.34), int(s * 0.26)),
        (int(s * 0.68), int(s * 0.69)),
        (int(s * 0.70), int(s * 0.63)),
        (int(s * 0.40), int(s * 0.22)),
    ]
    # Draw solid white diagonal
    draw.polygon(diag_pts, fill=(240, 240, 240, 255))
    draw.line(diag_pts + [diag_pts[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    # 4. The Iconic Period Dot '.' (Pure glowing white)
    dot_cx = int(s * 0.70)
    dot_cy = int(s * 0.72)
    dot_r = int(s * 0.042)

    # Draw glow layer
    glow_img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    glow_draw = ImageDraw.Draw(glow_img)
    glow_draw.ellipse(
        [dot_cx - dot_r * 2, dot_cy - dot_r * 2, dot_cx + dot_r * 2, dot_cy + dot_r * 2],
        fill=(255, 255, 255, 120)
    )
    glow_img = glow_img.filter(ImageFilter.GaussianBlur(int(s * 0.02)))
    img = Image.alpha_composite(img, glow_img)

    # Redraw main dot on top
    final_draw = ImageDraw.Draw(img)
    final_draw.ellipse(
        [dot_cx - dot_r, dot_cy - dot_r, dot_cx + dot_r, dot_cy + dot_r],
        fill=(255, 255, 255, 255)
    )

    # Downsample to target size with high-quality Lanczos filter
    final_img = img.resize((size, size), Image.Resampling.LANCZOS)
    return final_img

def convert_existing_to_monochrome(src_path, dst_path):
    orig = Image.open(src_path).convert("RGB")
    # Convert to grayscale
    gray = ImageOps.grayscale(orig)
    # Enhance contrast: pitch blacks stay pitch black, bright whites stay brilliant white
    lut = []
    for i in range(256):
        if i < 30:
            lut.append(int(i * 0.7)) # darken background
        elif i > 160:
            lut.append(min(255, int(i * 1.15))) # brighten highlights
        else:
            lut.append(i)
    boosted = gray.point(lut)
    boosted.save(dst_path, quality=95)

if __name__ == "__main__":
    # Generate clean vector-rendered monochrome icon
    pure_mono = create_monochrome_logo(1024)
    pure_mono.save("assets/nanti-icon-pure-mono.png")

    # Convert the sleek rendered 3D logo into true monochrome (zero blue/color)
    convert_existing_to_monochrome("assets/nanti-logo.png", "assets/nanti-logo.png")
    convert_existing_to_monochrome("assets/logo.png", "assets/logo.png")
    convert_existing_to_monochrome("assets/logo-detailed.png", "assets/logo-detailed.png")
    
    # Also save to artifact directory so user can preview directly
    convert_existing_to_monochrome(
        "assets/nanti-logo.png",
        r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\nanti_logo_monochrome.jpg"
    )
    print("Monochrome transformation complete!")

