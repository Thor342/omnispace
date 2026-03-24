<script lang="ts">
  import { onMount } from "svelte";
  import { slide } from "svelte/transition";
  import { spaces, activeSpaceId, categories } from "../stores/spaces";
  import {
    createSpace, deleteSpace, updateSpace,
    getCategories, createCategory, renameCategory, deleteCategory, assignSpaceToCategory
  } from "../api";
  import DeleteConfirmModal from "./DeleteConfirmModal.svelte";
  import IconPicker from "./IconPicker.svelte";
  import SettingsModal from "./SettingsModal.svelte";
  import appIcon from "../../assets/icon.png";

  let collapsed = true;
  let showCreate = false;
  let newName = "";
  let newIcon = "📁";

  let showSettings = false;

  // ── Category state ───────────────────────────────────────
  let collapsedCats = new Set<string>();
  let showCreateCat = false;
  let newCatName = "";
  let newCatIcon = "📂";

  let confirmDeleteCat: string | null = null;

  // ── Space inline edit ────────────────────────────────────
  let editSpaceId: string | null = null;
  let editSpaceName = "";
  let editSpaceIcon = "📁";

  let sidebarClientWidth = 52;
  let _clickedInside = false;
  let resizing = false;

  $: {
    // Footer usa el ancho real (evita que el logo tape los botones)
    document.documentElement.style.setProperty('--sidebar-w-current', sidebarClientWidth + 'px');
    // Header usa 0 cuando colapsado (logo está abajo, no tapa el header)
    document.documentElement.style.setProperty('--sidebar-header-pad', collapsed ? '0px' : sidebarClientWidth + 'px');
  }

  // ── Resize handle ─────────────────────────────────────────
  function onResizeStart(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    const startX = e.clientX;
    const startW = parseInt(getComputedStyle(document.documentElement).getPropertyValue('--sidebar-w')) || 310;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    document.body.classList.add('col-resize');

    function onMove(ev: PointerEvent) {
      const newW = Math.max(180, Math.min(520, startW + ev.clientX - startX));
      document.documentElement.style.setProperty('--sidebar-w', newW + 'px');
    }
    function onUp() {
      resizing = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      document.body.classList.remove('col-resize');
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('pointercancel', onUp);
    }
    window.addEventListener('pointermove', onMove);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('pointercancel', onUp);
  }

  function handleWindowClick() {
    if (_clickedInside) { _clickedInside = false; return; }
    collapsed = true;
    showCreate = false;
    showSettings = false;
    showCreateCat = false;
  }

  function startEditSpace(space: { id: string; name: string; icon: string; color: string }) {
    editSpaceId = space.id;
    editSpaceName = space.name;
    editSpaceIcon = space.icon;
  }
  async function commitEditSpace() {
    if (!editSpaceId || !editSpaceName.trim()) { editSpaceId = null; return; }
    const existing = $spaces.find(s => s.id === editSpaceId);
    const color = existing?.color ?? "#6366f1";
    await updateSpace(editSpaceId, editSpaceName.trim(), editSpaceIcon, color);
    spaces.update(s => s.map(sp => sp.id === editSpaceId
      ? { ...sp, name: editSpaceName.trim(), icon: editSpaceIcon }
      : sp));
    editSpaceId = null;
  }

  // ── Category inline edit ─────────────────────────────────
  let editCatId: string | null = null;
  let editCatName = "";
  let editCatIcon = "📂";

  function startEditCat(cat: { id: string; name: string; icon: string; color: string }) {
    editCatId = cat.id;
    editCatName = cat.name;
    editCatIcon = cat.icon;
  }
  async function commitEditCat() {
    if (!editCatId || !editCatName.trim()) { editCatId = null; return; }
    const existing = $categories.find(c => c.id === editCatId);
    const color = existing?.color ?? "#6366f1";
    await renameCategory(editCatId, editCatName.trim(), editCatIcon, color);
    categories.update(c => c.map(cat => cat.id === editCatId
      ? { ...cat, name: editCatName.trim(), icon: editCatIcon }
      : cat));
    editCatId = null;
  }

  // ── Delete space with countdown ──────────────────────────
  let deleteConfirmSpaceId: string | null = null;
  let deleteSpaceCountdown = 0;
  let deleteSpaceInterval: ReturnType<typeof setInterval> | null = null;

  function requestDeleteSpace(id: string) {
    if (deleteConfirmSpaceId === id) return;
    cancelDeleteSpace();
    deleteConfirmSpaceId = id;
    deleteSpaceCountdown = 5;
    deleteSpaceInterval = setInterval(() => {
      deleteSpaceCountdown--;
      if (deleteSpaceCountdown <= 0) { clearInterval(deleteSpaceInterval!); deleteSpaceInterval = null; }
    }, 1000);
  }
  function cancelDeleteSpace() {
    if (deleteSpaceInterval) { clearInterval(deleteSpaceInterval); deleteSpaceInterval = null; }
    deleteConfirmSpaceId = null; deleteSpaceCountdown = 0;
  }
  async function confirmDeleteSpaceAction() {
    if (!deleteConfirmSpaceId || deleteSpaceCountdown > 0) return;
    await handleDeleteSpace(deleteConfirmSpaceId);
    cancelDeleteSpace();
  }
  $: deleteTargetSpace = $spaces.find(s => s.id === deleteConfirmSpaceId);

  // ── Drag-and-drop ────────────────────────────────────────
  interface DragState {
    type: "space" | "cat";
    id: string; icon: string; name: string; color: string;
    x: number; y: number; startX: number; startY: number; active: boolean;
  }
  let drag: DragState | null = null;

  // Space drop: which category + before which space (null = end of list)
  let dropTargetCat: string | null | undefined = undefined; // undefined = no target
  let dropBeforeSpaceId: string | null = null;

  // Category drop: insert before which cat (null = end, undefined = no target)
  let dropBeforeCatId: string | null | undefined = undefined;

  function startDragSpace(e: PointerEvent, space: { id: string; icon: string; name: string; color: string }) {
    if (e.button !== 0) return;
    e.stopPropagation();
    drag = { type: "space", id: space.id, icon: space.icon, name: space.name, color: space.color,
             x: e.clientX, y: e.clientY, startX: e.clientX, startY: e.clientY, active: false };
    window.addEventListener("pointermove", onDragMove);
    window.addEventListener("pointerup",   onDragUp);
    window.addEventListener("pointercancel", onDragUp);
  }

  function startDragCat(e: PointerEvent, cat: { id: string; name: string; icon?: string; color: string }) {
    if (e.button !== 0) return;
    e.stopPropagation();
    drag = { type: "cat", id: cat.id, icon: cat.icon ?? "📂", name: cat.name, color: cat.color,
             x: e.clientX, y: e.clientY, startX: e.clientX, startY: e.clientY, active: false };
    window.addEventListener("pointermove", onDragMove);
    window.addEventListener("pointerup",   onDragUp);
    window.addEventListener("pointercancel", onDragUp);
  }

  function onDragMove(e: PointerEvent) {
    if (!drag) return;
    drag.x = e.clientX;
    drag.y = e.clientY;
    drag = drag;
    if (!drag.active) {
      if (Math.hypot(e.clientX - drag.startX, e.clientY - drag.startY) > 7) drag.active = true;
    }
    if (!drag.active) return;

    dropTargetCat    = undefined;
    dropBeforeSpaceId = null;
    dropBeforeCatId  = undefined;

    const el = document.elementFromPoint(e.clientX, e.clientY);

    if (drag.type === "space") {
      // 1. Over a specific space item → reorder + assign category
      const spaceEl = el?.closest("[data-space-item]") as HTMLElement | null;
      if (spaceEl && spaceEl.dataset.spaceItem !== drag.id) {
        const catVal = spaceEl.dataset.spaceCat;
        dropTargetCat = catVal === "none" ? null : catVal;
        const rect = spaceEl.getBoundingClientRect();
        dropBeforeSpaceId = e.clientY < rect.top + rect.height / 2
          ? spaceEl.dataset.spaceItem!
          : (spaceEl.dataset.spaceAfter || null);
        return;
      }
      // 2. Over a category header or cat-drop area → assign to that category (end)
      const catDropEl = el?.closest("[data-cat-drop]") as HTMLElement | null;
      if (catDropEl) {
        const v = catDropEl.dataset.catDrop;
        dropTargetCat     = v === "none" ? null : v;
        dropBeforeSpaceId = null;
      }
    } else {
      // Category reorder
      const catEl = el?.closest("[data-cat-item]") as HTMLElement | null;
      if (catEl && catEl.dataset.catItem !== drag.id) {
        const rect = catEl.getBoundingClientRect();
        dropBeforeCatId = e.clientY < rect.top + rect.height / 2
          ? catEl.dataset.catItem!
          : (catEl.dataset.catAfter || null);
      }
    }
  }

  async function onDragUp() {
    window.removeEventListener("pointermove", onDragMove);
    window.removeEventListener("pointerup",   onDragUp);
    window.removeEventListener("pointercancel", onDragUp);

    if (drag?.active) {
      if (drag.type === "space" && dropTargetCat !== undefined) {
        await doDropSpace(drag.id, dropTargetCat, dropBeforeSpaceId);
      } else if (drag.type === "cat" && dropBeforeCatId !== undefined) {
        doReorderCat(drag.id, dropBeforeCatId);
      }
    }
    drag = null;
    dropTargetCat    = undefined;
    dropBeforeSpaceId = null;
    dropBeforeCatId  = undefined;
  }

  async function doDropSpace(spaceId: string, newCatId: string | null, beforeId: string | null) {
    const space = $spaces.find(s => s.id === spaceId);
    if (!space) return;
    if (space.category_id !== newCatId) {
      await assignSpaceToCategory(spaceId, newCatId);
    }
    spaces.update(all => {
      const sp = { ...all.find(s => s.id === spaceId)!, category_id: newCatId };
      const others = all.filter(s => s.id !== spaceId);
      if (beforeId === null) return [...others, sp];
      const idx = others.findIndex(s => s.id === beforeId);
      return idx === -1 ? [...others, sp] : [...others.slice(0, idx), sp, ...others.slice(idx)];
    });
  }

  function doReorderCat(catId: string, beforeCatId: string | null) {
    // Reorder sectionOrder (handles both real cats and __uncat__)
    const filtered = sectionOrder.filter(s => s !== catId);
    if (beforeCatId === null) {
      sectionOrder = [...filtered, catId];
    } else {
      const idx = filtered.indexOf(beforeCatId);
      sectionOrder = idx === -1
        ? [...filtered, catId]
        : [...filtered.slice(0, idx), catId, ...filtered.slice(idx)];
    }
    // Sync $categories order (skip __uncat__)
    if (catId !== "__uncat__") {
      const realBefore = beforeCatId === "__uncat__" ? null : beforeCatId;
      categories.update(all => {
        const cat = all.find(c => c.id === catId);
        if (!cat) return all;
        const others = all.filter(c => c.id !== catId);
        if (realBefore === null) return [...others, cat];
        const idx = others.findIndex(c => c.id === realBefore);
        return idx === -1 ? [...others, cat] : [...others.slice(0, idx), cat, ...others.slice(idx)];
      });
    }
  }

  // ── Section order (categories + "__uncat__") ─────────
  let sectionOrder: string[] = ["__uncat__"];

  onMount(async () => {
    try {
      const cats = await getCategories();
      categories.set(cats);
      sectionOrder = [...cats.map(c => c.id), "__uncat__"];
    }
    catch (err) { console.error("Failed to load categories:", err); }
  });

  $: uncategorized = $spaces.filter(s => !s.category_id);
  $: catSpacesMap = new Map($categories.map(cat => [cat.id, $spaces.filter(s => s.category_id === cat.id)]));

  // ── Space actions ────────────────────────────────────────
  let createSpaceError = false;
  let createCatError = false;

  async function handleCreateSpace() {
    if (!newName.trim()) { createSpaceError = true; return; }
    createSpaceError = false;
    const space = await createSpace(newName.trim(), newIcon, "#6366f1");
    spaces.update(s => [...s, space]);
    activeSpaceId.set(space.id);
    newName = ""; newIcon = "📁"; showCreate = false;
  }

  async function handleDeleteSpace(id: string) {
    await deleteSpace(id);
    spaces.update(s => s.filter(sp => sp.id !== id));
    if ($activeSpaceId === id) {
      const remaining = $spaces.filter(s => s.id !== id);
      activeSpaceId.set(remaining.length > 0 ? remaining[0].id : null);
    }
  }

  // ── Category actions ─────────────────────────────────────
  async function handleCreateCat() {
    if (!newCatName.trim()) { createCatError = true; return; }
    createCatError = false;
    const cat = await createCategory(newCatName.trim(), newCatIcon, "#6366f1");
    categories.update(c => [...c, cat]);
    // Insert before __uncat__ in sectionOrder
    const ui = sectionOrder.indexOf("__uncat__");
    if (ui >= 0) sectionOrder = [...sectionOrder.slice(0, ui), cat.id, ...sectionOrder.slice(ui)];
    else sectionOrder = [...sectionOrder, cat.id];
    newCatName = ""; newCatIcon = "📂"; showCreateCat = false;
  }

  async function handleDeleteCat(id: string) {
    await deleteCategory(id);
    categories.update(c => c.filter(cat => cat.id !== id));
    sectionOrder = sectionOrder.filter(s => s !== id);
    spaces.update(s => s.map(sp => sp.category_id === id ? { ...sp, category_id: null } : sp));
    confirmDeleteCat = null;
  }

  function toggleCat(id: string) {
    const next = new Set(collapsedCats);
    if (next.has(id)) next.delete(id); else next.add(id);
    collapsedCats = next;
  }


