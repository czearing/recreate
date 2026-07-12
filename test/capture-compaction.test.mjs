import assert from 'node:assert/strict';
import test from 'node:test';

import { compactCapture } from '../src/capture-compaction.mjs';

test('removes encoded assets and downsamples exact evidence', () => {
  const capture = {
    document: {
      url: 'https://example.test/',
      title: 'Fixture',
      lang: 'en',
      viewport: { width: 100, height: 100, dpr: 1 },
      scroll: { width: 100, height: 100 },
      bodyStyle: { display: 'block' },
    },
    initialDocument: { file: 'documents/100x100.html' },
    nodes: [{
      path: 'doc(0)>html:nth-of-type(1)>body:nth-of-type(1)>img:nth-of-type(1)',
      parentPath: 'doc(0)>html:nth-of-type(1)>body:nth-of-type(1)',
      nodeType: 1,
      tag: 'img',
      attrs: { src: 'data:image/png;base64,encoded', alt: 'Preview' },
      visible: true,
      rect: { x: 0, y: 0, width: 10, height: 10, right: 10, bottom: 10 },
    }],
    behaviors: [],
    globalListeners: [],
    resources: [
      {
        url: 'data:image/svg+xml;base64,encoded',
        initiatorType: 'css',
      },
      {
        url: 'https://example.test/api/items?account=private&limit=10',
        initiatorType: 'fetch',
        startTime: 10.123,
        duration: 20.456,
        responseEnd: 30.579,
      },
    ],
    fonts: [],
    animations: [],
    animationElements: [],
    smoothScroll: null,
    horizontalTracks: [],
    exactAssets: [{
      type: 'canvas',
      path: 'canvas',
      dataUrl: 'data:image/png;base64,encoded',
      value: '<svg>encoded</svg>',
    }],
    scrollStates: [],
    lifecycleAnimation: {
      durationMs: 100,
      frameCount: 10,
      animationDefinitions: [{}],
      tracks: [{
        path: 'body',
        tag: 'body',
        samples: Array.from({ length: 10 }, (_, index) => ({
          time: index * 10,
          rect: { x: index, y: 0, width: 10, height: 10 },
          style: { opacity: String(index / 10) },
        })),
      }],
    },
    readiness: { ready: true },
    timings: {},
    cssomBlockedStylesheetCount: 0,
  };

  const compact = compactCapture(capture);
  const serialized = JSON.stringify(compact);

  assert.doesNotMatch(serialized, /base64|dataUrl|<svg>/);
  assert.equal(
    compact.nodes[0].attrs.src,
    '[asset stored in captured state HTML]',
  );
  assert.equal(compact.lifecycleAnimation.tracks[0].samples.length, 5);
  assert.deepEqual(compact.resources, [{
    url: 'https://example.test/api/items?account=%3Avalue&limit=%3Avalue',
    initiatorType: 'fetch',
    startTime: 10.12,
    duration: 20.46,
    fetchStart: undefined,
    responseStart: undefined,
    responseEnd: 30.58,
    transferSize: undefined,
    decodedBodySize: undefined,
    nextHopProtocol: undefined,
  }]);
});
