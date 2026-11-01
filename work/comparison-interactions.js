window.createWaffleryComparison = ({ app, selectedRun, context }) => {
  const fragment = document.getElementById('wh-comparison-template').content.cloneNode(true);
  const row = fragment.querySelector('.wh-baseline-row'), panel = fragment.querySelector('.wh-comparison');
  app.querySelector('.wh-result-actions').after(row);
  app.querySelector('.wh-inspector').before(panel);
  const q = selector => app.querySelector(selector);
  const baselines = new Map();
  let previousContext = '', mode = 'side';
  const element = (tag, text, className) => { const el = document.createElement(tag); if (text) el.textContent = text; if (className) el.className = className; return el; };
  function eligible() { const run = selectedRun(), baseline = baselines.get(context()); return run?.status === 'complete' && baseline && run.id !== baseline.id && run.context === baseline.context; }
  function refresh() {
    const run = selectedRun(), baseline = baselines.get(context());
    if (previousContext !== context() || !eligible()) panel.hidden = true;
    previousContext = context();
    row.hidden = !baseline && run?.status !== 'complete';
    q('.wh-baseline-label').textContent = baseline ? 'Baseline · ' + baseline.id : 'Pin a run to compare later attempts';
    q('.wh-pin-baseline').disabled = run?.status !== 'complete' || baseline?.id === run?.id;
    q('.wh-pin-baseline').textContent = baseline?.id === run?.id ? 'Pinned' : 'Pin this run';
    q('.wh-open-comparison').disabled = !eligible();
    q('.wh-clear-baseline').hidden = !baseline;
    if (!panel.hidden) draw();
  }
  function figure(run, label) {
    const figure = element('figure'); figure.dataset.comparedRun = run.id;
    figure.append(element('figcaption', label + ' · ' + run.id));
    const visual = ['Line', 'Feather', 'Create a mask'].includes(run.action) || run.tool === 'squint';
    if (visual) {
      const canvas = element('div', '', 'wh-canvas'); canvas.dataset.view = 'overlay';
      const image = document.createElement('img'); image.src = q('.wh-photo').src; image.alt = label + ' · illustrative output for ' + run.action; image.className = 'wh-photo'; canvas.append(image);
      if (run.action === 'Line') {
        const overlay = element('div', '', 'wh-line-overlay'); overlay.style.left = run.parameters.column + '%'; overlay.style.width = (+run.parameters.width_px / 10) + '%'; canvas.append(overlay);
      } else if (run.action === 'Feather' || run.action === 'Create a mask') {
        const overlay = element('div', '', 'wh-mask-shape');
        if (run.action === 'Feather') overlay.style.filter = 'blur(' + (+run.parameters.sigma_px / 2) + 'px)';
        canvas.append(overlay);
      }
      figure.append(canvas, element('p', 'Illustrative preview · not algorithm output'));
    } else figure.append(element('div', 'This operation’s typed output will be displayed here.', 'wh-compare-empty'));
    if (run.tool === 'squint') figure.append(element('p', 'Needs review · chair silhouette may differ. Confidence: unsure. Illustrative finding; no judge was called.'));
    if (run.action === 'Line') figure.append(element('pre', JSON.stringify({ type: 'Location', column: +run.parameters.column / 100, width_px: +run.parameters.width_px }, null, 2)));
    figure.append(element('p', run.status + ' · Spend $' + run.spend.usd.toFixed(2) + ' · estimated, simulated'));
    return figure;
  }
  function draw() {
    if (!eligible()) return;
    const baseline = baselines.get(context()), run = selectedRun();
    const keys = [...new Set([...Object.keys(baseline.parameters), ...Object.keys(run.parameters)])];
    const changed = keys.filter(key => JSON.stringify(baseline.parameters[key]) !== JSON.stringify(run.parameters[key]));
    q('.wh-comparison-caption').textContent = run.action + ' · ' + changed.length + ' changed setting' + (changed.length === 1 ? '' : 's') + ' · Same bundled example inputs. Current form edits are excluded.';
    q('.wh-comparison-pair').replaceChildren(figure(baseline, 'Baseline'), figure(run, 'Selected attempt'));
    const left = q('.wh-comparison-pair figure:first-child .wh-canvas'), right = q('.wh-comparison-pair figure:last-child .wh-canvas');
    const canWipe = !!left && !!right;
    if (!canWipe) mode = 'side';
    q('[data-compare-mode=wipe]').disabled = !canWipe;
    q('.wh-comparison-pair').hidden = mode === 'wipe'; q('.wh-comparison-wipe').hidden = mode !== 'wipe'; q('.wh-comparison-wipe-note').hidden = mode !== 'wipe';
    if (mode === 'wipe') {
      q('.wh-comparison-wipe').replaceChildren(window.createWaffleryWipe({ left: left.cloneNode(true), right: right.cloneNode(true), leftLabel: 'Baseline · ' + baseline.id, rightLabel: 'Attempt · ' + run.id }));
      q('.wh-comparison-wipe-note').textContent = 'Illustrative outputs · Baseline spend $' + baseline.spend.usd.toFixed(2) + ' / attempt $' + run.spend.usd.toFixed(2) + ' · estimated, simulated';
    }
    panel.querySelectorAll('[data-compare-mode]').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.compareMode === mode)));
    const body = q('.wh-parameter-diff tbody'); body.replaceChildren();
    for (const key of keys) {
      const tr = element('tr'); if (changed.includes(key)) tr.className = 'wh-parameter-changed';
      const label = element('th', key + (changed.includes(key) ? ' · changed' : '')); label.scope = 'row';
      tr.append(label, element('td', String(baseline.parameters[key] ?? 'Not supplied')), element('td', String(run.parameters[key] ?? 'Not supplied'))); body.append(tr);
    }
  }
  q('.wh-pin-baseline').addEventListener('click', () => { const run = selectedRun(); if (run?.status !== 'complete') return; baselines.set(context(), run); panel.hidden = true; refresh(); });
  q('.wh-clear-baseline').addEventListener('click', () => { baselines.delete(context()); panel.hidden = true; refresh(); });
  q('.wh-open-comparison').addEventListener('click', () => { if (!eligible()) return; draw(); panel.hidden = false; panel.scrollIntoView?.({ behavior: 'auto', block: 'nearest' }); });
  q('.wh-close-comparison').addEventListener('click', () => { panel.hidden = true; });
  panel.querySelectorAll('[data-compare-mode]').forEach(button => button.addEventListener('click', () => { mode = button.dataset.compareMode; draw(); }));
  return { refresh, close: () => { panel.hidden = true; }, reset: () => { baselines.clear(); previousContext = ''; mode = 'side'; panel.hidden = true; } };
};
