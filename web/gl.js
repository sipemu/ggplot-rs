// Minimal WebGL2 point renderer for the large-N scatter. The vertex shader
// projects data (x, y) → panel pixels → clip space using the frame the Rust
// side computed (panel rect + expanded domain); points are coloured by group
// index via a small palette uniform. Millions of points at interactive rates.

const VERT = `#version 300 es
precision highp float;
in vec2 a_xy;
in float a_g;
uniform vec4 u_plot;    // px, py, pw, ph  (device px)
uniform vec4 u_dom;     // xmin, xmax, ymin, ymax  (expanded)
uniform vec2 u_canvas;  // canvas w, h  (device px)
uniform float u_size;
uniform vec3 u_pal[9];
out vec3 v_col;
void main() {
  float nx = (a_xy.x - u_dom.x) / (u_dom.y - u_dom.x);
  float ny = (a_xy.y - u_dom.z) / (u_dom.w - u_dom.z);
  float px = u_plot.x + nx * u_plot.z;
  float py = u_plot.y + (1.0 - ny) * u_plot.w;   // pixel y (top-down)
  vec2 clip = vec2(px / u_canvas.x * 2.0 - 1.0, 1.0 - py / u_canvas.y * 2.0);
  gl_Position = vec4(clip, 0.0, 1.0);
  gl_PointSize = u_size;
  v_col = u_pal[int(a_g)];
}`;

const FRAG = `#version 300 es
precision highp float;
in vec3 v_col;
out vec4 o;
void main() {
  vec2 d = gl_PointCoord - vec2(0.5);
  if (dot(d, d) > 0.25) discard;   // round points
  o = vec4(v_col, 0.55);
}`;

// ColorBrewer Set1, normalised RGB (9 colours), flat for uniform3fv.
export const SET1 = new Float32Array([
  228, 26, 28, 55, 126, 184, 77, 175, 74, 152, 78, 163, 255, 127, 0,
  255, 255, 51, 166, 86, 40, 247, 129, 191, 153, 153, 153,
].map((v) => v / 255));

export function createGL(canvas) {
  const gl = canvas.getContext("webgl2", { antialias: true, premultipliedAlpha: false });
  if (!gl) throw new Error("WebGL2 not available");
  const compile = (type, src) => {
    const s = gl.createShader(type);
    gl.shaderSource(s, src); gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
    return s;
  };
  const prog = gl.createProgram();
  gl.attachShader(prog, compile(gl.VERTEX_SHADER, VERT));
  gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, FRAG));
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
  gl.useProgram(prog);

  const loc = {};
  for (const u of ["u_plot", "u_dom", "u_canvas", "u_size", "u_pal"]) loc[u] = gl.getUniformLocation(prog, u);
  const aXY = gl.getAttribLocation(prog, "a_xy");
  const aG = gl.getAttribLocation(prog, "a_g");
  const bufXY = gl.createBuffer();
  const bufG = gl.createBuffer();
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
  let n = 0;

  return {
    gl,
    // xy: Float32Array interleaved [x0,y0,x1,y1,…]; g: Float32Array group indices.
    setData(xy, g) {
      n = g.length;
      gl.bindBuffer(gl.ARRAY_BUFFER, bufXY);
      gl.bufferData(gl.ARRAY_BUFFER, xy, gl.STATIC_DRAW);
      gl.enableVertexAttribArray(aXY);
      gl.vertexAttribPointer(aXY, 2, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, bufG);
      gl.bufferData(gl.ARRAY_BUFFER, g, gl.STATIC_DRAW);
      gl.enableVertexAttribArray(aG);
      gl.vertexAttribPointer(aG, 1, gl.FLOAT, false, 0, 0);
    },
    draw(frame, dpr, size) {
      const W = canvas.width, H = canvas.height;
      gl.viewport(0, 0, W, H);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      const p = frame.plot;
      gl.uniform4f(loc.u_plot, p[0] * dpr, p[1] * dpr, p[2] * dpr, p[3] * dpr);
      gl.uniform4f(loc.u_dom, frame.xdom[0], frame.xdom[1], frame.ydom[0], frame.ydom[1]);
      gl.uniform2f(loc.u_canvas, W, H);
      gl.uniform1f(loc.u_size, size * dpr);
      gl.uniform3fv(loc.u_pal, SET1);
      gl.drawArrays(gl.POINTS, 0, n);
    },
  };
}
