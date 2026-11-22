import { copyText } from './dom.js';
import { uiIcon } from './icons.js';

const reducedMotion = () =>
  window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;

/**
 * Smoothly moves a shared indicator pill to match the active element's geometry.
 * Uses FLIP-style direct transform updates; skips animation on initial paint.
 */
export function syncSlidingIndicator(container, activeEl, indicatorEl, immediate = false) {
  if (!container || !indicatorEl) return;
  if (!activeEl) {
    indicatorEl.style.opacity = '0';
    return;
  }
  const containerRect = container.getBoundingClientRect();
  const activeRect = activeEl.getBoundingClientRect();
  if (containerRect.width === 0 || activeRect.width === 0) return;

  const x = activeRect.left - containerRect.left;
  const y = activeRect.top - containerRect.top;

  const skip = immediate || reducedMotion() || indicatorEl.dataset.ready !== '1';
  if (skip) {
    indicatorEl.style.transition = 'none';
  } else {
    indicatorEl.style.transition = '';
  }

  indicatorEl.style.width = `${activeRect.width}px`;
  indicatorEl.style.height = `${activeRect.height}px`;
  indicatorEl.style.transform = `translate3d(${x}px, ${y}px, 0)`;
  indicatorEl.style.opacity = '1';

  if (skip) {
    // Force commit before restoring transition for subsequent glides.
    void indicatorEl.offsetWidth;
    indicatorEl.dataset.ready = '1';
    indicatorEl.style.transition = '';
  }
}

/**
 * Attaches a subtle GPU-friendly cursor spotlight (--mx, --my) to a container.
 */
export function attachSpotlight(node) {
  if (!node || reducedMotion()) return;
  let raf = 0;
  let nextX = 0;
  let nextY = 0;

  node.addEventListener('pointermove', (event) => {
    const rect = node.getBoundingClientRect();
    nextX = event.clientX - rect.left;
    nextY = event.clientY - rect.top;
    if (raf) return;
    raf = window.requestAnimationFrame(() => {
      raf = 0;
      node.style.setProperty('--mx', `${nextX.toFixed(1)}px`);
      node.style.setProperty('--my', `${nextY.toFixed(1)}px`);
    });
  });

  node.addEventListener('pointerleave', () => {
    if (raf) {
      window.cancelAnimationFrame(raf);
      raf = 0;
    }
    node.style.removeProperty('--mx');
    node.style.removeProperty('--my');
  });
}

/**
 * Animates an integer counter smoothly from its previous displayed value.
 */
export function animateNumber(node, target, prevValue = null) {
  if (!node) return;
  const end = Number(target) || 0;
  const start = prevValue == null ? end : Number(prevValue) || 0;
  if (start === end || reducedMotion()) {
    node.textContent = String(end);
    return;
  }
  const duration = 260;
  const t0 = performance.now();
  function frame(now) {
    const progress = Math.min(1, (now - t0) / duration);
    const eased = 1 - Math.pow(1 - progress, 3);
    const current = Math.round(start + (end - start) * eased);
    node.textContent = String(current);
    if (progress < 1) {
      window.requestAnimationFrame(frame);
    }
  }
  window.requestAnimationFrame(frame);
}

/**
 * Tactile in-place morph for copy buttons (copy icon -> emerald checkmark -> restore).
 */
export async function morphCopyButton(button, text, idleLabel = '复制') {
  const ok = await copyText(text);
  if (!button || !button.isConnected) return ok;

  if (button._copyTimer) {
    window.clearTimeout(button._copyTimer);
  }

  button.classList.remove('is-copied');
  while (button.firstChild) button.removeChild(button.firstChild);

  if (ok) {
    button.classList.add('is-copied');
    button.appendChild(uiIcon('check', 'ui-icon'));
    button.appendChild(document.createTextNode('已复制'));
  } else {
    button.appendChild(document.createTextNode('复制失败'));
  }

  button._copyTimer = window.setTimeout(() => {
    if (!button.isConnected) return;
    button.classList.remove('is-copied');
    while (button.firstChild) button.removeChild(button.firstChild);
    button.appendChild(uiIcon('copy', 'ui-icon'));
    button.appendChild(document.createTextNode(idleLabel));
    button._copyTimer = null;
  }, 1500);

  return ok;
}

/**
 * DesignBookmark-inspired Hold-to-Confirm interaction for destructive actions.
 * Enforces the full hold duration (including under prefers-reduced-motion);
 * reduced motion only disables animated progress sweep and keeps static feedback.
 * Returns a dispose() function that cancels active timers and global listeners.
 */
export function attachHoldToConfirm(button, { durationMs = 780, onConfirm, holdingLabel = '保持按住…' }) {
  let holding = false;
  let triggered = false;
  let disposed = false;
  let startTime = 0;
  let raf = 0;

  const labelEl = button.querySelector('[data-hold-label]');
  const idleLabel = labelEl ? labelEl.textContent : '';

  const setProgress = (ratio) => {
    button.style.setProperty('--hold-progress', String(Math.max(0, Math.min(1, ratio))));
  };

  const reset = () => {
    holding = false;
    if (raf) {
      window.cancelAnimationFrame(raf);
      raf = 0;
    }
    button.classList.remove('is-holding');
    if (labelEl && !triggered) {
      labelEl.textContent = idleLabel;
    }
    if (!triggered) setProgress(0);
  };

  const tick = (now) => {
    if (disposed || !holding || !button.isConnected || button.disabled) {
      reset();
      return;
    }
    const elapsed = Math.max(0, now - startTime);
    const ratio = Math.min(1, elapsed / durationMs);
    if (!reducedMotion()) {
      setProgress(ratio);
    }
    if (elapsed >= durationMs) {
      holding = false;
      triggered = true;
      raf = 0;
      setProgress(1);
      button.classList.remove('is-holding');
      button.classList.add('is-confirmed');
      if (labelEl) {
        labelEl.textContent = idleLabel;
      }
      if (!disposed && button.isConnected && !button.disabled) {
        onConfirm();
      }
      return;
    }
    raf = window.requestAnimationFrame(tick);
  };

  const begin = () => {
    if (disposed || !button.isConnected || button.disabled || holding || triggered) return;
    holding = true;
    startTime = performance.now();
    button.classList.add('is-holding');
    if (reducedMotion()) {
      setProgress(0);
      if (labelEl) labelEl.textContent = holdingLabel;
    }
    raf = window.requestAnimationFrame(tick);
  };

  const onWindowBlur = () => reset();
  const onVisibilityChange = () => {
    if (document.hidden) reset();
  };

  button.addEventListener('pointerdown', (event) => {
    if (event.button !== 0) return;
    button.setPointerCapture?.(event.pointerId);
    begin();
  });
  button.addEventListener('pointerup', reset);
  button.addEventListener('pointercancel', reset);
  button.addEventListener('pointerleave', reset);
  button.addEventListener('keydown', (event) => {
    if ((event.key === 'Enter' || event.key === ' ') && !event.repeat) {
      event.preventDefault();
      begin();
    }
  });
  button.addEventListener('keyup', (event) => {
    if (event.key === 'Enter' || event.key === ' ') {
      reset();
    }
  });
  button.addEventListener('blur', reset);
  window.addEventListener('blur', onWindowBlur);
  document.addEventListener('visibilitychange', onVisibilityChange);

  return () => {
    disposed = true;
    reset();
    window.removeEventListener('blur', onWindowBlur);
    document.removeEventListener('visibilitychange', onVisibilityChange);
  };
}
