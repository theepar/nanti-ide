import math
import os
from PIL import Image, ImageDraw, ImageFilter

os.makedirs("assets/symmetrical", exist_ok=True)

# =========================================================================
# Concept S1: "The Nexus Hourglass" (Simetri Cermin Waktu & Kecepatan)
# Perfectly symmetrical left-right and top-bottom: Two faceted obsidian/white
# prisms meeting at a razor-sharp apex.
# Symbolizes "Nanti" (time) collapsed into the zero-latency "Now".
# =========================================================================
svg_s1 = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bgs1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="s1_top_l" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#cccccc"/>
    </linearGradient>
    <linearGradient id="s1_top_r" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#888888"/>
    </linearGradient>
    <linearGradient id="s1_bot_l" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#999999"/>
      <stop offset="100%" stop-color="#444444"/>
    </linearGradient>
    <linearGradient id="s1_bot_r" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#666666"/>
      <stop offset="100%" stop-color="#222222"/>
    </linearGradient>
    <filter id="sh_s1" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="24" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bgs1)" stroke="#333333" stroke-width="2"/>

  <!-- Perfectly Symmetrical Nexus / Hourglass (Center X = 256) -->
  <g filter="url(#sh_s1)">
    <!-- Top Left Facet -->
    <polygon points="144,136 256,136 256,252 188,252" fill="url(#s1_top_l)"/>
    <!-- Top Right Facet (Mirrored) -->
    <polygon points="368,136 256,136 256,252 324,252" fill="url(#s1_top_r)"/>
    <!-- Bottom Left Facet -->
    <polygon points="188,260 256,260 256,376 144,376" fill="url(#s1_bot_l)"/>
    <!-- Bottom Right Facet (Mirrored) -->
    <polygon points="324,260 256,260 256,376 368,376" fill="url(#s1_bot_r)"/>
  </g>
</svg>"""

with open("assets/symmetrical/sym-s1-nexus.svg", "w", encoding="utf-8") as f:
    f.write(svg_s1)


# =========================================================================
# Concept S2: "The Twin Monoliths / Pylon Gate" (Simetri Bilateral Murni)
# Two identical mirrored trapezoidal pylons with a central light slit.
# Simple, iconic, authoritative, balanced left and right.
# =========================================================================
svg_s2 = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bgs2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="pylon_l" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="60%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#8a8a8a"/>
    </linearGradient>
    <linearGradient id="pylon_r" x1="100%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="60%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#8a8a8a"/>
    </linearGradient>
    <filter id="sh_s2" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="22" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bgs2)" stroke="#333333" stroke-width="2"/>

  <!-- Twin Mirrored Monoliths (Center slit at X = 256, width 18px) -->
  <g filter="url(#sh_s2)">
    <!-- Left Pylon: slants inward toward the top -->
    <polygon points="140,380 190,132 247,132 247,380" fill="url(#pylon_l)"/>
    <!-- Right Pylon: perfectly mirrored -->
    <polygon points="372,380 322,132 265,132 265,380" fill="url(#pylon_r)"/>
  </g>
</svg>"""

with open("assets/symmetrical/sym-s2-pylons.svg", "w", encoding="utf-8") as f:
    f.write(svg_s2)


# =========================================================================
# Concept S3: "The Diamond Octahedron" (Simetri Sempurna Segala Sisi)
# Symmetrical left-right AND top-bottom: A 4-quadrant diamond crystal
# with a cross-hair negative space slit. Pure mathematical harmony.
# =========================================================================
svg_s3 = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bgs3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <!-- Top Quadrant (Light source) -->
    <linearGradient id="quad_top" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#d4d4d4"/>
    </linearGradient>
    <!-- Left Quadrant -->
    <linearGradient id="quad_left" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#808080"/>
    </linearGradient>
    <!-- Right Quadrant -->
    <linearGradient id="quad_right" x1="100%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#b0b0b0"/>
      <stop offset="100%" stop-color="#555555"/>
    </linearGradient>
    <!-- Bottom Quadrant -->
    <linearGradient id="quad_bot" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#666666"/>
      <stop offset="100%" stop-color="#242424"/>
    </linearGradient>
    <filter id="sh_s3" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="24" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bgs3)" stroke="#333333" stroke-width="2"/>

  <!-- 4-Quadrant Balanced Diamond (Center = 256, 256) -->
  <g filter="url(#sh_s3)">
    <!-- Top Wedge -->
    <polygon points="256,122 360,248 260,248 256,122" fill="url(#quad_top)"/>
    <polygon points="256,122 152,248 252,248 256,122" fill="url(#quad_top)"/>

    <!-- Bottom Wedge -->
    <polygon points="256,390 360,264 260,264 256,390" fill="url(#quad_bot)"/>
    <polygon points="256,390 152,264 252,264 256,390" fill="url(#quad_bot)"/>
  </g>
