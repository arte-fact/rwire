// Minimal DOM mock for unit tests, faithful where the runtime depends on it:
// text lives as child text NODES (the morph reconciles them), `children` is
// elements-only while first/next-sibling walk all nodes, listeners are
// captured and dispatchable via fire().

export interface MockText {
  nodeType: 3;
  nodeValue: string;
  parentNode: MockEl | null;
  readonly nextSibling: MockNode | null;
}
export type MockNode = MockEl | MockText;

export interface MockEl {
  nodeType: 1;
  tagName: string;
  id: string;
  className: string;
  textContent: string;
  value: string;
  style: Record<string, string> & { cssText?: string };
  dataset: Record<string, string>;
  classList: {
    add(c: string): void;
    remove(c: string): void;
    contains(c: string): boolean;
    all(): string[];
  };
  __hk?: string;
  __t?: unknown;
  parentNode: MockEl | null;
  readonly childNodes: MockNode[];
  readonly children: MockEl[];
  readonly attributes: { name: string; value: string }[];
  readonly firstChild: MockNode | null;
  readonly nextSibling: MockNode | null;
  listeners: Record<string, ((e: unknown) => void)[]>;
  setAttribute(n: string, v: string): void;
  getAttribute(n: string): string | null;
  hasAttribute(n: string): boolean;
  removeAttribute(n: string): void;
  appendChild(c: MockNode): MockNode;
  insertBefore(c: MockNode, ref: MockNode | null): MockNode;
  removeChild(c: MockNode): MockNode;
  addEventListener(t: string, fn: (e: unknown) => void): void;
  fire(type: string, e?: Record<string, unknown>): void;
  focus(): void;
  setSelectionRange(a: number, b: number): void;
  closest(sel: string): MockEl | null;
  querySelector(sel: string): MockEl | null;
  querySelectorAll(sel: string): MockEl[];
  scrollIntoView(opts?: unknown): void;
  sheet?: { cssRules: string[]; insertRule(r: string, i: number): void };
}

export interface MockDoc {
  body: MockEl;
  head: MockEl;
  activeElement: MockEl | null;
  createElement(t: string): MockEl;
  createElementNS(ns: string, t: string): MockEl;
  createTextNode(v: string): MockText;
  getElementById(id: string): MockEl | null;
  addEventListener(): void;
  _byId: Map<string, MockEl>;
}

export function makeDom(): { document: MockDoc } {
  const byId = new Map<string, MockEl>();

  const siblingOf = (n: MockNode): MockNode | null => {
    if (!n.parentNode) return null;
    const ch = n.parentNode.childNodes;
    const i = ch.indexOf(n);
    return i >= 0 && i + 1 < ch.length ? ch[i + 1] : null;
  };

  function txt(v: string): MockText {
    const t = {
      nodeType: 3 as const,
      nodeValue: String(v),
      parentNode: null as MockEl | null,
      get nextSibling() {
        return siblingOf(t);
      },
    };
    return t;
  }

  function el(tag: string): MockEl {
    const attrs = new Map<string, string>();
    const classes = new Set<string>();
    const _nodes: MockNode[] = [];
    const node: MockEl = {
      nodeType: 1,
      tagName: String(tag || "div").toUpperCase(),
      get id() {
        return attrs.get("id") || "";
      },
      set id(v: string) {
        attrs.set("id", v);
        byId.set(v, node);
      },
      get className() {
        return attrs.get("class") || "";
      },
      set className(v: string) {
        attrs.set("class", v);
      },
      get textContent() {
        return _nodes
          .map((c) => (c.nodeType === 3 ? c.nodeValue : c.textContent))
          .join("");
      },
      set textContent(v: string) {
        for (const c of _nodes) c.parentNode = null;
        _nodes.length = 0;
        if (String(v) !== "") node.appendChild(txt(String(v)));
      },
      value: "",
      // Minimal innerHTML: materialize an id="…"-bearing child per occurrence
      // so the runtime's overlay markup is queryable. Not a real parser.
      get innerHTML() {
        return (this as any)._html || "";
      },
      set innerHTML(v: string) {
        (this as any)._html = v;
        for (const c of _nodes) c.parentNode = null;
        _nodes.length = 0;
        for (const m of String(v).matchAll(/id="([^"]+)"/g)) {
          const c = el("div");
          c.id = m[1];
          node.appendChild(c);
        }
      },
      style: {},
      dataset: {},
      classList: {
        add: (c) => classes.add(c),
        remove: (c) => classes.delete(c),
        contains: (c) => classes.has(c),
        all: () => [...classes],
      },
      __hk: undefined,
      __t: undefined,
      parentNode: null,
      get childNodes() {
        return _nodes;
      },
      get children() {
        return _nodes.filter((c): c is MockEl => c.nodeType === 1);
      },
      get attributes() {
        return [...attrs].map(([name, value]) => ({ name, value }));
      },
      get firstChild() {
        return _nodes[0] || null;
      },
      get nextSibling() {
        return siblingOf(node);
      },
      listeners: {},
      setAttribute(n, v) {
        attrs.set(n, String(v));
        if (n === "id") byId.set(String(v), node);
      },
      getAttribute: (n) => (attrs.has(n) ? attrs.get(n)! : null),
      hasAttribute: (n) => attrs.has(n),
      removeAttribute: (n) => void attrs.delete(n),
      appendChild(c) {
        if (c.parentNode) c.parentNode.removeChild(c);
        c.parentNode = node;
        _nodes.push(c);
        return c;
      },
      insertBefore(c, ref) {
        if (c.parentNode) c.parentNode.removeChild(c);
        c.parentNode = node;
        const idx = ref ? _nodes.indexOf(ref) : -1;
        if (idx < 0) _nodes.push(c);
        else _nodes.splice(idx, 0, c);
        return c;
      },
      removeChild(c) {
        const i = _nodes.indexOf(c);
        if (i >= 0) _nodes.splice(i, 1);
        c.parentNode = null;
        return c;
      },
      addEventListener(t, fn) {
        (node.listeners[t] || (node.listeners[t] = [])).push(fn);
      },
      fire(type, e = {}) {
        const ev = { type, preventDefault() {}, target: node, ...e };
        for (const fn of node.listeners[type] || []) fn(ev);
      },
      focus() {
        doc.activeElement = node;
      },
      setSelectionRange() {},
      closest: () => null,
      // #id selectors only — enough for the runtime's overlay lookups.
      querySelector(sel) {
        if (!sel.startsWith("#")) return null;
        const id = sel.slice(1);
        const walk = (e: MockEl): MockEl | null => {
          if (e.id === id) return e;
          for (const c of e.children) {
            const hit = walk(c);
            if (hit) return hit;
          }
          return null;
        };
        return walk(node);
      },
      querySelectorAll: () => [],
      scrollIntoView() {},
    };
    if (node.tagName === "STYLE") {
      const rules: string[] = [];
      node.sheet = {
        cssRules: rules,
        insertRule: (r: string, i: number) => rules.splice(i, 0, r),
      };
    }
    return node;
  }

  const doc: MockDoc = {
    body: el("body"),
    head: el("head"),
    activeElement: null,
    createElement: (t) => el(t),
    createElementNS: (_ns, t) => {
      const n = el(t);
      (n as any).isSvg = true;
      return n;
    },
    createTextNode: (v) => txt(v),
    getElementById: (id) => byId.get(id) || null,
    addEventListener() {},
    _byId: byId,
  };
  return { document: doc };
}
