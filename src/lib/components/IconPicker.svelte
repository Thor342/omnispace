<script lang="ts">
  export let value: string;

  const ICONS = [
    // Archivos y trabajo
    "📁","📂","🗂️","📋","📊","📈","📉","📌","📍","🗓️","📅","⏰","🕐","⏱️",
    // Creatividad y arte
    "🎨","🖌️","✏️","📝","📖","📚","🗒️","📜","🎭","🎬","🎵","🎸","🎹","🎤","🎧",
    // Tecnología y ciencia
    "💡","🔬","🔭","⚗️","🧪","🧬","🤖","💻","🖥️","📱","⌨️","🖱️","⚡","🔋","🛸",
    // Naturaleza y elementos
    "🌿","🌳","🌻","🌈","🌊","🔥","❄️","⭐","🌙","☀️","🌸","🍀","🌴","🦋","🐉",
    // Objetivos y logros
    "🎯","🚀","🏆","🥇","✅","⚙️","🔧","🔑","🗝️","🛡️","⚔️","💎","👑","🏅","🎖️",
    // Lugares y vida
    "🏠","🏢","🌍","✈️","🚂","⚓","🏖️","🏔️","🌆","🏰","🎪","🎡","🛒","🍕","☕",
    // Social y personas
    "👥","🤝","👋","🧠","💪","🎓","💬","📣","🔔","💌","🤔","😊","🧩","🎲","🎮",
  ];
</script>

<div class="icon-picker">
  {#each ICONS as ic}
    <button class="ic-btn" class:selected={value === ic} on:click={() => value = ic}>{ic}</button>
  {/each}
</div>

<style>
  .icon-picker { display: flex; flex-wrap: wrap; gap: 2px; }
  .ic-btn { font-size: 14px; padding: 3px; border-radius: 4px; }
  .ic-btn:hover, .ic-btn.selected { background: var(--bg-active); }
</style>
