import math
import os
from PIL import Image, ImageDraw, ImageFilter

os.makedirs("assets/abstract", exist_ok=True)

# -------------------------------------------------------------------------
# Concept 1: "The Shard" / Acceleration Prism (Isometric Folded Monolith)
# Inspired by Linear & Zed: Two interlocking geometric angular facets with
# negative space cut, completely non-alphabetical.
# -------------------------------------------------------------------------
svg_shard = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="50%" stop-color="#0a0a0a"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="border" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#383838" stop-opacity="0.8"/>
      <stop offset="100%" stop-color="#141414" stop-opacity="0.8"/>
    </linearGradient>
    <linearGradient id="facet1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#a3a3a3"/>
    </linearGradient>
    <linearGradient id="facet2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#8c8c8c"/>
      <stop offset="100%" stop-color="#404040"/>
    </linearGradient>
    <linearGradient id="facet3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#f5f5f5"/>
      <stop offset="100%" stop-color="#d4d4d4"/>
    </linearGradient>
    <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="18" stdDeviation="24" flood-color="#000000" flood-opacity="0.85"/>
    </filter>
    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="10" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
  </defs>

  <!-- 1:1 Squircle Base Frame -->
  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg)" stroke="url(#border)" stroke-width="2"/>
  <rect x="25" y="25" width="462" height="462" rx="107" ry="107" fill="none" stroke="rgba(255, 255, 255, 0.05)" stroke-width="1"/>

  <!-- Abstract Prism / Folded Shard (Pure Geometry) -->
  <g filter="url(#shadow)">
    <!-- Primary Angled Blade (Forward Motion) -->
    <polygon points="150,370 230,140 310,140 230,370" fill="url(#facet1)"/>
    <!-- Interlocking Reflected Prism Wedge -->
    <polygon points="230,370 310,140 362,192 282,422" fill="url(#facet2)"/>
    <!-- Top Chamfer Highlight Cap -->
    <polygon points="230,140 310,140 362,192 282,192" fill="url(#facet3)"/>
  </g>

  <!-- The "Now" Period Dot: Off-center balance point -->
  <circle cx="366" cy="374" r="22" fill="#ffffff" opacity="0.3" filter="url(#glow)"/>
  <circle cx="366" cy="374" r="16" fill="#ffffff" filter="url(#glow)"/>
  <circle cx="363" cy="371" r="5" fill="#ffffff" opacity="0.95"/>
</svg>"""

with open("assets/abstract/logo-shard.svg", "w", encoding="utf-8") as f:
    f.write(svg_shard)


# -------------------------------------------------------------------------
# Concept 2: "The Singularity / Event Horizon" (Aperture & Focal Dot)
# Brutalist, ultra-minimalist: Intersecting circular arcs / horizon lines
# converging to zero latency.
# -------------------------------------------------------------------------
svg_singularity = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#141414"/>
      <stop offset="50%" stop-color="#0a0a0a"/>
      <stop offset="100%" stop-color="#000000"/>
    </linearGradient>
    <linearGradient id="ring-grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="50%" stop-color="#737373"/>
      <stop offset="100%" stop-color="#262626"/>
    </linearGradient>
    <filter id="shadow2" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="16" stdDeviation="20" flood-color="#000000" flood-opacity="0.8"/>
    </filter>
    <filter id="glow2" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="14" result="b1"/>
      <feMerge>
        <feMergeNode in="b1"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg2)" stroke="#2b2b2b" stroke-width="2"/>

  <!-- Minimalist Brutalist Orbital Horizons -->
  <g filter="url(#shadow2)">
    <!-- Outer Arc Segment -->
    <path d="M 140 372 A 160 160 0 1 1 372 140" fill="none" stroke="url(#ring-grad)" stroke-width="28" stroke-linecap="round"/>
    
    <!-- Inner Concentric High-Speed Chevron Wedge -->
    <path d="M 200 312 A 80 80 0 0 1 312 200" fill="none" stroke="#ffffff" stroke-width="22" stroke-linecap="round"/>
  </g>

  <!-- Central Dense Focus Dot (The Point of Now) -->
  <circle cx="256" cy="256" r="30" fill="#ffffff" filter="url(#glow2)"/>
  <circle cx="252" cy="252" r="9" fill="#ffffff" opacity="0.95"/>
</svg>"""

with open("assets/abstract/logo-singularity.svg", "w", encoding="utf-8") as f:
    f.write(svg_singularity)


# -------------------------------------------------------------------------
# Concept 3: "The Fast-Forward Core" / Dual Chevron Monolith
# Forward arrows / execution prompt >> abstracted into heavy solid geometry.
# Pure "Code Now."
# -------------------------------------------------------------------------
svg_chevron = """<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="bg3" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#121212"/>
      <stop offset="100%" stop-color="#050505"/>
    </linearGradient>
    <linearGradient id="chev1" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff"/>
      <stop offset="100%" stop-color="#808080"/>
    </linearGradient>
    <linearGradient id="chev2" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#e0e0e0"/>
      <stop offset="100%" stop-color="#505050"/>
    </linearGradient>
    <filter id="shadow3" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="16" stdDeviation="22" flood-color="#000000" flood-opacity="0.9"/>
    </filter>
  </defs>

  <rect x="24" y="24" width="464" height="464" rx="108" ry="108" fill="url(#bg3)" stroke="#303030" stroke-width="2"/>

  <!-- Dual Forward Heavy Blocks (Fast-forward / prompt execution) -->
  <g filter="url(#shadow3)">
    <!-- Primary Chevron -->
    <path d="M 148 148 L 244 256 L 148 364 L 208 364 L 304 256 L 208 148 Z" fill="url(#chev1)"/>
    <!-- Secondary Offset Chevron -->
    <path d="M 252 148 L 348 256 L 252 364 L 312 364 L 408 256 L 312 148 Z" fill="url(#chev2)"/>
  </g>
</svg>"""

with open("assets/abstract/logo-chevron.svg", "w", encoding="utf-8") as f:
    f.write(svg_chevron)


# -------------------------------------------------------------------------
# Render high-resolution PNGs of each concept with Pillow
# -------------------------------------------------------------------------
def render_concept_shard(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    # Squircle frame
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(48, 48, 48, 255), width=int(s * 0.004))

    # The Shard (Interlocking faceted polygons)
    p1 = [(int(s * 0.28), int(s * 0.72)), (int(s * 0.44), int(s * 0.26)), (int(s * 0.59), int(s * 0.26)), (int(s * 0.43), int(s * 0.72))]
    p2 = [(int(s * 0.43), int(s * 0.72)), (int(s * 0.59), int(s * 0.26)), (int(s * 0.70), int(s * 0.36)), (int(s * 0.54), int(s * 0.82))]
    p3 = [(int(s * 0.44), int(s * 0.26)), (int(s * 0.59), int(s * 0.26)), (int(s * 0.70), int(s * 0.36)), (int(s * 0.55), int(s * 0.36))]

    # Draw facet 2 (darker silver)
    draw.polygon(p2, fill=(80, 80, 80, 255))
    draw.line(p2 + [p2[0]], fill=(120, 120, 120, 255), width=int(s * 0.003))

    # Draw facet 1 (brilliant white)
    draw.polygon(p1, fill=(235, 235, 235, 255))
    draw.line(p1 + [p1[0]], fill=(255, 255, 255, 255), width=int(s * 0.003))

    # Draw facet 3 (top cap specular highlight)
    draw.polygon(p3, fill=(255, 255, 255, 255))

    # Floating Period Dot
    cx, cy, dot_r = int(s * 0.72), int(s * 0.73), int(s * 0.042)
    glow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow)
    gdraw.ellipse([cx - dot_r*2, cy - dot_r*2, cx + dot_r*2, cy + dot_r*2], fill=(255, 255, 255, 90))
    glow = glow.filter(ImageFilter.GaussianBlur(int(s * 0.02)))
    img = Image.alpha_composite(img, glow)

    fdraw = ImageDraw.Draw(img)
    fdraw.ellipse([cx - dot_r, cy - dot_r, cx + dot_r, cy + dot_r], fill=(255, 255, 255, 255))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_concept_singularity(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(48, 48, 48, 255), width=int(s * 0.004))

    # Center Singularity Dot
    cx, cy = s // 2, s // 2
    arc_w = int(s * 0.04)

    # Outer arc
    bbox_outer = [int(s * 0.22), int(s * 0.22), int(s * 0.78), int(s * 0.78)]
    draw.arc(bbox_outer, start=45, end=315, fill=(160, 160, 160, 255), width=arc_w)

    # Inner arc
    bbox_inner = [int(s * 0.33), int(s * 0.33), int(s * 0.67), int(s * 0.67)]
    draw.arc(bbox_inner, start=180, end=360, fill=(240, 240, 240, 255), width=int(arc_w * 0.8))

    # Core Focal Dot
    dot_r = int(s * 0.06)
    glow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    gdraw = ImageDraw.Draw(glow)
    gdraw.ellipse([cx - dot_r*2, cy - dot_r*2, cx + dot_r*2, cy + dot_r*2], fill=(255, 255, 255, 110))
    glow = glow.filter(ImageFilter.GaussianBlur(int(s * 0.025)))
    img = Image.alpha_composite(img, glow)

    fdraw = ImageDraw.Draw(img)
    fdraw.ellipse([cx - dot_r, cy - dot_r, cx + dot_r, cy + dot_r], fill=(255, 255, 255, 255))
    fdraw.ellipse([cx - int(dot_r*0.6), cy - int(dot_r*0.6), cx - int(dot_r*0.1), cy - int(dot_r*0.1)], fill=(255, 255, 255, 255))

    return img.resize((size, size), Image.Resampling.LANCZOS)

