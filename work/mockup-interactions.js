(() => {
  const root = document.getElementById('wh-hierarchy');
  const photo = JSON.parse(document.getElementById('wh-preview-image').textContent);
  const shell = root.querySelector('#wh-shell-template');
  const taskNames = {
    rascal: ['Repair a seam', 'Remove an object', 'Repair damage', 'Create a mask', 'Protect a previous edit'],
    squint: ['Check an edit', 'Check a reference subject', 'Check realism', 'Check a brief', 'Check a product'],
  };
  // Illustrative catalogue; the actual application will read tool declarations.
  const operations = [
    ['Line', 'Define regions', 'Image → Location', 'Define a line at a position or along a path. Does not detect a seam.'],
    ['Rectangle', 'Define regions', 'Image → Location', 'Define a rectangular region in image coordinates.'],
    ['Stroke', 'Define regions', 'Image → Location', 'Define a path with a brush radius.'],
    ['Segment', 'Find objects', 'Image + prompt → Locations + Images', 'Find object instances using the configured segmenter.'],
    ['Rasterize', 'Refine masks', 'Location + Image → Mask', 'Turn region geometry into a mask.'],
    ['Tighten', 'Refine masks', 'Stroke + Image → Mask + Series', 'Fit a rough stroke to the edges of a defect.'],
    ['Grow', 'Refine masks', 'Mask → Mask', 'Expand the mask core, optionally extending downward for a shadow.'],
    ['Shrink', 'Refine masks', 'Mask → Mask', 'Reduce the mask core.'],
    ['Close', 'Refine masks', 'Mask → Mask', 'Fill small gaps and notches in the mask.'],
    ['Subtract', 'Refine masks', 'Mask + Mask/Location → Mask', 'Protect an excluded region.'],
    ['Select', 'Refine masks', 'Instance images → Mask', 'Combine selected object instances into a mask.'],
    ['Invert', 'Refine masks', 'Mask → Mask', 'Make the previously untouched area available for editing.'],
    ['Fan', 'Refine masks', 'Image + Line → BandProfile', 'Measure an adaptive repair band around a known column line.'],
    ['Profile', 'Refine masks', 'BandProfile → Mask', 'Build a mask with a softened edge from the band profile.'],
    ['Feather', 'Refine masks', 'Mask → Mask', 'Replace a mask’s blend edge without rerunning the finder or filler.'],
    ['Cut', 'Prepare images', 'Panorama → Image + Plan', 'Bring the panorama’s edge strips together in a working image.'],
    ['Put', 'Prepare images', 'Images + Plan → Image', 'Return the working canvas to the panorama.'],
    ['Ramp', 'Prepare images', 'Image + Mask → Mask', 'Record a tone correction across a seam without painting yet.'],
    ['Apply', 'Prepare images', 'Image + Mask → Image', 'Apply the pixel adjustments recorded in the mask.'],
    ['Window', 'Prepare images', 'Image + Mask → Images + Mask', 'Crop around mask support with context for a filler.'],
    ['Unwindow', 'Prepare images', 'Parent + Crop → Image', 'Put the crop’s pixels back into its parent image.'],
    ['Render', 'Output and inspection', 'Mask → Image', 'Render a mask for a consumer or for inspection.'],
    ['Paste', 'Output and inspection', 'Base + Fill + Mask → Image', 'Composite a supplied fill through the mask.'],
    ['Measure', 'Output and inspection', 'Images + Mask → Series', 'Measure continuity, transition, texture, or pixel drift.'],
    ['Inspect', 'Output and inspection', 'Mask + Image → Image + Series', 'Inspect mask zones, shape, and provenance.'],
    ['Run a recipe', 'Recipe execution', 'Recipe inputs → Artifacts', 'Run a named recipe directly; this is a sequence of operations.'],
  ];
  const descriptions = {
    'Create a mask': 'Describe what to select. Inspect the mask before using it in an edit.',
    'Repair a seam': 'Repair a panorama’s wrap seam using your configured filler.',
    'Remove an object': 'Find the object, build its mask, and paint it out of the image.',
    'Repair damage': 'Describe the defect to repair. Prompt-based localization needs tool support.',
    'Protect a previous edit': 'Keep an earlier edit intact while preparing another pass.',
    'Check an edit': 'Check that an edit preserved what mattered in the source image.',
    'Check a reference subject': 'Check that the subject remains faithful to your references.',
    'Check realism': 'Look for anatomical, physical, and visual inconsistencies.',
    'Check a brief': 'Check the image against the content and requirements you describe.',
    'Check a product': 'Inspect product identity, labels, shape, and required details.',
  };
  const params = {
    Rectangle: [['bbox', 'Bounding box', '[0.05, 0.56, 0.30, 0.97]']],
    Stroke: [['path', 'Path points', '0.2,0.3;0.4,0.6'], ['radius_px', 'Radius (px)', 8]],
    Segment: [['prompt', 'Object prompt', 'The foreground chair'], ['max_instances', 'Maximum instances', 8]],
    Rasterize: [['use_case', 'Use case', 'custom']], Tighten: [['min_contrast', 'Minimum contrast', 12]],
    Grow: [['px', 'Grow (px)', 4], ['down_px', 'Shadow extension (px)', 0]],
    Shrink: [['px', 'Shrink (px)', 4]], Close: [['radius_px', 'Closing radius (px)', 3]],
    Subtract: [['grow_px', 'Protection growth (px)', 2]], Select: [['keep', 'Keep instances', '0,1']],
    Invert: [['to', 'Target frame', 'raw']], Fan: [['constants', 'Constant set', 'fan-v3']],
    Profile: [['feather_ratio', 'Feather ratio', 0.55]], Cut: [['strip', 'Edge strip fraction', 0.07]],
    Ramp: [['share', 'Correction share', 0.5], ['ramp_sigma', 'Row smoothing', 0.02]],
    Window: [['max_side', 'Maximum side (px)', 1024], ['context', 'Context ratio', 0.5]],
    Render: [['for', 'Rendering', 'grey']], Paste: [['mode', 'Paste mode', 'feather']],
    Measure: [['set', 'Measurement set', 'measures-v1']], 'Run a recipe': [['recipe', 'Recipe', 'rect']],
  };
  const models = [];
  let initialized = false;
  function remember() {
    if (initialized && window.openai?.setWidgetState) {
      window.openai.setWidgetState({ modelContent: { layouts: models.map(m => ({ layout: m.layout, tool: m.tool, task: m.task, operation: m.operation, mode: m.mode })) }, privateContent: null }).catch(() => {});
    }
  }
  for (const app of root.querySelectorAll('.wh-app')) {
    app.append(shell.content.cloneNode(true));
    app.querySelectorAll('.wh-asset').forEach(img => { img.src = photo; });
    const m = { layout: app.dataset.layout, tool: 'rascal', task: 'Create a mask', operation: 'Line', mode: 'playground', workspace: 'playground', query: '', preview: 'overlay', column: 50, width: 20, sigma: 4, support: 3 };
    models.push(m);
    const q = selector => app.querySelector(selector);
    const set = (selector, text) => { q(selector).textContent = text; };
    let journey;
    const opMode = () => m.workspace === 'operations';
    const current = () => opMode() ? m.operation : m.task;
    const info = () => operations.find(op => op[0] === m.operation);
    const supported = () => !opMode() || m.tool === 'rascal';
    const catalog = () => m.tool === 'rascal' ? operations : [];
    function chooseOperation(name) { m.operation = name; m.mode = m.workspace = 'operations'; m.query = ''; render(); remember(); }
    function chooseTask(name) { m.task = name; m.mode = m.workspace = 'playground'; render(); remember(); }
    function field(parent, name, labelText, value, help = '') {
      const label = document.createElement('label'); label.textContent = labelText;
      const input = document.createElement('input'); input.name = name; input.value = value;
      input.type = typeof value === 'number' ? 'number' : 'text'; if (input.type === 'number') input.step = 'any';
      label.append(input);
      if (help) { const small = document.createElement('span'); small.className = 'wh-help'; small.textContent = help; label.append(small); }
      input.addEventListener('input', () => { set('.wh-status', 'Settings changed. Run this operation to update the illustrative result.'); });
      parent.append(label); return input;
    }
    function populateCatalog(container) {
      container.replaceChildren();
      const results = catalog().filter(op => op.join(' ').toLowerCase().includes(m.query.toLowerCase()));
      const groupNames = [...new Set(results.map(op => op[1]))];
      for (const groupName of groupNames) {
        const group = document.createElement('div'); group.className = 'wh-op-group';
        const heading = document.createElement('h4'); heading.textContent = groupName; group.append(heading);
        for (const op of results.filter(row => row[1] === groupName)) {
          const button = document.createElement('button'); button.type = 'button'; button.className = 'wh-op-choice';
          button.setAttribute('aria-pressed', String(op[0] === m.operation));
          const name = document.createElement('span'); name.textContent = op[0];
          const help = document.createElement('small'); help.textContent = m.layout === 'sidebar' ? op[2] : op[3];
          button.append(name, help); button.addEventListener('click', () => chooseOperation(op[0])); group.append(button);
        }
        container.append(group);
      }
      if (!results.length) { const empty = document.createElement('p'); empty.className = 'wh-help'; empty.textContent = m.tool === 'squint' ? 'Independent evidence tools need a declared Squint interface.' : 'No matching operations.'; container.append(empty); }
      return results.length;
    }
    function searchBox(host, sidebar) {
      const input = document.createElement('input'); input.type = 'search'; input.placeholder = 'Search operations…'; input.setAttribute('aria-label', 'Search operations'); input.className = sidebar ? 'wh-side-search' : 'wh-op-search'; input.value = m.query;
      const list = document.createElement('div'); list.className = sidebar ? 'wh-op-list' : 'wh-op-groups';
      const count = document.createElement('p'); count.className = 'wh-catalog-status'; count.setAttribute('aria-live', 'polite');
      const filter = () => { m.query = input.value; const n = populateCatalog(list); count.textContent = n + ' operations'; };
      input.addEventListener('input', filter); host.append(input, list, count); filter();
    }
    function navigation() {
      q('.wh-nav-area').replaceChildren(); q('.wh-sidebar').replaceChildren();
      q('.wh-nav-area').hidden = m.mode === 'usage' || m.mode === 'runs';
      q('.wh-title-row').hidden = m.layout === 'picker' || m.mode === 'usage' || m.mode === 'runs';
      if (m.layout === 'sidebar') {
        const side = q('.wh-sidebar');
        if (opMode()) searchBox(side, true);
        else {
          const group = document.createElement('div'); group.className = 'wh-side-group';
          const title = document.createElement('p'); title.className = 'wh-group-label'; title.textContent = 'Tasks'; group.append(title);
          for (const task of taskNames[m.tool]) { const b = document.createElement('button'); b.textContent = task; b.setAttribute('aria-pressed', String(task === m.task)); b.addEventListener('click', () => chooseTask(task)); group.append(b); }
          const route = document.createElement('div'); route.className = 'wh-operation-route'; route.textContent = 'Test a single piece';
          const b = document.createElement('button'); b.textContent = 'Browse operations'; b.addEventListener('click', () => switchTab('operations')); route.append(b); side.append(group, route);
        }
      } else {
        const operation = opMode();
        const picker = document.createElement('details'); picker.className = 'wh-choice-picker';
        const summary = document.createElement('summary');
        summary.setAttribute('aria-label', (operation ? 'Operation: ' : 'Task: ') + current());
        const copy = document.createElement('span'); copy.className = 'wh-picker-copy';
        const name = document.createElement('span'); name.className = 'wh-picker-name'; name.textContent = operation ? m.tool === 'rascal' ? m.operation : 'Operations' : m.task;
        const signature = document.createElement('small'); signature.textContent = operation ? m.tool === 'rascal' ? info()[2] : 'Tool support needed' : 'Complete task'; copy.append(name, signature);
        const affordance = document.createElement('span'); affordance.className = 'wh-picker-affordance'; affordance.setAttribute('aria-hidden', 'true'); summary.append(copy, affordance);
        const label = document.createElement('span'); label.className = 'wh-picker-label'; label.textContent = operation ? 'Operation' : 'Task'; q('.wh-nav-area').append(label);
        const menu = document.createElement('div'); menu.className = 'wh-op-menu';
        if (operation) searchBox(menu, false);
        else for (const task of taskNames[m.tool]) {
          const button = document.createElement('button'); button.type = 'button'; button.className = 'wh-op-choice';
          button.textContent = task; button.setAttribute('aria-pressed', String(task === m.task));
          button.addEventListener('click', () => chooseTask(task)); menu.append(button);
        }
        picker.append(summary, menu); q('.wh-nav-area').append(picker);
      }
    }

    function preview(view = m.preview) {
      m.preview = view;
      const line = opMode() && m.operation === 'Line' && supported();
      const feather = opMode() && m.operation === 'Feather' && supported();
      const mask = !opMode() && m.tool === 'rascal' && m.task === 'Create a mask';
      const placeholder = opMode() && !line && !feather;
      q('.wh-canvas').hidden = view === 'json' || view === 'wipe' || placeholder;
      q('.wh-output-placeholder').hidden = view === 'json' || !placeholder;
      q('.wh-json:not(.wh-command)').hidden = view !== 'json';
      q('.wh-canvas').dataset.view = view;
      q('.wh-line-overlay').hidden = !line;
      q('.wh-line-overlay').style.left = m.column + '%'; q('.wh-line-overlay').style.width = (m.width / 10) + '%';
      q('.wh-mask-shape').hidden = !mask && !feather;
      q('.wh-mask-shape').style.filter = feather ? 'blur(' + m.sigma / 2 + 'px)' : '';
      q('.wh-selection').hidden = !mask && !feather && !line;
      if (line) q('.wh-selection').firstElementChild.textContent = 'Column ' + m.column + '% · width ' + m.width + ' px';
      else if (feather) q('.wh-selection').firstElementChild.textContent = 'Feather σ ' + m.sigma + ' px · support ' + m.support + 'σ';
      else q('.wh-selection').firstElementChild.textContent = 'White armchair · foreground left';
      q('.wh-wipe-preview').hidden = view !== 'wipe' || placeholder;
      if (view === 'wipe' && !placeholder) {
        const source = document.createElement('div'); source.className = 'wh-canvas'; source.append(q('.wh-photo').cloneNode(true));
        const result = q('.wh-canvas').cloneNode(true); result.hidden = false; result.dataset.view = 'overlay';
        q('.wh-wipe-preview').replaceChildren(window.createWaffleryWipe({ left: source, right: result, leftLabel: 'Source', rightLabel: line ? 'Location overlay' : mask || feather ? 'Mask overlay' : 'Result' }));
      }
      const data = line ? { preview: 'illustrative', type: 'Location', kind: 'line', column: m.column / 100, width_px: m.width, image: 'living-room.jpg' } : feather ? { preview: 'illustrative', type: 'Mask', sigma_px: m.sigma, support: m.support, input: 'saved-armchair-mask' } : { preview: 'illustrative', action: current(), output: opMode() && supported() ? info()[2].split('→')[1].trim() : 'No live tool result' };
      set('.wh-json:not(.wh-command)', JSON.stringify(data, null, 2));
      q('.wh-preview-buttons').replaceChildren();
      const views = placeholder ? [['artifact', 'Artifact'], ['json', 'JSON']] : line ? [['wipe', 'Slider'], ['overlay', 'Overlay'], ['source', 'Source'], ['json', 'JSON']] : mask || feather ? [['wipe', 'Slider'], ['overlay', 'Overlay'], ['source', 'Source'], ['mask', 'Mask'], ['json', 'JSON']] : [['source', 'Source'], ['json', 'JSON']];
      for (const [key, label] of views) { const b = document.createElement('button'); b.dataset.preview = key; b.textContent = label; b.setAttribute('aria-pressed', String(key === view || key === 'artifact' && view === 'overlay')); b.addEventListener('click', () => preview(key)); q('.wh-preview-buttons').append(b); }
    }
    function operationForm() {
      const form = q('.wh-operation-form'); form.replaceChildren();
      if (!supported()) { const p = document.createElement('p'); p.className = 'wh-help'; p.textContent = 'This view will list the independent operations Squint exposes. Its internal evidence tools are not assumed to be directly callable.'; form.append(p); return; }
      const signature = document.createElement('p'); signature.className = 'wh-op-inputs'; signature.textContent = info()[2]; form.append(signature);
      if (m.operation === 'Line') {
        field(form, 'column', 'Position across image (%)', m.column, '50% places the line at the center.');
        field(form, 'width_px', 'Line width (px)', m.width, 'Defines geometry; no pixels are painted.');
        form.querySelector('[name=column]').min = 0; form.querySelector('[name=column]').max = 100;
        form.querySelector('[name=width_px]').min = 0.001;
      } else if (m.operation === 'Feather') {
        const label = document.createElement('label'); label.textContent = 'Input mask'; const select = document.createElement('select'); select.setAttribute('aria-label', 'Input mask');
        const option = document.createElement('option'); option.textContent = 'Saved example · armchair core'; select.append(option); label.append(select); form.append(label);
        field(form, 'sigma_px', 'Feather sigma (px)', m.sigma, 'Only this operation runs; the finder and filler are not called.'); field(form, 'support', 'Support (σ)', m.support);
      } else {
        field(form, 'artifact', 'Input artifact(s)', 'Choose compatible saved artifacts…', 'Mock control. Required types are shown above.');
        for (const [name, label, value] of params[m.operation] || []) field(form, name, label, value);
      }
    }
    function render() {
      navigation();
      const op = opMode(), judge = m.tool === 'squint', active = m.mode === 'playground' || m.mode === 'operations';
      const name = current();
      set('.wh-tool-heading', judge ? 'Squint' : 'Rascal');
      set('.wh-tool-purpose', judge ? 'Check images and inspect the evidence behind each judgment.' : 'Find, mask, repair, and inspect your images.');
      set('.wh-title', supported() ? name : 'Operations');
      set('.wh-description', op ? supported() ? info()[3] : 'Independent evidence-tool access needs a supported Squint interface.' : descriptions[name]);
      q('.wh-description').hidden = !active;
      q('.wh-playground').hidden = !active; q('.wh-usage').hidden = m.mode !== 'usage'; q('.wh-runs-page').hidden = m.mode !== 'runs';
      app.querySelectorAll('[data-tab]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.tab === m.mode)));
      q('.wh-output-type').hidden = !op; set('.wh-output-type', supported() && op ? info()[2].split('→')[1].trim() : 'Unavailable');
      q('.wh-prompt-field').hidden = op || name === 'Repair a seam'; q('.wh-operation-form').hidden = !op;
      q('.wh-config-field').hidden = op; q('.wh-advanced').hidden = op;
      q('.wh-input-error').hidden = true;
      if (op) operationForm();
      set('.wh-image-label', op ? 'Source image / artifact context' : judge ? 'Candidate image' : name === 'Repair a seam' ? 'Panorama' : 'Source image');
      set('.wh-prompt-label', judge ? 'What should Squint check?' : name === 'Create a mask' ? 'What should the mask select?' : 'What should change?');
      q('.wh-prompt').value = judge ? 'Preserve the furniture, lighting, and room layout.' : 'The white armchair in the foreground on the left.';
      set('.wh-config-label', judge ? 'Judge model' : 'Tool configuration'); set('.wh-config-name', judge ? 'Configured judge' : 'Configured defaults');
      set('.wh-result-heading', op ? supported() ? info()[2].split('→')[1].trim() + ' preview' : 'Output' : name === 'Create a mask' ? 'Mask preview' : 'Image preview');
      q('.wh-output-placeholder strong').textContent = supported() && op ? info()[2].split('→')[1].trim() : 'No advertised operations';
      q('.wh-output-placeholder p').textContent = 'Typed-output layout placeholder. Line and Feather include interactive visual examples.';
      q('.wh-image-caption').firstElementChild.textContent = op && name === 'Line' ? 'Location overlay · mock geometry' : op && name === 'Feather' ? 'Illustrative feather preview · not algorithm output' : 'Illustrative preview · not a tool result';
      set('.wh-run', op ? 'Run ' + (supported() ? name.toLowerCase() : 'operation') : 'Run'); q('.wh-run').disabled = !supported();
      set('.wh-panel-foot .wh-subtle', op ? 'Run only this operation' : 'Uses your local configuration');
      q('.wh-history').hidden = op;
      q('.wh-inspector-body').replaceChildren();
      const rows = op ? [['Input', supported() ? info()[2].split('→')[0].trim() : 'None'], ['Execution', 'Single operation · no automatic upstream rerun'], ['Output', supported() ? info()[2].split('→')[1].trim() : 'None']] : [['Find', 'Prompt → candidate regions'], ['Refine', 'Select → grow → close → protect → feather'], ['Artifacts', 'Source image · regions · mask rendering']];
      for (const [label, text] of rows) { const row = document.createElement('div'); row.className = 'wh-inspector-row'; const key = document.createElement('span'); key.className = 'wh-muted'; key.textContent = label; const value = document.createElement('span'); value.textContent = text; row.append(key, value); q('.wh-inspector-body').append(row); }
      if (!op && m.tool === 'rascal') { const button = document.createElement('button'); button.textContent = 'Test Feather independently'; button.addEventListener('click', () => chooseOperation('Feather')); q('.wh-inspector-body').append(button); }
      q('.wh-command').hidden = judge || !op || name !== 'Line';
      set('.wh-command', 'rascal line --workspace ws --image living-room.jpg --column 0.5 --width_px 20 --projection flat');
      set('.wh-usage-copy', 'Reference documentation for ' + name + ': supported inputs, outputs, CLI, and MCP setup. This is a documentation placeholder; use the run’s “Use in your pipeline” pane for its saved configuration.');
      set('.wh-status', op ? 'Adjust inputs and run this piece independently. Preview execution is simulated.' : name === 'Repair damage' ? 'Proposed activity · automatic localization needs Rascal support.' : name === 'Repair a seam' ? 'This sample photo is flat. A wrap-seam repair needs a panorama.' : '');
      m.preview = op || name === 'Create a mask' ? 'wipe' : 'source'; preview();
      journey?.refresh();
    }
    function switchTab(mode) { m.mode = mode; if (mode === 'playground' || mode === 'operations') m.workspace = mode; m.query = ''; render(); remember(); }
    app.querySelectorAll('[data-tab]').forEach(b => b.addEventListener('click', () => switchTab(b.dataset.tab)));
    q('.wh-tool-select').addEventListener('change', e => { m.tool = e.target.value; m.task = m.tool === 'rascal' ? 'Create a mask' : 'Check an edit'; m.query = ''; render(); remember(); });
    q('.wh-run').addEventListener('click', () => {
      if (journey && !journey.canRun()) return;
      if (opMode() && m.operation === 'Line') {
        const column = q('[name=column]').value, width = q('[name=width_px]').value;
        if (column.trim() === '' || width.trim() === '' || !Number.isFinite(+column) || !Number.isFinite(+width) || +column < 0 || +column > 100 || +width <= 0) { set('.wh-input-error', 'Use a position from 0 to 100% and a positive width.'); q('.wh-input-error').hidden = false; return; }
        m.column = +column; m.width = +width;
      } else if (opMode() && m.operation === 'Feather') {
        const sigma = q('[name=sigma_px]').value, support = q('[name=support]').value;
        if (sigma.trim() === '' || support.trim() === '' || !Number.isFinite(+sigma) || !Number.isFinite(+support) || +sigma <= 0 || +support <= 0) { set('.wh-input-error', 'Use positive sigma and support values.'); q('.wh-input-error').hidden = false; return; }
        m.sigma = +sigma; m.support = +support;
      }
      q('.wh-input-error').hidden = true;
      if (journey) journey.execute(); else { preview(); set('.wh-status', 'Illustrative result updated. No tool or provider was called.'); }
    });
    q('.wh-upload').addEventListener('click', () => set('.wh-status', 'Input picker placeholder; the sample is fixed for this layout study.'));
    app.querySelectorAll('[data-attempt]').forEach(b => b.addEventListener('click', () => { m.tool = 'rascal'; q('.wh-tool-select').value = 'rascal'; chooseTask('Create a mask'); q('.wh-advanced-inner input').value = b.dataset.attempt; set('.wh-status', 'Opened sample attempt with grow ' + b.dataset.attempt + ' px.'); }));
    m.restore = saved => { if (!saved || !taskNames[saved.tool]) return; m.tool = saved.tool; m.task = taskNames[m.tool].includes(saved.task) ? saved.task : taskNames[m.tool][0]; m.operation = operations.some(op => op[0] === saved.operation) ? saved.operation : 'Line'; m.mode = ['playground', 'operations', 'usage', 'runs'].includes(saved.mode) ? saved.mode : 'playground'; m.workspace = m.mode === 'operations' ? 'operations' : 'playground'; q('.wh-tool-select').value = m.tool; render(); };
    render();
    journey = window.createWaffleryJourney({ app, m, q, set, render, preview, current, chooseOperation });
    journey.reset('wafflery');
  }
  function restore(state) { const saved = state?.modelContent?.layouts; if (Array.isArray(saved)) models.forEach(m => m.restore(saved.find(s => s.layout === m.layout))); }
  restore(window.openai?.widgetState);
  window.addEventListener('openai:set_globals', e => restore(e.detail?.globals?.widgetState));
  initialized = true;
  const design = { inputWidth: 39 };
  if (globalThis.Tweak) { const tweak = new Tweak({ container: root, onChange: () => root.style.setProperty('--wh-input-width', design.inputWidth + '%') }); tweak.addSlider(design, 'inputWidth', { label: 'Input panel width', min: 32, max: 48, step: 1, unit: '%' }); }
  if (globalThis.lucide) lucide.createIcons({ attrs: { width: 15, height: 15 } });
})();
