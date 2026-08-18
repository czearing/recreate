// A DOM double for the shared path definition, so the shipped rule runs under Node.
//
// The double models exactly what the definition is allowed to read — the parent element, the
// root of the tree, a root's children, and the host that opens a shadow tree — because those
// are the values that decide whether a shadow-resident element gets an address or a crash.
class ShadowRoot {
  constructor(host, mode) {
    this.host = host;
    this.mode = mode;
    this.children = [];
  }
}

const element = (tagName, parent) => {
  const node = { tagName, parentElement: null, children: [], shadowRoot: null, root: null };
  node.getRootNode = () => node.root;
  if (parent) {
    node.parentElement = parent;
    node.root = parent.root;
    parent.children.push(node);
  }
  return node;
};

const document = { documentElement: null };
document.documentElement = element('HTML');
document.documentElement.root = document;

/// Opens a shadow tree on `host` and returns its root, whose children have no parent element
/// — which is the platform's own shape and the case the light-DOM-only twin could not survive.
const attachShadow = (host, mode) => {
  const root = new ShadowRoot(host, mode);
  host.shadowRoot = root;
  return root;
};

const inShadow = (root, tagName) => {
  const node = element(tagName);
  node.root = root;
  root.children.push(node);
  return node;
};

__NODE_PATH__
