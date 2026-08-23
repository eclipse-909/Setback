/* ==========================================================================
   SETBACK — Shared UI behavior
   Generic, attribute-driven interactions reused by every screen. Screens
   add only page-specific logic (see the inline <script> near the bottom
   of each screen file) on top of what's wired up here.
   ========================================================================== */

document.addEventListener('DOMContentLoaded', () => {

  /* ---- Modals ------------------------------------------------------------
     <button data-modal-open="modalId">Open</button>
     <div class="modal-overlay" id="modalId" hidden> ... <button data-modal-close> */
  document.querySelectorAll('[data-modal-open]').forEach(btn => {
    btn.addEventListener('click', () => {
      const modal = document.getElementById(btn.dataset.modalOpen);
      if (modal) modal.hidden = false;
    });
  });
  document.querySelectorAll('[data-modal-close]').forEach(btn => {
    btn.addEventListener('click', () => {
      const overlay = btn.closest('.modal-overlay');
      if (overlay) overlay.hidden = true;
    });
  });
  document.querySelectorAll('.modal-overlay').forEach(overlay => {
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) overlay.hidden = true;
    });
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      document.querySelectorAll('.modal-overlay:not([hidden])').forEach(o => o.hidden = true);
    }
  });

  /* ---- Tabs ----------------------------------------------------------- */
  document.querySelectorAll('.tabs').forEach(tabs => {
    tabs.querySelectorAll('.tabs__tab').forEach(tab => {
      tab.addEventListener('click', () => {
        tabs.querySelectorAll('.tabs__tab').forEach(t => t.classList.remove('is-active'));
        tabs.querySelectorAll('.tabs__panel').forEach(p => p.hidden = true);
        tab.classList.add('is-active');
        const panel = tabs.querySelector(`.tabs__panel[data-panel="${tab.dataset.tab}"]`);
        if (panel) panel.hidden = false;
      });
    });
  });

  /* ---- Sliders (live fill + numeric readout) ---------------------------
     <input class="slider" data-value-target="#readoutId" data-value-suffix="%"> */
  document.querySelectorAll('.slider').forEach(slider => {
    const sync = () => {
      const min = Number(slider.min || 0), max = Number(slider.max || 100);
      const pct = ((slider.value - min) / (max - min)) * 100;
      slider.style.setProperty('--fill', pct + '%');
      const targetSel = slider.dataset.valueTarget;
      if (targetSel) {
        const target = document.querySelector(targetSel);
        if (target) target.textContent = slider.value + (slider.dataset.valueSuffix || '');
      }
    };
    slider.addEventListener('input', sync);
    sync();
  });

  /* ---- UI Scale slider — special case: writes --ui-scale on <html> -----
     <input class="slider" id="uiScaleSlider" data-ui-scale> */
  const scaleSlider = document.querySelector('[data-ui-scale]');
  if (scaleSlider) {
    scaleSlider.addEventListener('input', () => {
      document.documentElement.style.setProperty('--ui-scale', scaleSlider.value / 100);
    });
  }

  /* ---- Keybind rows ------------------------------------------------------ */
  document.querySelectorAll('.keybind-row').forEach(row => {
    const cap = row.querySelector('.keycap');
    const reset = row.querySelector('.keybind-row__reset');
    if (cap) {
      cap.addEventListener('click', () => {
        const prev = cap.textContent;
        cap.textContent = '…';
        cap.classList.add('is-listening');
        const onKey = (e) => {
          e.preventDefault();
          cap.textContent = e.key.length === 1 ? e.key.toUpperCase() : e.key.replace('Escape', 'ESC');
          cap.classList.remove('is-listening');
          window.removeEventListener('keydown', onKey);
        };
        window.addEventListener('keydown', onKey);
      });
    }
    if (reset) {
      reset.addEventListener('click', () => { cap.textContent = row.dataset.default; });
    }
  });

  /* ---- Toggle-driven visibility ------------------------------------------
     <input type="checkbox" data-toggles="#panelId"> shows/hides a panel */
  document.querySelectorAll('[data-toggles]').forEach(input => {
    const target = document.querySelector(input.dataset.toggles);
    const sync = () => { if (target) target.hidden = !input.checked; };
    input.addEventListener('change', sync);
    sync();
  });

  /* ---- Option-card single-select groups ---------------------------------
     <div data-option-group> wrapping several .option-card elements */
  document.querySelectorAll('[data-option-group]').forEach(group => {
    group.querySelectorAll('.option-card').forEach(card => {
      card.addEventListener('click', () => {
        group.querySelectorAll('.option-card').forEach(c => c.classList.remove('is-selected'));
        card.classList.add('is-selected');
      });
    });
  });

});

/* ---- Toast helper --------------------------------------------------------
   window.showToast('Message', 'error' | 'success') */
function showToast(message, type = 'error') {
  let region = document.querySelector('.toast-region');
  if (!region) {
    region = document.createElement('div');
    region.className = 'toast-region';
    document.body.appendChild(region);
  }
  const toast = document.createElement('div');
  toast.className = `toast toast--${type}`;
  const icon = type === 'success'
    ? '<path d="M20 6L9 17l-5-5"/>'
    : '<circle cx="12" cy="12" r="9"/><path d="M12 8v5M12 16h.01"/>';
  toast.innerHTML = `<svg class="toast__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">${icon}</svg><span></span>`;
  toast.querySelector('span').textContent = message;
  region.appendChild(toast);
  setTimeout(() => {
    toast.classList.add('is-leaving');
    setTimeout(() => toast.remove(), 250);
  }, 3400);
}
