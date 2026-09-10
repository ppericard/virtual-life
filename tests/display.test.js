import { test } from 'node:test';
import assert from 'node:assert/strict';
import { HISTORY_LIMIT, recordSample, sampleDescription, inspectionText, groupColor, groupLabel } from '../web/display.js';

test('inspection reports integrity and engine failure evidence without inventing a cause', () => {
  const sample={width:3,tick:'4',experiment:{maintenance:{maximum:10}},cells:[{id:'1',group:0,weights:[2,4,1,3],integrity:6}],failure_history:{discarded:'0',records:[]}};
  assert.match(inspectionText(sample,'1'),/copy, repair.*Integrity 6\/10/);
  sample.cells=[null];
  assert.match(inspectionText(sample,'1'),/No failure record available/);
  sample.failure_history.records=[{id:'1',tick:'3',position:{x:1,y:2},reason:'move wear',integrity_before:2,upkeep:1,action_wear:1}];
  assert.match(inspectionText(sample,'1'),/failed at tick 3.*move wear.*starting integrity 2.*upkeep 1.*extra wear 1/);
  sample.failure_history={discarded:'200',records:[]};
  assert.match(inspectionText(sample,'1'),/record has been discarded.*200/);
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
  const sample={width:8,tick:'7',experiment:{},cells:[{id:'9007199254740993',group:2,weights:[6,2,1,1]}]};
  assert.equal(inspectionText(sample,'9007199254740993'),'ID 9007199254740993 · Group C · Weights (wait, move, copy, remove): 6, 2, 1, 1 · Position (0, 0)');
});
test('large adjacent ticks remain adjacent and signed values remain exact', () => {
  const points=[];
  for(const tick of ['9007199254740992','9007199254740993','9007199254740995']) recordSample(points,{tick,count:'2'});
  assert.match(sampleDescription(points),/1 sampling gap\./);
  for(const value of ['-9223372036854775808','9223372036854775807']) {
    assert.equal(inspectionText({width:3,tick:'18446744073709551615',cells:[{id:'18446744073709551614',value}]},'18446744073709551614'),`ID 18446744073709551614 · Value ${value} · Position (0, 0)`);
  }
});
