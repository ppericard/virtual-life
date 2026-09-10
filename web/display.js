// Display only: these functions never compute a simulation transition.
export const HISTORY_LIMIT = 128;
const colors = ['#c89bc4', '#89baca', '#d6b36e', '#b4bf83', '#97bba0'];
export function colorFor(id) { return colors[Number(BigInt(id) % BigInt(colors.length))]; }
// Okabe–Ito-inspired palette; the persistent letter is a second identifier.
const groupColors = ['#0072b2', '#d55e00', '#009e73', '#cc79a7', '#e69f00', '#56b4e9', '#666666', '#f0e442'];
export const groupColor = index => groupColors[index % groupColors.length];
const groupInk = index => [4, 5, 7].includes(index) ? '#203333' : '#ffffff';
export const groupLabel = index => index < 26 ? String.fromCharCode(65 + index) : `G${index + 1}`;

export function failureText(failure) {
  return `Agent ${failure.id} failed at tick ${failure.tick} · ${failure.reason} · Position (${failure.position.x}, ${failure.position.y}) · starting integrity ${failure.integrity_before}, upkeep ${failure.upkeep}, extra wear ${failure.action_wear}.`;
}

export function inspectionText(sample, selected) {
  if (!selected) return 'Select an occupied square or choose an agent above.';
  const index = sample.cells.findIndex(agent => agent?.id === selected);
  if (index < 0) {
    if (sample.experiment?.maintenance) {
      const history = sample.failure_history;
      const failure = history?.records.find(record => record.id === selected);
      if (failure) return failureText(failure);
      return BigInt(history?.discarded ?? '0') > 0n ? `Agent ${selected} is absent; its failure record has been discarded from the bounded history (${history.discarded} older records discarded). Exact cause is unavailable.` : `Agent ${selected} is absent. No failure record available; no cause inferred.`;
    }
    return `Agent ${selected} was removed by tick ${sample.tick}.`;
  }
  const agent = sample.cells[index];
  const properties = sample.experiment ? `Group ${groupLabel(agent.group)} · ${sample.experiment.maintenance ? 'Base weights' : 'Weights'} (wait, move, copy, ${sample.experiment.maintenance ? 'repair' : 'remove'}): ${agent.weights.join(', ')}` : `Value ${agent.value}`;
  const integrity = sample.experiment?.maintenance ? ` · Integrity ${agent.integrity}/${sample.experiment.maintenance.maximum}` : '';
  return `ID ${agent.id} · ${properties} · Position (${index % sample.width}, ${Math.floor(index / sample.width)})${integrity}`;
}

export function renderReadouts(sample, selected, root = document) {
  for (const name of ['tick', 'count', 'status']) root.getElementById(name).textContent = sample[name];
  for (const [name, value] of Object.entries(sample.totals)) root.getElementById(name).textContent = value;
  root.getElementById('inspection').textContent = inspectionText(sample, selected);
}

export function renderExperiment(sample) {
  document.getElementById('end-tick').textContent = sample.end_tick;
  document.getElementById('dimensions').textContent = `${sample.width} × ${sample.height} · wraparound edges`;
  document.getElementById('intro').textContent = sample.experiment ? 'An autonomous experiment. Shared properties, individual choices, synchronous ticks.' : 'Five scripted transitions. A test of the machinery, before autonomous rules.';
  const legend = document.getElementById('legend'); legend.replaceChildren();
  document.getElementById('experiment-settings').hidden = !sample.experiment;
  document.getElementById('choice-rule').hidden = !sample.experiment?.maintenance;
  document.getElementById('failure-evidence').hidden = !sample.experiment?.maintenance;
  for (const element of document.querySelectorAll('.maintenance-measurement')) element.hidden = !sample.experiment?.maintenance;
  if (!sample.experiment) return;
  const info = sample.experiment;
  document.getElementById('action-order').textContent = `Wait / move / copy / ${info.maintenance ? 'repair' : 'remove'}`;
  const rules = info.maintenance;
  const maintenance = rules ? ` Maximum, initial and newborn integrity ${rules.maximum}; upkeep ${rules.upkeep}; extra move/copy wear ${rules.move_wear}/${rules.copy_wear}; gross repair ${rules.repair}. Repair costs the action opportunity. Failed spatial attempts pay wear.` : ' Random removal comparison; no integrity.';
  document.getElementById('configuration').textContent = `Seed ${info.seed} · ${info.generator} · crate ${info.version} · ${info.protocol} · occupancy ${info.occupancy} (rounded down to an exact count) · ${sample.end_tick} ticks.${maintenance} Illustrative settings, not calibrated biology.`;
  if (rules) {
    const history = sample.failure_history;
    document.getElementById('failure-summary').textContent = `${sample.totals.failures} cumulative failures. ${history.records.length} retained engine records (latest ${history.limit}); ${history.discarded} older records discarded. Recorded on every tick, independently of plot sampling.${history.records.length ? '' : ' No failures recorded yet.'}`;
    const list = document.getElementById('failure-list'); list.replaceChildren();
    for (const failure of [...history.records].reverse()) {
      const item = document.createElement('li'); item.textContent = failureText(failure); list.append(item);
    }
  }
  info.groups.forEach((group, index) => {
    const row = document.createElement('div'); row.className = 'group-row'; row.dataset.group = String(index);
    const swatch = document.createElement('span'); swatch.className = 'group-symbol'; swatch.style.backgroundColor = groupColor(index); swatch.style.color = groupInk(index); swatch.textContent = groupLabel(index);
    const details = document.createElement('span'); details.textContent = `Group ${groupLabel(index)} · ${group.weights.join(' / ')} · ratio ${group.proportion} · initially ${group.initial_count}`;
    const count = document.createElement('strong'); count.textContent = group.count; count.className = 'group-count';
    row.append(swatch, details, count); legend.append(row);
  });
}

