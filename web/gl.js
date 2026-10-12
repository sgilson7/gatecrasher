// The WebGL 2 renderer. It uploads what the core returns and draws it; every
// size, stride and offset comes from the core's numbers(), so this file holds
// no layout of its own. Instances and meshes are the core's Q16 integers,
// read as integer attributes and scaled in the shader.
//
// Two instance buffers: the zone (uploaded when the core's scenery version
// changes) and the moving things (each frame). The core lists the draws;
// this file sets the GL state each pass asks for.

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader));
  }
  return shader;
}

function link(gl, vertexSource, fragmentSource) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertexSource));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragmentSource));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program));
  }
  return program;
}

// Returns null when the browser has no WebGL 2; the page then says so.
export function makeRenderer(canvas, N, shaders, meshData) {
  const gl = canvas.getContext("webgl2", { antialias: true, powerPreference: "high-performance" });
  if (!gl) return null;

  const info = gl.getExtension("WEBGL_debug_renderer_info");
  const name = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";
  const software = shaders.software.some((s) => name.includes(s));

  const program = link(gl, shaders.vertex, shaders.fragment);
  gl.useProgram(program);
  gl.enable(gl.DEPTH_TEST);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  const vao = gl.createVertexArray();
  gl.bindVertexArray(vao);

  const meshes = [];
  for (let id = 0; id < N.mesh_count; id++) {
    const data = meshData(id);
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
    meshes.push({ buffer, vertices: data.length / N.vertex_values });
  }

  const attribute = ([attr, size, offset]) => ({ loc: gl.getAttribLocation(program, attr), size, offset });
  const vertexAttributes = N.vertex.attributes.map(attribute);
  const instanceAttributes = N.instance.attributes.map(attribute);
  for (const a of vertexAttributes.concat(instanceAttributes)) gl.enableVertexAttribArray(a.loc);
  for (const a of instanceAttributes) gl.vertexAttribDivisor(a.loc, 1);

  const uniform = (u) => gl.getUniformLocation(program, u);
  const sceneUniforms = N.uniforms.map(([u, size]) => ({ loc: uniform(u), size }));
  const uVP = uniform("uVP");
  const uEye = uniform("uEye");
  const uPass = uniform("uPass");

  const buffers = [gl.createBuffer(), gl.createBuffer()];
  const P = N.passes;
  let clear = [0, 0, 0];

  function enterPass(pass) {
    gl.uniform1f(uPass, pass);
    if (pass === P.tint) {
      gl.disable(gl.CULL_FACE);
      gl.enable(gl.BLEND);
      gl.depthMask(false);
      return;
    }
    gl.disable(gl.BLEND);
    gl.depthMask(true);
    gl.enable(gl.CULL_FACE);
    gl.cullFace(pass === P.ink ? gl.FRONT : gl.BACK);
  }

  return {
    software,
    setScenery(values) {
      gl.bindBuffer(gl.ARRAY_BUFFER, buffers[N.buffers.scenery]);
      gl.bufferData(gl.ARRAY_BUFFER, values, gl.STATIC_DRAW);
    },
    setUniforms(uniformValues, clearColor) {
      let at = 0;
      for (const u of sceneUniforms) {
        gl[`uniform${u.size}fv`](u.loc, uniformValues.subarray(at, at + u.size));
        at += u.size;
      }
      clear = clearColor;
    },
    resize(width, height) {
      canvas.width = width;
      canvas.height = height;
    },
    frame(mats, draws, dynamic) {
      gl.bindBuffer(gl.ARRAY_BUFFER, buffers[N.buffers.dynamic]);
      gl.bufferData(gl.ARRAY_BUFFER, dynamic, gl.STREAM_DRAW);
      gl.viewport(0, 0, canvas.width, canvas.height);
      const [r, g, b] = clear;
      gl.clearColor(r, g, b, 1);
      gl.depthMask(true);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      gl.uniformMatrix4fv(uVP, false, mats.subarray(0, N.view_values));
      gl.uniform3fv(uEye, mats.subarray(N.view_values));
      let pass = -1;
      for (let i = 0; i < draws.length; i += N.draw_values) {
        const [buffer, meshId, byteOffset, count, drawPass] = draws.subarray(i, i + N.draw_values);
        if (drawPass !== pass) {
          enterPass(drawPass);
          pass = drawPass;
        }
        const mesh = meshes[meshId];
        gl.bindBuffer(gl.ARRAY_BUFFER, mesh.buffer);
        for (const a of vertexAttributes) {
          gl.vertexAttribPointer(a.loc, a.size, gl.INT, false, N.vertex.stride, a.offset);
        }
        gl.bindBuffer(gl.ARRAY_BUFFER, buffers[buffer]);
        for (const a of instanceAttributes) {
          gl.vertexAttribPointer(a.loc, a.size, gl.INT, false, N.instance.stride, a.offset + byteOffset);
        }
        gl.drawArraysInstanced(gl.TRIANGLES, 0, mesh.vertices, count);
      }
      gl.depthMask(true);
    },
  };
}
