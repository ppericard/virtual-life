import { test } from 'node:test';
import assert from 'node:assert/strict';
import { HISTORY_LIMIT, recordSample, sampleDescription, inspectionText } from '../web/display.js';

test('history stays bounded, deduplicates ticks, and does not fabricate missed samples', () => {
  const points=[];
  for(let tick=0;tick<300;tick++) recordSample(points,{tick:String(tick*2),count:'3'});
  assert.equal(points.length,HISTORY_LIMIT);
  assert.equal(points[0].tick,'344');
  assert.equal(points.at(-1).tick,'598');
  recordSample(points,{tick:'598',count:'4'});
  assert.equal(points.length,HISTORY_LIMIT); assert.equal(points.at(-1).count,'4');
  recordSample(points,{tick:'0',count:'3'}); assert.equal(points.at(-1).tick,'598');
  assert.match(sampleDescription(points),/127 sampling gaps/);
});
test('large adjacent ticks remain adjacent and signed values remain exact', () => {
  const points=[];
  for(const tick of ['9007199254740992','9007199254740993','9007199254740995']) recordSample(points,{tick,count:'2'});
  assert.match(sampleDescription(points),/1 sampling gap\./);
  for(const value of ['-9223372036854775808','9223372036854775807']) {
    assert.equal(inspectionText({width:3,tick:'18446744073709551615',cells:[{id:'18446744073709551614',value}]},'18446744073709551614'),`ID 18446744073709551614 · Value ${value} · Position (0, 0)`);
  }
});
