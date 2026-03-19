# OmniSpace — Guía de Arranque

## Prerrequisitos

1. **Node.js 18+** → https://nodejs.org
2. **Rust** → https://rustup.rs (ejecuta `rustup update`)
3. **Tauri CLI v2**:
   ```bash
   npm install -g @tauri-apps/cli
   ```
4. **Dependencias del sistema** (Windows):
   - Microsoft Visual C++ Build Tools o Visual Studio con componente "Desktop development with C++"
   - WebView2 (ya incluido en Windows 11)

---

## Instalación y arranque en desarrollo

```bash
# 1. Entra al directorio del proyecto
cd OmniSpace

# 2. Instala dependencias Node
npm install

# 3. Arranca en modo desarrollo (abre la ventana nativa automáticamente)
npm run tauri dev
```

## Compilar ejecutable (.exe)

```bash
npm run tauri build
```

El `.exe` instalador estará en:
`src-tauri/target/release/bundle/msi/` o `nsis/`

---

## Arquitectura

```
OmniSpace/
├── src-tauri/              ← Backend Rust
│   ├── src/
│   │   ├── main.rs         ← Punto de entrada
│   │   ├── lib.rs          ← Setup Tauri + AppState
│   │   ├── db.rs           ← Inicialización SQLite
│   │   ├── models.rs       ← Structs de datos
│   │   └── commands/       ← Comandos IPC (Rust → Svelte)
│   │       ├── spaces.rs
│   │       ├── notes.rs
│   │       ├── files.rs
│   │       ├── links.rs
│   │       └── tasks.rs
│   ├── capabilities/
│   │   └── default.json    ← Permisos de seguridad Tauri
│   └── tauri.conf.json     ← Configuración de la app
│
└── src/                    ← Frontend Svelte
    ├── app.css             ← Estilos globales + tokens CSS
    ├── App.svelte          ← Raíz
    └── lib/
        ├── api.ts          ← invoke() hacia Rust
        ├── types.ts        ← Tipos TypeScript compartidos
        ├── stores/
        │   ├── spaces.ts   ← Estado activo de espacios
        │   └── pomodoro.ts ← Lógica del temporizador
        └── components/
            ├── Sidebar.svelte      ← Panel de espacios
            ├── WorkArea.svelte     ← Área principal + tabs
            ├── notes/NoteEditor.svelte
            ├── files/FileViewer.svelte
            ├── links/LinkManager.svelte
            ├── tasks/TaskList.svelte
            └── pomodoro/PomodoroTimer.svelte
```

## Datos persistidos

La base de datos SQLite y los archivos adjuntos se guardan en:
- **Windows**: `%APPDATA%\com.omnispace.app\`
- **macOS**: `~/Library/Application Support/com.omnispace.app/`
- **Linux**: `~/.local/share/com.omnispace.app/`

100% local. Sin servidores. Sin telemetría.
