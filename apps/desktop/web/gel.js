// WebGL2 gel: shades the slime outline as a thick translucent dome. The fragment shader derives
// a height field from the distance to the outline, then lights it (diffuse, two speculars,
// fresnel rim, sky reflection, subsurface glow, core, bubbles).

import { BUBBLE_COUNT } from './bubbles.js';

export const MAX_OUTLINE = 192;
export const MAX_POLYS = 8;
// Shading resolution relative to the screen. Below 1 the silhouette crawls when the sprite moves.
const RENDER_SCALE = 1;
export const MAX_BUBBLES = BUBBLE_COUNT;

export const PALETTES = {
  blue: { top: [0.55, 0.86, 1.0], mid: [0.2, 0.58, 0.95], bot: [0.08, 0.32, 0.85], light: [0.8, 0.94, 1.0], core: [0.06, 0.25, 0.73] },
  rose: { top: [1.0, 0.84, 0.9], mid: [1.0, 0.5, 0.72], bot: [0.87, 0.22, 0.54], light: [1.0, 0.9, 0.95], core: [0.73, 0.14, 0.42] },
  mint: { top: [0.82, 1.0, 0.91], mid: [0.36, 0.89, 0.67], bot: [0.07, 0.62, 0.46], light: [0.88, 1.0, 0.95], core: [0.03, 0.42, 0.31] },
  amber: { top: [1.0, 0.92, 0.63], mid: [1.0, 0.69, 0.24], bot: [0.83, 0.36, 0.07], light: [1.0, 0.95, 0.8], core: [0.65, 0.23, 0.03] },
  egg: { top: [1.0, 0.97, 0.9], mid: [0.96, 0.89, 0.76], bot: [0.82, 0.69, 0.5], light: [1.0, 0.98, 0.93], core: [0.86, 0.7, 0.45] },
  violet: { top: [0.85, 0.77, 1.0], mid: [0.59, 0.42, 0.94], bot: [0.31, 0.19, 0.7], light: [0.93, 0.88, 1.0], core: [0.18, 0.09, 0.5] },
};

