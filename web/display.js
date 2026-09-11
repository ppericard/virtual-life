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
  return `Agent ${failure.id} failed at tick ${failure.tick} · ${failure.reason} · Position (${failure.position.x}, ${failure.position.y}) · starting integrity ${failure.integrity_before}, occupied neighbours ${failure.occupied_neighbors}/8, effective upkeep ${failure.upkeep} (base ${failure.base_upkeep} + crowding ${failure.crowding_upkeep}), extra wear ${failure.action_wear}.`;
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
  const integrity = sample.experiment?.maintenance ? ` · Integrity ${agent.integrity}/${sample.experiment.maintenance.maximum} · Current occupied neighbours ${agent.occupied_neighbors}/8 · Next-tick effective upkeep ${agent.next_tick_upkeep} (base ${sample.experiment.maintenance.upkeep} + crowding ${agent.crowding_upkeep}; displayed neighbourhood)` : '';
  const action = sample.experiment?.maintenance ? ` · Last selected action: ${agent.last_action === null ? 'Not yet acted' : agent.last_action === undefined ? 'Unavailable' : `${agent.last_action} (success not implied)`}` : '';
  return `ID ${agent.id} · ${properties} · Position (${index % sample.width}, ${Math.floor(index / sample.width)})${integrity}${action}`;
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
  const counts = document.getElementById('population-groups'); counts.replaceChildren();
  document.getElementById('experiment-settings').hidden = !sample.experiment;
  document.getElementById('choice-rule').hidden = !sample.experiment?.maintenance;
  document.getElementById('failure-evidence').hidden = !sample.experiment?.maintenance;
  for (const element of document.querySelectorAll('.maintenance-measurement')) element.hidden = !sample.experiment?.maintenance;
  if (!sample.experiment) return;
  const info = sample.experiment;
  document.getElementById('action-order').textContent = `Wait / move / copy / ${info.maintenance ? 'repair' : 'remove'}`;
  const rules = info.maintenance;
  const compatibility = rules ? ' (Replaying wear-repair v1 requires its earlier code; crowding v2 requires --crowding-upkeep 0 with matching settings.)' : '';
  const maintenance = rules ? ` Maximum, initial and newborn integrity ${rules.maximum}; base upkeep ${rules.upkeep}, plus ${rules.crowding_upkeep} when at least ${rules.crowding_threshold}/8 starting neighbours are occupied; extra move/copy wear ${rules.move_wear}/${rules.copy_wear}; gross repair ${rules.repair}. Upkeep precedes every action, including Repair. Repair costs the action opportunity. Failed spatial attempts pay wear.` : ' Random removal comparison; no integrity.';
  document.getElementById('configuration').textContent = `Seed ${info.seed} · ${info.generator} · crate ${info.version} · ${info.protocol}${compatibility} · occupancy ${info.occupancy} (rounded down to an exact count) · ${sample.end_tick} ticks.${maintenance} Illustrative settings, not calibrated biology.`;
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
    const chip = document.createElement('span'); chip.className = 'group-chip';
    chip.setAttribute('aria-label', `Group ${groupLabel(index)}: ${group.count} agents`);
    chip.append(swatch.cloneNode(true), count.cloneNode(true)); counts.append(chip);
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

// One pitch for both axes, with a slim, even frame around the world.
function gridGeometry(sample) {
  const pitch = 540 / Math.max(sample.width, sample.height);
  const padding = 8;
  return {pitch, left: padding, top: padding, width: padding * 2 + sample.width * pitch, height: padding * 2 + sample.height * pitch};
}

// Fit the actual canvas box, including the frame used by drawing and hit testing.
export function fitGrid(canvas, sample, availableWidth, availableHeight) {
  const g = gridGeometry(sample);
  const scale = Math.max(0, Math.min(availableWidth / g.width, availableHeight / g.height));
  canvas.style.width = `${g.width * scale}px`;
  canvas.style.height = `${g.height * scale}px`;
}