</script>

<!-- ── Drag ghost ────────────────────────────────────────── -->
{#if drag?.active}
  <div class="drag-ghost" style="left:{drag.x}px; top:{drag.y}px;">
    <span style="filter:drop-shadow(0 0 4px {drag.color})">{drag.icon}</span>
    <span>{drag.name}</span>
  </div>
{/if}

<!-- ── Delete space modal ────────────────────────────────── -->
{#if deleteConfirmSpaceId && deleteTargetSpace}
  <DeleteConfirmModal
    title="¿Eliminar &quot;{deleteTargetSpace.name}&quot;?"
    countdown={deleteSpaceCountdown}
    confirmLabel="Sí, eliminar espacio"
    onConfirm={confirmDeleteSpaceAction}
    onCancel={cancelDeleteSpace}
  >
    Se eliminarán <strong>permanentemente</strong> todas las páginas, bloques,
    imágenes, trazos y datos de este espacio.<br/>
    <strong>Esta acción no se puede deshacer.</strong>
  </DeleteConfirmModal>
{/if}

<!-- ── Delete category modal ─────────────────────────────── -->
{#if confirmDeleteCat}
  {@const cat = $categories.find(c => c.id === confirmDeleteCat)}
  <DeleteConfirmModal
    title="¿Eliminar categoría &quot;{cat?.name}&quot;?"
    confirmLabel="Sí, eliminar categoría"
    onConfirm={() => confirmDeleteCat && handleDeleteCat(confirmDeleteCat)}
    onCancel={() => confirmDeleteCat = null}
  >
    Los espacios dentro quedarán sin categoría.<br/>
    <strong>Esta acción no se puede deshacer.</strong>
  </DeleteConfirmModal>
{/if}

<!-- ── Settings modal ───────────────────────────────────── -->
{#if showSettings}
  <SettingsModal onClose={() => showSettings = false} />
{/if}

<!-- ── Edit overlays (close on outside click) ────────────── -->
{#if editSpaceId || editCatId}
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="picker-overlay" on:click={() => { editSpaceId = null; editCatId = null; }} />
{/if}

<svelte:window on:click={handleWindowClick} />

<!-- svelte-ignore a11y-no-static-element-interactions -->
<!-- svelte-ignore a11y-click-events-have-key-events -->
<aside bind:clientWidth={sidebarClientWidth} class="sidebar" class:collapsed class:resizing
  on:click={() => { _clickedInside = true; if (collapsed) collapsed = false; }}>

  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="resize-handle" on:mousedown={onResizeStart}></div>

  <div class="sidebar-body">
      <!-- Space list -->
      <!-- svelte-ignore a11y-click-events-have-key-events -->
      <!-- svelte-ignore a11y-no-static-element-interactions -->
      <nav class="space-list" on:click={() => { editSpaceId = null; editCatId = null; }}>
    {#if !collapsed}

      <!-- ── Sections (categories + uncategorized, all reorderable) ── -->
      {#each sectionOrder as sectionId (sectionId)}

        <!-- Drop line before this section (cat reorder) -->
        {#if drag?.type === "cat" && dropBeforeCatId === sectionId}
          <div class="drop-line" />
        {/if}

        {#if sectionId === "__uncat__"}
          <!-- ── Uncategorized section ──────────────────── -->
          {@const nextSectionId = sectionOrder[sectionOrder.indexOf("__uncat__") + 1] ?? null}
          <!-- svelte-ignore a11y-no-static-element-interactions -->
          <div
            class="cat-header uncat-header"
            class:drag-over={drag?.type === "space" && dropTargetCat === null && dropBeforeSpaceId === null}
            class:is-dragging={drag?.id === "__uncat__" && drag.active}
            data-cat-drop="none"
            data-cat-item="__uncat__"
            data-cat-after={nextSectionId}
            on:pointerdown={e => startDragCat(e, { id: "__uncat__", name: "Sin Categoría", color: "#6b7280", icon: "📂" })}
          >
            <span class="cat-drag-handle" aria-hidden="true">⠿</span>
            <div
              class="section-label drop-label"
              class:drag-over={drag?.type === "space" && dropTargetCat === null}
              data-cat-drop="none"
            >{$categories.length > 0 ? "SIN CATEGORÍA" : "ESPACIOS"}</div>
          </div>

          <div class="uncategorized-list" class:drop-active={drag?.type === "space" && dropTargetCat === null} data-cat-drop="none">
            {#if uncategorized.length === 0 && $categories.length > 0}
              <p class="uncat-hint" class:drag-hint={drag?.type === "space" && dropTargetCat === null}>
                {drag?.type === "space" && dropTargetCat === null ? "Soltar aquí" : "Arrastra espacios aquí"}
              </p>
            {/if}
            {#each uncategorized as space, si (space.id)}
              {@const nextSpaceId = uncategorized[si + 1]?.id ?? null}

              {#if drag?.type === "space" && dropTargetCat === null && dropBeforeSpaceId === space.id}
                <div class="drop-line" />
              {/if}

              <div
                class="space-item"
                class:active={$activeSpaceId === space.id}
                class:dragging={drag?.id === space.id && drag.active}
                role="button" tabindex="0"
                data-space-item={space.id}
                data-space-cat="none"
                data-space-after={nextSpaceId}
                on:pointerdown={e => startDragSpace(e, space)}
                on:click|stopPropagation={() => { if (!drag?.active) activeSpaceId.set(space.id); }}
                on:keypress={e => e.key === "Enter" && activeSpaceId.set(space.id)}
              >
                <span class="space-icon" style="--c:{space.color}">{space.icon}</span>
                <span class="space-name truncate">{space.name}</span>
                <div class="item-actions" class:popup-open={editSpaceId === space.id} on:click|stopPropagation>
                  <div class="edit-wrap">
                    <button class="item-edit-btn" title="Editar"
                      on:click|stopPropagation={() => editSpaceId === space.id ? (editSpaceId = null) : startEditSpace(space)}>✎</button>
                    {#if editSpaceId === space.id}
                      <div class="edit-popup edit-popup-space" on:click|stopPropagation>
                        <input class="edit-input" bind:value={editSpaceName}
                          on:keydown={e => { if (e.key === "Enter") commitEditSpace(); if (e.key === "Escape") editSpaceId = null; }}
                          placeholder="Nombre" />
                        <IconPicker bind:value={editSpaceIcon} />
                        <div class="edit-actions">
                          <button class="edit-save" on:click={commitEditSpace}>Guardar</button>
                          <button class="edit-cancel" on:click={() => editSpaceId = null}>✕</button>
                        </div>
                      </div>
                    {/if}
                  </div>
                  <button class="item-del" on:click|stopPropagation={() => requestDeleteSpace(space.id)}>×</button>
                </div>
              </div>
            {/each}

            {#if drag?.type === "space" && dropTargetCat === null && dropBeforeSpaceId === null && uncategorized.length > 0}
              <div class="drop-line" />
            {/if}
          </div>

        {:else}
          <!-- ── Named category ─────────────────────────── -->
          {@const cat = $categories.find(c => c.id === sectionId)}
          {#if cat}
            {@const catSpaces = catSpacesMap.get(cat.id) ?? []}
            {@const isCatCollapsed = collapsedCats.has(cat.id)}
            {@const nextSectionId = sectionOrder[sectionOrder.indexOf(sectionId) + 1] ?? null}

            <!-- svelte-ignore a11y-no-static-element-interactions -->
            <div
              class="cat-header"
              class:drag-over={drag?.type === "space" && dropTargetCat === cat.id && dropBeforeSpaceId === null}
              class:is-dragging={drag?.id === cat.id && drag.active}
              data-cat-drop={cat.id}
              data-cat-item={cat.id}
              data-cat-after={nextSectionId}
              on:pointerdown={e => startDragCat(e, cat)}
            >
              <span class="cat-drag-handle" aria-hidden="true">⠿</span>

              <button class="cat-toggle" on:click|stopPropagation={() => { if (!drag?.active) toggleCat(cat.id); }} title={isCatCollapsed ? "Expandir" : "Colapsar"}>
                <span class="cat-arrow">{isCatCollapsed ? "▸" : "▾"}</span>
                <span class="cat-icon" style="filter:drop-shadow(0 0 4px {cat.color})">{cat.icon}</span>
                <span class="cat-name">{cat.name}</span>
                <span class="cat-count">{catSpaces.length}</span>
              </button>

              <div class="cat-actions" class:popup-open={editCatId === cat.id}>
                <div class="edit-wrap">
                  <button class="cat-btn" on:click|stopPropagation={() => editCatId === cat.id ? (editCatId = null) : startEditCat(cat)} title="Editar">✎</button>
                  {#if editCatId === cat.id}
                    <div class="edit-popup edit-popup-space" on:click|stopPropagation>
                      <input class="edit-input" bind:value={editCatName}
                        on:keydown={e => { if (e.key === "Enter") commitEditCat(); if (e.key === "Escape") editCatId = null; }}
                        placeholder="Nombre" />
                      <IconPicker bind:value={editCatIcon} />
                      <div class="edit-actions">
                        <button class="edit-save" on:click={commitEditCat}>Guardar</button>
                        <button class="edit-cancel" on:click={() => editCatId = null}>✕</button>
                      </div>
                    </div>
                  {/if}
                </div>
                <button class="cat-btn cat-btn-del" on:click|stopPropagation={() => confirmDeleteCat = cat.id} title="Eliminar">×</button>
              </div>
            </div>

            {#if !isCatCollapsed}
              <div class="cat-spaces" data-cat-drop={cat.id} transition:slide={{ duration: 180 }}>
                {#each catSpaces as space, si (space.id)}
                  {@const nextSpaceId = catSpaces[si + 1]?.id ?? null}

                  {#if drag?.type === "space" && dropTargetCat === cat.id && dropBeforeSpaceId === space.id}
                    <div class="drop-line" />
                  {/if}

                  <div
                    class="space-item space-item-indented"
                    class:active={$activeSpaceId === space.id}
                    class:dragging={drag?.id === space.id && drag.active}
                    role="button" tabindex="0"
                    data-space-item={space.id}
                    data-space-cat={cat.id}
                    data-space-after={nextSpaceId}
                    on:pointerdown={e => startDragSpace(e, space)}
                    on:click|stopPropagation={() => { if (!drag?.active) activeSpaceId.set(space.id); }}
                    on:keypress={e => e.key === "Enter" && activeSpaceId.set(space.id)}
                  >
                    <span class="space-icon" style="--c:{space.color}">{space.icon}</span>
                    <span class="space-name truncate">{space.name}</span>
                    <div class="item-actions" class:popup-open={editSpaceId === space.id} on:click|stopPropagation>
                      <div class="edit-wrap">
                        <button class="item-edit-btn" title="Editar"
                          on:click|stopPropagation={() => editSpaceId === space.id ? (editSpaceId = null) : startEditSpace(space)}>✎</button>
                        {#if editSpaceId === space.id}
                          <div class="edit-popup edit-popup-space" on:click|stopPropagation>
                            <input class="edit-input" bind:value={editSpaceName}
                              on:keydown={e => { if (e.key === "Enter") commitEditSpace(); if (e.key === "Escape") editSpaceId = null; }}
                              placeholder="Nombre" />
                            <IconPicker bind:value={editSpaceIcon} />
                            <div class="edit-actions">
                              <button class="edit-save" on:click={commitEditSpace}>Guardar</button>
                              <button class="edit-cancel" on:click={() => editSpaceId = null}>✕</button>
                            </div>
                          </div>
                        {/if}
                      </div>
                      <button class="item-del" on:click|stopPropagation={() => requestDeleteSpace(space.id)}>×</button>
                    </div>
                  </div>
                {/each}

                {#if drag?.type === "space" && dropTargetCat === cat.id && dropBeforeSpaceId === null && catSpaces.length > 0}
                  <div class="drop-line" />
                {/if}

                {#if catSpaces.length === 0 && !(drag?.type === "space" && dropTargetCat === cat.id)}
                  <p class="cat-empty">Sin espacios</p>
                {/if}
              </div>
            {/if}
          {/if}
        {/if}
      {/each}

      <!-- Drop line at end of section list -->
      {#if drag?.type === "cat" && dropBeforeCatId === null}
        <div class="drop-line" />
      {/if}

      {#if $spaces.length === 0}
        <p class="empty-hint">Crea tu primer espacio ↓</p>
      {/if}

    {:else}
      <!-- Collapsed: icons only -->
      {#each $spaces as space (space.id)}
        <div
          class="space-item"
          class:active={$activeSpaceId === space.id}
          title={space.name}
          role="button" tabindex="0"
          on:click={() => activeSpaceId.set(space.id)}
          on:keypress={e => e.key === "Enter" && activeSpaceId.set(space.id)}
        >
          <span class="space-icon" style="--c:{space.color}">{space.icon}</span>
        </div>
      {/each}
    {/if}
      </nav>

      <!-- Footer -->
      <div class="sidebar-footer">
    {#if collapsed}
      <button class="icon-btn" title="Nuevo Espacio" on:click|stopPropagation={() => { collapsed = false; showCreate = true; }}>+</button>
      <button class="icon-btn" title="Configuración" on:click|stopPropagation={() => { collapsed = false; showSettings = true; }}>⚙️</button>

    {:else if showCreate}
      <div class="create-panel">
        <input bind:value={newName} placeholder="Nombre del espacio"
          class:input-error={createSpaceError}
          on:input={() => createSpaceError = false}
          on:keydown={e => e.key === "Enter" && handleCreateSpace()} />
        {#if createSpaceError}
          <p class="field-error">Escribe un nombre para el espacio</p>
        {/if}
        <IconPicker bind:value={newIcon} />
        <div class="create-actions">
          <button class="btn-primary" on:click={handleCreateSpace}>Crear</button>
          <button class="btn-ghost" on:click={() => { showCreate = false; createSpaceError = false; }}>Cancelar</button>
        </div>
      </div>

    {:else if showCreateCat}
      <div class="create-panel">
        <input bind:value={newCatName} placeholder="Nombre de la categoría"
          class:input-error={createCatError}
          on:input={() => createCatError = false}
          on:keydown={e => e.key === "Enter" && handleCreateCat()} />
        {#if createCatError}
          <p class="field-error">Escribe un nombre para la categoría</p>
        {/if}
        <IconPicker bind:value={newCatIcon} />
        <div class="create-actions">
          <button class="btn-primary" on:click={handleCreateCat}>Crear</button>
          <button class="btn-ghost" on:click={() => { showCreateCat = false; createCatError = false; }}>Cancelar</button>
        </div>
      </div>

    {:else}
      <button class="new-space-btn" on:click={() => showCreate = true}>+ Nuevo Espacio</button>
      <button class="new-cat-btn" on:click={() => showCreateCat = true}>+ Nueva Categoría</button>

      <button class="settings-btn" on:click|stopPropagation={() => showSettings = true}>⚙️ Configuración</button>
    {/if}
      </div><!-- /sidebar-footer -->
    </div><!-- /sidebar-body -->

  <!-- Header: siempre visible al fondo, clic en cualquier parte para toggle -->
  <div class="sidebar-header" on:click|stopPropagation={() => {
    collapsed = !collapsed;
    if (collapsed) { showCreate = false; showSettings = false; showCreateCat = false; }
  }}>
    {#if !collapsed}
      <img src={appIcon} alt="OmniSpace" class="logo-img" /><span class="logo"> OmniSpace</span>
    {:else}
      <img src={appIcon} alt="OmniSpace" class="logo-img" />
    {/if}
    <span class="toggle-arrow">{collapsed ? '›' : '‹'}</span>
  </div>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-w); min-width: var(--sidebar-w);
    height: 100%;
    background: var(--bg-surface); border-right: 1px solid var(--border);
    display: flex; flex-direction: column; overflow: hidden;
    position: relative;
    pointer-events: auto; /* restaura clicks dentro del sidebar */
    transition: width 0.25s cubic-bezier(0.4,0,0.2,1), min-width 0.25s cubic-bezier(0.4,0,0.2,1);
  }
  .sidebar.resizing, .sidebar.resizing * { transition: none !important; }

  .resize-handle {
    position: absolute; top: 0; right: 0; bottom: 0; width: 5px;
    cursor: col-resize; z-index: 30;
    transition: background 0.15s;
  }
  .resize-handle:hover { background: var(--accent); opacity: 0.5; }
  .sidebar.collapsed { width: auto; min-width: unset; }
  .sidebar.collapsed .sidebar-header { border-top: none; }

  /* Body (expandable, slides up) */
  .sidebar-body {
    display: flex; flex-direction: column;
    overflow-y: auto; flex: 1; min-height: 0;
    border-bottom: 1px solid var(--border);
  }
  .sidebar.collapsed .sidebar-body { display: none; }

  /* Header: siempre al fondo, actúa como toggle */
  .sidebar-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 2px 10px 12px; border-top: 1px solid var(--border);
    flex-shrink: 0; min-height: 44px; cursor: pointer;
  }
  .sidebar-header:hover { background: var(--bg-hover); }
  .logo-img { width: 22px; height: 22px; object-fit: contain; flex-shrink: 0; }
  .toggle-arrow { font-size: 14px; color: var(--text-muted); margin-left: auto; flex-shrink: 0; }
  .logo { font-size: 14px; font-weight: 700; letter-spacing: -0.3px; white-space: nowrap; }
  /* Space list */
  .space-list { overflow-y: auto; padding: 10px 6px; overflow-x: hidden; }

  .section-label {
    font-size: 10px; font-weight: 600; letter-spacing: 1px;
    color: var(--text-muted); padding: 4px 6px 4px;
    border-radius: var(--radius-sm);
    transition: background var(--transition), color var(--transition);
  }

  /* Uncategorized list wrapper */
  .uncategorized-list {
    min-height: 40px; border-radius: var(--radius-sm);
    transition: background var(--transition);
  }
  .uncategorized-list.drop-active { background: var(--accent-dim); }

  .uncat-hint {
    font-size: 11px; color: var(--text-muted); font-style: italic;
    padding: 10px 12px; text-align: center;
    transition: color var(--transition);
  }
  .uncat-hint.drag-hint { color: var(--accent); }

  /* Category header — entire row is the drag target */
  .cat-header {
    display: flex; align-items: center;
    margin-bottom: 2px; border-radius: var(--radius-sm);
    padding-right: 2px;
    cursor: grab;
    transition: background var(--transition);
    user-select: none;
  }
  .cat-header:hover { background: var(--bg-hover); }
  .cat-header:active,
  .cat-header.is-dragging { cursor: grabbing; }
  .cat-header.drag-over { background: var(--accent-dim); outline: 1px dashed var(--accent); }

  /* Drag handle — visual indicator only, always visible */
  .cat-drag-handle {
    font-size: 11px; color: var(--text-muted);
    padding: 4px 3px; flex-shrink: 0;
    opacity: 0.3; line-height: 1;
    pointer-events: none;
    transition: opacity var(--transition);
  }
  .cat-header:hover .cat-drag-handle { opacity: 0.75; }

  .cat-toggle {
    display: flex; align-items: center; gap: 5px;
    flex: 1; padding: 5px 4px;
    font-size: 12px; font-weight: 600;
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    text-align: left; min-width: 0;
    cursor: pointer;
  }
  .cat-toggle:hover { color: var(--text-primary); }

  .cat-arrow { font-size: 9px; flex-shrink: 0; color: var(--text-muted); transition: transform 0.15s ease; }
  .cat-icon { font-size: 14px; flex-shrink: 0; line-height: 1; }
  .cat-name { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .cat-count {
    font-size: 10px; color: var(--text-muted); flex-shrink: 0;
    background: var(--bg-overlay); border-radius: 9px;
    padding: 0 5px; min-width: 16px; text-align: center; line-height: 16px;
  }

  .cat-actions {
    display: flex; gap: 1px; opacity: 0;
    transition: opacity var(--transition); flex-shrink: 0;
  }
  .cat-header:hover .cat-actions,
  .cat-actions.popup-open { opacity: 1; }

  .cat-btn {
    font-size: 12px; color: var(--text-muted);
    padding: 2px 5px; border-radius: 3px; line-height: 1;
    transition: color var(--transition), background var(--transition);
    cursor: pointer;
  }
  .cat-btn:hover { color: var(--text-primary); background: var(--bg-active); }
  .cat-btn-del:hover { color: var(--red); background: rgba(239,68,68,0.15); }

  /* Cat spaces — left accent border for visual grouping */
  .cat-spaces {
    margin-bottom: 6px;
    margin-left: 10px;
    padding-left: 6px;
    border-left: 2px solid var(--border);
    transition: border-color var(--transition);
  }
  .cat-empty { font-size: 11px; color: var(--text-muted); padding: 4px 8px; font-style: italic; }

  /* Drop indicator line */
  .drop-line {
    position: relative;
    height: 3px; background: var(--accent);
    border-radius: 2px; margin: 2px 4px;
    pointer-events: none;
  }
  .drop-line::before {
    content: ''; position: absolute;
    left: -4px; top: 50%; transform: translateY(-50%);
    width: 7px; height: 7px; border-radius: 50%;
    background: var(--accent);
  }

  /* Drop target: "SIN CATEGORÍA" label */
  .drop-label.drag-over {
    color: var(--accent); background: var(--accent-dim);
  }

  /* Uncategorized section header */
  .uncat-header { margin-top: 8px; }
  .uncat-header .section-label {
    flex: 1; padding: 5px 4px; font-size: 10px;
    font-weight: 600; letter-spacing: 1px;
    color: var(--text-muted); border-radius: var(--radius-sm);
    transition: background var(--transition), color var(--transition);
    cursor: grab;
  }
  .uncat-header.drag-over .section-label,
  .uncat-header .section-label.drag-over {
    color: var(--accent); background: var(--accent-dim);
  }

  /* Drag ghost */
  .drag-ghost {
    position: fixed; pointer-events: none; z-index: 9999;
    transform: translate(-50%, -50%);
    display: flex; align-items: center; gap: 6px;
    background: var(--bg-surface); border: 1px solid var(--accent);
    border-radius: var(--radius-sm); padding: 5px 10px;
    font-size: 13px; font-weight: 500; color: var(--text-primary);
    box-shadow: 0 8px 24px rgba(0,0,0,0.4); white-space: nowrap;
    rotate: 3deg; opacity: 0.92;
  }

  /* Space items */
  .space-item {
    display: flex; align-items: center; gap: 7px;
    padding: 6px 6px; border-radius: var(--radius-sm);
    cursor: grab; position: relative;
    transition: background var(--transition); margin-bottom: 1px;
    user-select: none;
  }
  .space-item:hover { background: var(--bg-hover); }
  .space-item:active { cursor: grabbing; }
  .space-item.active { background: var(--accent-dim); }
  .space-item.active:hover { background: var(--accent-dim); }
  .space-item.dragging { opacity: 0.3; pointer-events: none; }
  .space-item-indented { padding-left: 16px; }

  .space-icon {
    font-size: 18px; width: 26px; text-align: center; flex-shrink: 0;
    filter: drop-shadow(0 0 5px var(--c));
  }
  .space-name { flex: 1; font-size: 13px; font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

  .item-actions {
    display: flex; align-items: center; gap: 1px;
    opacity: 0; transition: opacity var(--transition); flex-shrink: 0;
  }
  .space-item:hover .item-actions,
  .item-actions.popup-open { opacity: 1; }

  .item-del {
    font-size: 14px; color: var(--text-muted);
    padding: 1px 4px; line-height: 1; border-radius: 3px;
    transition: color var(--transition), background var(--transition);
  }
  .item-del:hover { color: var(--red); background: rgba(239,68,68,0.15); }


  /* Edit popups */
  .picker-overlay { position: fixed; inset: 0; z-index: 40; }
  .edit-wrap { position: relative; }

  .item-edit-btn {
    font-size: 13px; color: var(--text-muted);
    padding: 1px 4px; line-height: 1; border-radius: 3px;
    transition: color var(--transition), background var(--transition);
  }
  .item-edit-btn:hover { color: var(--accent); background: var(--accent-dim); }

  .edit-popup {
    position: absolute; right: 0; top: calc(100% + 4px);
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius-md); padding: 8px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    z-index: 50; min-width: 180px;
    display: flex; flex-direction: column; gap: 6px;
  }
  .edit-popup-space { min-width: 200px; }

  .edit-input {
    width: 100%; padding: 5px 8px; font-size: 12px;
    background: var(--bg-overlay); border: 1px solid var(--accent);
    border-radius: var(--radius-sm); color: var(--text-primary); outline: none;
  }


  .edit-actions { display: flex; gap: 4px; }
  .edit-save {
    flex: 1; padding: 4px; background: var(--accent); color: #fff;
    border-radius: var(--radius-sm); font-size: 11px; font-weight: 600;
    transition: opacity var(--transition);
  }
  .edit-save:hover { opacity: 0.88; }
  .edit-cancel {
    padding: 4px 8px; background: var(--bg-overlay); color: var(--text-muted);
    border-radius: var(--radius-sm); font-size: 11px;
    border: 1px solid var(--border);
    transition: all var(--transition);
  }
  .edit-cancel:hover { color: var(--text-primary); background: var(--bg-hover); }

  .empty-hint { text-align: center; color: var(--text-muted); font-size: 12px; padding: 16px 8px; }

  /* Footer */
  .sidebar-footer {
    padding: 10px 6px; border-top: 1px solid var(--border);
    flex-shrink: 0; display: flex; flex-direction: column; gap: 6px;
  }

  .icon-btn {
    width: 100%; padding: 8px 0; border-radius: var(--radius-sm);
    font-size: 18px; text-align: center; color: var(--text-muted);
    transition: background var(--transition), color var(--transition);
  }
  .icon-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

  .new-space-btn {
    width: 100%; padding: 8px; border-radius: var(--radius-sm);
    background: var(--accent-dim); color: var(--accent);
    font-size: 13px; font-weight: 500; transition: background var(--transition);
  }
  .new-space-btn:hover { background: var(--accent); color: #fff; }

  .new-cat-btn {
    width: 100%; padding: 7px; border-radius: var(--radius-sm);
    background: var(--bg-overlay); color: var(--text-muted);
    font-size: 12px; border: 1px dashed var(--border);
    transition: all var(--transition);
  }
  .new-cat-btn:hover { color: var(--accent); border-color: var(--accent); background: var(--accent-dim); }

  .create-panel { display: flex; flex-direction: column; gap: 7px; }
  .create-panel input { width: 100%; padding: 7px 10px; }
  .create-panel input.input-error { border-color: var(--red, #ef4444); outline: none; box-shadow: 0 0 0 2px color-mix(in srgb, var(--red, #ef4444) 25%, transparent); }
  .field-error { margin: 0; font-size: 11px; color: var(--red, #ef4444); }
  .icon-picker { display: flex; flex-wrap: wrap; gap: 3px; }

  .create-actions { display: flex; gap: 6px; }
  .btn-primary { flex: 1; padding: 7px; background: var(--accent); color: #fff; border-radius: var(--radius-sm); font-size: 13px; font-weight: 500; }
  .btn-primary:hover { opacity: 0.9; }
  .btn-ghost { flex: 1; padding: 7px; background: var(--bg-active); color: var(--text-secondary); border-radius: var(--radius-sm); font-size: 13px; }

  .settings-btn {
    width: 100%; padding: 6px 8px; border-radius: var(--radius-sm);
    font-size: 12px; color: var(--text-muted); text-align: left;
    transition: all var(--transition);
  }
  .settings-btn:hover { color: var(--text-primary); background: var(--bg-hover); }
</style>
