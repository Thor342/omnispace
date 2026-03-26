<script lang="ts">
  import { onMount } from "svelte";

  let show = false;
  let step = 0;

  const SLIDES = [
    { icon: "🎉", title: "¡Bienvenido a OmniSpace!", body: "Tu espacio de trabajo personal. Organiza ideas, proyectos y notas en un canvas visual. Todo se guarda de forma segura en tu dispositivo." },
    { icon: "📁", title: "Crea tu primer espacio", body: "En el panel izquierdo, haz clic en '+ Nuevo Espacio'. Puedes organizar múltiples espacios en categorías arrastrándolos." },
    { icon: "📄", title: "Navega con páginas", body: "Cada espacio puede tener múltiples páginas. Usa el botón '+' en la barra de páginas para añadir nuevas. Haz doble clic en el nombre para renombrarlo." },
    { icon: "🧩", title: "Añade bloques al canvas", body: "Usa el botón '+ Añadir' en la barra inferior. Puedes insertar notas, enlaces, imágenes, videos, PDFs, tareas, calendarios y más." },
    { icon: "✏️", title: "Dibuja y personaliza", body: "Activa el lápiz o las figuras en la barra inferior para dibujar. Cambia el tema e idioma en '⚙️ Configuración'. Busca contenido rápido con Ctrl+K." },
  ];

  onMount(() => {
    if (!localStorage.getItem("omnispace_tutorial_v1")) {
      show = true;
    }
  });

  function dismiss() {
    localStorage.setItem("omnispace_tutorial_v1", "1");
    show = false;
  }

  function next() {
    if (step < SLIDES.length - 1) step++;
    else dismiss();
  }

  function prev() {
    if (step > 0) step--;
  }
</script>

{#if show}
  <div class="tutorial-backdrop">
    <div class="tutorial-card" role="dialog" aria-modal="true">
      <button class="skip-btn" on:click={dismiss}>Omitir</button>

      <div class="slide">
        <div class="slide-icon">{SLIDES[step].icon}</div>
        <h2 class="slide-title">{SLIDES[step].title}</h2>
        <p class="slide-body">{SLIDES[step].body}</p>
      </div>

      <div class="dots">
        {#each SLIDES as _, i}
          <button class="dot" class:active={i === step} on:click={() => step = i} aria-label="Ir al paso {i+1}" />
        {/each}
      </div>

      <div class="nav-row">
        <button class="nav-btn secondary" on:click={prev} disabled={step === 0}>← Anterior</button>
        <button class="nav-btn primary" on:click={next}>
          {step === SLIDES.length - 1 ? "¡Empezar! 🚀" : "Siguiente →"}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tutorial-backdrop {
    position: fixed; inset: 0; z-index: 9000;
    display: flex; align-items: center; justify-content: center;
    background: rgba(0,0,0,0.7); backdrop-filter: blur(4px);
  }
  .tutorial-card {
    position: relative;
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: 20px; padding: 40px 36px 32px;
    width: 440px; max-width: 92%;
    box-shadow: 0 32px 80px rgba(0,0,0,0.5);
    display: flex; flex-direction: column; align-items: center; gap: 24px;
  }
  .skip-btn {
    position: absolute; top: 16px; right: 16px;
    font-size: 12px; color: var(--text-muted);
    padding: 4px 10px; border-radius: 6px;
    transition: all var(--transition);
  }
  .skip-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .slide { text-align: center; display: flex; flex-direction: column; align-items: center; gap: 14px; }
  .slide-icon { font-size: 52px; line-height: 1; }
  .slide-title { font-size: 20px; font-weight: 700; color: var(--text-primary); margin: 0; }
  .slide-body { font-size: 14px; color: var(--text-secondary); line-height: 1.65; margin: 0; max-width: 340px; }
  .dots { display: flex; gap: 8px; }
  .dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--border); transition: all var(--transition);
    padding: 0;
  }
  .dot.active { background: var(--accent); width: 24px; border-radius: 4px; }
  .nav-row { display: flex; gap: 10px; width: 100%; }
  .nav-btn {
    flex: 1; padding: 10px 16px; border-radius: var(--radius-sm);
    font-size: 13px; font-weight: 600; cursor: pointer;
    transition: all var(--transition);
  }
  .nav-btn.secondary {
    background: var(--bg-overlay); border: 1px solid var(--border); color: var(--text-secondary);
  }
  .nav-btn.secondary:hover:not(:disabled) { background: var(--bg-hover); color: var(--text-primary); }
  .nav-btn.secondary:disabled { opacity: 0.3; cursor: not-allowed; }
  .nav-btn.primary {
    background: var(--accent); color: #fff; border: none;
  }
  .nav-btn.primary:hover { opacity: 0.88; }
</style>
