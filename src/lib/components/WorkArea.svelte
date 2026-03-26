<script lang="ts">
  import { fly } from "svelte/transition";
  import { tick, onMount, onDestroy } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { listen } from "@tauri-apps/api/event";
  import { spaces, activeSpaceId, pagesMap, activePageId } from "../stores/spaces";
  import { t } from "../stores/language";
  import SearchOverlay from "./SearchOverlay.svelte";
  import DeleteConfirmModal from "./DeleteConfirmModal.svelte";
  import { getPages, createPage, deletePage, updatePageTitle, reorderPages, exportPage, importPage, getBlocks } from "../api";
  import PageCanvas from "./canvas/PageCanvas.svelte";


  $: activeSpace = $spaces.find(s => s.id === $activeSpaceId) ?? null;
  $: spacePages  = $pagesMap[$activeSpaceId ?? ""] ?? [];
  $: activePage  = spacePages.find(p => p.id === $activePageId) ?? null;

  let editingPageId: string | null = null;
  let editingTitle = "";

  // ── Breadcrumb inline rename ──────────────────────────
  let breadEditing = false;
  let breadEditTitle = "";

  function startBreadEdit() {
    if (!activePage) return;
    breadEditTitle = activePage.title;
    breadEditing = true;
  }

  async function commitBreadEdit() {
    breadEditing = false;
    if (!activePage || !$activeSpaceId || !breadEditTitle.trim()) return;
    const newTitle = breadEditTitle.trim();
    await updatePageTitle(activePage.id, newTitle);
    pagesMap.update(m => ({
      ...m,
      [$activeSpaceId!]: (m[$activeSpaceId!] ?? []).map(p =>
        p.id === activePage!.id ? { ...p, title: newTitle } : p
      )
    }));
  }

  // ── Toast notification ────────────────────────────────
  let toastMsg = "";
  let toastType: "success" | "error" = "success";
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  function showToast(msg: string, type: "success" | "error" = "success") {
    toastMsg = msg;
    toastType = type;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => { toastMsg = ""; }, 4500);
  }

  // ── Tab scroll ────────────────────────────────────────
  let tabsEl: HTMLDivElement;
  function scrollTabs(dir: number) {
    tabsEl?.scrollBy({ left: dir * 140, behavior: "smooth" });
  }

  // ── Tab drag-scroll ───────────────────────────────────
  // scrollDragMoved: solo para el sistema de scroll del contenedor.
  // tabDragMoved: solo para detach/reorder de tabs individuales.
  // Separados para evitar que un scroll accidental bloquee clics en tabs.
  let tabDragging = false;
  let tabDragStartX = 0;
  let tabDragScrollLeft = 0;
  // tabDragMoved: solo para el sistema de detach/reorder de tabs individuales.
  // El drag-scroll del contenedor NO lo toca para evitar bloquear clics en tabs.
  let tabDragMoved = false;

  function onTabsMouseDown(e: MouseEvent) {
    tabDragging = true;
    tabDragStartX = e.clientX;
    tabDragScrollLeft = tabsEl?.scrollLeft ?? 0;
    window.addEventListener("mousemove", onTabsMouseMove);
    window.addEventListener("mouseup", onTabsMouseUp);
  }

  function onTabsMouseMove(e: MouseEvent) {
    if (!tabDragging) return;
    const dx = e.clientX - tabDragStartX;
    if (tabsEl) tabsEl.scrollLeft = tabDragScrollLeft - dx;
  }

  function onTabsMouseUp() {
    tabDragging = false;
    window.removeEventListener("mousemove", onTabsMouseMove);
    window.removeEventListener("mouseup", onTabsMouseUp);
  }

  // ── Load pages ────────────────────────────────────────
  // Recuerda la última página visitada por espacio para restaurarla al volver
  let lastPagePerSpace: Record<string, string> = {};
  let _prevSpaceId: string | null = null;

  $: if ($activeSpaceId) loadPages($activeSpaceId);

  async function loadPages(spaceId: string) {
    // Guardar la página actual del espacio anterior antes de resetear
    if (_prevSpaceId && $activePageId) {
      lastPagePerSpace[_prevSpaceId] = $activePageId;
    }
    _prevSpaceId = spaceId;

    // Resetear a null para que PageCanvas se remonte limpio (evita race condition
    // donde un fetch async tardío sobrescribe activePageId con otra página)
    activePageId.set(null);
    await tick();

    // Si las páginas ya están cacheadas, no hace falta ir al backend
    if ($pagesMap[spaceId]) {
      const pages = $pagesMap[spaceId];
      if (pages.length > 0) {
        const savedId = lastPagePerSpace[spaceId];
        const target = savedId ? pages.find(p => p.id === savedId) : null;
        activePageId.set(target?.id ?? pages[0].id);
      }
      return;
    }

    const pages = await getPages(spaceId);
    // Verificar que el usuario no cambió de espacio mientras esperábamos
    if ($activeSpaceId !== spaceId) return;
    pagesMap.update(m => ({ ...m, [spaceId]: pages }));
    if (pages.length > 0) {
      const savedId = lastPagePerSpace[spaceId];
      const target = savedId ? pages.find(p => p.id === savedId) : null;
      activePageId.set(target?.id ?? pages[0].id);
    } else {
      const page = await createPage(spaceId, `${$t.workArea.page} 1`);
      if ($activeSpaceId !== spaceId) return;
      pagesMap.update(m => ({ ...m, [spaceId]: [page] }));
      activePageId.set(page.id);
    }
  }

  // ── Add page ──────────────────────────────────────────
  async function addPage(afterIndex?: number) {
    if (!$activeSpaceId) return;
    const count = spacePages.length + 1;
    const page = await createPage($activeSpaceId, `${$t.workArea.page} ${count}`);
    pagesMap.update(m => {
      const pages = [...(m[$activeSpaceId!] ?? []), page];
      if (afterIndex !== undefined) {
        // Mover la nueva página a la posición correcta (justo después de afterIndex)
        const insertAt = afterIndex + 1;
        pages.splice(insertAt, 0, pages.splice(pages.length - 1, 1)[0]);
        reorderPages(pages.map(p => p.id)).catch(console.error);
      }
      return { ...m, [$activeSpaceId!]: pages };
    });
    activePageId.set(page.id);
    setTimeout(() => {
      const newTab = tabsEl?.querySelector(`[data-page-id="${page.id}"]`) as HTMLElement | null;
      newTab?.scrollIntoView({ behavior: "smooth", inline: "nearest", block: "nearest" });
    }, 50);
  }

  // ── Delete page with 5-second confirmation ────────────
  let deleteConfirmId: string | null = null;
  let deleteCountdown = 0;
  let deleteInterval: ReturnType<typeof setInterval> | null = null;
  let deleteShake = false;

  function requestDeletePage(pageId: string) {
    if (deleteConfirmId === pageId && deleteTargetPage) {
      // Modal ya visible para esta página: shake para indicar que debe esperar
      deleteShake = false;
      requestAnimationFrame(() => { deleteShake = true; });
      return;
    }
    if (deleteInterval) { clearInterval(deleteInterval); deleteInterval = null; }
    deleteConfirmId = pageId;
    deleteCountdown = 5;
    deleteInterval = setInterval(() => {
      deleteCountdown--;
      if (deleteCountdown <= 0) {
        clearInterval(deleteInterval!);
        deleteInterval = null;
      }
    }, 1000);
  }

  function cancelDeletePage() {
    if (deleteInterval) { clearInterval(deleteInterval); deleteInterval = null; }
    deleteConfirmId = null;
    deleteCountdown = 0;
    deleteShake = false;
  }

  async function confirmDeletePage() {
    if (!deleteConfirmId || deleteCountdown > 0 || !$activeSpaceId) return;
    const pageId = deleteConfirmId;
    const spaceId = $activeSpaceId;
    cancelDeletePage();
    await deletePage(pageId);
    let remaining = spacePages.filter(p => p.id !== pageId);
    if (remaining.length === 0) {
      const newPage = await createPage(spaceId, `${$t.workArea.page} 1`);
      remaining = [newPage];
    }
    pagesMap.update(m => ({ ...m, [spaceId]: remaining }));
    if ($activePageId === pageId) activePageId.set(remaining[0].id);
  }

  // ── Rename page (dblclick) ────────────────────────────
  function startEdit(pageId: string, title: string) {
    editingPageId = pageId; editingTitle = title;
  }

  async function commitEdit() {
    if (!editingPageId || !$activeSpaceId) { editingPageId = null; return; }
    await updatePageTitle(editingPageId, editingTitle.trim() || $t.workArea.page);
    pagesMap.update(m => ({
      ...m,
      [$activeSpaceId!]: (m[$activeSpaceId!] ?? []).map(p =>
        p.id === editingPageId ? { ...p, title: editingTitle.trim() || $t.workArea.page } : p
      )
    }));
    editingPageId = null;
  }

  // ── Export active page ────────────────────────────────
  let exporting = false;
  async function handleExport() {
    if (!activePage || !$activeSpaceId || exporting) return;
    const safe = activePage.title.replace(/[^a-zA-Z0-9 \-]/g, "_").trim();
    const destPath = await save({
      title: $t.workArea.savePageAs,
      defaultPath: `${safe}.omnipage.zip`,
      filters: [{ name: "OmniSpace Page", extensions: ["zip", "omnipage"] }],
    });
    if (!destPath) return;
    exporting = true;
    try {
      await exportPage(activePage.id, destPath);
      showToast($t.workArea.exportedOk);
    } catch (e) {
      showToast($t.workArea.exportError(e), "error");
    } finally {
      exporting = false;
    }
  }

  // ── Import page ───────────────────────────────────────
  let importing = false;
  async function handleImport() {
    if (!$activeSpaceId || importing) return;
    const filePath = await open({
      title: $t.workArea.importPageTitle,
      filters: [{ name: "OmniSpace Page", extensions: ["zip", "omnipage"] }],
      multiple: false,
      directory: false,
    });
    if (!filePath || typeof filePath !== "string") return;
    importing = true;
    try {
      const page = await importPage($activeSpaceId, filePath);
      pagesMap.update(m => ({ ...m, [$activeSpaceId!]: [...(m[$activeSpaceId!] ?? []), page] }));
      activePageId.set(page.id);
      showToast($t.workArea.importedOk(page.title));
      setTimeout(() => tabsEl?.scrollTo({ left: tabsEl.scrollWidth, behavior: "smooth" }), 50);
    } catch (e) {
      showToast($t.workArea.importError(e), "error");
    } finally {
      importing = false;
    }
  }

  $: deleteTargetPage = spacePages.find(p => p.id === deleteConfirmId);

  // Palabras "Página" en todos los idiomas soportados
  const PAGE_WORDS = ["Página", "Page", "Seite", "Pagina", "Nova página", "Nouvelle page"];
  const DEFAULT_PAGE_RE = new RegExp(`^(${PAGE_WORDS.join("|")})\\s+(\\d+)$`, "i");

  // Si el título es el nombre por defecto (ej. "Página 1"), lo muestra en el idioma actual.
  // Si el usuario lo renombró, lo muestra tal cual.
  function displayPageTitle(title: string): string {
    const m = title.match(DEFAULT_PAGE_RE);
    if (m) return `${$t.workArea.page} ${m[2]}`;
    return title;
  }

  // ── Miniaturas de páginas (hover preview) ─────────────
  interface PagePreview { pageId: string; blocks: { type: string; label: string }[]; x: number; y: number; }
  let preview: PagePreview | null = null;
  let previewCache: Record<string, { type: string; label: string }[]> = {};
  let previewTimer: ReturnType<typeof setTimeout> | null = null;

  function blockLabel(b: { block_type: string; content: string }): string {
    try {
      const d = JSON.parse(b.content || "{}");
      if (b.block_type === "note")  return d.title || d.text?.slice(0, 40) || $t.search.labelNote;
      if (b.block_type === "link")  return d.title || d.url || $t.search.labelLink;
      if (b.block_type === "task")  return `${d.tasks?.length ?? 0} ${$t.toolbar.tasks.toLowerCase()}`;
      if (b.block_type === "file")  return d.name || $t.search.labelDoc;
      if (b.block_type === "shape") return d.text || $t.toolbar.shape.replace(":", "");
    } catch {}
    return b.block_type.charAt(0).toUpperCase() + b.block_type.slice(1);
  }

  async function showPreview(e: MouseEvent, pageId: string) {
    if (previewTimer) clearTimeout(previewTimer);
    const target = e.currentTarget as HTMLElement;
    previewTimer = setTimeout(async () => {
      if (!target) return;
      const rect = target.getBoundingClientRect();
      if (!previewCache[pageId]) {
        try {
          const rawBlocks = await getBlocks(pageId);
          previewCache[pageId] = rawBlocks
            .filter(b => b.block_type !== "shape")
            .slice(0, 6)
            .map(b => ({ type: b.block_type, label: blockLabel(b) }));
        } catch { previewCache[pageId] = []; }
      }
      const rawX = rect.left + rect.width / 2;
      const clampedX = Math.max(120, Math.min(window.innerWidth - 120, rawX));
      preview = { pageId, blocks: previewCache[pageId], x: clampedX, y: rect.top };
    }, 500);
  }

  function hidePreview() {
    if (previewTimer) { clearTimeout(previewTimer); previewTimer = null; }
    preview = null;
  }

  function focusEl(node: HTMLElement) { node.focus(); return {}; }

  // ── Tab detach (drag fuera de la barra) ──────────────
  const DETACH_THRESHOLD = 55;
  let detachCandidate: { id: string; title: string } | null = null;
  let detachStartY = 0;
  let detachStartX = 0;
  let detachDragActive = false;
  let detachedPageIds = new Set<string>();

  // Ghost visual durante el arrastre
  let ghost: { x: number; y: number; title: string; nearThreshold: boolean } | null = null;

  // ── Tab reorder drag ──────────────────────────────────
  type DragIntent = "none" | "reorder" | "detach";
  let dragIntent: DragIntent = "none";
  let reorderDragId: string | null = null;
  let reorderInsertIndex: number = -1;
  let dropIndicatorX: number = -1;
  let reorderMouseX: number = 0;
  let reorderMouseY: number = 0;

  function calculateInsertIndex(clientX: number): { index: number; x: number } {
    if (!tabsEl) return { index: -1, x: -1 };
    const tabs = Array.from(tabsEl.querySelectorAll<HTMLElement>(".page-tab"));
    const barRect = tabsEl.getBoundingClientRect();
    for (let i = 0; i < tabs.length; i++) {
      const rect = tabs[i].getBoundingClientRect();
      if (clientX < rect.left + rect.width / 2) {
        return { index: i, x: rect.left - barRect.left + tabsEl.scrollLeft };
      }
    }
    const last = tabs[tabs.length - 1];
    const lastRect = last?.getBoundingClientRect();
    return {
      index: tabs.length,
      x: lastRect ? lastRect.right - barRect.left + tabsEl.scrollLeft : -1,
    };
  }

  function commitReorder(pageId: string, insertIndex: number) {
    if (!$activeSpaceId) return;
    const pages = spacePages.filter(p => !detachedPageIds.has(p.id));
    const fromIndex = pages.findIndex(p => p.id === pageId);
    if (fromIndex === -1 || fromIndex === insertIndex || fromIndex === insertIndex - 1) return;
    const reordered = [...pages];
    const [moved] = reordered.splice(fromIndex, 1);
    const target = insertIndex > fromIndex ? insertIndex - 1 : insertIndex;
    reordered.splice(Math.min(target, reordered.length), 0, moved);
    pagesMap.update(m => ({ ...m, [$activeSpaceId!]: reordered }));
    // Persistir en BD
    reorderPages(reordered.map(p => p.id)).catch(console.error);
  }

  // ── Búsqueda global (Ctrl+K) ──────────────────────────
  let searchOpen = false;

  function onGlobalKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "k") {
      e.preventDefault();
      searchOpen = true;
    }
  }

  let unlistenReattach: (() => void) | null = null;

  onMount(async () => {
    window.addEventListener("keydown", onGlobalKey);
    const unlisten = await listen<{ pageId: string; spaceId: string }>(
      "reattach-tab",
      (event) => {
        const { pageId } = event.payload;
        detachedPageIds.delete(pageId);
        detachedPageIds = new Set(detachedPageIds);
        activePageId.set(pageId);
      }
    );
    unlistenReattach = unlisten;
  });

  onDestroy(() => {
    unlistenReattach?.();
    window.removeEventListener("mousemove", onTabDetachMove);
    window.removeEventListener("mouseup",   onTabDetachUp);
    window.removeEventListener("keydown", onGlobalKey);
  });

  function onTabMouseDown(e: MouseEvent, pageId: string, pageTitle: string) {
    if (e.button !== 0) return;
    e.stopPropagation();
    detachCandidate = { id: pageId, title: pageTitle };
    detachStartY = e.clientY;
    detachStartX = e.clientX;
    dragIntent = "none";
    detachDragActive = false;
    tabDragMoved = false;  // reset del sistema tab (detach/reorder)
    ghost = null;
    window.addEventListener("mousemove", onTabDetachMove);
    window.addEventListener("mouseup",   onTabDetachUp);
  }

  function onTabDetachMove(e: MouseEvent) {
    if (!detachCandidate) return;
    const dx = e.clientX - detachStartX;
    const dy = e.clientY - detachStartY;
    const dist = Math.sqrt(dx * dx + dy * dy);

    // Determinar intención tras mover 8px
    if (dragIntent === "none" && dist > 8) {
      dragIntent = dy > Math.abs(dx) && dy > 0 ? "detach" : "reorder";
      if (dragIntent === "reorder") tabDragMoved = true;
    }

    if (dragIntent === "detach") {
      ghost = {
        x: e.clientX, y: e.clientY,
        title: detachCandidate.title,
        nearThreshold: dy > DETACH_THRESHOLD * 0.6,
      };
      if (dy > DETACH_THRESHOLD) {
        const page = detachCandidate;
        detachCandidate = null; ghost = null;
        detachDragActive = true; dragIntent = "none";
        reorderDragId = null; reorderInsertIndex = -1; dropIndicatorX = -1;
        window.removeEventListener("mousemove", onTabDetachMove);
        window.removeEventListener("mouseup",   onTabDetachUp);
        openDetachedWindow(page.id, page.title);
      }
    } else if (dragIntent === "reorder") {
      reorderDragId = detachCandidate.id;
      reorderMouseX = e.clientX;
      reorderMouseY = e.clientY;
      const { index, x } = calculateInsertIndex(e.clientX);
      reorderInsertIndex = index;
      dropIndicatorX = x;
    }
  }

  function onTabDetachUp() {
    if (dragIntent === "reorder" && reorderDragId !== null && reorderInsertIndex >= 0) {
      commitReorder(reorderDragId, reorderInsertIndex);
    }
    ghost = null;
    reorderDragId = null;
    reorderInsertIndex = -1;
    dropIndicatorX = -1;
    dragIntent = "none";
    detachCandidate = null;
    tabDragMoved = false;
    window.removeEventListener("mousemove", onTabDetachMove);
    window.removeEventListener("mouseup",   onTabDetachUp);
    setTimeout(() => { detachDragActive = false; }, 50);
  }

  async function openDetachedWindow(pageId: string, title: string) {
    if (!$activeSpaceId) return;
    const label = `page-${pageId}`;
    const url = `${window.location.origin}${window.location.pathname}?detached=true&pageId=${pageId}&spaceId=${$activeSpaceId}`;

    // Marcar como desprendida y cambiar de página si era la activa
    detachedPageIds = new Set([...detachedPageIds, pageId]);
    if ($activePageId === pageId) {
      const others = spacePages.filter(p => p.id !== pageId && !detachedPageIds.has(p.id));
      activePageId.set(others[0]?.id ?? null);
    }

    try {
      const win = new WebviewWindow(label, {
        url, title, width: 960, height: 700,
        center: true, resizable: true, decorations: true,
      });
      win.once("tauri://error", (err) => console.error("Detach window error:", err));
      // Si el usuario cierra la ventana con la X sin reunir, restaurar la pestaña
      win.once("tauri://destroyed", () => {
        detachedPageIds.delete(pageId);
        detachedPageIds = new Set(detachedPageIds);
      });
    } catch {
      const existing = await WebviewWindow.getByLabel(label);
      existing?.setFocus();
    }
  }
