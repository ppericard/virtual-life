// THROWAWAY: compare agent-only material transfers and shared cells.
// Manual steps, not an autonomous simulation or an accepted model protocol.
export const W = 4, H = 3, CELL_MATERIAL = 20;
export const ACTIONS = ['wait', 'move', 'copy', 'repair', 'take'];
export const graph = (waitOnly = false) => ACTIONS.map(() => ACTIONS.map(action => Number(!waitOnly || action === 'wait')));
export const row = a => a.graph[ACTIONS.indexOf(a.last ?? 'wait')];
const totalOf = a => a.structure + a.reserve + a.loose;
export const material = world => world.agents.reduce((n, a) => n + totalOf(a), 0);
export const cellMaterial = (world, x, y) => world.agents.filter(a => a.x === x && a.y === y).reduce((n, a) => n + totalOf(a), 0);
export const parts = world => ['structure', 'reserve', 'loose'].map(key => world.agents.reduce((n, a) => n + a[key], 0));
export function nearby(a, b) {
  const dx = Math.abs(a.x - b.x), dy = Math.abs(a.y - b.y);
  return Math.min(dx, W - dx) <= 1 && Math.min(dy, H - dy) <= 1;
}
const sameCell = (a, b) => a.x === b.x && a.y === b.y;
const agent = (id, x, y, structure, reserve, loose = 0, transitions = graph()) => ({id, x, y, structure, reserve, loose, graph:structuredClone(transitions), last: null});

export function initial(scenario = 'repair', space = 'shared', acquisition = 'take') {
  let agents, steps;
  if (scenario === 'repair') {
    agents = [agent(1, 1, 1, 6, 1), agent(2, 2, 1, 0, 0, 8, graph(true))];
    steps = [
      {actor:1, op:'move', x:2, y:1},
      ...(acquisition === 'take' ? [{actor:1, op:'take', target:2}] : []),
      {actor:1, op:'repair', target:2},
      {actor:1, op:'wear'}, {actor:1, op:'release'},
      {actor:1, op:'repair', target:2},
    ];
  } else if (scenario === 'copy') {
    agents = [agent(1, 1, 1, 8, 9), agent(2, 2, 1, 0, 0, 5, graph(true))];
    steps = [{actor:1, op:'copy', x:2, y:1}, {actor:1, op:'release'}, {actor:1, op:'copy', x:1, y:1}];
  } else if (scenario === 'failure') {
    agents = [agent(1, 1, 1, 2, 1), agent(2, 2, 1, 4, 0)];
    steps = [{actor:1, op:'wear'}, {actor:1, op:'wear'},
      ...(acquisition === 'take' ? [{actor:2, op:'take', target:1}] : []),
      {actor:2, op:'repair', target:1}, {actor:1, op:'repair'}];
  } else {
    agents = [agent(1, 1, 1, 6, 1), agent(2, 2, 1, 0, 0, 16, graph(true))];
    steps = [{actor:1, op:'move', x:2, y:1}, {actor:1, op:acquisition === 'take' ? 'take' : 'repair', target:2}, {actor:1, op:'release'}];
  }
  const world = {scenario, space, acquisition, agents, steps, guide:0, step:0, nextId:3, message:'Choose Next worked step, or select agents and act manually.', history:[], before:null};
  world.initialMaterial = material(world);
  return world;
}

function wear(a, amount, messages) {
  const moved = Math.min(a.structure, amount);
  a.structure -= moved; a.loose += moved;
  messages.push(`#${a.id}: ${moved} structure → loose material, retained by the same agent.`);
}

function take(world, a, target, requested, messages) {
  if (!target || target.id === a.id || !nearby(a, target)) {
    messages.push('Choose another agent in this cell or one of the eight neighbouring cells.'); return 0;
  }
  const free = sameCell(a, target) ? requested : Math.max(0, CELL_MATERIAL - cellMaterial(world, a.x, a.y));
  const amount = Math.min(requested, target.loose, free);
  target.loose -= amount; a.reserve += amount;
  messages.push(`#${target.id} → #${a.id}: ${amount} loose material transferred into reserve.`);
  if (!amount) messages.push(target.loose ? 'No material capacity in the recipient cell.' : 'The target has no loose material available.');
  return amount;
}

function room(world, a, destination, amount, newAgent) {
  if (!nearby(a, destination)) return 'The destination must be this cell or a neighbouring cell.';
  if (world.space === 'single' && world.agents.some(b => b.x === destination.x && b.y === destination.y && (newAgent || b.id !== a.id))) {
    return 'Blocked: this setting permits one agent per cell.';
  }
  // A split in place redistributes the material already occupying that cell.
  if (!sameCell(a, destination) && cellMaterial(world, destination.x, destination.y) + amount > CELL_MATERIAL) {
    return `Blocked: the destination would exceed ${CELL_MATERIAL} material units.`;
  }
  return null;
}

