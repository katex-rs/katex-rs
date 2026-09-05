// Readiness is per render, not just per page: MathML may select fonts lazily.
export function abortable(promise, signal) {
  if (signal.aborted) return Promise.reject(signal.reason);
  return new Promise((resolve, reject) => {
    const abort = () => reject(signal.reason);
    signal.addEventListener("abort", abort, { once: true });
    Promise.resolve(promise).then(resolve, reject).finally(() => {
      signal.removeEventListener("abort", abort);
    });
  });
}

export async function withRenderTimeout(task, timeoutMs) {
  const controller = new AbortController();
  const timer = setTimeout(() => {
    controller.abort(new Error(`Render readiness timed out after ${timeoutMs}ms`));
  }, timeoutMs);
  try {
    return await abortable(task(controller.signal), controller.signal);
  } finally {
    clearTimeout(timer);
  }
}

export async function waitForImage(img, signal) {
  if (!img.complete) {
    await new Promise((resolve, reject) => {
      const cleanup = () => {
        img.removeEventListener("load", loaded);
        img.removeEventListener("error", failed);
        signal.removeEventListener("abort", aborted);
      };
      const loaded = () => { cleanup(); resolve(); };
      const failed = () => {
        cleanup();
        reject(new Error(`Image failed to load: ${img.currentSrc || img.src}`));
      };
      const aborted = () => { cleanup(); reject(signal.reason); };
      img.addEventListener("load", loaded, { once: true });
      img.addEventListener("error", failed, { once: true });
      signal.addEventListener("abort", aborted, { once: true });
      // The load/error event may have fired before the listeners were attached.
      if (signal.aborted) aborted();
      else if (img.complete) (img.naturalWidth > 0 ? loaded : failed)();
    });
  }
  if (img.naturalWidth === 0) {
    throw new Error(`Image failed to load: ${img.currentSrc || img.src}`);
  }
  if (typeof img.decode === "function") await abortable(img.decode(), signal);
}

function nextFrame(signal) {
  return new Promise((resolve, reject) => {
    if (signal.aborted) { reject(signal.reason); return; }
    const abort = () => { cancelAnimationFrame(id); reject(signal.reason); };
    const id = requestAnimationFrame(() => {
      signal.removeEventListener("abort", abort);
      resolve();
    });
    signal.addEventListener("abort", abort, { once: true });
  });
}

export async function waitForRender(doc, signal, frame = nextFrame) {
  await Promise.all(Array.from(doc.querySelectorAll("img"), img => waitForImage(img, signal)));
  // Force layout before reading fonts.ready so newly inserted MathML starts
  // its font requests. Preloading only KaTeX's HTML fonts is insufficient.
  doc.body.getBoundingClientRect();
  if (doc.fonts) await abortable(doc.fonts.ready, signal);

  let previous = null;
  let stableFrames = 0;
  while (stableFrames < 2) {
    await frame(signal);
    const geometry = Array.from(
      doc.querySelectorAll("#pre, #pre *, #math, #math *, #post, #post *"),
      node => {
        const { x, y, width, height } = node.getBoundingClientRect();
        return [x, y, width, height];
      },
    );
    const current = JSON.stringify(geometry);
    stableFrames = current === previous && (!doc.fonts || doc.fonts.status === "loaded")
      ? stableFrames + 1 : 0;
    previous = current;
  }
}