</script>

{#if searchOpen}
  <SearchOverlay onClose={() => searchOpen = false} />
{/if}

<!-- ── Page preview popup ─────────────────────────────── -->
{#if preview}
  <div class="page-preview" style="left:{preview.x}px; top:{preview.y}px;">
    {#if preview.blocks.length === 0}
      <p class="preview-empty">{$t.workArea.pageEmpty}</p>
    {:else}
      {#each preview.blocks as b}
        <div class="preview-row">
          <span class="preview-icon">{
            b.type === "note" ? "📝" :
            b.type === "link" ? "🔗" :
            b.type === "task" ? "✅" :
            b.type === "file" ? "📄" :
            b.type === "calendar" ? "📅" :
            b.type === "clock" ? "🕐" : "◻"
          }</span>
          <span class="preview-label">{b.label}</span>
        </div>
      {/each}
    {/if}
  </div>
{/if}

<!-- ── Tab drag ghost (detach) ────────────────────────── -->
{#if ghost}
  <div
    class="tab-ghost"
    class:near-threshold={ghost.nearThreshold}
    style="left:{ghost.x}px; top:{ghost.y}px;"
  >
    <span class="ghost-icon">📄</span>
    <span class="ghost-title">{ghost.title}</span>
    {#if ghost.nearThreshold}<span class="ghost-hint">{$t.workArea.detachHint}</span>{/if}
  </div>
{/if}

<!-- ── Tab drag ghost (reorder) ───────────────────────── -->
{#if reorderDragId !== null}
  {@const reorderPage = spacePages.find(p => p.id === reorderDragId)}
  {#if reorderPage}
    <div class="tab-ghost tab-ghost-reorder" style="left:{reorderMouseX}px; top:{reorderMouseY}px;">
      <span class="ghost-title">{reorderPage.title}</span>
    </div>
  {/if}
{/if}

<!-- ── Toast ───────────────────────────────────────────── -->
{#if toastMsg}
  <div class="toast" class:toast-error={toastType === "error"} role="status">
    <span class="toast-icon">{toastType === "error" ? "✗" : "✓"}</span>
    <span class="toast-text">{toastMsg}</span>
    <button class="toast-close" on:click={() => toastMsg = ""}>×</button>
  </div>
{/if}

<!-- ── Delete confirmation modal ──────────────────────── -->
{#if deleteConfirmId && deleteTargetPage}
  <DeleteConfirmModal
    title={$t.deleteModal.deletePage(deleteTargetPage.title)}
    countdown={deleteCountdown}
    confirmLabel={$t.deleteModal.deletePageConfirm}
    shake={deleteShake}
    onConfirm={confirmDeletePage}
    onCancel={cancelDeletePage}
    on:shakeend={() => deleteShake = false}
  >
    {$t.deleteModal.deletePageWarning}<br/>
    <strong>{$t.deleteModal.irreversible}</strong>
  </DeleteConfirmModal>
{/if}

<main class="work-area">
  {#if !activeSpace}
    <div class="empty-state">
      <span class="empty-icon">⬡</span>
      <h2>{$t.workArea.welcome}</h2>
      <p>{$t.workArea.selectSpace}</p>
      {#if $spaces.length === 0}
        <button class="create-space-btn" on:click|stopPropagation={() => window.dispatchEvent(new CustomEvent("omni:create-space"))}>
          {$t.sidebar.newSpace}
        </button>
      {/if}
    </div>
  {:else}
    <!-- Top bar -->
    <header class="work-header">
      <div class="space-info">
        <span class="space-ico" style="--c:{activeSpace.color}">{activeSpace.icon}</span>
        <span class="space-nm">{activeSpace.name}</span>
        <span class="sep-arrow">›</span>
        {#if activePage}
          {#if breadEditing}
            <input
              class="bread-edit"
              bind:value={breadEditTitle}
              on:blur={commitBreadEdit}
              on:keydown={e => { if (e.key === "Enter") commitBreadEdit(); if (e.key === "Escape") breadEditing = false; }}
              on:click|stopPropagation
              use:focusEl
            />
          {:else}
            {#key $activePageId}
              <span
                class="bread-page"
                title={$t.workArea.renameTip}
                role="button" tabindex="0"
                on:dblclick|stopPropagation={startBreadEdit}
                on:keydown={e => e.key === 'Enter' && startBreadEdit()}
                in:fly={{ x: 14, duration: 220, delay: 30 }}
              >{displayPageTitle(activePage.title)}</span>
            {/key}
          {/if}
        {/if}
      </div>

      <!-- Page tabs with scroll -->
      <div class="tabs-wrap">
        <button class="scroll-arrow" on:click={() => scrollTabs(-1)} title={$t.workArea.prev}>‹</button>

        <div
          class="page-tabs"
          class:tab-drag={tabDragging}
          role="tablist"
          tabindex="0"
          bind:this={tabsEl}
          on:mousedown={onTabsMouseDown}
        >
          <!-- Drop indicator -->
          {#if reorderDragId !== null && dropIndicatorX >= 0}
            <div class="drop-indicator" style="left:{dropIndicatorX}px;"></div>
          {/if}

          {#each spacePages.filter(p => !detachedPageIds.has(p.id)) as page, i (page.id)}
            <div class="tab-group" in:fly={{ y: -10, duration: 200 }}>
            <div
              class="page-tab"
              class:active={$activePageId === page.id}
              class:tab-reordering={reorderDragId === page.id}
              role="button" tabindex="0"
              data-page-id={page.id}
              on:mousedown={e => onTabMouseDown(e, page.id, page.title)}
              on:mouseenter={e => showPreview(e, page.id)}
              on:mouseleave={hidePreview}
              on:click={() => { if (!tabDragMoved && !detachDragActive) { activePageId.set(page.id); cancelDeletePage(); } }}
              on:dblclick|stopPropagation={() => startEdit(page.id, page.title)}
              on:keypress={e => e.key === "Enter" && activePageId.set(page.id)}
            >
              {#if editingPageId === page.id}
                <input
                  class="tab-edit"
                  bind:value={editingTitle}
                  on:blur={commitEdit}
                  on:keydown={e => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") editingPageId = null; }}
                  on:click|stopPropagation
                  use:focusEl
                />
              {:else}
                <span class="tab-label">{displayPageTitle(page.title)}</span>
              {/if}

              <button
                class="tab-del"
                class:armed={deleteConfirmId === page.id}
                on:click|stopPropagation={() => requestDeletePage(page.id)}
                title={$t.workArea.deletePage}
              >×</button>
            </div>
            <button
              class="tab-add-after"
              on:click|stopPropagation={() => addPage(i)}
              title={$t.workArea.newPage}
            >+</button>
            </div>
          {/each}
        </div>

        <button class="scroll-arrow" on:click={() => scrollTabs(1)} title={$t.workArea.next}>›</button>
      </div>

      <!-- Export / Import -->
      <div class="page-actions">
        <button class="action-btn" on:click={handleExport} disabled={!activePage || exporting}
          title={$t.workArea.exportTitle}>
          {exporting ? "⏳" : "⬇"} {$t.workArea.export}
        </button>
        <button class="action-btn" on:click={handleImport} disabled={!$activeSpaceId || importing}
          title={$t.workArea.importTitle}>
          {importing ? "⏳" : "⬆"} {$t.workArea.importing}
        </button>
      </div>
    </header>

    {#if activePage}
      {#key activeSpace.id + "_" + activePage.id}
        <PageCanvas pageId={activePage.id} spaceId={activeSpace.id} />
      {/key}
    {/if}
  {/if}
</main>

<style>
  /* ── Layout ── */
  .work-area {
    flex: 1; display: flex; flex-direction: column;
    overflow: hidden; background: var(--bg-base);
  }

  .empty-state {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    gap: 12px; color: var(--text-muted);
  }
  .empty-icon { font-size: 56px; }
  .empty-state h2 { color: var(--text-primary); font-size: 22px; font-weight: 600; }
  .empty-state p  { font-size: 14px; }
  .create-space-btn {
    margin-top: 8px;
    padding: 10px 28px;
    background: var(--accent); color: #fff;
    border-radius: var(--radius-md);
    font-size: 14px; font-weight: 600;
    transition: opacity var(--transition);
  }
  .create-space-btn:hover { opacity: 0.88; }

  /* ── Toast ── */
  .toast {
    position: fixed; bottom: 20px; left: 50%; transform: translateX(-50%);
    display: flex; align-items: center; gap: 10px;
    background: var(--bg-surface); border: 1px solid var(--border);
    border-left: 4px solid #22c55e;
    border-radius: var(--radius-md); padding: 10px 14px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.4);
    z-index: 9999; max-width: 480px;
    animation: toast-in 0.2s ease;
  }
  .toast.toast-error { border-left-color: var(--red); }
  .toast-icon { font-size: 14px; font-weight: 700; color: #22c55e; flex-shrink: 0; }
  .toast-error .toast-icon { color: var(--red); }
  .toast-text { font-size: 12px; color: var(--text-secondary); flex: 1; word-break: break-all; }
  .toast-close {
    font-size: 16px; color: var(--text-muted); padding: 0 3px; line-height: 1;
    flex-shrink: 0; border-radius: 3px;
  }
  .toast-close:hover { color: var(--text-primary); background: var(--bg-hover); }
  @keyframes toast-in {
    from { opacity: 0; transform: translateX(-50%) translateY(12px); }
    to   { opacity: 1; transform: translateX(-50%) translateY(0); }
  }

  /* ── Header ── */
  .work-header {
    display: flex; align-items: stretch;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0; min-height: 46px;
    overflow: visible; gap: 0;
    user-select: none;
    padding-left: var(--sidebar-header-pad, 0px);
    transition: padding-left 0.25s cubic-bezier(0.4,0,0.2,1);
  }

  .space-info {
    display: flex; align-items: center; gap: 6px;
    flex-shrink: 0; padding: 0 10px 0 16px;
    align-self: stretch;
  }
  .space-ico { font-size: 18px; filter: drop-shadow(0 0 6px var(--c)); }
  .space-nm  { font-size: 13px; font-weight: 600; color: var(--text-primary); white-space: nowrap; }
  .sep-arrow { color: var(--text-muted); font-size: 16px; }

  .bread-page {
    font-size: 13px; font-weight: 500; color: var(--text-secondary);
    white-space: nowrap; cursor: text; max-width: 220px;
    overflow: hidden; text-overflow: ellipsis;
    transition: color var(--transition);
  }
  .bread-page:hover { color: var(--accent); }

  .bread-edit {
    font-size: 13px; font-weight: 500; color: var(--text-primary);
    background: var(--bg-overlay); border: 1px solid var(--accent);
    border-radius: 4px; padding: 2px 7px; width: 160px; outline: none;
  }

  /* ── Tabs ── */
  .tabs-wrap {
    display: flex; align-items: flex-end; flex: 1;
    overflow: hidden; min-width: 0;
    padding-top: 8px; /* room for tabs to grow upward */
  }

  .scroll-arrow {
    flex-shrink: 0; padding: 0 8px;
    font-size: 16px; font-weight: 700;
    color: var(--text-muted);
    border-right: 1px solid var(--border);
    transition: color var(--transition), background var(--transition);
    align-self: stretch; display: flex; align-items: center;
  }
  .scroll-arrow:last-of-type { border-right: none; border-left: 1px solid var(--border); }
  .scroll-arrow:hover { color: var(--accent); background: var(--accent-dim); }

  .page-tabs {
    position: relative;
    display: flex; align-items: flex-end;
    overflow-x: auto; flex: 1;
    scrollbar-width: none;
    cursor: default;
    gap: 2px; padding: 0 4px;
  }
  .page-tabs::-webkit-scrollbar { display: none; }
  .page-tabs.tab-drag { cursor: grabbing; }

  .page-tab {
    position: relative;
    display: flex; align-items: center; gap: 6px;
    padding: 0 10px 0 14px;
    height: 30px;
    min-width: 80px; max-width: 190px;
    border-radius: 8px 8px 0 0;
    border: 1px solid transparent;
    border-bottom: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px; cursor: pointer;
    white-space: nowrap; flex-shrink: 0;
    transition: background 0.15s, color 0.15s, height 0.12s, border-color 0.15s;
    outline: none; z-index: 0;
  }
  .page-tab:hover:not(.active) {
    background: var(--bg-overlay);
    color: var(--text-secondary);
    border-color: var(--border);
    height: 33px;
  }
  .page-tab.active {
    background: var(--bg-base);
    color: var(--text-primary);
    border-color: var(--border);
    border-bottom: none;
    height: 38px;
    z-index: 2;
    font-weight: 500;
    box-shadow: inset 0 2px 0 var(--accent);
  }

  /* Chrome-style curved fillets on active tab */
  .page-tab.active::before,
  .page-tab.active::after {
    content: '';
    position: absolute;
    bottom: 0;
    width: 8px; height: 8px;
    pointer-events: none; z-index: 3;
  }
  .page-tab.active::before {
    left: -8px;
    background: radial-gradient(circle at 0% 100%, transparent 8px, var(--bg-base) 8px);
  }
  .page-tab.active::after {
    right: -8px;
    background: radial-gradient(circle at 100% 100%, transparent 8px, var(--bg-base) 8px);
  }

  .tab-label {
    user-select: none; pointer-events: none;
    flex: 1; min-width: 0;
    overflow: hidden; white-space: nowrap; text-overflow: ellipsis;
  }

  .tab-edit {
    width: 90px; font-size: 12px; padding: 2px 4px;
    background: var(--bg-overlay); border: 1px solid var(--accent);
    border-radius: 3px; color: var(--text-primary); outline: none;
  }

  .tab-del {
    font-size: 13px; color: var(--text-muted);
    opacity: 0; padding: 1px 4px; line-height: 1;
    border-radius: 50%;
    flex-shrink: 0;
    transition: opacity var(--transition), color var(--transition), background var(--transition);
    outline: none;
  }
  .page-tab:hover .tab-del,
  .page-tab.active .tab-del { opacity: 0.6; }
  .tab-del:hover { opacity: 1 !important; color: var(--red); background: rgba(239,68,68,0.18); }
  .tab-del.armed { opacity: 1 !important; color: var(--red); }

  .tab-group {
    display: flex; align-items: flex-end; flex-shrink: 0;
  }
  .tab-add-after {
    flex-shrink: 0;
    width: 18px; height: 18px;
    border-radius: 50%;
    font-size: 13px; line-height: 1;
    color: var(--text-muted);
    background: var(--bg-overlay);
    border: 1px solid var(--border);
    display: flex; align-items: center; justify-content: center;
    opacity: 0;
    transition: opacity var(--transition), color var(--transition), background var(--transition);
    align-self: center;
    margin-left: 3px;
    outline: none;
    pointer-events: none;
  }
  .tab-group:hover .tab-add-after {
    opacity: 1;
    pointer-events: auto;
  }
  .tab-add-after:hover { color: var(--accent); border-color: var(--accent); background: var(--accent-dim); }

  /* ── Export / Import ── */
  .page-actions {
    display: flex; align-items: center; gap: 4px;
    flex-shrink: 0; padding: 0 10px;
    border-left: 1px solid var(--border);
    align-self: stretch;
  }
  .action-btn {
    display: flex; align-items: center; gap: 4px;
    padding: 4px 10px; border-radius: var(--radius-sm);
    font-size: 11px; font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-overlay); border: 1px solid var(--border);
    transition: all var(--transition); white-space: nowrap; outline: none;
  }
  .action-btn:hover:not(:disabled) { color: var(--accent); border-color: var(--accent); background: var(--accent-dim); }
  .action-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  /* ── Tab drag ghost ── */
  .tab-ghost {
    position: fixed;
    pointer-events: none;
    z-index: 99999;
    display: flex; align-items: center; gap: 6px;
    padding: 0 14px;
    height: 34px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 8px 8px 0 0;
    font-size: 12px; color: var(--text-primary);
    box-shadow: 0 8px 24px rgba(0,0,0,0.35);
    transform: translate(-50%, calc(-100% - 6px)) rotate(-4deg);
    transition: border-color 0.15s, box-shadow 0.15s;
    white-space: nowrap;
  }
  .tab-ghost.near-threshold {
    border-color: var(--accent);
    box-shadow: 0 8px 28px rgba(99,102,241,0.4);
    color: var(--accent);
  }
  .ghost-icon { font-size: 13px; }
  .ghost-title { font-weight: 500; }
  .ghost-hint {
    font-size: 10px; color: var(--accent);
    font-weight: 600; margin-left: 4px;
  }

  /* Reorder ghost: sigue al cursor limpio, sin rotación */
  .tab-ghost-reorder {
    transform: translate(-50%, calc(-100% - 4px)) rotate(0deg);
    border-color: var(--accent);
    box-shadow: 0 6px 20px rgba(99,102,241,0.35);
    color: var(--accent);
    opacity: 0.92;
  }

  /* Pestaña siendo arrastrada: semitransparente */
  .page-tab.tab-reordering {
    opacity: 0.35;
    pointer-events: none;
  }

  /* Indicador de inserción */
  .drop-indicator {
    position: absolute;
    top: 4px; bottom: 0;
    width: 2px;
    background: var(--accent);
    border-radius: 2px;
    pointer-events: none;
    z-index: 10;
    box-shadow: 0 0 6px rgba(99,102,241,0.6);
  }

  /* ── Page preview popup ── */
  .page-preview {
    position: fixed;
    transform: translate(-50%, calc(-100% - 10px));
    z-index: 99998;
    pointer-events: none;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 10px 12px;
    min-width: 160px; max-width: 240px;
    box-shadow: 0 8px 28px rgba(0,0,0,0.3);
    display: flex; flex-direction: column; gap: 5px;
    animation: preview-in 0.12s ease;
  }
  @keyframes preview-in {
    from { opacity: 0; transform: translate(-50%, calc(-100% - 4px)); }
    to   { opacity: 1; transform: translate(-50%, calc(-100% - 10px)); }
  }
  .preview-empty { font-size: 11px; color: var(--text-muted); text-align: center; margin: 0; }
  .preview-row { display: flex; align-items: center; gap: 6px; }
  .preview-icon { font-size: 12px; flex-shrink: 0; }
  .preview-label {
    font-size: 11px; color: var(--text-secondary);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    flex: 1;
  }
</style>
