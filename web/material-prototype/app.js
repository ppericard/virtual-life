import {W,H,CELL_MATERIAL,ACTIONS,initial,apply,guided,material,cellMaterial,parts,nearby,row} from './model.js';
const $ = id => document.getElementById(id);
let world, selected=1, target=2, destination={x:2,y:1}, undo=[];
const name = a => `#${a.id} · (${a.x}, ${a.y})`;
const arrows = a => ACTIONS.filter((op,i)=>row(a)[i] && (op!=='take'||world.acquisition==='take'));
const sum = a => a.structure+a.reserve+a.loose;
function bar(a) {
  const el=document.createElement('div'); el.className='bar';
  for (const key of ['structure','reserve','loose']) {
    // Width is an SVG attribute below to keep the preview CSP free of inline styles.
    const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');
    svg.setAttribute('width', `${100*a[key]/Math.max(1,sum(a))}%`); svg.setAttribute('height','7');
    const rect=document.createElementNS(svg.namespaceURI,'rect'); rect.setAttribute('width','100%'); rect.setAttribute('height','7');
    rect.setAttribute('fill', {structure:'#4b73bd',reserve:'#cf9725',loose:'#a7755d'}[key]);
    svg.append(rect); el.append(svg);
  }
  return el;
}
function option(value,text){const o=document.createElement('option');o.value=value;o.textContent=text;return o;}
function run(command, useGuide=false){
  undo.push(structuredClone(world));
  try {if(useGuide)selected=world.steps[world.guide].actor;world=useGuide?guided(world):apply(world,command);}
  catch(e){undo.pop();$('message').textContent=e.message;$('message').className='error';return;}
  render();
}
function reset(){world=initial($('scenario').value,$('space').value,$('acquisition').value);selected=1;target=2;destination={x:2,y:1};undo=[];render();}
function render(){
  if(!world.agents.some(a=>a.id===selected)) selected=world.agents[0]?.id;
  const actor=world.agents.find(a=>a.id===selected);
  $('total').textContent=`${material(world)} / ${world.initialMaterial}`;
  $('conserved').textContent=material(world)===world.initialMaterial?'Conserved':'MISMATCH';
  ['structure','reserve','loose'].forEach((k,i)=>$(`${k}-total`).textContent=parts(world)[i]);
  $('agents-total').textContent=world.agents.length;
  $('world').replaceChildren();
  for(let y=0;y<H;y++)for(let x=0;x<W;x++){
    const cell=document.createElement('div');cell.className='cell'+(destination.x===x&&destination.y===y?' destination':'');
    const head=document.createElement('button');head.className='cell-head';head.setAttribute('aria-label',`Destination cell ${x}, ${y}`);
    const pos=document.createElement('span');pos.textContent=`(${x}, ${y})`;
    const cap=document.createElement('span');cap.className='cell-cap';cap.textContent=`${cellMaterial(world,x,y)}/${CELL_MATERIAL}`;
    head.append(pos,cap);head.onclick=()=>{destination={x,y};render();};cell.append(head);
    for(const a of world.agents.filter(b=>b.x===x&&b.y===y)){
      const btn=document.createElement('button');btn.className='agent'+(a.id===selected?' selected':'');
      btn.setAttribute('aria-label',`Inspect agent ${a.id}`);btn.onclick=()=>{selected=a.id;render();};
      const title=document.createElement('div');title.className='agent-title';title.textContent=`#${a.id} · ${sum(a)} material`;
      const status=document.createElement('span');status.className='agent-status';status.textContent=arrows(a).length===1&&arrows(a)[0]==='wait'?'→ Wait 100%':'→ several actions';title.append(status);
      const nums=document.createElement('div');nums.className='numbers';nums.textContent=`S ${a.structure} · R ${a.reserve} · L ${a.loose}`;
      btn.append(title,bar(a),nums);cell.append(btn);
    }
    $('world').append(cell);
  }
  $('actor').replaceChildren(...world.agents.map(a=>option(a.id,name(a))));$('actor').value=selected;
  $('agent-detail').replaceChildren();
  if(actor){
    const info=document.createElement('div');info.className='agent-info';
    info.textContent=`Structure ${actor.structure}/10 · Reserve ${actor.reserve} · Loose ${actor.loose}. Current graph row: ${actor.last??'wait'} → ${arrows(actor).join(', ')}. ${arrows(actor).length===1?'One certain transition.':'Choose an outgoing arrow manually below.'}`;
    $('agent-detail').append(bar(actor),info);
  }
  const targets=world.agents.filter(a=>a.id!==selected&&actor&&nearby(a,actor));
  if(!targets.some(a=>a.id===target))target=targets[0]?.id;
  $('target').replaceChildren(...targets.map(a=>option(a.id,`${name(a)} · ${a.loose} loose`)));
  if(!targets.length)$('target').append(option('','No other local agents'));
  $('target').value=target??'';
  $('destination').replaceChildren();
  for(let y=0;y<H;y++)for(let x=0;x<W;x++)$('destination').append(option(`${x},${y}`,`(${x}, ${y}) · ${cellMaterial(world,x,y)}/${CELL_MATERIAL} material`));
  $('destination').value=`${destination.x},${destination.y}`;
  document.querySelectorAll('[data-op]').forEach(b=>b.disabled=!actor||(ACTIONS.includes(b.dataset.op)&&!arrows(actor).includes(b.dataset.op)));
  $('take').hidden=world.acquisition==='pull';
  $('guide').textContent=`${world.guide} / ${world.steps.length} worked steps`;
  $('next').disabled=world.guide>=world.steps.length;$('undo').disabled=!undo.length;
  const next=world.steps[world.guide];
  $('upcoming').textContent=next?`Next: #${next.actor} · ${next.op}${next.target?` from #${next.target}`:''}${next.x!==undefined?` toward (${next.x}, ${next.y})`:''}. Switching either hypothesis resets the same initial world.`:'Example complete. Try manual actions, Undo, or compare the other hypothesis.';
  $('message').className='';$('message').textContent=world.message;$('step-number').textContent=`Manual step ${world.step}`;
  $('balance').replaceChildren();$('changes').replaceChildren();
  if(world.before){
    const balance=document.createElement('div');balance.className='balance';
    balance.textContent=`Before: ${world.before.totals.join(' + ')} = ${world.initialMaterial}   →   After: ${parts(world).join(' + ')} = ${world.initialMaterial}   (structure + reserve + loose)`;
    $('balance').append(balance);
    const table=document.createElement('table');table.className='changes-table';
    const header=table.createTHead().insertRow();['Agent','Before S / R / L','After S / R / L','Position'].forEach(t=>{const th=document.createElement('th');th.textContent=t;header.append(th);});
    const body=table.createTBody(), ids=[...new Set([...world.before.agents,...world.agents].map(a=>a.id))];
    for(const id of ids){const old=world.before.agents.find(a=>a.id===id),a=world.agents.find(b=>b.id===id);
      if(JSON.stringify(old)===JSON.stringify(a))continue;
      const row=body.insertRow();const triple=b=>b?`${b.structure} / ${b.reserve} / ${b.loose}`:'—';
      [`#${id}`,triple(old),triple(a),a?`(${a.x}, ${a.y})`: 'Fully absorbed'].forEach(t=>row.insertCell().textContent=t);
    }
    $('changes').append(table);
  }
  $('history').replaceChildren(...world.history.map(h=>{const li=document.createElement('li');li.textContent=`${h.step}. #${h.actor} ${h.op}: ${h.message}`;return li;}));
}
for(const id of ['scenario','space','acquisition'])$(id).onchange=reset;
$('reset').onclick=reset;$('next').onclick=()=>run(null,true);
$('undo').onclick=()=>{world=undo.pop();render();};
$('actor').onchange=()=>{selected=Number($('actor').value);render();};
$('target').onchange=()=>{target=Number($('target').value);};
$('destination').onchange=()=>{const [x,y]=$('destination').value.split(',').map(Number);destination={x,y};render();};
document.querySelectorAll('[data-op]').forEach(b=>b.onclick=()=>run({actor:selected,op:b.dataset.op,target,...destination}));
reset();
