import { drawGrid, drawPlot, gridPosition, recordSample, renderReadouts, sampleDescription } from './display.js';
const byId = id => document.getElementById(id);
const history = [];
let sample, selected = '', connected = false, busy = false, receipt = null, commandVersion = 0;

function controls() {
  const ready = connected && !busy && sample;
  byId('step').disabled = !ready || sample.status !== 'paused';
  byId('pause').disabled = !ready || sample.status !== 'running';
  byId('resume').disabled = !ready || sample.status !== 'paused';
}
function render() {
  renderReadouts(sample, selected);
  const selector = byId('agent');
  selector.replaceChildren(new Option('Choose an agent', ''));
  for (const agent of sample.cells.filter(Boolean)) selector.add(new Option(`ID ${agent.id} · Value ${agent.value}`, agent.id));
  if (selected && !sample.cells.some(agent => agent?.id === selected)) selector.add(new Option(`ID ${selected} · removed`, selected));
  selector.value = selected;
  drawGrid(byId('grid'), sample, selected);
  drawPlot(byId('plot'), history);
  byId('samples').textContent = sampleDescription(history);
  controls();
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
    sample = next;
    connected = true;
    byId('connection').hidden = true;
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
    connectionError(`Connection lost (${error.message}). The server may have stopped. Last received state remains below; reconnecting…`);
  } finally {
    setTimeout(poll, 100);
  }
}

async function command(name) {
  if (busy) return;
  busy = true; commandVersion++; controls();
  byId('control-message').textContent = 'Request sent; waiting for the worker to apply it…';
  try {
    const response = await fetch('/api/control', {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ command: name }),
      signal: AbortSignal.timeout(4000),
    });
    const result = await response.json();
    if (!response.ok) {
      byId('control-message').textContent = result.error || result.message;
      busy = false;
    } else {
      receipt = result;
      byId('control-message').textContent = `${name === 'step' ? 'Step' : name === 'pause' ? 'Pause' : 'Resume'} applied at tick ${result.tick}.`;
    }
  } catch {
    busy = false;
    connectionError('Control outcome unknown. Check the current tick before another command. The request was not retried.');
    byId('control-message').textContent = 'Outcome unknown; inspect the current tick before another command. No automatic retry.';
  } finally {
    commandVersion++;
    controls();
  }
}
for (const name of ['pause', 'resume', 'step']) byId(name).addEventListener('click', () => command(name));
byId('agent').addEventListener('change', event => { selected = event.target.value; if (sample) render(); });
byId('grid').addEventListener('click', event => {
  if (!sample) return;
  const index = gridPosition(byId('grid'), event, sample);
  selected = sample.cells[index]?.id || '';
  render();
});
poll();