export function gridPosition(canvas, event, sample) {
  const box = canvas.getBoundingClientRect(), g = gridGeometry(sample), scale = box.width / g.width;
  const x = Math.floor(((event.clientX - box.left) / scale - g.left) / g.pitch);
  const y = Math.floor(((event.clientY - box.top) / scale - g.top) / g.pitch);
  return x >= 0 && y >= 0 && x < sample.width && y < sample.height ? y * sample.width + x : -1;
}

// Keep drawing and hit testing in logical coordinates; CSS owns the displayed size.
function prepareCanvas(canvas, width, height, maxSide = Infinity) {
  const box = canvas.getBoundingClientRect();
  const ratio = Math.min(window.devicePixelRatio || 1, maxSide / box.width, maxSide / box.height);
  const pixelsWide = Math.max(1, Math.round(box.width * ratio));
  const pixelsHigh = Math.max(1, Math.round(box.height * ratio));
  if (canvas.width !== pixelsWide) canvas.width = pixelsWide;
  if (canvas.height !== pixelsHigh) canvas.height = pixelsHigh;
  const ctx = canvas.getContext('2d');
  ctx.setTransform(canvas.width / width, 0, 0, canvas.height / height, 0, 0);
  ctx.clearRect(0, 0, width, height);
  return ctx;
}

export function drawGrid(canvas, sample, selected) {
  const g = gridGeometry(sample), w = g.pitch, h = g.pitch;
  canvas.style.aspectRatio = `${g.width} / ${g.height}`;
  const box = canvas.getBoundingClientRect();
  if (!box.width || !box.height) return;
  // Cap raster allocation at 4096² pixels. CSS geometry stays square at any DPR,
  // including fractional backing-size rounding and very elongated worlds.
  const ctx = prepareCanvas(canvas, box.width, box.height, 4096), scale = box.width / g.width;
  ctx.scale(scale, scale);
  const showLetters = w * scale >= 14;
  ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
  sample.cells.forEach((agent, index) => {
    const x = g.left + (index % sample.width) * w, y = g.top + Math.floor(index / sample.width) * h;
    ctx.fillStyle = agent ? (sample.experiment ? groupColor(agent.group) : colorFor(agent.id)) : '#f5f7f2';
    const gap = Math.min(2, Math.min(w, h) / 10);
    ctx.fillRect(x + gap, y + gap, w - gap * 2, h - gap * 2);
    if (agent) {
      ctx.fillStyle = sample.experiment ? groupInk(agent.group) : '#203333'; ctx.font = `${sample.experiment ? Math.min(18, Math.min(w, h) * .65) : 22}px system-ui`;
      if (!sample.experiment || showLetters) ctx.fillText(sample.experiment ? groupLabel(agent.group) : agent.id, x + w / 2, y + h / 2);
      if (agent.id === selected) { ctx.strokeStyle = '#203333'; ctx.lineWidth = Math.min(4, w / 8); ctx.strokeRect(x + gap, y + gap, w - gap * 2, h - gap * 2); }
    }
  });
  canvas.setAttribute('aria-label', `World at tick ${sample.tick}, ${sample.count} agents. Use the agent selector for exact IDs and values.`);
}

export function drawPlot(canvas, history) {
  // Retain accessible evidence even when closed; allocate/paint only when visible.
  canvas.setAttribute('aria-label', history.map(point => point.groups
    ? `Tick ${point.tick}: ${point.groups.map((count, group) => `Group ${groupLabel(group)} ${count}`).join(', ')}; total ${point.count}`
    : `Tick ${point.tick}: ${point.count} agents`).join('; '));
  if (!canvas.getBoundingClientRect().width) return;
  const ctx = prepareCanvas(canvas, 600, 230);
  if (history[0]?.groups) { drawGroupPlot(canvas, ctx, history); return; }
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
}

function drawGroupPlot(canvas, ctx, history) {
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
}
