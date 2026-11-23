// reset_on_ack (R3): a composer field marked data-reset-on-ack is emptied by
// the runtime at the EVENT_ACK of a successful submit — and keeps its text
// when the submit's action came back errored ([data-action-error]). Plus the
// [data-enter-submit] keyboard contract: Enter submits, Shift+Enter and IME
// composition do not.
import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { x } from "../src/executor.ts";
import { st, resetSession } from "../src/state.ts";
import { bind, resetDelegation } from "../src/delegate.ts";
import { resetActions } from "../src/actions.ts";
import { resetLive } from "../src/live.ts";
import { installRouter } from "../src/router.ts";
import { makeDom, type MockEl } from "./dom.ts";

let doc: ReturnType<typeof makeDom>["document"];
let sent: Uint8Array[];

beforeEach(() => {
  doc = makeDom().document;
  (globalThis as any).document = doc;
  (globalThis as any).history = { pushState() {}, replaceState() {} };
  (globalThis as any).window = { addEventListener() {}, removeEventListener() {} };
  resetDelegation();
  resetSession();
  resetActions();
  resetLive();
  sent = [];
  st.w = { send: (m: Uint8Array) => sent.push(m), close() {}, readyState: 1 } as any;
  // The composer's forms submit with FormData(form) in gp(); read named fields.
  (globalThis as any).FormData = class {
    fields: [string, string][] = [];
    constructor(form: any) {
      const walk = (n: any) => {
        const name = n.getAttribute?.("name");
        if (name) this.fields.push([name, n.value || ""]);
        for (const c of n.children || []) walk(c);
      };
      walk(form);
    }
    forEach(cb: (v: string, k: string) => void) {
      for (const [k, v] of this.fields) cb(v, k);
    }
  };
});

function composerForm(): { form: MockEl; field: MockEl } {
  const form = doc.createElement("form") as MockEl;
  const field = doc.createElement("textarea") as MockEl;
  field.setAttribute("name", "message");
  field.setAttribute("data-enter-submit", "1");
  field.setAttribute("data-reset-on-ack", "1");
  field.value = "hello hub";
  form.appendChild(field);
  doc.body.appendChild(form);
  // A submit binding on the form, like BIND_REMOTE produces.
  bind(form as any, "submit", { h: 7, t: 6, f: 0 });
  return { form, field };
}

const ACK = new Uint8Array([0xf3, 0xff]); // EVENT_ACK, BATCH_END

test("EVENT_ACK of a successful submit empties the data-reset-on-ack field", () => {
  const { form, field } = composerForm();
  form.fire("submit");
  assert.equal(sent.length, 1, "the submit went out");
  assert.deepEqual(JSON.parse(new TextDecoder().decode(sent[0].slice(5)) as string).v, {
    message: "hello hub",
  });
  assert.equal(field.value, "hello hub", "not cleared before the ack");
  x(ACK);
  assert.equal(field.value, "", "cleared at the ack of a successful send");
});

test("an errored send keeps the field's text", () => {
  const { form, field } = composerForm();
  form.fire("submit");
  // The reflected control comes back in its errored phase: data-action-error.
  const err = doc.createElement("button") as MockEl;
  err.setAttribute("data-action-error", "hub refused");
  form.appendChild(err);
  x(ACK);
  assert.equal(field.value, "hello hub", "never cleared after an error");
});

test("an ack with no pending reset changes nothing; a detached form keeps its text", () => {
  const { form, field } = composerForm();
  form.fire("submit");
  form.removeChild; // no-op, keep the reference alive
  doc.body.removeChild(form); // detached before the ack arrives
  x(ACK);
  assert.equal(field.value, "hello hub", "a detached form is not cleared");

  const other = doc.createElement("textarea") as MockEl;
  other.value = "untouched";
  doc.body.appendChild(other);
  x(ACK);
  assert.equal(other.value, "untouched");
});

test("Enter submits a [data-enter-submit] field; Shift+Enter and IME do not", () => {
  installRouter();
  const { form, field } = composerForm();
  let submits = 0;
  (form as any).requestSubmit = () => {
    submits++;
    form.fire("submit"); // the browser fires a submit event on requestSubmit
  };
  field.fire("keydown", { key: "Enter", shiftKey: false, isComposing: false });
  assert.equal(submits, 1, "Enter submits");
  assert.equal(sent.length, 1, "the submit went out");
  field.fire("keydown", { key: "Enter", shiftKey: true, isComposing: false });
  assert.equal(submits, 1, "Shift+Enter inserts a newline, does not submit");
  assert.equal(sent.length, 1);
  field.fire("keydown", { key: "Enter", shiftKey: false, isComposing: true });
  assert.equal(submits, 1, "IME composition does not submit");
  assert.equal(sent.length, 1);
  // A field without data-enter-submit is left alone.
  const plain = doc.createElement("textarea") as MockEl;
  doc.body.appendChild(plain);
  plain.fire("keydown", { key: "Enter", shiftKey: false, isComposing: false });
  assert.equal(submits, 1);
});
