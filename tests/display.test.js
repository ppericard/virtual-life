import { test } from 'node:test';
import assert from 'node:assert/strict';
import { HISTORY_LIMIT, recordSample, sampleDescription, inspectionText, groupColor, groupLabel } from '../web/display.js';

test('stored selected action is distinct from success and unavailable state', () => {
  const agent={id:'1',group:0,weights:[0,1,0,0],integrity:7};
  const sample={width:3,tick:'1',experiment:{groups:[{automaton_name:'Custom automaton'}],maintenance:{maximum:10}},cells:[agent]};
  assert.match(inspectionText(sample,'1'), /Last selected action: Unavailable/);
  agent.last_action=null;
  assert.match(inspectionText(sample,'1'), /Last selected action: Not yet acted/);
  for(const action of ['Wait','Move','Copy','Repair']) {
    agent.last_action=action;
    assert.ok(inspectionText(sample,'1').includes(`Last selected action: ${action} (success not implied)`));
  }
});

test('inspection reports integrity and engine failure evidence without inventing a cause', () => {
  const sample={width:3,tick:'4',experiment:{groups:[{automaton_name:'Custom automaton'}],maintenance:{maximum:10,upkeep:1}},cells:[{id:'1',group:0,weights:[2,4,1,3],integrity:6,occupied_neighbors:5,next_tick_upkeep:'2',crowding_upkeep:1}],failure_history:{discarded:'0',records:[]}};
  assert.match(inspectionText(sample,'1'),/Custom automaton.*Integrity 6\/10/);
  assert.match(inspectionText(sample,'1'),/Group A.*Custom automaton/);
  assert.match(inspectionText(sample,'1'),/Current occupied neighbours 5\/8 · Next-tick effective upkeep 2 \(base 1 \+ crowding 1; displayed neighbourhood\)/);
  sample.cells=[null];
  assert.match(inspectionText(sample,'1'),/No failure record available/);
  sample.failure_history.records=[{id:'1',tick:'3',position:{x:1,y:2},reason:'move wear',integrity_before:2,occupied_neighbors:4,base_upkeep:1,crowding_upkeep:0,upkeep:'1',action_wear:1}];
  assert.match(inspectionText(sample,'1'),/failed at tick 3.*move wear.*starting integrity 2.*upkeep 1.*extra wear 1/);
  assert.match(inspectionText(sample,'1'),/occupied neighbours 4\/8, effective upkeep 1 \(base 1 \+ crowding 0\)/);
  sample.failure_history={discarded:'200',records:[]};
  assert.match(inspectionText(sample,'1'),/record has been discarded.*200/);
});

test('inspection and historic failure preserve effective upkeep above u32 maximum', () => {
  const sample={width:3,tick:'0',experiment:{groups:[{automaton_name:'Custom automaton'}],maintenance:{maximum:4294967295,upkeep:4294967295}},cells:[{id:'1',group:0,weights:[0,0,0,1],integrity:4294967295,occupied_neighbors:8,next_tick_upkeep:'8589934590',crowding_upkeep:4294967295}],failure_history:{discarded:'0',records:[]}};
  assert.match(inspectionText(sample,'1'),/Next-tick effective upkeep 8589934590 \(base 4294967295 \+ crowding 4294967295; displayed neighbourhood\)/);
  sample.cells=[null]; sample.tick='1';
  sample.failure_history.records=[{id:'1',tick:'1',position:{x:0,y:0},reason:'upkeep',integrity_before:4294967295,occupied_neighbors:8,base_upkeep:4294967295,crowding_upkeep:4294967295,upkeep:'8589934590',action_wear:0}];
  assert.match(inspectionText(sample,'1'),/occupied neighbours 8\/8, effective upkeep 8589934590 \(base 4294967295 \+ crowding 4294967295\), extra wear 0/);
});

test('history stays bounded, deduplicates ticks, and does not fabricate missed samples', () => {
  const points=[];
  for(let tick=0;tick<300;tick++) recordSample(points,{tick:String(tick*2),count:'3'});
  assert.equal(points.length,HISTORY_LIMIT);
  assert.equal(points[0].tick,'344');
  assert.equal(points.at(-1).tick,'598');
  recordSample(points,{tick:'598',count:'4'});
  assert.equal(points.length,HISTORY_LIMIT); assert.equal(points.at(-1).count,'4');
  recordSample(points,{tick:'0',count:'3'}); assert.equal(points.at(-1).tick,'598');
  assert.match(sampleDescription(points),/^Ticks 344–598 · 128 samples · 127 sampling gaps\./);
});

test('species history keeps extinct zeros, bounded exact samples and stable identifiers', () => {
  const points=[];
  for(let tick=0;tick<300;tick++) recordSample(points,{tick:String(tick),count:'4',experiment:{groups:[{count:'0'},{count:'4'},{count:'0'},{count:'0'}]}});
  assert.equal(points.length,HISTORY_LIMIT); assert.deepEqual(points.at(-1).groups,['0','4','0','0']);
  assert.match(sampleDescription(points),/Older samples are discarded/); assert.match(sampleDescription(points),/not a complete recording/);
  assert.equal(new Set(Array.from({length:8},(_,i)=>groupColor(i))).size,8);
  assert.deepEqual([0,1,2,3].map(groupLabel),['A','B','C','D']);
  const sample={width:8,tick:'7',experiment:null,cells:[{id:'9007199254740993',value:'42'}]};
  assert.equal(inspectionText(sample,'9007199254740993'),'ID 9007199254740993 · Value 42 · Position (0, 0)');
});
test('large adjacent ticks remain adjacent and signed values remain exact', () => {
  const points=[];
  for(const tick of ['9007199254740992','9007199254740993','9007199254740995']) recordSample(points,{tick,count:'2'});
  assert.match(sampleDescription(points),/1 sampling gap\./);
  for(const value of ['-9223372036854775808','9223372036854775807']) {
    assert.equal(inspectionText({width:3,tick:'18446744073709551615',cells:[{id:'18446744073709551614',value}]},'18446744073709551614'),`ID 18446744073709551614 · Value ${value} · Position (0, 0)`);
  }
});
