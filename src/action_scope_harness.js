// A DOM double for the action scope, so the shipped scope recorder runs under Node.
//
// The double models only what the recorder reads: a node tree it can address, the two event
// hooks it installs, and a mutation observer it never has to fire. The page it stands for is
// one button below the fold, which is the shape that makes the harness scroll to reach it.
const Node = { TEXT_NODE: 3 };

class Element {
  constructor(tagName, parent) {
    this.tagName = tagName;
    this.nodeType = 1;
    this.parentElement = parent || null;
    this.children = [];
    this.scrollLeft = 0;
    this.scrollTop = 0;
    this.root = parent ? parent.root : null;
    if (parent) parent.children.push(this);
  }

  getRootNode() {
    return this.root;
  }

  querySelectorAll() {
    return this.children.flatMap(child => [child, ...child.querySelectorAll()]);
  }
}

class MutationObserver {
  constructor(callback) {
    this.callback = callback;
  }

  observe() {}

  disconnect() {}
}

const document = { documentElement: null };
document.documentElement = new Element('HTML');
document.documentElement.root = document;
document.scrollingElement = document.documentElement;
const body = new Element('BODY', document.documentElement);
const button = new Element('BUTTON', body);
const addEventListener = () => {};
const removeEventListener = () => {};
const TRIGGER = 'html>body:nth-of-type(1)>button:nth-of-type(1)';
