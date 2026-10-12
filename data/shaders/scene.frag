#version 300 es
precision highp float;
// Three passes (the core says which): 0 solid, lit in flat bands like the
// TikZ art's fills; 1 ink, the plum outline; 2 tint, see-through shapes
// (sight cones, the landing zone's glow).
// Styles (crates/core/src/world.rs): 0 plain, 1 concrete panels,
// 2 corrugated metal, 3 camouflage, 4 glow, 5 planks, 6 ground,
// 7 active camo, 8 portal swirl.
in vec3 vWorld;
in vec3 vNormal;
in vec4 vColor;
in vec3 vLocal;
in vec3 vLocalNormal;
in vec3 vSize;
uniform vec3 uEye;
uniform vec3 uLightDir;
uniform vec3 uSky;
uniform vec3 uFog;
uniform vec2 uFogRange;
uniform vec3 uLight;
uniform vec3 uInk;
uniform float uPass;
out vec4 outColor;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453);
}

float blotch(vec3 p) {
  vec2 q = p.xz * 2.3 + p.y * 1.7;
  vec2 i = floor(q);
  vec2 f = fract(q);
  f = f * f * (3.0 - 2.0 * f);
  float a = mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x);
  float b = mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x);
  return mix(a, b, f.y);
}

void main() {
  float d = length(vWorld - uEye);
  float f = clamp((d - uFogRange.x) / (uFogRange.y - uFogRange.x), 0.0, 1.0);
  f = f * f * (3.0 - 2.0 * f);
  if (uPass > 1.5) {
    outColor = vec4(mix(vColor.rgb, uFog, f), vColor.a);
    return;
  }
  float style = floor(vColor.a + 0.001);
  if (style > 6.5 && style < 7.5) {
    // active camo: most of her is not drawn at all
    vec2 s = floor(gl_FragCoord.xy);
    if (mod(s.x + s.y * 3.0, 5.0) > 0.5) discard;
  }
  if (uPass > 0.5) {
    outColor = vec4(mix(uInk, uFog, f * 0.85), 1.0);
    return;
  }
  vec3 n = normalize(vNormal);
  vec3 base = vColor.rgb;
  vec3 ln = vLocalNormal;
  bool side = abs(ln.y) < 0.5;
  float u = abs(ln.x) > abs(ln.z) ? (vLocal.z + 0.5) * vSize.z : (vLocal.x + 0.5) * vSize.x;
  float v = (vLocal.y + 0.5) * vSize.y;
  if (style > 0.5 && style < 1.5 && side) {
    // concrete panels: a seam every 3 m and a line at 1.2 m
    if (fract(u / 3.0) < 0.02 || abs(v - 1.2) < 0.03) base *= 0.82;
  }
  if (style > 1.5 && style < 2.5 && side) {
    // corrugated sheet: vertical ribs
    base *= 0.9 + 0.1 * step(0.5, fract(u * 3.0));
  }
  if (style > 2.5 && style < 3.5) {
    // camouflage: dark blotches over the base colour
    float b = blotch(vWorld * 1.0);
    if (b > 0.55) base *= 0.6;
    else if (b < 0.25) base *= 1.12;
  }
  if (style > 4.5 && style < 5.5 && side) {
    // planks
    if (fract(v / 0.3) < 0.08) base *= 0.75;
  }
  if (style > 5.5 && style < 6.5) {
    float g = hash(floor(vWorld.xz * 1.5));
    base *= 0.95 + 0.07 * g;
  }
  if (style > 7.5) {
    // the portal: rings that brighten toward the rim
    float r = length(vLocal.xz) * 2.0;
    float ring = step(0.5, fract(r * 4.0 - atan(vLocal.z, vLocal.x) * 0.3));
    outColor = vec4(mix(base * 0.35, base * 1.25, ring * r + r * 0.5), 1.0);
    return;
  }
  if (style > 3.5 && style < 4.5) {
    outColor = vec4(mix(base * 1.15, uFog, f * 0.6), 1.0);
    return;
  }
  float diffuse = dot(n, uLightDir);
  float band = diffuse > 0.55 ? 1.0 : (diffuse > 0.1 ? 0.78 : 0.58);
  float sky = 0.5 + 0.5 * n.y;
  vec3 ambient = mix(uFog * 0.55, uSky * 0.8, sky);
  vec3 lit = base * (ambient * 0.62 + uLight * band * 0.55);
  outColor = vec4(mix(lit, uFog, f), 1.0);
}
