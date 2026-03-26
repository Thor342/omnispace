<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { theme } from "../stores/theme";
  import { language, t } from "../stores/language";
  import { LANGUAGES } from "../i18n/translations";
  import appIcon from "../../assets/icon.png";
  import faqImg1 from "../../assets/faq/faq-1.png";
  import faqImg2 from "../../assets/faq/faq-2.png";
  import faqImg3 from "../../assets/faq/faq-3.png";
  import faqImg4 from "../../assets/faq/faq-4.png";
  import faqImg41 from "../../assets/faq/faq-4.1.png";
  import faqImg5 from "../../assets/faq/faq-5.png";

  export let onClose: () => void;

  let appVersion = "";
  onMount(async () => { appVersion = await getVersion(); });

  let legalDoc: null | "privacy" | "terms" = null;

  let showFaq = false;
  let openFaqIndex: number | null = null;
  let emailCopied = false;
  let emailCopyTimer: ReturnType<typeof setTimeout> | null = null;

  function copyEmail() {
    navigator.clipboard.writeText("soporteomnispace@gmail.com");
    emailCopied = true;
    if (emailCopyTimer) clearTimeout(emailCopyTimer);
    emailCopyTimer = setTimeout(() => { emailCopied = false; }, 2000);
  }

  const FAQ_DATA: Record<string, { q: string; a: string; imgs?: string[] }[]> = {
    es: [
      { q: "¿Cómo creo un nuevo espacio?", a: "En el panel izquierdo haz clic en '+ Nuevo Espacio', escribe el nombre y presiona Enter o el botón Crear.", imgs: [faqImg1] },
      { q: "¿Cómo añado páginas?", a: "Selecciona un espacio y haz clic en el botón '+' en la barra de páginas superior. Haz doble clic en el nombre de la pestaña para renombrarla.", imgs: [faqImg2] },
      { q: "¿Cómo añado bloques al canvas?", a: "Usa el botón '+ Añadir' en la barra inferior para insertar notas, enlaces, imágenes, videos, PDFs, tareas, calendarios y más.", imgs: [faqImg3] },
      { q: "¿Puedo dibujar en el canvas?", a: "Sí. Haz clic en '✏️ Lápiz' en la barra inferior para activar el modo dibujo. Puedes cambiar el color y el grosor.", imgs: [faqImg4, faqImg41] },
      { q: "¿Cómo exporto o comparto una página?", a: "Usa los botones de Exportar e Importar en la barra de páginas. Exportar guarda la página como una carpeta que puedes compartir con otros usuarios.", imgs: [faqImg5] },
      { q: "¿Cómo organizo espacios en categorías?", a: "Haz clic en '+ Nueva Categoría' en el panel izquierdo y arrastra los espacios hacia ella." },
      { q: "¿Mis datos se guardan automáticamente?", a: "Sí. OmniSpace guarda todos los cambios automáticamente en tu dispositivo. No necesitas conexión a internet." },
      { q: "¿Puedo incrustar YouTube o Canva?", a: "Sí. En el menú '+ Añadir', elige YouTube o Canva y pega el enlace para incrustarlo en el canvas." },
      { q: "¿Cómo busco contenido?", a: "Usa el atajo Ctrl+K para abrir el buscador global. Busca en todos tus espacios y páginas a la vez." },
      { q: "¿Cómo elimino un espacio o página?", a: "Para espacios, haz clic derecho en el panel izquierdo. Para páginas, usa el botón de papelera en la barra de páginas. Ambas acciones requieren confirmación." },
    ],
    en: [
      { q: "How do I create a new space?", a: "In the left panel, click '+ New Space', type a name and press Enter or the Create button." },
      { q: "How do I add pages?", a: "Select a space and click '+' in the top page bar. Double-click the tab name to rename it." },
      { q: "How do I add blocks to the canvas?", a: "Use the '+ Add' button in the bottom toolbar to insert notes, links, images, videos, PDFs, tasks, calendars and more." },
      { q: "Can I draw on the canvas?", a: "Yes. Click '✏️ Pencil' in the bottom toolbar to activate draw mode. You can change color and thickness." },
      { q: "How do I export or share a page?", a: "Use the Export/Import buttons in the page bar. Export saves the page as a folder you can share with other OmniSpace users." },
      { q: "How do I organize spaces into categories?", a: "Click '+ New Category' in the left panel and drag spaces into it." },
      { q: "Are my data saved automatically?", a: "Yes. OmniSpace saves all changes automatically on your device. No internet connection required." },
      { q: "Can I embed YouTube or Canva?", a: "Yes. In the '+ Add' menu, choose YouTube or Canva and paste the link to embed it on the canvas." },
      { q: "How do I search for content?", a: "Use Ctrl+K to open the global search. Search across all your spaces and pages at once." },
      { q: "How do I delete a space or page?", a: "For spaces, right-click in the left panel. For pages, use the trash button in the page bar. Both require confirmation." },
    ],
    pt: [
      { q: "Como crio um novo espaço?", a: "No painel esquerdo, clique em '+ Novo Espaço', escreva o nome e pressione Enter ou o botão Criar." },
      { q: "Como adiciono páginas?", a: "Selecione um espaço e clique em '+' na barra de páginas. Clique duas vezes no nome da aba para renomear." },
      { q: "Como adiciono blocos ao canvas?", a: "Use o botão '+ Adicionar' na barra inferior para inserir notas, links, imagens, vídeos, PDFs, tarefas e mais." },
      { q: "Posso desenhar no canvas?", a: "Sim. Clique em '✏️ Lápis' na barra inferior. Você pode mudar cor e espessura." },
      { q: "Meus dados são salvos automaticamente?", a: "Sim. O OmniSpace salva tudo automaticamente no seu dispositivo, sem internet." },
    ],
    fr: [
      { q: "Comment créer un espace ?", a: "Dans le panneau gauche, cliquez sur '+ Nouvel espace', tapez un nom et appuyez sur Entrée." },
      { q: "Comment ajouter des pages ?", a: "Sélectionnez un espace et cliquez sur '+' dans la barre de pages. Double-cliquez sur le nom pour le renommer." },
      { q: "Comment ajouter des blocs au canvas ?", a: "Utilisez le bouton '+ Ajouter' dans la barre inférieure : notes, liens, images, vidéos, PDFs, tâches et plus." },
      { q: "Mes données sont-elles sauvegardées automatiquement ?", a: "Oui. OmniSpace sauvegarde tout automatiquement sur votre appareil, sans connexion internet." },
    ],
    de: [
      { q: "Wie erstelle ich einen neuen Bereich?", a: "Im linken Panel auf '+ Neuer Bereich' klicken, Namen eingeben und Enter drücken." },
      { q: "Wie füge ich Seiten hinzu?", a: "Wählen Sie einen Bereich und klicken Sie auf '+' in der Seitenleiste. Doppelklick auf den Namen zum Umbenennen." },
      { q: "Wie füge ich Blöcke hinzu?", a: "Verwenden Sie die Schaltfläche '+ Hinzufügen' in der unteren Leiste: Notizen, Links, Bilder, Videos, PDFs, Aufgaben und mehr." },
      { q: "Werden meine Daten automatisch gespeichert?", a: "Ja. OmniSpace speichert alles automatisch auf Ihrem Gerät, ohne Internetverbindung." },
    ],
    it: [
      { q: "Come creo un nuovo spazio?", a: "Nel pannello sinistro, clicca su '+ Nuovo Spazio', scrivi il nome e premi Invio." },
      { q: "Come aggiungo pagine?", a: "Seleziona uno spazio e clicca su '+' nella barra delle pagine. Doppio clic sul nome per rinominare." },
      { q: "Come aggiungo blocchi al canvas?", a: "Usa il pulsante '+ Aggiungi' nella barra inferiore: note, link, immagini, video, PDF, attività e altro." },
      { q: "I miei dati vengono salvati automaticamente?", a: "Sì. OmniSpace salva tutto automaticamente sul tuo dispositivo, senza internet." },
    ],
  };

  $: faqItems = FAQ_DATA[$language] ?? FAQ_DATA.es;

  type UpdateStatus = "idle" | "checking" | "upToDate" | "found" | "installing";
  let updateStatus: UpdateStatus = "idle";
  let updateVersion = "";
  let updateObj: Awaited<ReturnType<typeof check>> | null = null;
  let updateProgress = 0;
  let updateError = "";

  async function checkForUpdates() {
    if (updateStatus === "checking" || updateStatus === "installing") return;
    updateStatus = "checking";
    updateError = "";
    try {
      updateObj = await check();
      if (updateObj?.available) {
        updateVersion = updateObj.version;
        updateStatus = "found";
      } else {
        updateStatus = "upToDate";
      }
    } catch {
      updateStatus = "idle";
    }
  }

  async function installUpdate() {
    if (!updateObj) return;
    updateStatus = "installing";
    updateProgress = 0;
    updateError = "";
    try {
      let downloaded = 0;
      let total = 0;
      await updateObj.downloadAndInstall(event => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          updateProgress = total > 0 ? Math.round((downloaded / total) * 100) : 0;
        }
      });
      await relaunch();
    } catch (e) {
      updateError = String(e);
      updateStatus = "found";
    }
  }
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="modal-backdrop"
  role="button" tabindex="0"
  aria-label={$t.settings.close}
  on:click={e => { if (e.target === e.currentTarget) onClose(); }}
  on:keydown={e => e.key === "Escape" && legalDoc === null && !showFaq && onClose()}