const VERTEX = `#version 300 es
uniform vec4 uRect;
uniform vec2 uView;
uniform vec2 uViewSize;
out vec2 vP;
void main() {
  vec2 c = vec2(float(gl_VertexID & 1), float((gl_VertexID >> 1) & 1));
  vec2 p = mix(uRect.xy, uRect.zw, c);
  vP = p;
  vec2 ndc = (p - uView) / uViewSize * 2.0 - 1.0;
  gl_Position = vec4(ndc.x, -ndc.y, 0.0, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 vP;
out vec4 outColor;

uniform vec2 uPts[${MAX_OUTLINE}];
uniform int uCount;
uniform int uPolys;
uniform int uEnds[${MAX_POLYS}];
uniform vec2 uCenter;
uniform vec2 uUp;
uniform vec2 uBodyUp;
uniform vec3 uSupport;
uniform vec2 uHalf;
uniform float uThick;
uniform float uPolyThick[${MAX_POLYS}];
uniform float uSquash;
uniform float uTime;
uniform float uContact;
uniform float uOpacity;
uniform float uEdge;
uniform float uGlow;
uniform float uGloss;
uniform vec4 uLayers;
uniform vec2 uLayersExtra;
uniform float uRipple;
uniform float uCoreStrength;
uniform vec2 uCore;
uniform float uCoreRadius;
uniform vec3 uTop;
uniform vec3 uMid;
uniform vec3 uBot;
uniform vec3 uLight;
uniform vec3 uCoreCol;
uniform vec4 uBubbles[${MAX_BUBBLES}];

float smin(float a, float b, float k) {
  float h = max(k - abs(a - b), 0.0) / k;
  return min(a, b) - h * h * k * 0.25;
}

float segment(vec2 p, vec2 a, vec2 b, out vec2 q) {
  vec2 ab = b - a;
  float t = clamp(dot(p - a, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0);
  q = a + ab * t;
  return length(p - q);
}

void main() {
  vec2 p = vP;
  float sd = 1e9;
  vec2 pulls[${MAX_POLYS}];
  float dists[${MAX_POLYS}];
  // The body is the first polygon; limbs (claws, legs, a tail) are further polygons merged by a smooth union.
  int start = 0;
  for (int k = 0; k < ${MAX_POLYS}; k++) {
    if (k >= uPolys) break;
    int end = uEnds[k];
    float dmin = 1e9;
    bool inside = false;
    vec2 pullK = vec2(0.0);
    for (int i = 0; i < ${MAX_OUTLINE}; i++) {
      int at = start + i;
      if (at >= end) break;
      vec2 a = uPts[at];
      vec2 b = uPts[at + 1 < end ? at + 1 : start];
      vec2 q;
      float d = segment(p, a, b, q);
      dmin = min(dmin, d);
      pullK += (p - q) / max(d, 1e-3) * exp(-0.25 * d);
      if ((a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x) inside = !inside;
    }
    float sdK = inside ? -dmin : dmin;
    pulls[k] = pullK;
    dists[k] = sdK;
    sd = k == 0 ? sdK : smin(sd, sdK, 7.0);
    start = end;
  }
  // Each polygon is lit with its own gel thickness: a thin claw or tail would otherwise look like a flat sheet.
  float weights = 0.0, thicks = 0.0;
  for (int k = 0; k < ${MAX_POLYS}; k++) {
    if (k >= uPolys) break;
    float w = exp(-0.5 * max(dists[k] - sd, 0.0));
    weights += w;
    thicks += w * uPolyThick[k];
  }
  float thickHere = thicks / max(weights, 1e-4);
  // Only the edges that make up the merged outline bend the surface: a limb's hidden edge inside the body must not.
  vec2 pull = vec2(0.0);
  for (int k = 0; k < ${MAX_POLYS}; k++) {
    if (k >= uPolys) break;
    pull += pulls[k] * exp(-0.5 * max(dists[k] - sd, 0.0));
  }
  float cover = smoothstep(1.0, -1.0, sd);
  if (cover <= 0.002) {
    vec2 tang = vec2(-uUp.y, uUp.x);
    float across = dot(p - uSupport.xy, tang) / (uSupport.z + 4.0);
    float height = dot(p - uSupport.xy, uUp);
    float shadow = exp(-3.0 * across * across - pow(height / 2.8, 2.0)) * 0.25 * uContact;
    float tint = exp(-5.0 * pow(across - 0.3, 2.0) - pow(height / 1.4, 2.0)) * min(uGlow, 0.5) * 0.12 * uContact;
    float a = shadow + tint * (1.0 - shadow);
    vec3 color = vec3(0.025, 0.035, 0.05) * shadow + uMid * tint * (1.0 - shadow);
    outColor = vec4(color, a);
    return;
  }

  float d = max(-sd, 0.0);
  vec2 inward = d > 0.5 && length(pull) > 1e-12 ? normalize(pull) : normalize(uCenter - p + vec2(1e-4));
  float t = clamp(d / thickHere, 0.0, 1.0);
  float h = sqrt(max(1.0 - (1.0 - t) * (1.0 - t), 0.0));
  float slope = min((1.0 - t) / max(h, 0.08), 2.6);
  vec2 rel = (p - uCenter) / max(uHalf, vec2(1.0));
  vec2 ripple = uRipple * vec2(sin(p.x * 0.11 + uTime * 1.3 + p.y * 0.07), cos(p.y * 0.13 - uTime * 1.1 + p.x * 0.05)) * smoothstep(0.0, thickHere, d);
  vec3 n = normalize(vec3(-inward * slope * 0.9 + rel * 0.32 + ripple, 1.0));

  vec3 L1 = normalize(vec3(-0.45, -0.75, 0.55));
  vec3 L2 = normalize(vec3(0.62, 0.45, 0.5));
  vec3 V = vec3(0.0, 0.0, 1.0);

  float g = clamp(dot(p - uCenter, uUp) / max(uHalf.y, 1.0), -1.0, 1.0);
  vec3 col = mix(uBot, uMid, smoothstep(-1.0, 0.15, g));
  col = mix(col, uTop, smoothstep(0.05, 1.0, g));

  float layerWeight = dot(uLayers, vec4(1.0)) + dot(uLayersExtra, vec2(1.0));
  if (layerWeight > 0.0) {
    vec2 tangent = vec2(-uBodyUp.y, uBodyUp.x);
    vec2 uv = vec2(dot(p - uCenter, tangent), dot(p - uCenter, uBodyUp)) / max(uHalf, vec2(1.0));
    vec3 coating = vec3(0.0);
    if (uLayers.x > 0.0) {
      float stripe = sin(uv.y * 17.0 + sin(uv.x * 7.0) * 1.8);
      float aa = max(fwidth(stripe), 0.02);
      coating += uLayers.x * mix(col, uBot * 0.65, smoothstep(0.35 - aa, 0.35 + aa, stripe) * 0.55);
    }
    if (uLayers.y > 0.0) {
      vec2 q = uv * 5.0;
      vec2 cell = vec2(1.73205, 1.0);
      vec2 a = mod(q, cell) - cell * 0.5;
      vec2 b = mod(q - cell * 0.5, cell) - cell * 0.5;
      vec2 hex = dot(a, a) < dot(b, b) ? a : b;
      float edge = max(dot(abs(hex), vec2(0.866025, 0.5)), abs(hex.y));
      float aa = max(fwidth(edge), 0.01);
      float seam = smoothstep(0.42 - aa, 0.42 + aa, edge);
      coating += uLayers.y * mix(mix(col, uLight, 0.12), uCoreCol, seam * 0.4);
    }
    if (uLayers.z > 0.0) {
      vec2 q = uv * 4.0;
      vec2 tile = floor(q);
      float triangle = step(fract(q).x, fract(q).y);
      float facet = fract(sin(dot(tile, vec2(127.1, 311.7)) + triangle * 74.7) * 43758.5453);
      coating += uLayers.z * mix(col * 0.85, mix(col, uLight, 0.3), facet);
    }
    if (uLayers.w > 0.0) {
      float angle = atan(uv.y, uv.x);
      float strand = sin(angle * 80.0 + length(uv) * 22.0);
      float aa = max(fwidth(strand), 0.03);
      float fringe = (1.0 - smoothstep(0.0, thickHere * 0.8, d)) * smoothstep(-aa, aa, strand);
      coating += uLayers.w * mix(col, uLight, fringe * 0.4);
    }
    if (uLayersExtra.x > 0.0) {
      float vein = abs(sin(uv.x * 8.0 + sin(uv.y * 9.0)) + sin(uv.y * 7.0 + uv.x * 2.0));
      float aa = max(fwidth(vein), 0.02);
      float molten = 1.0 - smoothstep(0.12, 0.12 + aa, vein);
      coating += uLayersExtra.x * mix(col * 0.8, mix(uCoreCol, uLight, 0.6), molten * 0.75);
    }
    if (uLayersExtra.y > 0.0) {
      vec2 q = uv * 6.0;
      vec2 tile = floor(q);
      float seed = fract(sin(dot(tile, vec2(127.1, 311.7))) * 43758.5453);
      vec2 star = fract(q) - vec2(0.25 + seed * 0.5, 0.3 + fract(seed * 7.0) * 0.4);
      float rays = min(length(star / vec2(0.18, 0.045)), length(star / vec2(0.045, 0.18)));
      float aa = max(fwidth(rays), 0.05);
      float shine = (1.0 - smoothstep(1.0 - aa, 1.0 + aa, rays)) * step(0.65, seed);
      coating += uLayersExtra.y * mix(col, uLight, shine * 0.8);
    }
    col = mix(col, coating / layerWeight, min(layerWeight, 1.0));
  }

  float diffuse = dot(n, L1) * 0.5 + 0.5;
  col *= 0.7 + 0.45 * diffuse;
  col = mix(col, uBot * 0.7, smoothstep(0.0, thickHere * 1.6, d) * 0.3 * smoothstep(0.3, -0.9, g));

  float fres = pow(1.0 - n.z, 1.6);
  float alpha = uOpacity + uEdge * fres + 0.14 * smoothstep(0.1, -1.0, g);

  float pit = length((p - uCore) / vec2(1.0, max(uSquash, 0.5))) / uCoreRadius;
  float core = exp(-pit * pit);
  col = mix(col, uCoreCol, core * uCoreStrength);
  alpha += core * 0.28;

  float sss = smoothstep(0.1, -1.0, g) * smoothstep(0.0, thickHere * 1.4, d) * (1.0 - fres);
  col += uLight * 0.14 * sss;

  col += uLight * pow(1.0 - n.z, 3.0) * 0.5;
  vec3 r = reflect(-V, n);
  vec3 sky = mix(vec3(0.2, 0.3, 0.5), vec3(0.85, 0.93, 1.0), smoothstep(-0.2, 0.7, -r.y));
  col += sky * 0.2 * fres;
  float window = smoothstep(0.07, 0.0, abs(r.x + 0.36) - 0.2) * smoothstep(0.07, 0.0, abs(r.y + 0.56) - 0.13);
  col += vec3(1.0) * window * 0.5 * uGloss;
  float glint = pow(max(dot(n, normalize(normalize(vec3(0.15, -0.9, 0.35)) + V)), 0.0), 160.0);
  col += vec3(1.0) * glint * 0.9 * uGloss;
  alpha = max(alpha, window * 0.8 + glint);

  float wave = sin(p.x * 0.09 + sin(p.y * 0.13 + uTime * 0.7) * 1.6) * sin(p.y * 0.11 + sin(p.x * 0.1 - uTime * 0.6) * 1.5);
  col += uLight * pow(max(wave, 0.0), 3.0) * smoothstep(0.2, -0.8, g) * 0.12;
  col += uLight * 0.14 * exp(-pit * pit * 0.25);

  float s1 = pow(max(dot(n, normalize(L1 + V)), 0.0), 70.0) + 0.3 * pow(max(dot(n, normalize(L1 + V)), 0.0), 14.0) + 0.12 * pow(max(dot(n, normalize(L1 + V)), 0.0), 5.0);
  float s2 = 0.12 * pow(max(dot(n, normalize(L2 + V)), 0.0), 40.0);
  col += (vec3(1.0) * s1 + uLight * s2) * uGloss;
  alpha = max(alpha, clamp(s1 + s2, 0.0, 1.0));

  for (int k = 0; k < ${MAX_BUBBLES}; k++) {
    vec4 b = uBubbles[k];
    if (b.z <= 0.0 || b.w <= 0.0) continue;
    vec2 q = (p - b.xy) / b.z;
    float rr = length(q);
    float edge = 1.0 / b.z;
    float within = smoothstep(1.0 + edge, 1.0 - edge, rr);
    if (within <= 0.001) continue;
    vec3 bn = vec3(q, sqrt(max(1.0 - rr * rr, 0.0)));
    float rim = pow(1.0 - bn.z, 2.2);
    float glint = pow(max(dot(bn, normalize(L1 + V)), 0.0), 36.0);
    float focus = pow(max(dot(bn, normalize(vec3(-L1.xy, L1.z) + V)), 0.0), 12.0);
    float shade = smoothstep(0.7, 1.0, rr) * 0.2;
    col = col * (1.0 - shade * b.w * within) + vec3(rim * 0.55 + focus * 0.3 + glint * 1.1 + 0.03) * b.w * within;
    alpha = max(alpha, (rim * 0.65 + glint + focus * 0.3 + 0.05) * b.w * within);
  }

  float contactShade = uContact * exp(-max(dot(p - uSupport.xy, uUp), 0.0) / 4.0);
  col = mix(col, uBot * 0.38, contactShade * 0.62);
  col *= 1.0 - smoothstep(2.0, 0.0, d) * 0.1;
  alpha = clamp(alpha, 0.0, 1.0) * cover;
  outColor = vec4(clamp(col, 0.0, 1.0) * alpha, alpha);
}`;

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
  return shader;
}

