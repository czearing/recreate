import React,{useLayoutEffect,useState,useRef} from 'react';
import {createPortal} from 'react-dom';
import {adoptInto} from './style.mjs';

// A shadow root is a tree scope, not an element, so it cannot be written as a tag. It is
// opened on the element that already stands above it in the tree, and its contents are
// portalled in, which leaves the browser to compute the flattened tree the page really had.
//
// The root is memoised per host because `attachShadow` throws when a host already has one,
// and because a closed root is not reachable through `host.shadowRoot` afterwards.
const opened = new WeakMap();

// A `<template>` is slottable, so an anchor with no slot would be projected into a default
// slot of the very tree it opens. Naming a slot the page never declared keeps it unassigned.
const ANCHOR = {slot: 'recreate-shadow-anchor', 'data-recreate-shadow': ''};

const rootFor = (host, mode) => {
  let root = opened.get(host);
  if (!root) {
    root = host.shadowRoot || host.attachShadow({mode});
    opened.set(host, root);
    adoptInto(root);
  }
  return root;
};

export function ShadowRoot({mode = 'open', children}) {
  const anchor = useRef(null);
  const [root, setRoot] = useState(null);
  useLayoutEffect(() => {
    const host = anchor.current && anchor.current.parentNode;
    if (host && host.nodeType === 1) setRoot(rootFor(host, mode));
  }, [mode]);
  return React.createElement(
    React.Fragment,
    null,
    React.createElement('template', {...ANCHOR, ref: anchor}),
    root ? createPortal(children, root) : null
  );
}