>
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div class="settings-modal" on:click|stopPropagation role="dialog" aria-modal="true">
    <div class="settings-modal-header">
      <div class="settings-modal-title-row">
        <img src={appIcon} alt="OmniSpace" class="settings-modal-icon" />
        <div>
          <p class="settings-modal-name">{$t.settings.title}</p>
        </div>
      </div>
      <button class="settings-modal-close" on:click={onClose}>✕</button>
    </div>

    <div class="settings-modal-section">
      <p class="settings-section-label">{$t.settings.appearance}</p>
      <div class="theme-row">
        <button class="theme-option" class:active={$theme === "dark"} on:click={() => theme.set("dark")}>
          <span>🌙</span><span>{$t.settings.dark}</span>
        </button>
        <button class="theme-option" class:active={$theme === "light"} on:click={() => theme.set("light")}>
          <span>☀️</span><span>{$t.settings.light}</span>
        </button>
      </div>
    </div>

    <div class="settings-modal-section">
      <p class="settings-section-label">{$t.settings.language}</p>
      <div class="lang-select-wrap">
        <select class="lang-select" bind:value={$language}>
          {#each LANGUAGES as lang}
            <option value={lang.code}>{lang.label}</option>
          {/each}
        </select>
        <svg class="lang-chevron" viewBox="0 0 10 6" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M1 1l4 4 4-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
    </div>

    <div class="settings-modal-section">
      <p class="settings-section-label">{$t.settings.support}</p>
      <div class="support-row">
        <span class="support-email">{$t.settings.supportEmail}</span>
        <button class="btn-copy-email" on:click={copyEmail}>
          {emailCopied ? $t.settings.emailCopied : "📋"}
        </button>
      </div>
      <button class="btn-faq" on:click={() => showFaq = true}>{$t.settings.faq}</button>
    </div>

    <div class="settings-modal-section">
      <p class="settings-section-label">{$t.settings.about}</p>
      <div class="about-row">
        <span class="about-version">{$t.settings.version} {appVersion}</span>
        {#if updateStatus === "idle"}
          <button class="btn-check-update" on:click={checkForUpdates}>
            {$t.settings.checkUpdates}
          </button>
        {:else if updateStatus === "checking"}
          <span class="update-status-text muted">{$t.settings.checking}</span>
        {:else if updateStatus === "upToDate"}
          <span class="update-status-text ok">✓ {$t.settings.upToDate}</span>
        {:else if updateStatus === "found"}
          <div class="update-found-row">
            <span class="update-status-text accent">{$t.settings.updateFound(updateVersion)}</span>
            <button class="btn-install-update" on:click={installUpdate}>
              {$t.updater.installNow}
            </button>
          </div>
        {:else if updateStatus === "installing"}
          <div class="update-installing">
            <div class="update-bar-wrap">
              <div class="update-bar" style="width:{updateProgress}%"></div>
            </div>
            <span class="update-status-text muted">
              {updateProgress < 100 ? $t.updater.downloading(updateProgress) : $t.updater.installing}
            </span>
          </div>
        {/if}
        {#if updateError}
          <span class="update-error-text">{updateError}</span>
        {/if}
        <div class="legal-links">
          <button class="legal-link" on:click={() => legalDoc = "privacy"}>{$t.settings.privacy}</button>
          <span class="legal-sep">·</span>
          <button class="legal-link" on:click={() => legalDoc = "terms"}>{$t.settings.terms}</button>
        </div>
      </div>
    </div>
  </div>
</div>

{#if legalDoc !== null}
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="legal-backdrop"
    on:click|self={() => legalDoc = null}
    on:keydown={e => e.key === "Escape" && (legalDoc = null)}
    role="button" tabindex="-1"
  >
    <div class="legal-modal" role="dialog" aria-modal="true">
      <div class="legal-modal-header">
        <p class="legal-modal-title">
          {legalDoc === "privacy" ? $t.settings.privacyTitle : $t.settings.termsTitle}
        </p>
        <button class="legal-modal-close" on:click={() => legalDoc = null}>✕</button>
      </div>
      <div class="legal-modal-body">
        <p class="legal-text">
          {legalDoc === "privacy" ? $t.settings.privacyText : $t.settings.termsText}
        </p>
      </div>
    </div>
  </div>
{/if}

{#if showFaq}
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="legal-backdrop"
    on:click|self={() => { showFaq = false; openFaqIndex = null; }}
    on:keydown={e => e.key === "Escape" && (showFaq = false, openFaqIndex = null)}
    role="button" tabindex="-1"
  >
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="legal-modal faq-modal" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation>
      <div class="legal-modal-header">
        <p class="legal-modal-title">{$t.settings.faqTitle}</p>
        <button class="legal-modal-close" on:click={() => { showFaq = false; openFaqIndex = null; }}>✕</button>
      </div>
      <div class="legal-modal-body faq-body">
        {#each faqItems as item, i}
          <div class="faq-item">
            <button class="faq-q" on:click={() => openFaqIndex = openFaqIndex === i ? null : i}>
              <span class="faq-arrow" class:open={openFaqIndex === i}>▶</span>
              {item.q}
            </button>
            {#if openFaqIndex === i}
              <div class="faq-a">
                <p>{item.a}</p>
                {#if item.imgs}
                  <div class="faq-imgs">
                    {#each item.imgs as img}
                      <img src={img} alt="" class="faq-img" />
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.65);
    display: flex; align-items: center; justify-content: center;
    z-index: 200; backdrop-filter: blur(3px);
    pointer-events: auto;
  }
  .settings-modal {
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius-lg); padding: 24px;
    width: 340px; max-width: 92%;
    box-shadow: 0 24px 60px rgba(0,0,0,0.5);
    display: flex; flex-direction: column; gap: 20px;
  }
  .settings-modal-header { display: flex; align-items: flex-start; justify-content: space-between; }
  .settings-modal-title-row { display: flex; align-items: center; gap: 12px; }
  .settings-modal-icon { width: 40px; height: 40px; object-fit: contain; }
  .settings-modal-name { font-size: 16px; font-weight: 700; color: var(--text-primary); margin: 0; }
  .settings-modal-close {
    font-size: 14px; color: var(--text-muted);
    padding: 4px 7px; border-radius: 6px; transition: all var(--transition);
  }
  .settings-modal-close:hover { background: var(--bg-hover); color: var(--text-primary); }
  .settings-modal-section { display: flex; flex-direction: column; gap: 10px; }
  .settings-section-label {
    font-size: 10px; font-weight: 700; letter-spacing: 1px;
    text-transform: uppercase; color: var(--text-muted); margin: 0;
  }
  .theme-row { display: flex; gap: 6px; }
  .theme-option {
    flex: 1; display: flex; flex-direction: column; align-items: center; gap: 4px;
    padding: 10px 6px; border-radius: var(--radius-sm); font-size: 12px;
    color: var(--text-secondary); background: var(--bg-overlay);
    border: 1px solid var(--border); transition: all var(--transition);
  }
  .theme-option span:first-child { font-size: 18px; }
  .theme-option:hover { border-color: var(--accent); }
  .theme-option.active { background: var(--accent-dim); border-color: var(--accent); color: var(--accent); font-weight: 600; }
  .lang-select-wrap {
    position: relative; display: flex; align-items: center;
  }
  .lang-chevron {
    position: absolute; right: 10px;
    width: 10px; height: 6px;
    color: var(--text-muted); pointer-events: none;
  }
  .lang-select {
    width: 100%; padding: 9px 30px 9px 12px;
    background: var(--bg-overlay); border: 1px solid var(--border);
    border-radius: var(--radius-sm); color: var(--text-primary);
    font-size: 13px; font-weight: 500; cursor: pointer;
    appearance: none; -webkit-appearance: none;
    transition: border-color var(--transition);
  }
  .lang-select:hover, .lang-select:focus {
    border-color: var(--accent); outline: none;
  }

  .about-row {
    display: flex; flex-direction: column; gap: 8px;
  }
  .about-version {
    font-size: 12px; color: var(--text-muted);
  }
  .btn-check-update {
    align-self: flex-start;
    padding: 7px 14px; font-size: 12px; font-weight: 600;
    border-radius: var(--radius-sm);
    background: var(--bg-overlay); border: 1px solid var(--border);
    color: var(--text-secondary); cursor: pointer;
    transition: all var(--transition);
  }
  .btn-check-update:hover { border-color: var(--accent); color: var(--accent); }
  .update-status-text { font-size: 12px; }
  .update-status-text.muted { color: var(--text-muted); }
  .update-status-text.ok { color: #22c55e; }
  .update-status-text.accent { color: var(--accent); font-weight: 600; }
  .update-found-row { display: flex; flex-direction: column; gap: 6px; }
  .btn-install-update {
    align-self: flex-start;
    padding: 7px 14px; font-size: 12px; font-weight: 700;
    border-radius: var(--radius-sm);
    background: var(--accent); color: #fff; border: none;
    cursor: pointer; transition: opacity var(--transition);
  }
  .btn-install-update:hover { opacity: 0.85; }
  .update-installing { display: flex; flex-direction: column; gap: 6px; }
  .update-bar-wrap {
    height: 5px; background: var(--bg-overlay);
    border-radius: 3px; overflow: hidden;
  }
  .update-bar {
    height: 100%; background: var(--accent);
    border-radius: 3px; transition: width 0.2s;
  }
  .update-error-text { font-size: 11px; color: #ef4444; }

  .legal-links { display: flex; align-items: center; gap: 6px; margin-top: 2px; }
  .legal-sep { font-size: 11px; color: var(--text-muted); }
  .legal-link {
    font-size: 11px; color: var(--text-muted);
    text-decoration: underline; text-underline-offset: 2px;
    cursor: pointer; background: none; border: none; padding: 0;
    transition: color var(--transition);
  }
  .legal-link:hover { color: var(--accent); }

  /* Legal sub-modal */
  .legal-backdrop {
    position: fixed; inset: 0; z-index: 300;
    display: flex; align-items: center; justify-content: center;
    background: rgba(0,0,0,0.5); backdrop-filter: blur(2px);
    pointer-events: auto;
  }
  .legal-modal {
    background: var(--bg-surface); border: 1px solid var(--border);
    border-radius: var(--radius-lg); width: 420px; max-width: 92%;
    max-height: 75vh; display: flex; flex-direction: column;
    box-shadow: 0 24px 60px rgba(0,0,0,0.5);
    pointer-events: auto;
  }
  .legal-modal-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 16px 20px; border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .legal-modal-title { font-size: 14px; font-weight: 700; color: var(--text-primary); margin: 0; }
  .legal-modal-close {
    font-size: 14px; color: var(--text-muted);
    padding: 4px 7px; border-radius: 6px; transition: all var(--transition);
  }
  .legal-modal-close:hover { background: var(--bg-hover); color: var(--text-primary); }
  .legal-modal-body { padding: 20px; overflow-y: auto; flex: 1; }
  .legal-text {
    font-size: 12px; color: var(--text-secondary);
    line-height: 1.7; margin: 0; white-space: pre-wrap;
  }

  .support-row { display: flex; align-items: center; gap: 8px; }
  .support-email { font-size: 12px; color: var(--text-secondary); flex: 1; }
  .btn-copy-email {
    font-size: 13px; padding: 4px 8px; border-radius: var(--radius-sm);
    background: var(--bg-overlay); border: 1px solid var(--border);
    color: var(--text-muted); cursor: pointer; transition: all var(--transition);
    min-width: 32px;
  }
  .btn-copy-email:hover { border-color: var(--accent); color: var(--accent); }
  .btn-faq {
    align-self: flex-start;
    padding: 7px 14px; font-size: 12px; font-weight: 600;
    border-radius: var(--radius-sm);
    background: var(--bg-overlay); border: 1px solid var(--border);
    color: var(--text-secondary); cursor: pointer; transition: all var(--transition);
  }
  .btn-faq:hover { border-color: var(--accent); color: var(--accent); }

  .faq-modal { width: 680px; max-height: 88vh; }
  .faq-body { display: flex; flex-direction: column; gap: 2px; }
  .faq-item {
    border: 1px solid var(--border); border-radius: var(--radius-sm);
  }
  .faq-item + .faq-item { margin-top: 4px; }
  .faq-q {
    width: 100%; text-align: left;
    font-size: 13px; font-weight: 600; color: var(--text-primary);
    padding: 12px 14px; cursor: pointer;
    display: flex; align-items: center; gap: 8px;
    background: none; border: none;
    transition: background var(--transition);
  }
  .faq-q:hover { background: var(--bg-hover); }
  .faq-arrow {
    font-size: 9px; color: var(--accent);
    transition: transform 0.2s; flex-shrink: 0;
  }
  .faq-arrow.open { transform: rotate(90deg); }
  .faq-a {
    font-size: 12px; color: var(--text-secondary); line-height: 1.6;
    padding: 12px 14px 14px 34px; margin: 0;
    border-top: 1px solid var(--border);
  }
  .faq-a p { margin: 0 0 10px; }
  .faq-imgs { display: flex; flex-direction: column; gap: 8px; }
  .faq-img {
    width: 100%; height: auto; display: block;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
  }
</style>
