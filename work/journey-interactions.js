// Local prototype only. No installation, tool process, or provider calls occur here.
window.createWaffleryJourney = ({ app, m, q, set, render, preview, current, chooseOperation }) => {
  const fragment = document.getElementById('wh-journey-template').content.cloneNode(true);
  const take = name => fragment.querySelector(name);
  const onboarding = take('.wh-onboarding');
  app.insertBefore(onboarding, q('.wh-main-layout'));
  q('.wh-description').after(take('.wh-readiness'));
  q('.wh-panels > section:nth-child(2) .wh-panel-body').prepend(take('.wh-progress'), take('.wh-run-banner'));
  q('.wh-selection').after(take('.wh-findings'));
  q('.wh-inspector').after(take('.wh-pipeline'));
  const drawer = window.createWaffleryDrawer({ app });
  q('.wh-inspector').hidden = true;
  const manage = button('Tools & setup', () => { stage = 'setup'; refresh(); });
  manage.className = 'wh-manage'; q('.wh-chrome').append(manage);
  const use = button('Use in your pipeline', () => openExport(selected));
  use.className = 'wh-use-pipeline'; q('.wh-result-actions').prepend(use);
  const reference = q('.wh-upload').parentElement.cloneNode(true); reference.classList.add('wh-reference-input');
  reference.querySelector('.wh-image-label').textContent = 'Reference image';
  reference.querySelector('.wh-file-copy').textContent = 'source-room.jpg · bundled reference example';
  reference.querySelector('button').addEventListener('click', () => set('.wh-status', 'Reference input is fixed in this layout study.'));
  q('.wh-upload').parentElement.after(reference);
  let stage = 'workspace', installed = { rascal: true, squint: true };
  let configured = { rascal: true, squint: true }, setupOpen = '', selected = null, pending = null;
  let timer = null, runs = [], exportTab = 'command', sequence = 0, lastContext = '', estimateTimer = null, estimateKey = '';
  const locked = new Map();
  const comparison = window.createWaffleryComparison({ app, selectedRun: () => selected, context: () => lastContext });
  const examples = window.createWaffleryExamples({ app, drawer, selectedRun: () => selected, tool: () => m.tool, applyExample: example => {
    loadRun(example, true);
    set('.wh-status', 'Loaded example: ' + example.name + '. Review its inputs and settings, then Run to create a new attempt.');
  } });
  let budgetCents = { rascal: 480, squint: 480 };
  const paid = () => !(m.tool === 'rascal' && m.workspace === 'operations' && current() !== 'Segment');
  const title = tool => tool === 'rascal' ? 'Rascal' : 'Squint';
  function button(text, fn) { const b = document.createElement('button'); b.type = 'button'; b.textContent = text; b.addEventListener('click', fn); return b; }
  function element(tag, text, className) { const e = document.createElement(tag); e.textContent = text; if (className) e.className = className; return e; }
  function enter(tool, local = false) {
    m.tool = tool; q('.wh-tool-select').value = tool; m.task = tool === 'rascal' ? 'Create a mask' : 'Check an edit';
    m.mode = m.workspace = 'playground'; stage = 'workspace';
    if (local) chooseOperation('Line'); else render();
    refresh();
  }
  function readiness() {
    if (!installed[m.tool]) return { ready: false, name: title(m.tool) + ' is not installed', detail: 'Add this tool to use its workspace.' };
    if (m.tool === 'squint' && m.workspace === 'operations') return { ready: false, name: 'Operation interface needed', detail: 'Only advertised standalone operations will appear here.' };
    if (m.tool === 'rascal' && current() === 'Repair damage') return { ready: false, name: 'Tool support needed', detail: 'Prompt-based damage localization is a proposed Rascal capability.' };
    const local = m.tool === 'rascal' && m.workspace === 'operations' && current() !== 'Segment';
    if (!local && !configured[m.tool]) return { ready: false, name: m.tool === 'rascal' ? 'Set up this task’s providers' : 'Configure a judge', detail: m.tool === 'rascal' ? 'Local geometry operations are available now. Prompt tasks need their configured providers.' : 'Choose the judge in Squint’s configuration and provide its required environment variables.' };
    return { ready: true, name: local ? 'Ready · local operation' : 'Ready · configured defaults', detail: local ? 'Runs this piece independently. No provider needed for this example.' : 'Uses the tool’s configuration. Provider execution is simulated in this preview.' };
  }
  function showSetup() {
    onboarding.replaceChildren();
    onboarding.append(element('div', 'Local workspace', 'wh-eyebrow'), element('h2', Object.values(installed).some(Boolean) ? 'Your tools' : 'Start with the tool you need'), element('p', 'Use Rascal and Squint independently. Add the other whenever it becomes useful to your pipeline.'));
    const list = element('div', '', 'wh-setup-tools');
    for (const tool of ['rascal', 'squint']) {
      const card = element('section', '', 'wh-setup-card'); card.dataset.setupTool = tool;
      const row = element('div', '', 'wh-row'); row.append(element('h3', title(tool)), element('span', !installed[tool] ? 'Not installed' : configured[tool] ? 'Configured' : tool === 'rascal' ? 'Local operations ready' : 'Judge setup needed', 'wh-setup-state'));
      card.append(row, element('p', tool === 'rascal' ? 'Define regions, create masks, and prepare image repairs.' : 'Check images against a brief or references and inspect the evidence.'));
      const actions = element('div', '', 'wh-actions');
      if (installed[tool]) {
        actions.append(button(tool === 'rascal' && !configured[tool] ? 'Try a local operation' : 'Open ' + title(tool), () => enter(tool, tool === 'rascal' && !configured[tool])));
        actions.append(button(configured[tool] ? 'Review configuration' : 'Set up providers', () => { setupOpen = setupOpen === tool ? '' : tool; refresh(); }));
      } else actions.append(button('Installation steps', () => { setupOpen = setupOpen === tool ? '' : tool; refresh(); }));
      card.append(actions);
      if (setupOpen === tool) {
        const instructions = element('div', '', 'wh-setup-instructions');
        instructions.append(element('p', !installed[tool] ? 'Install the supported ' + title(tool) + ' release, make its executable available on PATH, then recheck. Exact distribution links will be supplied by the tool.' : tool === 'rascal' ? 'Configure a segmenter for prompt-based masks and a filler for painting. Wafflery reads readiness from Rascal; it does not store your keys.' : 'Choose a judge in Squint’s configuration and set its provider’s required environment variables. Wafflery shows missing variable names, never their values.'));
        instructions.append(button(!installed[tool] ? 'Recheck installation · simulate found' : 'Recheck configuration · simulate ready', () => { if (!installed[tool]) installed[tool] = true; else configured[tool] = true; setupOpen = ''; refresh(); })); card.append(instructions);
      }
      list.append(card);
    }
    onboarding.append(list, element('p', 'Setup changes here are simulated and stay in this preview. Nothing is installed, and no credentials are requested.'));
  }
  function refresh() {
    const setup = stage !== 'workspace'; onboarding.hidden = !setup; q('.wh-main-layout').hidden = setup; q('.wh-tool-select').hidden = setup; manage.hidden = setup;
    if (setup) showSetup();
    const state = readiness(); q('.wh-readiness').hidden = !['playground', 'operations'].includes(m.mode);
    set('.wh-readiness strong', state.name); set('.wh-readiness p', state.detail);
    q('.wh-readiness-action').textContent = state.ready ? 'Tools & setup' : 'Review setup';
    q('.wh-run').disabled = !state.ready || !!pending;
    reference.hidden = m.tool !== 'squint' || m.workspace !== 'playground' || ['Check realism', 'Check a brief'].includes(m.task);
    if (m.tool === 'squint') {
      q('.wh-advanced').hidden = true;
      set('.wh-prompt-field .wh-help', 'Describe the requirements this image should meet.');
      q('.wh-result-heading').textContent = 'Findings & evidence';
      q('.wh-inspector-body').replaceChildren();
      for (const [label, text] of [['Inputs', 'Candidate · reference · requirements'], ['Judge', 'Configured judge · simulated response'], ['Evidence', 'Tool-owned findings and detailed report']]) {
        const row = element('div', '', 'wh-inspector-row'); row.append(element('span', label, 'wh-muted'), element('span', text)); q('.wh-inspector-body').append(row);
      }
    }
    const context = m.tool + ':' + m.workspace + ':' + current();
    if (lastContext !== context) {
      drawer.close();
      selected = runs.find(run => run.context === context) || null;
      q('.wh-pipeline').hidden = true; lastContext = context;
    }
    paintResult(); updateHistory(); updateCost(true); examples.refresh();
  }
  q('.wh-readiness-action').addEventListener('click', () => { stage = 'setup'; setupOpen = m.tool; refresh(); });
  function reset(route) {
    drawer.close();
    comparison.reset();
    examples.reset();
    clearTimeout(timer); unlock(); pending = null; selected = null; runs = []; sequence = 0; lastContext = ''; clearTimeout(estimateTimer); estimateKey = ''; budgetCents = { rascal: 480, squint: 480 };
    stage = route === 'wafflery' ? 'setup' : 'workspace';
    installed = { rascal: route === 'rascal' || route === 'ready', squint: route === 'squint' || route === 'ready' };
    configured = { rascal: route === 'ready', squint: route === 'ready' }; setupOpen = '';
    m.tool = route === 'squint' ? 'squint' : 'rascal'; m.task = m.tool === 'rascal' ? 'Create a mask' : 'Check an edit'; m.mode = m.workspace = 'playground'; q('.wh-tool-select').value = m.tool;
    q('.wh-pipeline').hidden = true; q('.wh-progress').hidden = true;
    render(); refresh();
  }
  window.addEventListener('wh:scenario', e => reset(e.detail));
  function inputsNow() {
    const parameters = {};
    if (m.workspace === 'operations') q('.wh-operation-form').querySelectorAll('input,select').forEach(input => { parameters[input.name || 'mask'] = input.value; });
    else {
      if (!q('.wh-prompt-field').hidden) parameters.prompt = q('.wh-prompt').value;
      parameters.configuration = 'Tool-configured defaults (illustrative)';
      if (m.tool === 'rascal') {
        const fields = q('.wh-advanced-inner').querySelectorAll('input');
        parameters.grow_px = fields[0].value; parameters.feather_px = fields[1].value;
      }
    }
    return parameters;
  }
  function snapshot() {
    return {
      id: 'demo-' + String(++sequence).padStart(3, '0'), context: lastContext, tool: m.tool, action: current(), workspace: m.workspace,
      parameters: inputsNow(), status: 'running', simulated: true,
      inputs: [{ role: m.tool === 'squint' ? 'candidate' : 'source', path: 'living-room.jpg', fingerprint: 'Not computed in this mock' },
        ...(!reference.hidden ? [{ role: 'reference', path: 'source-room.jpg', fingerprint: 'Not computed in this mock' }] : []),
        ...(current() === 'Feather' ? [{ role: 'mask', artifact: 'saved-armchair-mask', portability: 'Existing Rascal workspace required; import/export contract pending' }] : [])],
      tool_version: 'Not detected in this mock', provider: 'No provider called', seed: 'Not supplied',
      cost: { may_spend: paid(), max_spend_usd: paid() ? 0.05 : 0 },
    };
  }
  function paintResult() {
    const complete = selected?.status === 'complete';
    use.disabled = !complete; q('.wh-report').disabled = !selected;
    q('.wh-save').disabled = !complete;
    q('.wh-findings').hidden = !complete || selected.tool !== 'squint';
    q('.wh-preview-buttons').hidden = !complete;
    q('.wh-image-caption').hidden = !complete;
    if (!complete) {
      q('.wh-wipe-preview').hidden = true;
      q('.wh-canvas').hidden = true; q('.wh-json:not(.wh-command)').hidden = true; q('.wh-selection').hidden = true;
      q('.wh-output-placeholder').hidden = false;
      set('.wh-output-placeholder strong', pending ? 'Run in progress' : selected?.status === 'cancelled' ? 'Run cancelled' : 'Your result will appear here');
      set('.wh-output-placeholder p', pending ? 'The tool reports progress while it works.' : 'Choose inputs, run, then inspect the output and its saved configuration.');
    } else {
      // Keep the displayed output tied to its saved inputs while the form is edited.
      const saved = { column: m.column, width: m.width, sigma: m.sigma, support: m.support };
      if (selected.action === 'Line') { m.column = +selected.parameters.column; m.width = +selected.parameters.width_px; }
      if (selected.action === 'Feather') { m.sigma = +selected.parameters.sigma_px; m.support = +selected.parameters.support; }
      preview(); Object.assign(m, saved);
    }
    updateBanner();
    comparison.refresh();
  }
  function updateBanner() {
    const changed = selected && JSON.stringify(inputsNow()) !== JSON.stringify(selected.parameters);
    set('.wh-run-banner', selected ? selected.id + ' · ' + selected.status + ' · simulated · Spend $' + selected.spend.usd.toFixed(2) + ' (estimated)' + (changed ? ' · Form changed; result uses saved inputs' : '') : 'No run selected');
  }
  function costKey() { return lastContext + JSON.stringify(inputsNow()); }
  function updateCost(immediate = false) {
    clearTimeout(estimateTimer);
    const footer = q('.wh-panel-foot .wh-subtle');
    if (!readiness().ready) { footer.textContent = 'Setup required'; return; }
    if (!paid()) { footer.textContent = 'No provider cost'; q('.wh-run').disabled = !!pending; return; }
    const key = costKey();
    const apply = () => {
      if (key !== costKey() || !readiness().ready) return;
      estimateKey = key;
      footer.textContent = 'Up to $0.05 · Tool session: $' + (budgetCents[m.tool] / 100).toFixed(2) + ' left · Demo estimate';
      q('.wh-run').textContent = 'Run';
      q('.wh-run').disabled = !!pending || budgetCents[m.tool] < 5;
      if (budgetCents[m.tool] < 5) footer.textContent = 'Insufficient tool session budget · Demo';
    };
    if (immediate) apply();
    else { estimateKey = ''; footer.textContent = 'Updating estimate…'; q('.wh-run').disabled = true; estimateTimer = setTimeout(apply, 250); }
  }
  q('.wh-playground').addEventListener('input', event => { if (event.target.closest('.wh-save-example,.wh-wipe')) return; updateBanner(); updateCost(); });

  function lock() {
    app.querySelectorAll('button,input,textarea,select').forEach(control => {
      if (control.classList.contains('wh-stop')) return;
      locked.set(control, control.disabled); control.disabled = true;
    });
  }
  function unlock() { for (const [control, disabled] of locked) control.disabled = disabled; locked.clear(); }
  function start() {
    drawer.close();
    comparison.close();
    examples.close();
    pending = snapshot();
    q('.wh-progress').hidden = false; q('.wh-progress progress').value = 15;
    set('.wh-progress p', 'Starting ' + pending.action.toLowerCase() + '…');
    set('.wh-status', 'Simulated execution · no tool or provider is called.');
    q('.wh-pipeline').hidden = true; paintResult(); lock();
    timer = setTimeout(() => {
      if (!pending) return;
      q('.wh-progress progress').value = 65; set('.wh-progress p', pending.tool === 'squint' ? 'Reviewing evidence…' : 'Preparing output artifacts…');
      timer = setTimeout(() => finish('complete'), 1100);
    }, 800);
  }
  function finish(status) {
    if (!pending) return;
    clearTimeout(timer);
    const spendCents = pending.cost.may_spend ? status === 'complete' ? 3 : 1 : 0;
    budgetCents[pending.tool] -= spendCents;
    selected = Object.freeze({ ...pending, status, spend: { usd: spendCents / 100, basis: 'estimated', simulated: true } }); runs.unshift(selected); pending = null;
    unlock(); q('.wh-progress').hidden = true; refresh();
    set('.wh-status', status === 'complete' ? 'Simulated run complete. Inspect it or use its saved configuration in your pipeline.' : 'Simulated run cancelled. Inputs and terminal state are retained.');
  }
  function canRun() {
    return readiness().ready && !pending && (!paid() || estimateKey === costKey() && budgetCents[m.tool] >= 5);
  }
  function execute() { if (canRun()) start(); }
  q('.wh-stop').addEventListener('click', () => finish('cancelled'));
  function loadRun(run, asDraft = false) {
    drawer.close();
    m.query = '';
    m.tool = run.tool; m.mode = m.workspace = run.workspace;
    if (run.workspace === 'operations') m.operation = run.action; else m.task = run.action;
    if (run.action === 'Line') { m.column = +run.parameters.column; m.width = +run.parameters.width_px; }
    if (run.action === 'Feather') { m.sigma = +run.parameters.sigma_px; m.support = +run.parameters.support; }
    q('.wh-tool-select').value = m.tool; stage = 'workspace'; render(); selected = asDraft ? null : run;
    if (run.workspace === 'operations') q('.wh-operation-form').querySelectorAll('input').forEach(input => { if (run.parameters[input.name] !== undefined) input.value = run.parameters[input.name]; });
    else {
      if (run.parameters.prompt !== undefined) q('.wh-prompt').value = run.parameters.prompt;
      const fields = q('.wh-advanced-inner').querySelectorAll('input'); fields[0].value = run.parameters.grow_px || '4'; fields[1].value = run.parameters.feather_px || '4';
    }
    q('.wh-pipeline').hidden = true; comparison.close(); refresh();
  }
  function updateHistory() {
    const listFor = target => {
      target.replaceChildren(element('h3', 'Runs this session'), element('p', 'Demo history. Each attempt keeps its own inputs; changing the starting point clears this list.', 'wh-help'));
      const list = element('div', '', 'wh-snapshot-list');
      for (const run of runs.filter(run => run.tool === m.tool)) {
        const b = button('', () => loadRun(run));
        b.append(element('span', run.action + ' · ' + run.id), element('span', run.status, 'wh-muted')); list.append(b);
      }
      if (!list.childElementCount) list.append(element('p', 'No runs yet.', 'wh-help'));
      target.append(list);
    };
    listFor(q('.wh-history')); listFor(q('.wh-runs-page'));
    q('.wh-history').hidden = !runs.some(run => run.tool === m.tool);
  }
  const quote = value => "'" + String(value).replace(/'/g, "'\\''") + "'";
  function exportContent(run) {
    if (exportTab === 'configuration') return { text: JSON.stringify({ run: run.id, tool: run.tool, action: run.action, parameters: run.parameters, tool_version: run.tool_version, provider: run.provider, seed: run.seed, simulated: true }, null, 2), note: 'Saved configuration for this simulated attempt. Editing the form does not change it.', label: 'Copy configuration' };
    if (exportTab === 'inputs') return { text: JSON.stringify(run.inputs, null, 2), note: 'These example paths must resolve in your environment. No input assets or credentials are included in the manifest.', label: 'Copy input manifest' };
    if (exportTab === 'mcp') return { text: 'The exact recorded MCP call will appear here.\n\nLive runs capture the action name and arguments sent to the tool.\nA runnable example also includes client setup and required inputs.', note: 'This simulated run made no MCP call. The contract already defines action names and schemas; artifact portability and effective configuration still need specification.', unavailable: true };
    if (run.tool === 'rascal' && run.action === 'Line') {
      const p = run.parameters;
      return { text: ['rascal line \\', '  --workspace ' + quote('ws') + ' \\', '  --image ' + quote('living-room.jpg') + ' \\', '  --column ' + (+p.column / 100) + ' \\', '  --width_px ' + Number(p.width_px) + ' \\', '  --projection flat'].join('\n'), note: 'POSIX shell example using the saved inputs. Requires Rascal, an initialized workspace at ws, and living-room.jpg. This preview did not run or package those prerequisites.', label: 'Copy command' };
    }
    return { text: 'Command export needs tool-provided invocation metadata.\n\nYou can inspect and copy this run’s configuration and inputs now.', note: run.action === 'Feather' ? 'This run depends on a saved Mask artifact. A PNG preview cannot replace it; workspace portability needs tool support.' : 'Supported CLI flags and recipe mappings will come from the tool, not from UI labels.', unavailable: true };
  }
  function renderExport() {
    if (!selected) return;
    const data = exportContent(selected);
    drawer.setContext(selected.id + ' · ' + selected.tool + ' / ' + selected.action + ' · Saved inputs · Simulated run');
    set('.wh-export-code', data.text); set('.wh-export-note', data.note); set('.wh-export-feedback', '');
    q('.wh-copy-export').disabled = !!data.unavailable; q('.wh-copy-export').textContent = data.label || 'Not available yet';
    app.querySelectorAll('[data-export]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.export === exportTab)));
  }
  function openExport(run) {
    if (!run || run.status !== 'complete') return;
    selected = run; exportTab = run.tool === 'rascal' && run.action === 'Line' ? 'command' : 'configuration';
    drawer.open({ heading: 'Use in your pipeline', description: '', node: q('.wh-pipeline'), trigger: use }); renderExport();
  }
  app.querySelectorAll('[data-export]').forEach(b => b.addEventListener('click', () => { exportTab = b.dataset.export; renderExport(); }));
  q('.wh-report').addEventListener('click', () => {
    if (!selected) return;
    const body = q('.wh-inspector-body'); body.replaceChildren();
    for (const [label, value] of [['Action', selected.action], ['Status', selected.status], ['Spend', '$' + selected.spend.usd.toFixed(2) + ' · estimated, simulated'], ['Inputs', selected.inputs.map(input => input.path || input.artifact).join(', ')]]) {
      const row = element('div', '', 'wh-inspector-row'); row.append(element('span', label, 'wh-muted'), element('span', value)); body.append(row);
    }
    body.append(element('h4', 'Saved settings'), element('pre', JSON.stringify(selected.parameters, null, 2), 'wh-export-code'), element('p', 'The tool’s detailed report and event journal will appear here. This run is simulated.', 'wh-help'));
    if (selected.tool === 'rascal' && selected.workspace === 'playground') body.append(button('Test Feather independently', () => { drawer.close(); chooseOperation('Feather'); }));
    drawer.open({ heading: 'Run details', description: selected.id + ' · ' + selected.tool + ' / ' + selected.action, node: q('.wh-inspector'), trigger: q('.wh-report') });
  });
  q('.wh-copy-export').addEventListener('click', async () => {
    if (!selected) return;
    const data = exportContent(selected); if (data.unavailable) return;
    try { await window.navigator.clipboard.writeText(data.text); set('.wh-export-feedback', 'Copied.'); }
    catch { const selection = window.getSelection?.(); if (selection && document.createRange) { const range = document.createRange(); range.selectNodeContents(q('.wh-export-code')); selection.removeAllRanges(); selection.addRange(range); } set('.wh-export-feedback', 'Automatic copy is unavailable. Select the text above and copy it.'); }
  });
  q('.wh-download-export').addEventListener('click', () => {
    if (!selected) return;
    const payload = { ...selected, export_kind: 'mock-run-manifest', portable_bundle: false, note: 'No input files, credentials, or executable client are included.' };
    const url = URL.createObjectURL(new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' }));
    const a = document.createElement('a'); a.href = url; a.download = selected.id + '-manifest.json'; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
    set('.wh-export-feedback', 'Downloaded the run manifest. Input assets are not bundled.');
  });
  return { refresh, reset, canRun, execute };
};
