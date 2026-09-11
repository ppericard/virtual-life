import {test} from 'node:test';
import assert from 'node:assert/strict';
import {probabilityLabels} from '../web/automaton.js';

test('probability labels preserve exact zero, certain and tiny arrows with wide integer tickets',()=> {
  assert.deepEqual(probabilityLabels(['0','1','0','0']),['0%','100%','0%','0%']);
  assert.deepEqual(probabilityLabels(['0','3','1','6']),['0%','30%','10%','60%']);
  assert.deepEqual(probabilityLabels(['1','18014398509481983','0','0']),['<0.1%','99.9%','0%','0%']);
});
