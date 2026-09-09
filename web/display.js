// Display only: these functions never compute a simulation transition.
export const HISTORY_LIMIT = 128;
const colors = ['#c89bc4', '#89baca', '#d6b36e', '#b4bf83', '#97bba0'];
export function colorFor(id) { return colors[Number(BigInt(id) % BigInt(colors.length))]; }

export function inspectionText(sample, selected) {
  if (!selected) return 'Select an occupied square or choose an agent above.';
  const index = sample.cells.findIndex(agent => agent?.id === selected);
  if (index < 0) return `Agent ${selected} was removed by tick ${sample.tick}.`;
  const agent = sample.cells[index];
  return `ID ${agent.id} · Value ${agent.value} · Position (${index % sample.width}, ${Math.floor(index / sample.width)})`;
}

export function renderReadouts(sample, selected, root = document) {
  for (const name of ['tick', 'count', 'status']) root.getElementById(name).textContent = sample[name];
  for (const [name, value] of Object.entries(sample.totals)) root.getElementById(name).textContent = value;
  root.getElementById('inspection').textContent = inspectionText(sample, selected);
}

export function recordSample(history, sample) {
  const point = { tick: sample.tick, count: sample.count };
  if (history.at(-1)?.tick === point.tick) history[history.length - 1] = point;
  else if (!history.length || BigInt(point.tick) > BigInt(history.at(-1).tick)) history.push(point);
  if (history.length > HISTORY_LIMIT) history.shift();
}

export function sampleDescription(history) {
  const gaps = history.slice(1).filter((point, index) => BigInt(point.tick) - BigInt(history[index].tick) > 1n).length;
  return `Observed ticks: ${history.map(point => point.tick).join(', ')}. ${gaps} sampling gap${gaps === 1 ? '' : 's'}. History starts at tick ${history[0]?.tick ?? '—'}.`;
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
  for (let x = 0; x < sample.width; x++) ctx.fillText(String(x), 40 + (x + .5) * w, 22);
  for (let y = 0; y < sample.height; y++) ctx.fillText(String(y), 20, 40 + (y + .5) * h);
  sample.cells.forEach((agent, index) => {
    const x = 40 + (index % sample.width) * w, y = 40 + Math.floor(index / sample.width) * h;
    ctx.fillStyle = agent ? colorFor(agent.id) : '#f5f7f2';
    ctx.fillRect(x + 2, y + 2, w - 4, h - 4);
    if (agent) {
      ctx.fillStyle = '#203333'; ctx.font = '22px system-ui';
      ctx.fillText(agent.id, x + w / 2, y + h / 2, w - 14);
      if (agent.id === selected) { ctx.strokeStyle = '#254f45'; ctx.lineWidth = 4; ctx.strokeRect(x + 5, y + 5, w - 10, h - 10); }
    }
  });
  canvas.setAttribute('aria-label', `World at tick ${sample.tick}, ${sample.count} agents. Use the agent selector for exact IDs and values.`);
}

export function drawPlot(canvas, history) {
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
