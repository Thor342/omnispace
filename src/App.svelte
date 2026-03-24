<script lang="ts">
  import { onMount } from "svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import WorkArea from "./lib/components/WorkArea.svelte";
  import DetachedView from "./lib/components/DetachedView.svelte";
  import Updater from "./lib/components/Updater.svelte";
  import { spaces, activeSpaceId } from "./lib/stores/spaces";
  import { getSpaces } from "./lib/api";
  import "./lib/stores/theme";

  // Detectar si esta ventana es una pestaña desprendida
  const params = new URLSearchParams(window.location.search);
  const isDetached    = params.get("detached") === "true";
  const detachedPage  = params.get("pageId")  ?? "";
  const detachedSpace = params.get("spaceId") ?? "";

  onMount(async () => {
    if (isDetached) return; // ventana desprendida no necesita cargar spaces
    try {
      const data = await getSpaces();
      spaces.set(data);
      if (data.length > 0) activeSpaceId.set(data[0].id);
    } catch (e) {
      console.error("Failed to load spaces:", e);
    }
  });
</script>

{#if isDetached && detachedPage && detachedSpace}
  <DetachedView pageId={detachedPage} spaceId={detachedSpace} />
{:else}
  <Updater />
  <div class="app-layout">

    <!-- WorkArea ocupa toda la pantalla -->
    <div class="workarea-layer">
      <WorkArea />
    </div>

    <!-- Sidebar flotante, ocupa toda la altura izquierda -->
    <div class="sidebar-float">
      <Sidebar />
    </div>

  </div>
{/if}

<style>
  .app-layout {
    position: relative;
    height: 100vh; width: 100vw;
    overflow: hidden;
    background: var(--bg-base);
  }

  .workarea-layer {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
  }

  .sidebar-float {
    position: absolute;
    left: 0; bottom: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    transition: top 0.25s cubic-bezier(0.4,0,0.2,1);
    pointer-events: none; /* deja pasar clicks al canvas */
  }
  /* Cuando el sidebar está expandido, sube hasta el tope */
  :global(.sidebar-float:has(.sidebar:not(.collapsed))) {
    top: 0;
  }
</style>
