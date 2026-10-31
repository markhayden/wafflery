window.createWafflehouseExamples = ({ app, drawer, selectedRun, tool, applyExample }) => {
  const fragment = document.getElementById('wh-examples-template').content.cloneNode(true);
  const gallery = fragment.querySelector('.wh-examples'), form = fragment.querySelector('.wh-save-example');
  app.querySelector('.wh-playground').prepend(gallery);
  app.querySelector('.wh-baseline-row').after(form);
  const q = selector => app.querySelector(selector);
  const element = (tag, text) => { const e = document.createElement(tag); e.textContent = text; return e; };
  const bundled = [
    { name: 'Place a line', notes: 'Define a region at a known position. Runs locally.', tool: 'rascal', action: 'Line', workspace: 'operations', parameters: { column: '50', width_px: '20' } },
    { name: 'Soften a saved mask', notes: 'Inspect the blend edge without rerunning a finder.', tool: 'rascal', action: 'Feather', workspace: 'operations', parameters: { sigma_px: '4', support: '3', mask: 'Saved example · armchair core' } },
    { name: 'Select the foreground chair', notes: 'Prompt-based mask. Requires a configured segmenter.', tool: 'rascal', action: 'Create a mask', workspace: 'playground', parameters: { prompt: 'The white armchair in the foreground on the left.', grow_px: '4', feather_px: '4', configuration: 'Tool-configured defaults (illustrative)' } },
    { name: 'Review an interior edit', notes: 'Inspect preservation of the room and furniture. Requires a judge.', tool: 'squint', action: 'Check an edit', workspace: 'playground', parameters: { prompt: 'Preserve the furniture, lighting, and room layout.', configuration: 'Tool-configured defaults (illustrative)' } },
  ];
  let saved = [], source = null, previousTool = tool();
  function draw() {
    const groups = q('.wh-example-groups'); groups.replaceChildren();
    for (const [name, list] of [['Bundled', bundled], ['Saved by you', saved]]) {
      groups.append(element('h4', name));
      const matches = list.filter(example => example.tool === tool());
      if (!matches.length) groups.append(element('p', 'No saved examples yet. Save a completed run to add one.'));
      for (const example of matches) {
        const row = element('div', ''); row.className = 'wh-example-entry';
        const copy = element('div', ''); copy.append(element('strong', example.name), element('p', example.action + ' · ' + (example.notes || (example.source_run ? 'From ' + example.source_run : 'Bundled starter'))));
        const open = element('button', 'Open'); open.type = 'button'; open.setAttribute('aria-label', 'Open example: ' + example.name);
        open.addEventListener('click', () => { drawer.close(); applyExample(JSON.parse(JSON.stringify(example))); });
        row.append(copy, open); groups.append(row);
      }
    }
  }
  q('.wh-example').addEventListener('click', () => { draw(); drawer.open({ heading: 'Examples', description: tool() === 'rascal' ? 'Rascal · Bundled starters and your saved setups' : 'Squint · Bundled starters and your saved setups', node: gallery, trigger: q('.wh-example') }); });
  q('.wh-save').addEventListener('click', () => {
    const run = selectedRun(); if (run?.status !== 'complete') return;
    source = JSON.parse(JSON.stringify(run));
    q('.wh-example-source').textContent = 'Uses saved inputs and settings from ' + run.id + ', including its required artifacts. Current form edits are excluded. Saved only in this demo session.';
    q('[name=example_name]').value = run.action + ' · ' + run.id; q('[name=example_notes]').value = '';
    q('.wh-example-error').hidden = true; drawer.open({ heading: 'Save as example', description: run.id + ' · ' + run.tool + ' / ' + run.action, node: form, trigger: q('.wh-save') }); q('[name=example_name]').focus();
  });
  q('.wh-cancel-example').addEventListener('click', () => { drawer.close(); source = null; });
  form.addEventListener('submit', event => {
    event.preventDefault(); if (!source) return;
    const name = q('[name=example_name]').value.trim();
    if (!name) { q('.wh-example-error').textContent = 'Give this example a name.'; q('.wh-example-error').hidden = false; return; }
    saved.unshift({ name, notes: q('[name=example_notes]').value.trim(), source_run: source.id, tool: source.tool, action: source.action, workspace: source.workspace, parameters: source.parameters, inputs: source.inputs });
    drawer.close(); source = null; q('.wh-status').textContent = 'Example saved for this demo session. Find it under Open example → Saved by you.';
  });
  return {
    refresh: () => { if (previousTool !== tool()) { gallery.hidden = true; form.hidden = true; source = null; previousTool = tool(); } },
    reset: () => { saved = []; source = null; gallery.hidden = true; form.hidden = true; },
    close: () => { gallery.hidden = true; form.hidden = true; source = null; },
  };
};
