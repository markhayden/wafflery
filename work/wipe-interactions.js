window.createWaffleryWipe = ({ left, right, leftLabel, rightLabel }) => {
  const root = document.createElement('div'); root.className = 'wh-wipe';
  const before = document.createElement('div'); before.className = 'wh-wipe-layer'; before.append(left);
  const after = document.createElement('div'); after.className = 'wh-wipe-layer wh-wipe-after'; after.append(right);
  const divider = document.createElement('div'); divider.className = 'wh-wipe-divider'; divider.setAttribute('aria-hidden', 'true');
  const handle = document.createElement('span'); handle.className = 'wh-wipe-handle'; handle.textContent = '↔'; divider.append(handle);
  const label = (text, className) => { const el = document.createElement('span'); el.textContent = text; el.className = 'wh-wipe-label ' + className; return el; };
  const range = document.createElement('input'); range.type = 'range'; range.min = '0'; range.max = '100'; range.step = '1'; range.value = '50';
  range.setAttribute('aria-label', 'Compare ' + leftLabel + ' and ' + rightLabel + ': drag divider or use arrow keys');
  const update = () => { const position = Math.max(0, Math.min(100, Number(range.value))); after.style.clipPath = 'inset(0 0 0 ' + position + '%)'; divider.style.left = position + '%'; range.setAttribute('aria-valuetext', leftLabel + ' ' + position + '%, ' + rightLabel + ' ' + (100 - position) + '%'); };
  range.addEventListener('input', update);
  root.append(before, after, divider, label(leftLabel, 'wh-wipe-label-left'), label(rightLabel, 'wh-wipe-label-right'), range); update();
  return root;
};