</svg>"""

with open("assets/symmetrical/sym-s3-diamond.svg", "w", encoding="utf-8") as f:
    f.write(svg_s3)


# =========================================================================
# Concept S4: "The Inverted Chevron Gate" / Supersonic Wings
# Left-right mirrored inward-folding wings `< >` meeting at a center slit.
# Looks like code brackets `< >` abstracted into solid titanium aircraft wings.
# =========================================================================
svg_s4 = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bgs4" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="wing_l" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="50%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#999999"/>
    </linearGradient>
    <linearGradient id="wing_r" x1="100%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="50%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#999999"/>
    </linearGradient>
    <filter id="sh_s4" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="22" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bgs4)" stroke="#333333" stroke-width="2"/>

  <!-- Symmetrical Inward Wings (Center X = 256) -->
  <g filter="url(#sh_s4)">
    <!-- Left Wing -->
    <polygon points="140,152 248,256 140,360 196,360 252,256 196,152" fill="url(#wing_l)"/>
    <!-- Right Wing (Mirrored) -->
    <polygon points="372,152 264,256 372,360 316,360 260,256 316,152" fill="url(#wing_r)"/>
  </g>
</svg>"""

with open("assets/symmetrical/sym-s4-wings.svg", "w", encoding="utf-8") as f:
    f.write(svg_s4)


