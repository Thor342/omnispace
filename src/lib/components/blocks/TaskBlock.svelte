<script lang="ts">
  import type { TaskContent, TaskItem } from "../../types";
  import { t as lang } from "../../stores/language";

  export let content: string;
  export let onContentChange: (c: string) => void;

  let data: TaskContent = JSON.parse(content || '{"tasks":[]}');
  let tasks: TaskItem[] = data.tasks ?? [];
  let newTitle = "";
  let editingId: string | null = null;
  let editingTitle = "";

  function save() { onContentChange(JSON.stringify({ tasks })); }

  function addTask() {
    if (!newTitle.trim()) return;
    tasks = [...tasks, { id: crypto.randomUUID(), title: newTitle.trim(), completed: false }];
    newTitle = "";
    save();
  }

  function toggle(id: string) {
    tasks = tasks.map(t => t.id === id ? { ...t, completed: !t.completed } : t);
    save();
  }

  function remove(id: string) {
    tasks = tasks.filter(t => t.id !== id);
    save();
  }

  function startEdit(id: string, title: string) {
    editingId = id;
    editingTitle = title;
  }

  function commitEdit() {
    if (!editingId) return;
    const trimmed = editingTitle.trim();
    if (trimmed) {
      tasks = tasks.map(t => t.id === editingId ? { ...t, title: trimmed } : t);
      save();
    }
    editingId = null;
  }

  function cancelEdit() {
    editingId = null;
  }

  $: done    = tasks.filter(t =>  t.completed).length;
  $: total   = tasks.length;
  $: pct     = total > 0 ? Math.round((done / total) * 100) : 0;
  $: pending = tasks.filter(t => !t.completed);
  $: completed = tasks.filter(t => t.completed);
</script>

<div class="task-block">
  {#if total > 0}
    <div class="progress-bar">
      <div class="fill" style="width:{pct}%" />
    </div>
    <div class="progress-label">{done}/{total} · {pct}%</div>
  {/if}

  <div class="task-list">
    {#each pending as t (t.id)}
      <div class="task-row">
        <button class="check" on:click={() => toggle(t.id)} title={$lang.taskBlock.complete}><span class="circle"></span></button>
        {#if editingId === t.id}
          <input
            class="edit-input"
            bind:value={editingTitle}
            on:blur={commitEdit}
            on:keydown={e => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }}
            on:click|stopPropagation
          />
        {:else}
          <span class="task-title" on:dblclick|stopPropagation={() => startEdit(t.id, t.title)}>{t.title}</span>
        {/if}
        <button class="del" title={$lang.taskBlock.delete} on:click={() => remove(t.id)}>×</button>
      </div>
    {/each}
    {#each completed as t (t.id)}
      <div class="task-row done">
        <button class="check done" on:click={() => toggle(t.id)} title={$lang.taskBlock.uncheck}><span class="circle filled">✓</span></button>
        {#if editingId === t.id}
          <input
            class="edit-input"
            bind:value={editingTitle}
            on:blur={commitEdit}
            on:keydown={e => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") cancelEdit(); }}
            on:click|stopPropagation
          />
        {:else}
          <span class="task-title done-text" on:dblclick|stopPropagation={() => startEdit(t.id, t.title)}>{t.title}</span>
        {/if}
        <button class="del" title={$lang.taskBlock.delete} on:click={() => remove(t.id)}>×</button>
      </div>
    {/each}
    {#if tasks.length === 0}
      <p class="empty">{$lang.taskBlock.empty}</p>
    {/if}
  </div>

  <div class="add-row">
    <input
      bind:value={newTitle}
      placeholder={$lang.taskBlock.newTaskPlaceholder}
      on:keydown={e => e.key === "Enter" && addTask()}
      class="add-input"
    />
    <button class="add-btn" on:click={addTask}>{$lang.taskBlock.add}</button>
  </div>
</div>

<style>
  .task-block { display: flex; flex-direction: column; flex: 1; overflow: hidden; }

  .progress-bar { height: calc(3px * var(--s, 1)); background: var(--bg-active); flex-shrink: 0; }
  .fill { height: 100%; background: linear-gradient(90deg, var(--accent), var(--green)); transition: width 0.4s; }
  .progress-label { font-size: calc(10px * var(--s, 1)); color: var(--text-muted); padding: calc(3px * var(--s, 1)) calc(10px * var(--s, 1)); flex-shrink: 0; }

  .task-list { flex: 1; overflow-y: auto; padding: calc(6px * var(--s, 1)) calc(8px * var(--s, 1)); }

  .task-row {
    display: flex; align-items: center; gap: calc(8px * var(--s, 1));
    padding: calc(5px * var(--s, 1)) calc(4px * var(--s, 1)); border-radius: var(--radius-sm);
    transition: background var(--transition);
  }
  .task-row:hover { background: var(--bg-hover); }
  .task-row.done { opacity: 0.6; }

  .check { flex-shrink: 0; }
  .circle {
    display: block; width: calc(17px * var(--s, 1)); height: calc(17px * var(--s, 1)); border-radius: 50%;
    border: 2px solid var(--border); transition: border-color var(--transition);
  }
  .check:hover .circle { border-color: var(--accent); }
  .circle.filled {
    background: var(--green); border-color: var(--green);
    color: #fff; font-size: calc(10px * var(--s, 1));
    display: flex; align-items: center; justify-content: center;
  }

  .task-title { flex: 1; font-size: calc(12px * var(--s, 1)); color: var(--text-primary); cursor: text; }
  .done-text  { text-decoration: line-through; color: var(--text-muted); }
  .edit-input {
    flex: 1; font-size: calc(12px * var(--s, 1)); padding: calc(2px * var(--s, 1)) calc(6px * var(--s, 1));
    border-radius: var(--radius-sm); background: var(--bg-overlay);
    color: var(--text-primary); border: 1px solid var(--accent);
    outline: none;
  }

  .del {
    font-size: calc(14px * var(--s, 1)); color: var(--text-muted); opacity: 0;
    padding: calc(1px * var(--s, 1)) calc(4px * var(--s, 1)); line-height: 1;
    transition: opacity var(--transition), color var(--transition);
  }
  .task-row:hover .del { opacity: 1; }
  .del:hover { color: var(--red); }

  .empty { text-align: center; color: var(--text-muted); font-size: calc(12px * var(--s, 1)); padding: calc(20px * var(--s, 1)) calc(8px * var(--s, 1)); }

  .add-row { display: flex; gap: calc(6px * var(--s, 1)); padding: calc(8px * var(--s, 1)); border-top: 1px solid var(--border); flex-shrink: 0; }
  .add-input { flex: 1; padding: calc(6px * var(--s, 1)) calc(10px * var(--s, 1)); font-size: calc(12px * var(--s, 1)); border-radius: var(--radius-sm); }
  .add-btn {
    width: calc(30px * var(--s, 1)); height: calc(30px * var(--s, 1)); border-radius: var(--radius-sm);
    background: var(--accent); color: #fff; font-size: calc(18px * var(--s, 1)); line-height: 1;
    display: flex; align-items: center; justify-content: center;
    flex-shrink: 0;
  }
  .add-btn:hover { background: var(--accent-hover); }
</style>