export function recordSample(history, sample) {
  const point = { tick: sample.tick, count: sample.count, groups: sample.experiment?.groups.map(group => group.count) };
  if (history.at(-1)?.tick === point.tick) history[history.length - 1] = point;
  else if (!history.length || BigInt(point.tick) > BigInt(history.at(-1).tick)) history.push(point);
  if (history.length > HISTORY_LIMIT) history.shift();
}

export function sampleDescription(history) {
  if (!history.length) return 'Waiting for the first sample.';
  const gaps = history.slice(1).filter((point, index) => BigInt(point.tick) - BigInt(history[index].tick) > 1n).length;
  const ticks = history.length === 1 ? `Tick ${history[0].tick}` : `Ticks ${history[0].tick}–${history.at(-1).tick}`;
  return `${ticks} · ${history.length} sample${history.length === 1 ? '' : 's'} · ${gaps} sampling gap${gaps === 1 ? '' : 's'}.${history.length === HISTORY_LIMIT ? ' Older samples are discarded as new ones arrive.' : ''} Sampled history, not a complete recording.`;
}

// Logical canvas coordinates stay fixed; CSS scales them to the window.
export function gridPosition(canvas, event, sample) {
  const box = canvas.getBoundingClientRect();
  const x = Math.floor(((event.clientX - box.left) * 600 / box.width - 40) / (540 / sample.width));
  const y = Math.floor(((event.clientY - box.top) * 600 / box.height - 40) / (540 / sample.height));
  return x >= 0 && y >= 0 && x < sample.width && y < sample.height ? y * sample.width + x : -1;
}

export function drawGrid(canvas, sample, selected) {
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, 600, 600);
  const w = 540 / sample.width, h = 540 / sample.height;
  ctx.textAlign = 'center'; ctx.textBaseline = 'middle'; ctx.font = '16px system-ui'; ctx.fillStyle = '#536662';
  for (let x = 0; x < sample.width; x += Math.max(1, Math.ceil(24 / w))) ctx.fillText(String(x), 40 + (x + .5) * w, 22);
  for (let y = 0; y < sample.height; y += Math.max(1, Math.ceil(24 / h))) ctx.fillText(String(y), 20, 40 + (y + .5) * h);
  sample.cells.forEach((agent, index) => {
    const x = 40 + (index % sample.width) * w, y = 40 + Math.floor(index / sample.width) * h;
    ctx.fillStyle = agent ? (sample.experiment ? groupColor(agent.group) : colorFor(agent.id)) : '#f5f7f2';
    const gap = Math.min(2, Math.min(w, h) / 10);
    ctx.fillRect(x + gap, y + gap, w - gap * 2, h - gap * 2);
    if (agent) {
      ctx.fillStyle = sample.experiment ? groupInk(agent.group) : '#203333'; ctx.font = `${sample.experiment ? Math.min(18, Math.min(w, h) * .65) : 22}px system-ui`;
      if (!sample.experiment || Math.min(w, h) >= 14) ctx.fillText(sample.experiment ? groupLabel(agent.group) : agent.id, x + w / 2, y + h / 2, Math.max(1, w - gap * 2));
      if (agent.id === selected) { ctx.strokeStyle = '#203333'; ctx.lineWidth = Math.min(4, w / 8); ctx.strokeRect(x + gap, y + gap, w - gap * 2, h - gap * 2); }
    }
  });
  canvas.setAttribute('aria-label', `World at tick ${sample.tick}, ${sample.count} agents. Use the agent selector for exact IDs and values.`);
}