# =========================================================================
# Render high-resolution PNGs (1024x1024)
# =========================================================================
def render_s1(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    cx = s // 2
    gap_y = int(s * 0.015)
    cy = s // 2

    # Top Half
    p_tl = (int(s * 0.28), int(s * 0.26))
    p_tr = (int(s * 0.72), int(s * 0.26))
    p_tc = (cx, int(s * 0.26))
    p_mid_top = (cx, cy - gap_y)
    p_mid_l = (int(s * 0.38), cy - gap_y)
    p_mid_r = (int(s * 0.62), cy - gap_y)

    # Bottom Half
    p_bl = (int(s * 0.28), int(s * 0.74))
    p_br = (int(s * 0.72), int(s * 0.74))
    p_bc = (cx, int(s * 0.74))
    p_mid_bot = (cx, cy + gap_y)
    p_mid_bl = (int(s * 0.38), cy + gap_y)
    p_mid_br = (int(s * 0.62), cy + gap_y)

    # Draw Bottom
    draw.polygon([p_mid_bot, p_mid_br, p_br, p_bc], fill=(55, 55, 55, 255))
    draw.polygon([p_mid_bot, p_mid_bl, p_bl, p_bc], fill=(110, 110, 110, 255))
    draw.line([p_bl, p_br, p_mid_br, p_mid_bot, p_mid_bl, p_bl], fill=(130, 130, 130, 255), width=int(s * 0.003))

    # Draw Top
    draw.polygon([p_tc, p_tr, p_mid_r, p_mid_top], fill=(195, 195, 195, 255))
    draw.polygon([p_tc, p_tl, p_mid_l, p_mid_top], fill=(255, 255, 255, 255))
    draw.line([p_tl, p_tr, p_mid_r, p_mid_top, p_mid_l, p_tl], fill=(255, 255, 255, 255), width=int(s * 0.004))

    # Vertical spine
    draw.line([(cx, int(s * 0.26)), (cx, cy - gap_y)], fill=(255, 255, 255, 255), width=int(s * 0.005))
    draw.line([(cx, cy + gap_y), (cx, int(s * 0.74))], fill=(140, 140, 140, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_s2(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    cx = s // 2
    slit = int(s * 0.018)

    # Left Pylon
    p_l = [
        (int(s * 0.27), int(s * 0.74)),
        (int(s * 0.37), int(s * 0.26)),
        (cx - slit, int(s * 0.26)),
        (cx - slit, int(s * 0.74)),
    ]
    # Right Pylon (Exact Mirror)
    p_r = [
        (int(s * 0.73), int(s * 0.74)),
        (int(s * 0.63), int(s * 0.26)),
        (cx + slit, int(s * 0.26)),
        (cx + slit, int(s * 0.74)),
    ]

    draw.polygon(p_r, fill=(245, 245, 245, 255))
    draw.line(p_r + [p_r[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    draw.polygon(p_l, fill=(245, 245, 245, 255))
    draw.line(p_l + [p_l[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_s3(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    cx = s // 2
    cy = s // 2
    slit = int(s * 0.015)

    # Top Diamond Half
    top_w = [
        (cx, int(s * 0.24)),
        (int(s * 0.70), cy - slit),
        (cx, cy - slit),
        (int(s * 0.30), cy - slit),
    ]
    # Bottom Diamond Half
    bot_w = [
        (cx, int(s * 0.76)),
        (int(s * 0.70), cy + slit),
        (cx, cy + slit),
        (int(s * 0.30), cy + slit),
    ]

    # Draw bottom
    draw.polygon([(cx, int(s * 0.76)), (int(s * 0.70), cy + slit), (cx, cy + slit)], fill=(70, 70, 70, 255))
    draw.polygon([(cx, int(s * 0.76)), (int(s * 0.30), cy + slit), (cx, cy + slit)], fill=(120, 120, 120, 255))
    draw.line(bot_w + [bot_w[0]], fill=(140, 140, 140, 255), width=int(s * 0.003))

    # Draw top
    draw.polygon([(cx, int(s * 0.24)), (int(s * 0.70), cy - slit), (cx, cy - slit)], fill=(200, 200, 200, 255))
    draw.polygon([(cx, int(s * 0.24)), (int(s * 0.30), cy - slit), (cx, cy - slit)], fill=(255, 255, 255, 255))
    draw.line(top_w + [top_w[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    # Center vertical spine
    draw.line([(cx, int(s * 0.24)), (cx, cy - slit)], fill=(255, 255, 255, 255), width=int(s * 0.005))
    draw.line([(cx, cy + slit), (cx, int(s * 0.76))], fill=(150, 150, 150, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_s4(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    cx = s // 2
    slit = int(s * 0.015)

    # Inward wings < >
    # Left Wing
    w_l = [
        (int(s * 0.27), int(s * 0.30)),
        (cx - slit, int(s * 0.50)),
        (int(s * 0.27), int(s * 0.70)),
        (int(s * 0.38), int(s * 0.70)),
        (cx - slit + int(s * 0.11), int(s * 0.50)),
        (int(s * 0.38), int(s * 0.30)),
    ]
    # Right Wing (Mirrored)
    w_r = [
        (int(s * 0.73), int(s * 0.30)),
        (cx + slit, int(s * 0.50)),
        (int(s * 0.73), int(s * 0.70)),
        (int(s * 0.62), int(s * 0.70)),
        (cx + slit - int(s * 0.11), int(s * 0.50)),
        (int(s * 0.62), int(s * 0.30)),
    ]

    draw.polygon(w_r, fill=(245, 245, 245, 255))
    draw.line(w_r + [w_r[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    draw.polygon(w_l, fill=(245, 245, 245, 255))
    draw.line(w_l + [w_l[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

if __name__ == "__main__":
    img_s1 = render_s1(1024)
    img_s1.save("assets/symmetrical/sym-s1-nexus.png")
    img_s1.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\sym_s1_nexus.png")

    img_s2 = render_s2(1024)
    img_s2.save("assets/symmetrical/sym-s2-pylons.png")
    img_s2.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\sym_s2_pylons.png")

    img_s3 = render_s3(1024)
    img_s3.save("assets/symmetrical/sym-s3-diamond.png")
    img_s3.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\sym_s3_diamond.png")

    img_s4 = render_s4(1024)
    img_s4.save("assets/symmetrical/sym-s4-wings.png")
    img_s4.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\sym_s4_wings.png")

    print("Rendered 4 symmetrical concepts successfully!")
