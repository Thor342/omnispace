<script lang="ts">
  import { onMount } from "svelte";
  import { getTasks, createTask, toggleTask, deleteTask, updateTaskTitle } from "../../api";
  import type { Task } from "../../types";

  export let spaceId: string;

  let tasks: Task[] = [];
  let newTaskTitle = "";
  let editingId: string | null = null;
  let editingText = "";

  onMount(() => load());
  $: if (spaceId) load();

  async function load() {
    tasks = await getTasks(spaceId);
  }

  async function handleAdd() {
    if (!newTaskTitle.trim()) return;
    const task = await createTask(spaceId, newTaskTitle.trim());
    tasks = [...tasks, task];
    newTaskTitle = "";
  }

  async function handleToggle(id: string) {
    const newVal = await toggleTask(id);
    tasks = tasks.map((t) => t.id === id ? { ...t, completed: newVal } : t);
  }

  async function handleDelete(id: string) {
    await deleteTask(id);
    tasks = tasks.filter((t) => t.id !== id);
  }

  function startEdit(task: Task) {
    editingId = task.id;
    editingText = task.title;
  }

  async function commitEdit() {
    if (!editingId || !editingText.trim()) { cancelEdit(); return; }
    await updateTaskTitle(editingId, editingText.trim());
    tasks = tasks.map((t) => t.id === editingId ? { ...t, title: editingText.trim() } : t);
    editingId = null;
  }

  function cancelEdit() { editingId = null; editingText = ""; }

  $: pending   = tasks.filter((t) => !t.completed);
  $: completed = tasks.filter((t) =>  t.completed);
  $: progress  = tasks.length > 0 ? Math.round((completed.length / tasks.length) * 100) : 0;
</script>

<div class="tasks-layout">
  <!-- Progress header -->
  <div class="progress-bar-wrap">
    <div class="progress-info">
      <span>{completed.length}/{tasks.length} completadas</span>
      <span>{progress}%</span>
    </div>
    <div class="progress-track">
      <div class="progress-fill" style="width:{progress}%"></div>
    </div>
  </div>

  <!-- New task input -->
  <div class="new-task-row">
    <input
      bind:value={newTaskTitle}
      placeholder="Nueva tarea… (Enter para añadir)"
      on:keydown={(e) => e.key === "Enter" && handleAdd()}
      class="new-task-input"
    />
    <button class="add-task-btn" on:click={handleAdd}>+</button>
  </div>

  <!-- Task list -->
  <div class="task-list">
    {#if tasks.length === 0}
      <div class="empty-tasks">
        <span>✅</span>
        <p>¡Sin tareas! Añade una arriba.</p>
      </div>
    {:else}
      <!-- Pending -->
      {#if pending.length > 0}
        <div class="task-section-label">Pendientes</div>
        {#each pending as task (task.id)}
          <div class="task-item">
            <button
              class="check-btn"
              on:click={() => handleToggle(task.id)}
              title="Completar"
            >
              <span class="check-circle"></span>
            </button>

            {#if editingId === task.id}
              <input
                class="task-edit-input"
                bind:value={editingText}
                on:keydown={(e) => {
                  if (e.key === "Enter") commitEdit();
                  if (e.key === "Escape") cancelEdit();
                }}
                on:blur={commitEdit}
                autofocus
              />
            {:else}
              <span
                class="task-title"
                role="button"
                tabindex="0"
                on:dblclick={() => startEdit(task)}
                on:keypress={(e) => e.key === "Enter" && startEdit(task)}
              >{task.title}</span>
            {/if}

            <button class="task-del" on:click={() => handleDelete(task.id)}>×</button>
          </div>
        {/each}
      {/if}

      <!-- Completed -->
      {#if completed.length > 0}
        <div class="task-section-label done-label">Completadas</div>
        {#each completed as task (task.id)}
          <div class="task-item done">
            <button class="check-btn done" on:click={() => handleToggle(task.id)} title="Deshacer">
              <span class="check-circle filled">✓</span>
            </button>
            <span class="task-title done-title">{task.title}</span>
            <button class="task-del" on:click={() => handleDelete(task.id)}>×</button>
          </div>
        {/each}
      {/if}
    {/if}
  </div>
</div>

<style>
  .tasks-layout {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
    max-width: 680px;
    margin: 0 auto;
    width: 100%;
    padding: 24px 24px 0;
  }

  .progress-bar-wrap { margin-bottom: 20px; }

  .progress-info {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 6px;
  }

  .progress-track {
    height: 5px;
    background: var(--bg-active);
    border-radius: 99px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--green));
    border-radius: 99px;
    transition: width 0.4s ease;
  }

  .new-task-row {
    display: flex;
    gap: 8px;
    margin-bottom: 20px;
  }

  .new-task-input {
    flex: 1;
    padding: 10px 14px;
    font-size: 14px;
    border-radius: var(--radius-md);
  }

  .add-task-btn {
    width: 40px;
    height: 40px;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: #fff;
    font-size: 20px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background var(--transition);
    flex-shrink: 0;
  }

  .add-task-btn:hover { background: var(--accent-hover); }

  .task-list { flex: 1; overflow-y: auto; }

  .task-section-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--text-muted);
    padding: 4px 0 8px;
    text-transform: uppercase;
  }

  .done-label { color: var(--green); opacity: 0.7; margin-top: 16px; }

  .task-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    margin-bottom: 4px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    transition: border-color var(--transition), background var(--transition);
    position: relative;
  }

  .task-item:hover { border-color: var(--border-focus); }
  .task-item.done { opacity: 0.6; }

  .check-btn { flex-shrink: 0; }

  .check-circle {
    display: block;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: 2px solid var(--border);
    transition: border-color var(--transition), background var(--transition);
  }

  .check-btn:hover .check-circle { border-color: var(--accent); }

  .check-circle.filled {
    background: var(--green);
    border-color: var(--green);
    color: #fff;
    font-size: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .task-title {
    flex: 1;
    font-size: 14px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .done-title {
    text-decoration: line-through;
    color: var(--text-muted);
  }

  .task-edit-input {
    flex: 1;
    padding: 2px 8px;
    font-size: 14px;
    background: var(--bg-overlay);
    border: 1px solid var(--accent);
  }

  .task-del {
    font-size: 16px;
    color: var(--text-muted);
    opacity: 0;
    transition: opacity var(--transition), color var(--transition);
    padding: 2px 5px;
  }

  .task-item:hover .task-del { opacity: 1; }
  .task-del:hover { color: var(--red); }

  .empty-tasks {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 60px 16px;
    color: var(--text-muted);
    font-size: 14px;
    text-align: center;
  }

  .empty-tasks span { font-size: 48px; }
</style>
