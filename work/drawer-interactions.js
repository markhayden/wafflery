window.createWafflehouseDrawer = ({ app }) => {
  const dialog = document.createElement('dialog'); dialog.className = 'wh-drawer';
  dialog.setAttribute('aria-labelledby', 'wh-drawer-title'); dialog.setAttribute('aria-describedby', 'wh-drawer-context');
  const header = document.createElement('header'); header.className = 'wh-drawer-header';
  const row = document.createElement('div'); row.className = 'wh-row';
  const title = document.createElement('h2'); title.id = 'wh-drawer-title';
  const closeButton = document.createElement('button'); closeButton.type = 'button'; closeButton.textContent = 'Close'; closeButton.className = 'wh-close-drawer'; closeButton.setAttribute('aria-label', 'Close panel');
  const context = document.createElement('p'); context.id = 'wh-drawer-context'; context.className = 'wh-drawer-context';
  const content = document.createElement('div'); content.className = 'wh-drawer-content';
  row.append(title, closeButton); header.append(row, context); dialog.append(header, content); app.append(dialog);
  let mounted = null, returnFocus = null, previousOverflow = '';
  function restoreContent() {
    if (!mounted) return;
    mounted.node.hidden = true;
    mounted.parent.insertBefore(mounted.node, mounted.next?.parentNode === mounted.parent ? mounted.next : null);
    mounted = null;
  }
  function finishClose() {
    if (!mounted) return;
    restoreContent(); document.documentElement.style.overflow = previousOverflow;
    if (returnFocus?.isConnected && !returnFocus.disabled && !returnFocus.closest('[hidden]')) returnFocus.focus();
    returnFocus = null;
  }
  function close() { if (dialog.open) dialog.close(); finishClose(); }
  function open({ heading, description, node, trigger }) {
    const alreadyOpen = dialog.open;
    restoreContent();
    mounted = { node, parent: node.parentNode, next: node.nextSibling };
    title.textContent = heading; context.textContent = description; context.hidden = !description;
    node.hidden = false; content.replaceChildren(node); content.scrollTop = 0;
    if (!alreadyOpen) { returnFocus = trigger || document.activeElement; previousOverflow = document.documentElement.style.overflow; document.documentElement.style.overflow = 'hidden'; dialog.showModal(); }
    closeButton.focus();
  }
  closeButton.addEventListener('click', close);
  dialog.addEventListener('cancel', event => { event.preventDefault(); close(); });
  dialog.addEventListener('close', () => { if (!dialog.open) finishClose(); });
  dialog.addEventListener('click', event => {
    if (event.target !== dialog) return;
    const rect = dialog.getBoundingClientRect();
    if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) close();
  });
  return { open, close, setContext: text => { context.textContent = text; context.hidden = !text; } };
};