export function createGel(canvas) {
  const gl = canvas.getContext('webgl2', { alpha: true, premultipliedAlpha: true, preserveDrawingBuffer: false, antialias: false });
  if (!gl) throw new Error('WebGL2 is not available');
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, VERTEX));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, FRAGMENT));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
  gl.useProgram(program);
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);

  const loc = {};
  for (const name of ['uRect', 'uView', 'uViewSize', 'uCount', 'uPolys', 'uCenter', 'uUp', 'uBodyUp', 'uSupport', 'uHalf', 'uThick', 'uSquash', 'uTime', 'uContact', 'uOpacity', 'uEdge', 'uGlow', 'uGloss', 'uRipple', 'uCoreStrength', 'uCore', 'uCoreRadius',
    'uLayers', 'uLayersExtra', 'uTop', 'uMid', 'uBot', 'uLight', 'uCoreCol']) loc[name] = gl.getUniformLocation(program, name);
  loc.uPts = gl.getUniformLocation(program, 'uPts[0]');
  loc.uEnds = gl.getUniformLocation(program, 'uEnds[0]');
  loc.uPolyThick = gl.getUniformLocation(program, 'uPolyThick[0]');
  const thickBuffer = new Float32Array(MAX_POLYS);
  const endsBuffer = new Int32Array(MAX_POLYS);
  loc.uBubbles = gl.getUniformLocation(program, 'uBubbles[0]');
  const bubbleBuffer = new Float32Array(MAX_BUBBLES * 4);

  return {
    resize(cssWidth, cssHeight, dpr) {
      canvas.width = Math.round(cssWidth * dpr * RENDER_SCALE);
      canvas.height = Math.round(cssHeight * dpr * RENDER_SCALE);
      gl.viewport(0, 0, canvas.width, canvas.height);
    },
    /** Clears the sprite and repaints the slime; view is the sprite origin in world px. */
    draw({ view, viewSize, quad, outline, count, ends, polyThick, center, up, bodyUp = up, support, half, thick, squash, time, contact, core, coreRadius, palette, material, bubbles }) {
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.uniform4f(loc.uRect, quad.x0, quad.y0, quad.x1, quad.y1);
      gl.uniform2f(loc.uView, view.x, view.y);
      gl.uniform2f(loc.uViewSize, viewSize.w, viewSize.h);
      gl.uniform2fv(loc.uPts, outline);
      gl.uniform1i(loc.uCount, count);
      const polys = ends && ends.length ? ends : [count];
      endsBuffer.fill(0);
      endsBuffer.set(polys.slice(0, MAX_POLYS));
      gl.uniform1i(loc.uPolys, Math.min(polys.length, MAX_POLYS));
      gl.uniform1iv(loc.uEnds, endsBuffer);
      thickBuffer.fill(thick);
      if (polyThick) thickBuffer.set(polyThick.slice(0, MAX_POLYS));
      thickBuffer[0] = thick;
      gl.uniform1fv(loc.uPolyThick, thickBuffer);
      gl.uniform2f(loc.uCenter, center.x, center.y);
      gl.uniform2f(loc.uUp, up.x, up.y);
      gl.uniform2f(loc.uBodyUp, bodyUp.x, bodyUp.y);
      gl.uniform3f(loc.uSupport, support.x, support.y, support.width);
      gl.uniform2f(loc.uHalf, half.x, half.y);
      gl.uniform1f(loc.uThick, thick);
      gl.uniform1f(loc.uSquash, squash);
      gl.uniform1f(loc.uTime, time);
      gl.uniform1f(loc.uContact, contact);
      gl.uniform1f(loc.uOpacity, material.opacity);
      gl.uniform1f(loc.uEdge, material.edgeOpacity);
      gl.uniform1f(loc.uGlow, material.glow);
      gl.uniform1f(loc.uGloss, material.gloss);
      const layers = material.layers;
      gl.uniform4f(loc.uLayers, layers?.stripes ?? 0, layers?.honey ?? 0, layers?.crystal ?? 0, layers?.fur ?? 0);
      gl.uniform2f(loc.uLayersExtra, layers?.lava ?? 0, layers?.stars ?? 0);
      gl.uniform1f(loc.uRipple, material.ripple);
      gl.uniform1f(loc.uCoreStrength, material.coreStrength);
      gl.uniform2f(loc.uCore, core.x, core.y);
      gl.uniform1f(loc.uCoreRadius, coreRadius);
      gl.uniform3fv(loc.uTop, palette.top);
      gl.uniform3fv(loc.uMid, palette.mid);
      gl.uniform3fv(loc.uBot, palette.bot);
      gl.uniform3fv(loc.uLight, palette.light);
      gl.uniform3fv(loc.uCoreCol, palette.core);
      bubbleBuffer.fill(0);
      bubbles.slice(0, MAX_BUBBLES).forEach((b, i) => bubbleBuffer.set([b.x, b.y, b.r, b.a], i * 4));
      gl.uniform4fv(loc.uBubbles, bubbleBuffer);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    },
  };
}
