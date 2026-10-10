// A tiny DOM helper: h('div.card', {onclick}, children).

type Attrs = Record<string, unknown> & { style?: Partial<CSSStyleDeclaration> | string };
type Child = Node | string | number | null | undefined | false | Child[];

export function h<K extends keyof HTMLElementTagNameMap>(sel: K | string, attrs?: Attrs | Child, ...children: Child[]): HTMLElement {
  const [tag, ...classes] = sel.split('.');
  const el = document.createElement(tag || 'div');
  if (classes.length) el.className = classes.join(' ');
  if (attrs && (typeof attrs !== 'object' || attrs instanceof Node || Array.isArray(attrs))) {
    children.unshift(attrs as Child);
  } else if (attrs) {
    for (const [k, v] of Object.entries(attrs)) {
      if (v === undefined || v === null || v === false) continue;
      if (k === 'style' && typeof v === 'object') {
        for (const [sk, sv] of Object.entries(v as Record<string, string>)) {
          if (sk.startsWith('--')) el.style.setProperty(sk, sv);
          else (el.style as unknown as Record<string, string>)[sk] = sv;
        }
      }
      else if (k.startsWith('on') && typeof v === 'function') el.addEventListener(k.slice(2), v as EventListener);
      else if (k === 'html') el.innerHTML = String(v);
      else if (k === 'class') el.className += ' ' + String(v);
      else if (k in el && typeof v !== 'string') (el as unknown as Record<string, unknown>)[k] = v;
      else el.setAttribute(k, v === true ? '' : String(v));
    }
  }
  append(el, children);
  return el;
}

function append(el: HTMLElement, children: Child[]) {
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    if (Array.isArray(c)) append(el, c);
    else if (c instanceof Node) el.appendChild(c);
    else el.appendChild(document.createTextNode(String(c)));
  }
}

export function svg(markup: string, cls = ''): HTMLElement {
  const d = document.createElement('div');
  d.className = `svg-wrap ${cls}`;
  d.innerHTML = markup;
  return d;
}

export function clear(el: HTMLElement) {
  while (el.firstChild) el.removeChild(el.firstChild);
}

export function modal(content: HTMLElement, onClose?: () => void): () => void {
  const back = h('div.modal-back');
  const box = h('div.modal', content);
  const close = () => {
    back.remove();
    onClose?.();
  };
  const x = h('button.modal-x', { onclick: close, title: 'Close' }, '×');
  box.prepend(x);
  back.appendChild(box);
  back.addEventListener('pointerdown', (e) => {
    if (e.target === back) close();
  });
  document.body.appendChild(back);
  return close;
}

export function toast(text: string, ms = 2600) {
  const t = h('div.toast', text);
  document.body.appendChild(t);
  setTimeout(() => t.classList.add('out'), ms);
  setTimeout(() => t.remove(), ms + 500);
}