def render_concept_chevron(size=1024):
    scale = 2
    s = size * scale
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    pad = int(s * 0.04)
    r = int(s * 0.22)
    draw.rounded_rectangle([pad, pad, s - pad, s - pad], radius=r, fill=(10, 10, 10, 255), outline=(48, 48, 48, 255), width=int(s * 0.004))

    # Fast-Forward Dual Heavy Chevrons
    ch1 = [
        (int(s * 0.28), int(s * 0.28)),
        (int(s * 0.48), int(s * 0.50)),
        (int(s * 0.28), int(s * 0.72)),
        (int(s * 0.39), int(s * 0.72)),
        (int(s * 0.59), int(s * 0.50)),
        (int(s * 0.39), int(s * 0.28)),
    ]
    ch2 = [
        (int(s * 0.49), int(s * 0.28)),
        (int(s * 0.69), int(s * 0.50)),
        (int(s * 0.49), int(s * 0.72)),
        (int(s * 0.60), int(s * 0.72)),
        (int(s * 0.80), int(s * 0.50)),
        (int(s * 0.60), int(s * 0.28)),
    ]

    draw.polygon(ch2, fill=(130, 130, 130, 255))
    draw.line(ch2 + [ch2[0]], fill=(160, 160, 160, 255), width=int(s * 0.003))

    draw.polygon(ch1, fill=(245, 245, 245, 255))
    draw.line(ch1 + [ch1[0]], fill=(255, 255, 255, 255), width=int(s * 0.003))

    return img.resize((size, size), Image.Resampling.LANCZOS)

if __name__ == "__main__":
    img_shard = render_concept_shard(1024)
    img_shard.save("assets/abstract/logo-shard.png")
    img_shard.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\abstract_shard.png")

    img_singularity = render_concept_singularity(1024)
    img_singularity.save("assets/abstract/logo-singularity.png")
    img_singularity.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\abstract_singularity.png")

    img_chevron = render_concept_chevron(1024)
    img_chevron.save("assets/abstract/logo-chevron.png")
    img_chevron.save(r"C:\Users\Deva\.gemini\antigravity\brain\a126a8d4-8565-4cbf-b8d8-483396d337fe\abstract_chevron.png")

    print("Successfully generated all 3 abstract concepts!")