export function drawPlot(canvas, history) {
  if (history[0]?.groups) { drawGroupPlot(canvas, history); return; }
  const ctx = canvas.getContext('2d');
  ctx.clearRect(0, 0, 600, 230);
  if (!history.length) return;
  const first = BigInt(history[0].tick), last = BigInt(history.at(-1).tick);
  const range = last - first || 1n;
  const max = history.reduce((a, b) => BigInt(a.count) >= BigInt(b.count) ? a : b).count;
  const top = Math.max(1, Number(max));
  // Only pixel coordinates are approximate; labels/readouts keep decimal strings.
  const x = point => 44 + Number((BigInt(point.tick) - first) * 1_000_000n / range) / 1_000_000 * 520;
  const y = point => 184 - Number(point.count) / top * 145;
  ctx.strokeStyle = '#ccd7d0'; ctx.lineWidth = 1;
  ctx.beginPath(); ctx.moveTo(44, 20); ctx.lineTo(44, 184); ctx.lineTo(564, 184); ctx.stroke();
  ctx.fillStyle = '#536662'; ctx.font = '16px system-ui'; ctx.textAlign = 'left';
  ctx.fillText('0', 8, 190); ctx.fillText(max, 8, 43, 32); ctx.fillText(history[0].tick, 44, 214, 230);
  if (history.length > 1) { ctx.textAlign = 'right'; ctx.fillText(history.at(-1).tick, 564, 214, 230); }
  ctx.textAlign = 'center'; ctx.fillText('tick', 300, 224);
  history.forEach((point, index) => {
    const previous = history[index - 1];
    if (previous && BigInt(point.tick) - BigInt(previous.tick) === 1n) {
      ctx.strokeStyle = '#356c5b'; ctx.lineWidth = 2;
      ctx.beginPath(); ctx.moveTo(x(previous), y(previous)); ctx.lineTo(x(point), y(point)); ctx.stroke();
    }
    ctx.fillStyle = '#254f45'; ctx.beginPath(); ctx.arc(x(point), y(point), 5, 0, Math.PI * 2); ctx.fill();
  });
  canvas.setAttribute('aria-label', history.map(p => `Tick ${p.tick}: ${p.count} agents`).join('; '));
}

function drawGroupPlot(canvas, history) {
  const ctx = canvas.getContext('2d'); ctx.clearRect(0, 0, 600, 230);
  const first = BigInt(history[0].tick), last = BigInt(history.at(-1).tick), range = last - first || 1n;
  const top = Math.max(1, ...history.flatMap(point => point.groups.map(Number)));
  const x = point => 44 + Number((BigInt(point.tick) - first) * 1_000_000n / range) / 1_000_000 * 485;
  const y = count => 184 - Number(count) / top * 145;
  ctx.strokeStyle = '#ccd7d0'; ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(44, 20); ctx.lineTo(44, 184); ctx.lineTo(540, 184); ctx.stroke();
  ctx.fillStyle = '#536662'; ctx.font = '15px system-ui'; ctx.textAlign = 'left'; ctx.fillText('0', 8, 190); ctx.fillText(String(top), 8, 43, 32); ctx.fillText(history[0].tick, 44, 213);
  ctx.textAlign = 'right'; ctx.fillText(history.at(-1).tick, 529, 213); ctx.textAlign = 'center'; ctx.fillText('tick', 300, 228);
  history[0].groups.forEach((_, group) => {
    ctx.strokeStyle = groupColor(group); ctx.fillStyle = groupColor(group); ctx.lineWidth = 2;
    ctx.setLineDash([[], [6, 3], [2, 3], [8, 3, 2, 3]][group % 4]);
    history.forEach((point, index) => {
      const previous = history[index - 1];
      if (previous && BigInt(point.tick) - BigInt(previous.tick) === 1n) { ctx.beginPath(); ctx.moveTo(x(previous), y(previous.groups[group])); ctx.lineTo(x(point), y(point.groups[group])); ctx.stroke(); }
      ctx.beginPath(); ctx.arc(x(point), y(point.groups[group]), 3, 0, Math.PI * 2); ctx.fill();
    });
    ctx.setLineDash([]);
  });
  // Endpoint letters use separate rows, so extinct/overlapping zero series stay identifiable.
  const final = history.at(-1);
  ctx.textAlign = 'left'; ctx.font = '12px system-ui';
  final.groups.forEach((count, group) => { ctx.fillStyle = groupColor(group); ctx.fillText(`${groupLabel(group)} ${count}`, 544, 28 + group * 18, 55); });
  canvas.setAttribute('aria-label', history.map(point => `Tick ${point.tick}: ${point.groups.map((count, group) => `Group ${groupLabel(group)} ${count}`).join(', ')}; total ${point.count}`).join('; '));
}
