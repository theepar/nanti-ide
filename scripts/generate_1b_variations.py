import math
import os
from PIL import Image, ImageDraw, ImageFilter

os.makedirs("assets/variations_1b", exist_ok=True)

# =========================================================================
# 1B.1: "Faceted Chiseled Blades" (3D Creased Spine)
# Each of the two twin blades has a chiseled center spine with light & dark
# facets, giving them metallic dimensional depth while keeping the 1B silhouette.
# =========================================================================
svg_1b1 = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="50%" stop-color="#090909"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="border1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#383838" stop-opacity="0.9"/>
      <stop offset="100%" stop-color="#181818" stop-opacity="0.7"/>
    </linearGradient>
    <!-- Blade 1 Left Facet (Direct Light) -->
    <linearGradient id="b1_left" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#d4d4d4"/>
    </linearGradient>
    <!-- Blade 1 Right Facet (Specular Falloff) -->
    <linearGradient id="b1_right" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#999999"/>
    </linearGradient>
    <!-- Blade 2 Left Facet (Midtone) -->
    <linearGradient id="b2_left" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#b0b0b0"/>
      <stop offset="100%" stop-color="#707070"/>
    </linearGradient>
    <!-- Blade 2 Right Facet (Deep Shadow) -->
    <linearGradient id="b2_right" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#606060"/>
      <stop offset="100%" stop-color="#2a2a2a"/>
    </linearGradient>
    <filter id="sh1" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="22" stdDeviation="24" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg1)" stroke="url(#border1)" stroke-width="2"/>
  <rect x="25" y="25" width="462" height="462" rx="107" ry="107" fill="none" stroke="rgba(255, 255, 255, 0.05)" stroke-width="1"/>

  <g filter="url(#sh1)">
    <!-- Blade 1 (Primary Front Blade, Faceted) -->
    <polygon points="170,390 236,126 270,126 204,390" fill="url(#b1_left)"/>
    <polygon points="204,390 270,126 304,126 238,390" fill="url(#b1_right)"/>

    <!-- Blade 2 (Secondary Stepped Blade, Faceted) -->
    <polygon points="256,386 322,126 336,146 270,406" fill="url(#b2_left)"/>
    <polygon points="270,406 336,146 350,166 284,426" fill="url(#b2_right)"/>
  </g>
</svg>"""

with open("assets/variations_1b/1b-faceted.svg", "w", encoding="utf-8") as f:
    f.write(svg_1b1)


# =========================================================================
# 1B.2: "Razor Chamfer" (Angled Aerodynamic Cuts)
# The end caps are cut at a high-speed reverse 45-degree angle rather than flat.
# Gives an intense aerodynamic jet / razor blade sensation.
# =========================================================================
svg_1b2 = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="border2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#383838"/>
      <stop offset="100%" stop-color="#141414"/>
    </linearGradient>
    <linearGradient id="b_white" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="60%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#a0a0a0"/>
    </linearGradient>
    <linearGradient id="b_dark" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#888888"/>
      <stop offset="60%" stop-color="#555555"/>
      <stop offset="100%" stop-color="#2a2a2a"/>
    </linearGradient>
    <filter id="sh2" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="22" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg2)" stroke="url(#border2)" stroke-width="2"/>

  <g filter="url(#sh2)">
    <!-- Blade 1: Chamfered Sharp Tips (Top right slant, Bottom left slant) -->
    <polygon points="178,396 230,146 304,116 252,366" fill="url(#b_white)"/>
    
    <!-- Blade 2: Trailing Chamfered Blade -->
    <polygon points="260,396 312,146 356,166 304,416" fill="url(#b_dark)"/>
  </g>
</svg>"""

with open("assets/variations_1b/1b-razor.svg", "w", encoding="utf-8") as f:
    f.write(svg_1b2)


# =========================================================================
# 1B.3: "Golden Proportion" (Refined Minimal Slit)
# The pure 1B geometry perfectly calibrated: optimal negative-space gap (14px),
# balanced optical center, and pristine silver-metallic gradients.
# =========================================================================
svg_1b3 = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="50%" stop-color="#090909"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="border3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#3c3c3c"/>
      <stop offset="100%" stop-color="#161616"/>
    </linearGradient>
    <linearGradient id="g_lead" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="50%" stop-color="#ebebeb"/>
      <stop offset="100%" stop-color="#b5b5b5"/>
    </linearGradient>
    <linearGradient id="g_trail" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#a0a0a0"/>
      <stop offset="60%" stop-color="#555555"/>
      <stop offset="100%" stop-color="#282828"/>
    </linearGradient>
    <filter id="sh3" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="24" stdDeviation="26" flood-color="#000000" flood-opacity="0.95"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg3)" stroke="url(#border3)" stroke-width="2"/>
  <rect x="25" y="25" width="462" height="462" rx="107" ry="107" fill="none" stroke="rgba(255, 255, 255, 0.06)" stroke-width="1"/>

  <g filter="url(#sh3)">
    <!-- Primary High Blade -->
    <polygon points="176,380 236,132 304,132 244,380" fill="url(#g_lead)"/>
    <!-- Secondary Trailing Blade (Precision 16px gap, 44px downward step) -->
    <polygon points="260,380 320,132 352,176 292,424" fill="url(#g_trail)"/>
  </g>
</svg>"""

with open("assets/variations_1b/1b-refined.svg", "w", encoding="utf-8") as f:
    f.write(svg_1b3)


# =========================================================================
# 1B.4: "Core & Ray" (Asymmetric Widths: 70/30 Power Ratio)
# The leading blade is wider and bolder (the powerhouse engine), while the
# trailing blade is a sharp, ultra-thin laser shard.
# =========================================================================
svg_1b4 = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg4" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="border4" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#363636"/>
      <stop offset="100%" stop-color="#141414"/>
    </linearGradient>
    <linearGradient id="g_thick" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="70%" stop-color="#d4d4d4"/>
      <stop offset="100%" stop-color="#909090"/>
    </linearGradient>
    <linearGradient id="g_thin" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="70%" stop-color="#777777"/>
      <stop offset="100%" stop-color="#333333"/>
    </linearGradient>
    <filter id="sh4" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="20" stdDeviation="22" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg4)" stroke="url(#border4)" stroke-width="2"/>

  <g filter="url(#sh4)">
    <!-- Primary Heavy Blade (90px wide) -->
    <polygon points="160,384 224,128 314,128 250,384" fill="url(#g_thick)"/>
    <!-- Trailing Sleek Ray (36px wide) -->
    <polygon points="268,384 332,128 358,168 294,424" fill="url(#g_thin)"/>
  </g>
</svg>"""

with open("assets/variations_1b/1b-asymmetric.svg", "w", encoding="utf-8") as f:
    f.write(svg_1b4)


# =========================================================================
# Render high-res PNGs (1024x1024)
# =========================================================================
def render_1b_faceted(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    # Blade 1 Facets
    b1_f1 = [(int(s * 0.33), int(s * 0.76)), (int(s * 0.46), int(s * 0.25)), (int(s * 0.53), int(s * 0.25)), (int(s * 0.40), int(s * 0.76))]
    b1_f2 = [(int(s * 0.40), int(s * 0.76)), (int(s * 0.53), int(s * 0.25)), (int(s * 0.59), int(s * 0.25)), (int(s * 0.46), int(s * 0.76))]

    # Blade 2 Facets
    b2_f1 = [(int(s * 0.50), int(s * 0.75)), (int(s * 0.63), int(s * 0.25)), (int(s * 0.66), int(s * 0.29)), (int(s * 0.53), int(s * 0.79))]
    b2_f2 = [(int(s * 0.53), int(s * 0.79)), (int(s * 0.66), int(s * 0.29)), (int(s * 0.68), int(s * 0.32)), (int(s * 0.55), int(s * 0.83))]

    draw.polygon(b2_f2, fill=(45, 45, 45, 255))
    draw.polygon(b2_f1, fill=(90, 90, 90, 255))
    draw.line(b2_f1 + [b2_f1[0]], fill=(110, 110, 110, 255), width=int(s * 0.002))

    draw.polygon(b1_f2, fill=(195, 195, 195, 255))
    draw.polygon(b1_f1, fill=(255, 255, 255, 255))
    draw.line(b1_f1 + [b1_f1[0]], fill=(255, 255, 255, 255), width=int(s * 0.003))

    # Center crease spine highlight
    draw.line([b1_f1[1], b1_f1[3]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_1b_razor(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    b1 = [
        (int(s * 0.35), int(s * 0.77)),
        (int(s * 0.45), int(s * 0.29)),
        (int(s * 0.59), int(s * 0.23)),
        (int(s * 0.49), int(s * 0.71)),
    ]
    b2 = [
        (int(s * 0.51), int(s * 0.77)),
        (int(s * 0.61), int(s * 0.29)),
        (int(s * 0.70), int(s * 0.33)),
        (int(s * 0.59), int(s * 0.81)),
    ]

    draw.polygon(b2, fill=(80, 80, 80, 255))
    draw.line(b2 + [b2[0]], fill=(120, 120, 120, 255), width=int(s * 0.003))

    draw.polygon(b1, fill=(245, 245, 245, 255))
    draw.line(b1 + [b1[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_1b_refined(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    b1 = [
        (int(s * 0.34), int(s * 0.74)),
        (int(s * 0.46), int(s * 0.26)),
        (int(s * 0.59), int(s * 0.26)),
        (int(s * 0.47), int(s * 0.74)),
    ]
    b2 = [
        (int(s * 0.51), int(s * 0.74)),
        (int(s * 0.63), int(s * 0.26)),
        (int(s * 0.69), int(s * 0.34)),
        (int(s * 0.57), int(s * 0.83)),
    ]

    draw.polygon(b2, fill=(85, 85, 85, 255))
    draw.line(b2 + [b2[0]], fill=(120, 120, 120, 255), width=int(s * 0.003))

    draw.polygon(b1, fill=(245, 245, 245, 255))
    draw.line(b1 + [b1[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_1b_asymmetric(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(46, 46, 46, 255), width=int(s * 0.004))

    # Heavy Core Blade
    b1 = [
        (int(s * 0.31), int(s * 0.75)),
        (int(s * 0.44), int(s * 0.25)),
        (int(s * 0.61), int(s * 0.25)),
        (int(s * 0.49), int(s * 0.75)),
    ]
    # Slim Trailing Ray
    b2 = [
        (int(s * 0.52), int(s * 0.75)),
        (int(s * 0.65), int(s * 0.25)),
        (int(s * 0.70), int(s * 0.33)),
        (int(s * 0.57), int(s * 0.83)),
    ]

    draw.polygon(b2, fill=(90, 90, 90, 255))
    draw.line(b2 + [b2[0]], fill=(130, 130, 130, 255), width=int(s * 0.003))

    draw.polygon(b1, fill=(248, 248, 248, 255))
    draw.line(b1 + [b1[0]], fill=(255, 255, 255, 255), width=int(s * 0.004))

    return img.resize((size, size), Image.Resampling.LANCZOS)

if __name__ == "__main__":
    img_faceted = render_1b_faceted(1024)
    img_faceted.save("assets/variations_1b/1b-faceted.png")
    img_faceted.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\1b_faceted.png")

    img_razor = render_1b_razor(1024)
    img_razor.save("assets/variations_1b/1b-razor.png")
    img_razor.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\1b_razor.png")

    img_refined = render_1b_refined(1024)
    img_refined.save("assets/variations_1b/1b-refined.png")
    img_refined.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\1b_refined.png")

    img_asym = render_1b_asymmetric(1024)
    img_asym.save("assets/variations_1b/1b-asymmetric.png")
    img_asym.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\1b_asymmetric.png")

    print("Rendered 4 variations of 1B successfully!")
