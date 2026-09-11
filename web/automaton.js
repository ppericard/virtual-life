// Live tickets come from Rust; no simulation policy runs in the page.
const states = ['Wait', 'Move', 'Copy', 'Repair'];
const positions = [[85,75],[295,75],[85,225],[295,225]];
const ns = 'http://www.w3.org/2000/svg';
let serial = 0;
function svgElement(tag, attrs = {}, text = '') {
  const node = document.createElementNS(ns, tag);
  for (const [key,value] of Object.entries(attrs)) node.setAttribute(key, String(value));
  node.textContent = text;
  return node;
}
export function probabilityLabels(tickets) {
  const values = tickets.map(BigInt), total = values.reduce((a,b) => a+b, 0n);
  return values.map(value => {
    if (value === 0n || total === 0n) return '0%';
    if (value === total) return '100%';
    if (value * 1000n < total) return '<0.1%';
    return `${Number(value * 1000n / total) / 10}%`;
  });
}
export function graph(source, weights, probabilities = false) {
  const svg = svgElement('svg', {viewBox:'0 0 380 320', role:'img', class:'automaton-graph'});
  const labels = probabilities ? probabilityLabels(weights) : weights.map(w => `weight ${w}`);
  const edges = weights.map((w,i) => BigInt(w) > 0n ? `${source} to ${states[i]}: ${labels[i]}` : '').filter(Boolean);
  svg.setAttribute('aria-label', edges.join('; '));
  svg.append(svgElement('title', {}, edges.join('; ')));
  const markerId = `arrow-${++serial}`, defs = svgElement('defs');
  const marker = svgElement('marker', {id:markerId,viewBox:'0 0 10 10',refX:9,refY:5,markerWidth:7,markerHeight:7,orient:'auto-start-reverse'});
  marker.append(svgElement('path',{d:'M 0 0 L 10 5 L 0 10 z',fill:'#245047'})); defs.append(marker); svg.append(defs);
  const from = states.indexOf(source), [x,y] = positions[from];
  weights.forEach((weight,i) => {
    if (BigInt(weight) === 0n) return;
    const [tx,ty] = positions[i]; let d, lx, ly;
    if (i === from) {
      const side = y < 150 ? -1 : 1;
      d = `M ${x-23} ${y+side*30} C ${x-85} ${y+side*90}, ${x+85} ${y+side*90}, ${x+23} ${y+side*30}`;
      lx=x; ly=y+side*62;
    } else {
      const length = Math.hypot(tx-x,ty-y), dx=(tx-x)/length*40, dy=(ty-y)/length*40;
      d=`M ${x+dx} ${y+dy} L ${tx-dx} ${ty-dy}`;
      lx=(x+tx)/2; ly=(y+ty)/2-8;
    }
    svg.append(svgElement('path',{d,fill:'none',stroke:'#245047','stroke-width':2,'marker-end':`url(#${markerId})`}));
    svg.append(svgElement('text',{x:lx,y:ly,'text-anchor':'middle',class:'edge-label'},labels[i]));
  });
  positions.forEach(([cx,cy],i) => {
    svg.append(svgElement('circle',{cx,cy,r:38,fill:i===from?'#dfede7':'#fff',stroke:'#245047','stroke-width':i===from?3:1.5}));
    svg.append(svgElement('text',{x:cx,y:cy+5,'text-anchor':'middle'},states[i]));
  });
  return svg;
}
function card(machine, name) {
  const root = document.createElement('section'); root.className='automaton-card';
  const title = document.createElement('h3'); title.textContent=name;
  const label = document.createElement('label'); label.textContent='View transitions: ';
  const select = document.createElement('select'); select.setAttribute('aria-label', `${name} transitions`);
  for (const [value,text] of [['current','Current probabilities'], ...states.map(s=>[s,`Inherited arrows from ${s}`])]) {
    const option=document.createElement('option'); option.value=value; option.textContent=text; select.append(option);
  }
  const note=document.createElement('p'); note.className='hint';
  const drawing=document.createElement('div');
  const response=document.createElement('p'); response.className='hint';
  response.textContent=`Repair weight follows missing integrity (zero at full integrity). Copy damage gain: ${machine.copy_damage_gain}; Move crowding gain: ${machine.move_crowding_gain}. Crowding transfers part of Copy to Wait. No eligible arrow means Wait.`;
  label.append(select); root.append(title,label,note,drawing,response);
  let current = null;
  function draw() {
    if (select.value !== 'current') {
      note.textContent = 'Inherited relative weights before integrity and crowding adjustments. Zero-weight arrows are absent.';
      drawing.replaceChildren(graph(select.value, machine.rows[states.indexOf(select.value)]));
    } else if (current?.transition_tickets) {
      note.textContent = `Next choice from ${current.state}, using integrity ${current.integrity_after_upkeep} after upkeep and this neighbourhood. Percentages are rounded down; action success is not guaranteed.`;
      drawing.replaceChildren(graph(current.state, current.transition_tickets, true));
    } else {
      note.textContent = 'Upkeep is fatal before another choice; no transition is drawn.';
      drawing.replaceChildren();
    }
  }
  select.addEventListener('change',draw);
  return {root, update(agent) {current=agent; draw();}, preset() {select.options[0].remove(); select.value=machine.initial; draw();}};
}
export function renderAgentAutomaton(root, sample, selected) {
  if (!root) return;
  const agent=sample.cells.find(agent=>agent?.id===selected), group=sample.experiment?.groups[agent?.group];
  root.hidden=!group?.automaton;
  if (root.hidden) {root.replaceChildren(); root.widget=null; return;}
  const key=JSON.stringify([agent.id,group.automaton]);
  if (root.dataset.key!==key || !root.widget) {
    root.dataset.key=key; root.widget=card(group.automaton,group.automaton_name); root.replaceChildren(root.widget.root);
  }
  root.widget.update(agent);
}
export function renderPresetAutomata(root, groups) {
  root.replaceChildren();
  groups.forEach((group,i)=> {
    const widget=card(group.automaton,`Group ${String.fromCharCode(65+i)} automaton`);
    widget.preset(); root.append(widget.root);
  });
}
