import { drawGrid, drawPlot, gridPosition, recordSample, renderReadouts, renderExperiment, groupLabel, sampleDescription } from './display.js';
const byId = id => document.getElementById(id);
const history = [];
const runHeader = 'X-VirtualLife-Run';
let runId = null;
let sample, selected = '', connected = false, busy = false, receipt = null, commandVersion = 0;

function controls() {
  const ready = connected && !busy && sample;
  byId('step').disabled = !ready || sample.status !== 'paused';
  byId('pause').disabled = !ready || sample.status !== 'running';
  byId('resume').disabled = !ready || sample.status !== 'paused';
  for (const id of ['seed', 'restart-seed', 'restart-random']) byId(id).disabled = !ready || !sample.experiment;
}
function paintCanvases() {
  if (!sample) return;
  drawGrid(byId('grid'), sample, selected);
  drawPlot(byId('plot'), history);
}
function render() {
  renderReadouts(sample, selected);
  renderExperiment(sample);
  byId('restart-controls').hidden = !sample.experiment;
  const selector = byId('agent');
  selector.replaceChildren(new Option('Choose an agent', ''));
  for (const agent of sample.cells.filter(Boolean)) selector.add(new Option(`ID ${agent.id} · ${sample.experiment ? `Group ${groupLabel(agent.group)}` : `Value ${agent.value}`}`, agent.id));
  if (sample.experiment?.maintenance) for (const failure of sample.failure_history.records) selector.add(new Option(`ID ${failure.id} · failed: ${failure.reason}`, failure.id));
  if (selected && ![...selector.options].some(option => option.value === selected)) selector.add(new Option(`ID ${selected} · removed`, selected));
  selector.value = selected;
  paintCanvases();
  byId('samples').textContent = sampleDescription(history);
  controls();
}

function receiveRun(next, nextRun) {
  if (nextRun !== runId) {
    if (runId !== null) {
      history.length = 0;
      selected = ''; receipt = null; busy = false;
      commandVersion++; // Retire every old snapshot, command and restart callback.
      byId('control-message').textContent = '';
      byId('run-message').textContent = 'New experiment connected. Previous page history and selection cleared. No commands retried.';
    }
    // Seed edits survive ordinary polling and same-run reconnection.
    byId('seed').value = next.experiment?.seed ?? '1';
    byId('seed').removeAttribute('aria-invalid');
  }
  runId = nextRun;
  sample = next;
  connected = true;
  byId('connection').hidden = true;
}
function connectionError(message) {
  connected = false;
  byId('connection').hidden = false;
  byId('connection').textContent = message;
  byId('status').textContent = 'Disconnected';
  controls();
}

// Schedule the next read only after this one settles: at most one snapshot in flight.
async function poll() {
  const version = commandVersion;
  try {
    const response = await fetch('/api/snapshot', { cache: 'no-store', signal: AbortSignal.timeout(4000) });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const next = await response.json();
    if (version !== commandVersion) return; // Ignore a read started before the last control.
    const nextRun = response.headers.get(runHeader);
    if (!nextRun) throw new Error('Missing experiment identity; reload after upgrading the server');
    receiveRun(next, nextRun);
    // commandVersion excludes reads started before the receipt. Another control
    // can supersede its status at the same tick, so wait only for the applied tick.
    if (receipt && BigInt(sample.tick) >= BigInt(receipt.tick)) {
      busy = false; receipt = null;
    }
    recordSample(history, sample);
    render();
    if (sample.status === 'failed' || sample.status === 'stopped') {
      byId('connection').hidden = false;
      byId('connection').textContent = sample.error || 'The simulation worker stopped.';
    }
  } catch (error) {
    if (version !== commandVersion) return;
    connectionError(`Connection lost (${error.message}). The server may have stopped. Last received state remains below; reconnecting…`);
  } finally {
    setTimeout(poll, 100);
  }
}

async function command(name, restart = false) {
  if (busy || !connected || !runId) return;
  let body = { command: name };
  if (restart) {
    if (!sample.experiment) return;
    const seed = byId('seed').value;
    if (name === 'seed' && (!/^[0-9]+$/.test(seed) || BigInt(seed) > 18446744073709551615n)) {
      byId('seed').setAttribute('aria-invalid', 'true');
      byId('control-message').textContent = 'Enter a whole seed from 0 to 18446744073709551615, using digits only.';
      byId('seed').focus();
      return;
    }
    byId('seed').removeAttribute('aria-invalid');
    body = name === 'seed' ? { seed: BigInt(seed).toString() } : { random: true };
  }
  const version = ++commandVersion;
  busy = true; controls();
  byId('control-message').textContent = restart ? 'Restart sent; waiting for the new experiment…' : 'Request sent; waiting for the worker to apply it…';
  try {
    const response = await fetch(restart ? '/api/restart' : '/api/control', {
      method: 'POST', headers: { 'Content-Type': 'application/json', [runHeader]: runId }, body: JSON.stringify(body),
      signal: AbortSignal.timeout(4000),
    });
    const result = await response.json();
    if (version !== commandVersion) return; // A newer run/command owns the page now.
    if (restart && response.ok) {
      const nextRun = response.headers.get(runHeader);
      if (!nextRun || nextRun === runId) throw new Error('Missing new experiment identity');
      receiveRun(result, nextRun);
      byId('control-message').textContent = `Restarted with seed ${sample.experiment.seed}. ${sample.status === 'completed' ? 'Completed' : 'Paused'} at tick 0.`;
      recordSample(history, sample);
      render();
      return;
    }
    if (response.headers.get(runHeader) !== runId) {
      busy = false;
      connectionError('Server changed; waiting for its current experiment. No command was retried.');
      return;
    }
    if (!response.ok) {
      byId('control-message').textContent = result.error || result.message;
      busy = false;
    } else {
      receipt = result;
      byId('control-message').textContent = `${name === 'step' ? 'Step' : name === 'pause' ? 'Pause' : 'Resume'} applied at tick ${result.tick}.`;
    }
  } catch {
    if (version !== commandVersion) return;
    busy = false;
    connectionError(restart ? 'Restart outcome unknown. Check the current experiment and seed before another command. The request was not retried.' : 'Control outcome unknown. Check the current tick before another command. The request was not retried.');
    byId('control-message').textContent = restart ? 'Restart outcome unknown; inspect the current experiment and seed. No automatic retry.' : 'Outcome unknown; inspect the current tick before another command. No automatic retry.';
  } finally {
    if (version === commandVersion) {
      commandVersion++;
      controls();
    }
  }
}
for (const name of ['pause', 'resume', 'step']) byId(name).addEventListener('click', () => command(name));
for (const name of ['seed', 'random']) byId(`restart-${name}`).addEventListener('click', () => command(name, true));
byId('agent').addEventListener('change', event => { selected = event.target.value; if (sample) render(); });
byId('grid').addEventListener('click', event => {
  if (!sample) return;
  const index = gridPosition(byId('grid'), event, sample);
  selected = sample.cells[index]?.id || '';
  render();
});
// Resize only repaints retained data, including while paused or disconnected.
let paintFrame = 0;
function schedulePaint() {
  if (paintFrame) return;
  paintFrame = requestAnimationFrame(() => { paintFrame = 0; paintCanvases(); });
}
const canvasResize = new ResizeObserver(schedulePaint);
for (const id of ['grid', 'plot']) canvasResize.observe(byId(id));
function watchPixelRatio() {
  matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`).addEventListener('change', () => {
    schedulePaint();
    watchPixelRatio();
  }, { once: true });
}
watchPixelRatio();
poll();