export function apply(world, command) {
  const next = structuredClone(world);
  const messages = [], a = next.agents.find(b => b.id === command.actor);
  next.before = {agents:structuredClone(world.agents), totals:parts(world), command:structuredClone(command)};
  next.step++;
  if (!a) messages.push('That agent no longer holds any material. Select another agent.');
  else if (ACTIONS.includes(command.op) && (!row(a)[ACTIONS.indexOf(command.op)] || (command.op === 'take' && next.acquisition !== 'take'))) messages.push(`The current outgoing arrows do not offer ${command.op}. Material properties do not select a different agent class.`);
  else {
    const target = next.agents.find(b => b.id === command.target);
    if (command.op === 'wear') wear(a, 1, messages);
    else if (command.op === 'take') take(next, a, target, 3, messages);
    else if (command.op === 'repair') {
      const need = Math.min(4, 10 - a.structure);
      if (next.acquisition === 'pull' && need > a.reserve) take(next, a, target, Math.min(3, need - a.reserve), messages);
      const restored = Math.min(need, a.reserve);
      a.reserve -= restored; a.structure += restored;
      messages.push(`#${a.id}: ${restored} reserve → structure. Only actual restoration is charged.`);
      if (!restored) messages.push(need ? 'Repair has no available reserve.' : 'Structure is already full.');
    } else if (command.op === 'move') {
      const dest = {x:command.x, y:command.y};
      if (sameCell(a, dest)) messages.push('Choose a different destination cell.');
      else {
        wear(a, 1, messages);
        const blocked = room(next, a, dest, totalOf(a), false);
        if (!a.structure) messages.push('No movement: this attempt leaves no structure to move with. Material and graph remain here.');
        else if (blocked) messages.push(blocked);
        else { a.x = dest.x; a.y = dest.y; messages.push(`#${a.id} moved to (${a.x}, ${a.y}), carrying all its material.`); }
      }
    } else if (command.op === 'copy') {
      wear(a, 2, messages);
      const dest = {x:command.x, y:command.y};
      const blocked = room(next, a, dest, 5, true);
      if (!a.structure) messages.push('No child: this attempt leaves no structure to build with. Material and graph remain here.');
      else if (a.reserve < 5) messages.push('No child: construction needs 5 reserve (3 structure + 2 reserve).');
      else if (blocked) messages.push(blocked);
      else {
        a.reserve -= 5;
        const child = agent(next.nextId++, dest.x, dest.y, 3, 2, 0, a.graph);
        next.agents.push(child);
        messages.push(`Child #${child.id} receives 3 structure + 2 reserve, entirely funded by #${a.id}.`);
      }
    } else if (command.op === 'release') {
      // Splitting off held loose material is a candidate generic transformation,
      // manually invoked for inspection, not an automatic recycling service.
      const dest = {x:command.x ?? a.x, y:command.y ?? a.y};
      const amount = Math.min(3, a.loose), blocked = room(next, a, dest, amount, true);
      if (!amount) messages.push('No loose material to separate.');
      else if (blocked) messages.push(`${blocked} Loose material stays with #${a.id}; it is never discarded.`);
      else {
        a.loose -= amount;
        const fragment = agent(next.nextId++, dest.x, dest.y, 0, 0, amount, a.graph);
        next.agents.push(fragment);
        messages.push(`#${a.id} separates ${amount} loose material into agent #${fragment.id}.`);
      }
    } else messages.push(`#${a.id} waits. Wear is a separate manual step in this laboratory.`);
    if (['wait','move','copy','repair','take'].includes(command.op)) a.last = command.op;
  }
  const empty = next.agents.filter(b => totalOf(b) === 0);
  next.agents = next.agents.filter(b => totalOf(b) > 0);
  for (const b of empty) messages.push(`#${b.id} was fully absorbed: no material is erased when the empty record is removed.`);
  const valid = next.agents.every(b => ['structure','reserve','loose'].every(k => Number.isInteger(b[k]) && b[k] >= 0));
  const capacityOK = next.agents.every(b => cellMaterial(next, b.x, b.y) <= CELL_MATERIAL);
  if (!valid || !capacityOK || material(next) !== world.initialMaterial) throw new Error('Prototype invariant failed: material or capacity changed unexpectedly.');
  next.message = messages.join(' ');
  next.history.unshift({step:next.step, op:command.op, actor:command.actor, message:next.message, totals:parts(next)});
  next.history = next.history.slice(0, 15);
  return next;
}

export function guided(world) {
  if (world.guide >= world.steps.length) return world;
  const next = apply(world, world.steps[world.guide]);
  next.guide = world.guide + 1;
  return next;
}
