<script lang="ts">
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { tick } from "svelte";
  import { readFileAsBase64 } from "../../api";
  import type { FileContent } from "../../types";

  export let content: string;
  export let onUpdate: ((content: string) => void) | undefined = undefined;

  $: data = JSON.parse(content || '{"stored_path":"","name":"","file_type":"other","size":0}') as FileContent;
  $: fileUrl = data.stored_path ? convertFileSrc(data.stored_path) : "";

  function formatSize(b: number) {
    if (b < 1024) return `${b} B`;
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
    return `${(b / (1024 * 1024)).toFixed(1)} MB`;
  }

  // ── Helpers ────────────────────────────────────────────
  function b64ToArrayBuffer(b64: string): ArrayBuffer {
    const binary = atob(b64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes.buffer;
  }

  // ── Word viewer/editor ─────────────────────────────────
  let wordHtml = "";
  let wordLoading = false;
  let wordEditing = false;
  let wordEditEl: HTMLDivElement;

  // Restaurar HTML guardado o cargar desde el archivo
  $: if (data.file_type === "word" && data.stored_path && !wordHtml) {
    if (data.word_html) { wordHtml = data.word_html; } else { loadWord(); }
  }

  async function loadWord() {
    if (wordLoading) return;
    wordLoading = true;
    try {
      const { default: mammoth } = await import("mammoth");
      const b64 = await readFileAsBase64(data.stored_path);
      const ab = b64ToArrayBuffer(b64);
      const result = await mammoth.convertToHtml({ arrayBuffer: ab });
      wordHtml = result.value;
    } catch (e) {
      wordHtml = `<p style="color:red">Error al cargar: ${e}</p>`;
    } finally {
      wordLoading = false;
    }
  }

  let wordSaving = false;

  function escXml(s: string): string {
    return s.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;").replace(/"/g,"&quot;");
  }

  function nodeToRuns(node: Node): string {
    if (node.nodeType === Node.TEXT_NODE) {
      const t = node.textContent ?? "";
      return t ? `<w:r><w:t xml:space="preserve">${escXml(t)}</w:t></w:r>` : "";
    }
    if (node.nodeType === Node.ELEMENT_NODE) {
      const el = node as Element;
      const tag = el.tagName.toLowerCase();
      const children = Array.from(el.childNodes).map(nodeToRuns).join("");
      const isBold = tag === "strong" || tag === "b";
      const isItalic = tag === "em" || tag === "i";
      if (isBold || isItalic) {
        const rPr = `<w:rPr>${isBold ? "<w:b/>" : ""}${isItalic ? "<w:i/>" : ""}</w:rPr>`;
        return children.replace(/<w:r>/g, `<w:r>${rPr}`);
      }
      return children;
    }
    return "";
  }

  function elToOoxml(el: Element): string {
    const tag = el.tagName.toLowerCase();
    const styleMap: Record<string, string> = { h1:"Heading1", h2:"Heading2", h3:"Heading3" };
    const style = styleMap[tag];
    const pPr = style ? `<w:pPr><w:pStyle w:val="${style}"/></w:pPr>` : "";
    const runs = Array.from(el.childNodes).map(nodeToRuns).join("") ||
                 `<w:r><w:t>${escXml(el.textContent ?? "")}</w:t></w:r>`;
    return `<w:p>${pPr}${runs}</w:p>`;
  }

  async function saveWordEdit() {
    if (!wordEditEl || !onUpdate) return;
    const newHtml = wordEditEl.innerHTML;
    wordHtml = newHtml;
    wordEditing = false;
    wordSaving = true;
    try {
      const { default: JSZip } = await import("jszip");
      const { writeFile } = await import("@tauri-apps/plugin-fs");

      // Leer el .docx original
      const b64 = await readFileAsBase64(data.stored_path);
      const ab = b64ToArrayBuffer(b64);
      const zip = await JSZip.loadAsync(ab);

      // Convertir HTML editado a párrafos OOXML
      const htmlDoc = new DOMParser().parseFromString(newHtml, "text/html");
      const elems = Array.from(htmlDoc.body.children);
      const paragraphsXml = elems.length > 0
        ? elems.map(elToOoxml).join("\n")
        : `<w:p><w:r><w:t>${escXml(htmlDoc.body.textContent ?? "")}</w:t></w:r></w:p>`;

      // Preservar sectPr (márgenes, orientación de página) del documento original
      const docXml = await zip.files["word/document.xml"].async("string");
      const sectPrMatch = docXml.match(/<w:sectPr[\s\S]*?<\/w:sectPr>/);
      const sectPr = sectPrMatch ? sectPrMatch[0] : "<w:sectPr/>";

      const newDocXml = docXml.replace(
        /<w:body>[\s\S]*?<\/w:body>/,
        `<w:body>\n${paragraphsXml}\n${sectPr}\n</w:body>`
      );

      zip.file("word/document.xml", newDocXml);
      const newBuffer = await zip.generateAsync({ type: "uint8array" });
      await writeFile(data.stored_path, newBuffer);
    } catch (e) {
      console.error("Error guardando docx:", e);
    } finally {
      wordSaving = false;
    }
    onUpdate(JSON.stringify({ ...data, word_html: newHtml }));
  }

  // ── Excel viewer/editor ────────────────────────────────
  let excelSheets: string[] = [];
  let activeSheet = 0;
  let excelLoading = false;
  let excelSaving = false;
  let excelWb: any = null;
  // Grid data: rows of cells {v: value, t: type}
  interface XCell { v: string; t: string; addr: string; }
  let excelGrid: XCell[][] = [];
  let excelEditing = false;

  $: if (data.file_type === "excel" && !excelWb && data.stored_path) loadExcel();

  async function loadExcel() {
    if (excelLoading) return;
    excelLoading = true;
    try {
      const XLSX = await import("xlsx");
      const b64 = await readFileAsBase64(data.stored_path);
      const ab = b64ToArrayBuffer(b64);
      excelWb = XLSX.read(new Uint8Array(ab), { type: "array" });
      excelSheets = excelWb.SheetNames;
      renderSheetGrid(0);
    } catch (e) {
      excelGrid = [];
    } finally {
      excelLoading = false;
    }
  }

  async function renderSheetGrid(idx: number) {
    if (!excelWb) return;
    activeSheet = idx;
    const XLSX = await import("xlsx");
    const ws = excelWb.Sheets[excelSheets[idx]];
    const range = XLSX.utils.decode_range(ws["!ref"] || "A1:A1");
    const rows: XCell[][] = [];
    for (let r = range.s.r; r <= Math.min(range.e.r, 200); r++) {
      const row: XCell[] = [];
      for (let c = range.s.c; c <= Math.min(range.e.c, 50); c++) {
        const addr = XLSX.utils.encode_cell({ r, c });
        const cell = ws[addr];
        row.push({ v: cell ? String(cell.v ?? "") : "", t: cell?.t ?? "s", addr });
      }
      rows.push(row);
    }
    excelGrid = rows;
  }

  function onCellInput(e: Event, r: number, c: number) {
    onCellEdit(r, c, (e.target as HTMLInputElement).value);
  }

  function onCellEdit(r: number, c: number, val: string) {
    excelGrid = excelGrid.map((row, ri) =>
      ri === r ? row.map((cell, ci) => ci === c ? { ...cell, v: val } : cell) : row
    );
    excelEditing = true;
  }

  async function saveExcel() {
    if (!excelWb || excelSaving) return;
    excelSaving = true;
    try {
      const XLSX = await import("xlsx");
      const { writeFile } = await import("@tauri-apps/plugin-fs");
      const ws = excelWb.Sheets[excelSheets[activeSheet]];
      // Actualizar celdas modificadas
      for (let r = 0; r < excelGrid.length; r++) {
        for (let c = 0; c < excelGrid[r].length; c++) {
          const { addr, v } = excelGrid[r][c];
          if (ws[addr]) ws[addr].v = isNaN(Number(v)) ? v : Number(v);
          else if (v) ws[addr] = { v, t: "s" };
        }
      }
      const buf = XLSX.write(excelWb, { type: "array", bookType: "xlsx" });
      await writeFile(data.stored_path, new Uint8Array(buf));
      excelEditing = false;
    } catch (e) { console.error("Error guardando Excel:", e); }
    finally { excelSaving = false; }
  }

  // ── PowerPoint viewer (rich layout) ────────────────────
  const NSA = "http://schemas.openxmlformats.org/drawingml/2006/main";
  const NSP = "http://schemas.openxmlformats.org/presentationml/2006/main";
  const NSR = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

  interface PptRun  { text: string; bold: boolean; italic: boolean; size: number; color: string; }
  interface PptPara { align: string; runs: PptRun[]; }
  interface PptEl   {
    kind: "text"|"image"|"video"|"shape";
    x:number; y:number; w:number; h:number;
    // text
    paras?: PptPara[]; isTitle?: boolean;
    // image / video
    src?: string; mime?: string;
    // shape
    fillColor?: string; strokeColor?: string; strokeW?: number;
    geom?: string; rot?: number;
  }
  interface Slide   { index: number; bgColor: string; elements: PptEl[]; }

  let pptxSlides: Slide[] = [];
  let pptxLoading = false;
  let pptxActiveSlide = 0;

  // Track video play state per slide element key
  let videoEls: Record<string, HTMLVideoElement> = {};
  let videoPaused: Record<string, boolean> = {};

  function onVideoRef(e: Event, key: string) {
    videoEls = { ...videoEls, [key]: e.currentTarget as HTMLVideoElement };
    if (!(key in videoPaused)) videoPaused = { ...videoPaused, [key]: true };
  }

  $: if (data.file_type === "powerpoint" && pptxSlides.length === 0 && data.stored_path) loadPptx();

  function resolveZipPath(base: string, target: string): string {
    if (target.startsWith("/")) return target.slice(1);
    const parts = base.split("/"); parts.pop();
    for (const seg of target.split("/")) {
      if (seg === "..") parts.pop(); else if (seg !== ".") parts.push(seg);
    }
    return parts.join("/");
  }

  function pptIsDark(hex: string): boolean {
    const h = hex.replace("#","");
    if (h.length < 6) return true;
    const r = parseInt(h.slice(0,2),16), g = parseInt(h.slice(2,4),16), b = parseInt(h.slice(4,6),16);
    return (r*299 + g*587 + b*114)/1000 < 128;
  }

  function pptSolidColor(el: Element, tc: Record<string,string>): string {
    const srgb = el.getElementsByTagNameNS(NSA,"srgbClr")[0];
    if (srgb) return "#" + srgb.getAttribute("val");
    const sys  = el.getElementsByTagNameNS(NSA,"sysClr")[0];
    if (sys)  return "#" + (sys.getAttribute("lastClr") ?? "000000");
    const sch  = el.getElementsByTagNameNS(NSA,"schemeClr")[0];
    if (sch)  return "#" + (tc[sch.getAttribute("val") ?? ""] ?? "cccccc");
    return "";
  }

  function pptTransform(spPr: Element|undefined, sw: number, sh: number) {
    if (!spPr) return { x:0, y:0, w:100, h:100 };
    const xfrm = spPr.getElementsByTagNameNS(NSA,"xfrm")[0];
    if (!xfrm) return { x:0, y:0, w:100, h:100 };
    const off = xfrm.getElementsByTagNameNS(NSA,"off")[0];
    const ext = xfrm.getElementsByTagNameNS(NSA,"ext")[0];
    return {
      x: parseFloat(off?.getAttribute("x") ?? "0") / sw * 100,
      y: parseFloat(off?.getAttribute("y") ?? "0") / sh * 100,
      w: parseFloat(ext?.getAttribute("cx") ?? String(sw)) / sw * 100,
      h: parseFloat(ext?.getAttribute("cy") ?? String(sh)) / sh * 100,
    };
  }

  function pptParas(txBody: Element, tc: Record<string,string>, bgColor: string): PptPara[] {
    const defColor = pptIsDark(bgColor) ? "#FFFFFF" : "#111111";
    const alignMap: Record<string,string> = { ctr:"center", r:"right", just:"justify", l:"left" };
    const result: PptPara[] = [];
    for (const para of Array.from(txBody.getElementsByTagNameNS(NSA,"p"))) {
      const pPr  = para.getElementsByTagNameNS(NSA,"pPr")[0];
      const align = alignMap[pPr?.getAttribute("algn") ?? "l"] ?? "left";
      const runs: PptRun[] = [];
      for (const r of Array.from(para.getElementsByTagNameNS(NSA,"r"))) {
        const rPr = r.getElementsByTagNameNS(NSA,"rPr")[0];
        const tEl = r.getElementsByTagNameNS(NSA,"t")[0];
        if (!tEl?.textContent) continue;
        const sz    = parseInt(rPr?.getAttribute("sz") ?? "1800") / 100;
        const bold  = rPr?.getAttribute("b") === "1";
        const italic = rPr?.getAttribute("i") === "1";
        const color = (rPr ? pptSolidColor(rPr, tc) : "") || defColor;
        runs.push({ text: tEl.textContent, bold, italic, size: sz, color });
      }
      if (runs.length) result.push({ align, runs });
    }
    return result;
  }

  async function parseRels(zip: any, xmlPath: string): Promise<Record<string,string>> {
    const parts = xmlPath.split("/"); const file = parts.pop()!;
    const relsPath = parts.join("/") + "/_rels/" + file + ".rels";
    const rMap: Record<string,string> = {};
    if (zip.files[relsPath]) {
      const rd = new DOMParser().parseFromString(await zip.files[relsPath].async("string"), "text/xml");
      for (const rel of Array.from(rd.getElementsByTagName("Relationship")) as Element[])
        rMap[rel.getAttribute("Id") ?? ""] = rel.getAttribute("Target") ?? "";
    }
    return rMap;
  }

  function getBgFromDoc(doc: Document, tc: Record<string,string>): string {
    const bg = doc.getElementsByTagNameNS(NSP, "bg")[0];
    if (!bg) return "";
    return pptSolidColor(bg, tc);
  }

  async function parseSpTree(
    spTree: Element, rMap: Record<string,string>, zip: any,
    tc: Record<string,string>, sw: number, sh: number, bgColor: string, basePath: string
  ): Promise<PptEl[]> {
    const elements: PptEl[] = [];
    for (const child of Array.from(spTree.children) as Element[]) {
      const local = child.localName;

      if (local === "sp") {
        const spPr   = child.getElementsByTagNameNS(NSP, "spPr")[0];
        const txBody = child.getElementsByTagNameNS(NSP, "txBody")[0];
        const pos    = pptTransform(spPr, sw, sh);
        let fillColor = "", strokeColor = "", strokeW = 0, geom = "rect";
        if (spPr) {
          const solidFill = spPr.getElementsByTagNameNS(NSA, "solidFill")[0];
          const gradFill  = spPr.getElementsByTagNameNS(NSA, "gradFill")[0];
          if (solidFill) { fillColor = pptSolidColor(solidFill, tc); }
          else if (gradFill) {
            const firstGs = gradFill.getElementsByTagNameNS(NSA, "gs")[0];
            if (firstGs) fillColor = pptSolidColor(firstGs, tc) || "#888888";
          }
          const ln = spPr.getElementsByTagNameNS(NSA, "ln")[0];
          if (ln) {
            const lnFill = ln.getElementsByTagNameNS(NSA, "solidFill")[0];
            if (lnFill) { strokeColor = pptSolidColor(lnFill, tc); strokeW = parseInt(ln.getAttribute("w") ?? "0") / 12700; }
          }
          const prstGeom = spPr.getElementsByTagNameNS(NSA, "prstGeom")[0];
          if (prstGeom) geom = prstGeom.getAttribute("prst") ?? "rect";
        }
        if (txBody) {
          const paras = pptParas(txBody, tc, bgColor);
          if (paras.some(p => p.runs.some(r => r.text.trim()))) {
            const phEl = child.getElementsByTagNameNS(NSP, "ph")[0];
            const phType = phEl?.getAttribute("type") ?? "";
            const isTitle = phType === "title" || phType === "ctrTitle";
            elements.push({ kind:"text", ...pos, paras, isTitle, fillColor: fillColor||undefined, strokeColor: strokeColor||undefined, strokeW: strokeW||undefined, geom });
          }
        } else if (fillColor) {
          const xfrm = spPr?.getElementsByTagNameNS(NSA, "xfrm")[0];
          const rot  = xfrm ? parseInt(xfrm.getAttribute("rot") ?? "0") / 60000 : 0;
          elements.push({ kind:"shape", ...pos, fillColor, strokeColor: strokeColor||undefined, strokeW: strokeW||undefined, geom, rot: rot||undefined });
        }

      } else if (local === "pic") {
        const spPr    = child.getElementsByTagNameNS(NSP, "spPr")[0];
        const pos     = pptTransform(spPr, sw, sh);
        const nvPr    = child.getElementsByTagNameNS(NSP, "nvPr")[0];
        const vidFile = nvPr?.getElementsByTagNameNS(NSA, "videoFile")[0];
        if (vidFile) {
          const rId = vidFile.getAttributeNS(NSR,"link") ?? vidFile.getAttribute("r:link") ?? "";
          const rel = rMap[rId];
          if (rel) {
            const vidKey = resolveZipPath(basePath, rel);
            try {
              const f = zip.files[vidKey];
              if (f) {
                const ext  = vidKey.split(".").pop()?.toLowerCase() ?? "mp4";
                const mime = ext==="webm"?"video/webm":ext==="mov"?"video/quicktime":ext==="avi"?"video/avi":"video/mp4";
                const b64v = await f.async("base64");
                elements.push({ kind:"video", ...pos, src:`data:${mime};base64,${b64v}`, mime });
              }
            } catch {}
          }
          continue;
        }
        const blip = child.getElementsByTagNameNS(NSA, "blip")[0];
        const rId  = blip?.getAttributeNS(NSR,"embed") ?? blip?.getAttribute("r:embed") ?? "";
        const rel  = rMap[rId];
        if (rel) {
          const imgKey = resolveZipPath(basePath, rel);
          try {
            const f = zip.files[imgKey];
            if (f) {
              const ext  = imgKey.split(".").pop()?.toLowerCase() ?? "png";
              const mime = ext==="jpg"||ext==="jpeg"?"image/jpeg":ext==="gif"?"image/gif":ext==="svg"?"image/svg+xml":"image/png";
              const b64i = await f.async("base64");
              elements.push({ kind:"image", ...pos, src:`data:${mime};base64,${b64i}` });
            }
          } catch {}
        }

      } else if (local === "grpSp") {
        const sub = await parseSpTree(child, rMap, zip, tc, sw, sh, bgColor, basePath);
        elements.push(...sub);
      }
    }
    return elements;
  }

  async function loadPptx() {
    if (pptxLoading) return;
    pptxLoading = true;
    try {
      const { default: JSZip } = await import("jszip");
      const b64 = await readFileAsBase64(data.stored_path);
      const zip = await JSZip.loadAsync(b64ToArrayBuffer(b64));

      // Theme colors
      const tc: Record<string,string> = { dk1:"000000", lt1:"FFFFFF", dk2:"44546A", lt2:"E7E6E6",
        accent1:"4472C4", accent2:"ED7D31", accent3:"A9D18E", accent4:"FFC000", accent5:"5B9BD5", accent6:"70AD47" };
      const themeKey = Object.keys(zip.files).find(k => /^ppt\/theme\/theme\d+\.xml$/.test(k));
      if (themeKey) {
        try {
          const tdoc = new DOMParser().parseFromString(await zip.files[themeKey].async("string"), "text/xml");
          const scheme = tdoc.getElementsByTagNameNS(NSA,"clrScheme")[0];
          if (scheme) for (const ch of Array.from(scheme.children)) {
            const ce = ch.children[0];
            const v  = ce?.getAttribute("val") ?? ce?.getAttribute("lastClr");
            if (v) tc[ch.localName] = v;
          }
        } catch {}
      }

      // Slide dimensions
      let sw = 9144000, sh = 5143500;
      try {
        const presXml = await zip.files["ppt/presentation.xml"].async("string");
        const pd = new DOMParser().parseFromString(presXml,"text/xml");
        const sz = pd.getElementsByTagNameNS(NSP,"sldSz")[0];
        if (sz) { sw = parseInt(sz.getAttribute("cx") ?? String(sw)); sh = parseInt(sz.getAttribute("cy") ?? String(sh)); }
      } catch {}

      const slideKeys = Object.keys(zip.files)
        .filter(k => /^ppt\/slides\/slide\d+\.xml$/.test(k))
        .sort((a, b) => (parseInt(a.match(/\d+/)?.[0]??"0")) - (parseInt(b.match(/\d+/)?.[0]??"0")));

      const layoutCache: Record<string, { doc: Document; rMap: Record<string,string> }> = {};
      const masterCache: Record<string, { doc: Document; rMap: Record<string,string> }> = {};
      const slides: Slide[] = [];

      for (let i = 0; i < slideKeys.length; i++) {
        const sk   = slideKeys[i];
        const doc  = new DOMParser().parseFromString(await zip.files[sk].async("string"), "text/xml");
        const rMap = await parseRels(zip, sk);

        // Load slide layout
        let layoutPath = "";
        let layoutDoc: Document | null = null;
        let layoutRMap: Record<string,string> = {};
        for (const t of Object.values(rMap)) { if (t.includes("slideLayout")) { layoutPath = resolveZipPath(sk, t); break; } }
        if (layoutPath) {
          if (!layoutCache[layoutPath]) {
            try {
              const lXml = await zip.files[layoutPath]?.async("string");
              if (lXml) layoutCache[layoutPath] = { doc: new DOMParser().parseFromString(lXml,"text/xml"), rMap: await parseRels(zip, layoutPath) };
            } catch {}
          }
          if (layoutCache[layoutPath]) { layoutDoc = layoutCache[layoutPath].doc; layoutRMap = layoutCache[layoutPath].rMap; }
        }

        // Load slide master
        let masterPath = "";
        let masterDoc: Document | null = null;
        let masterRMap: Record<string,string> = {};
        for (const t of Object.values(layoutRMap)) { if (t.includes("slideMaster")) { masterPath = resolveZipPath(layoutPath, t); break; } }
        if (masterPath) {
          if (!masterCache[masterPath]) {
            try {
              const mXml = await zip.files[masterPath]?.async("string");
              if (mXml) masterCache[masterPath] = { doc: new DOMParser().parseFromString(mXml,"text/xml"), rMap: await parseRels(zip, masterPath) };
            } catch {}
          }
          if (masterCache[masterPath]) { masterDoc = masterCache[masterPath].doc; masterRMap = masterCache[masterPath].rMap; }
        }

        // Background: master → layout → slide (each overrides)
        let bgColor = "#" + (tc.lt1 ?? "FFFFFF");
        const masterBg = masterDoc ? getBgFromDoc(masterDoc, tc) : "";
        const layoutBg = layoutDoc ? getBgFromDoc(layoutDoc, tc) : "";
        const slideBg  = getBgFromDoc(doc, tc);
        if (masterBg) bgColor = masterBg;
        if (layoutBg) bgColor = layoutBg;
        if (slideBg)  bgColor = slideBg;

        const masterSpTree = masterDoc?.getElementsByTagNameNS(NSP, "spTree")[0];
        const layoutSpTree = layoutDoc?.getElementsByTagNameNS(NSP, "spTree")[0];
        const slideSpTree  = doc.getElementsByTagNameNS(NSP, "spTree")[0];

        const masterEls = masterSpTree ? await parseSpTree(masterSpTree, masterRMap, zip, tc, sw, sh, bgColor, masterPath||"ppt/slideMasters/slideMaster1.xml") : [];
        const layoutEls = layoutSpTree ? await parseSpTree(layoutSpTree, layoutRMap, zip, tc, sw, sh, bgColor, layoutPath||"ppt/slideLayouts/slideLayout1.xml") : [];
        const slideEls  = slideSpTree  ? await parseSpTree(slideSpTree,  rMap,        zip, tc, sw, sh, bgColor, sk) : [];

        slides.push({ index: i+1, bgColor, elements: [...masterEls, ...layoutEls, ...slideEls] });
      }
      pptxSlides = slides;
    } catch (e) {
      pptxSlides = [{ index:1, bgColor:"#1e1e2e", elements:[
        { kind:"text", x:5, y:35, w:90, h:30, isTitle:true,
          paras:[{ align:"center", runs:[{ text:`Error al cargar: ${e}`, bold:false, italic:false, size:14, color:"#ff6b6b" }] }] }
      ]}];
    } finally { pptxLoading = false; }
  }
</script>

<div class="file-block">
  {#if !data.stored_path}
    <div class="empty-file">
      <span>📄</span>
      <p>Sin documento</p>
    </div>

  {:else if data.file_type === "image"}
    <img src={fileUrl} alt={data.name} class="file-img" />
    <div class="file-footer">{data.name} · {formatSize(data.size)}</div>

  {:else if data.file_type === "video"}
    <video controls class="file-video">
      <source src={fileUrl} />
      <track kind="captions" />
    </video>
    <div class="file-footer">{data.name} · {formatSize(data.size)}</div>

  {:else if data.file_type === "pdf"}
    <div class="pdf-wrapper">
      <iframe src="{fileUrl}#toolbar=0&view=FitH" class="file-pdf" title={data.name} />
      <button class="open-btn" on:click={() => invoke('open_file', { path: data.stored_path })} title="Abrir con app del sistema">↗</button>
    </div>

  {:else if data.file_type === "word"}
    <div class="doc-wrapper">
      <!-- Toolbar -->
      <div class="doc-toolbar">
        <div class="doc-actions">
          {#if !wordEditing}
            <button class="doc-btn" on:click={async () => { wordEditing = true; await tick(); if (wordEditEl) { wordEditEl.innerHTML = wordHtml; wordEditEl.focus(); } }}>✏ Editar</button>
          {:else}
            <button class="doc-btn accent" disabled={wordSaving} on:click={saveWordEdit}>
              {wordSaving ? "⏳ Guardando…" : "💾 Guardar"}
            </button>
            <button class="doc-btn" on:click={() => wordEditing = false}>✕ Cancelar</button>
          {/if}
          <button class="doc-btn" on:click={() => invoke('open_file', { path: data.stored_path })} title="Abrir con Word">↗</button>
        </div>
      </div>

      {#if wordLoading}
        <div class="doc-loading">Cargando documento…</div>
      {:else if wordEditing}
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
          class="word-editor"
          bind:this={wordEditEl}
          contenteditable="true"
          role="textbox"
          tabindex="0"
          aria-multiline="true"
          aria-label="Editor de documento"
          on:keydown={e => e.ctrlKey && e.key === 's' && (e.preventDefault(), saveWordEdit())}
        >
          <!-- Will be filled via wordHtml reactivity -->
        </div>
      {:else}
        <div class="word-view">
          {@html wordHtml || "<p style='color:var(--text-muted);text-align:center;margin-top:40px'>Documento vacío</p>"}
        </div>
      {/if}
    </div>

  {:else if data.file_type === "excel"}
    <div class="excel-wrapper">
      <div class="doc-toolbar">
        <div class="sheet-tabs-inline">
          {#each excelSheets as sheet, i}
            <button class="sheet-tab" class:active={activeSheet === i}
              on:click={() => renderSheetGrid(i)}>{sheet}</button>
          {/each}
        </div>
        <div class="doc-actions">
          {#if excelEditing}
            <button class="doc-btn accent" disabled={excelSaving} on:click={saveExcel}>
              {excelSaving ? "⏳ Guardando…" : "💾 Guardar"}
            </button>
          {/if}
          <button class="doc-btn" on:click={() => invoke('open_file', { path: data.stored_path })} title="Abrir con Excel">↗</button>
        </div>
      </div>

      {#if excelLoading}
        <div class="doc-loading">Cargando hoja de cálculo…</div>
      {:else}
        <div class="excel-scroll">
          <table class="excel-table">
            <thead>
              <tr>
                <th class="excel-row-header"></th>
                {#each excelGrid[0] ?? [] as _, c}
                  <th class="excel-col-header">{String.fromCharCode(65 + c)}</th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each excelGrid as row, r}
                <tr>
                  <td class="excel-row-header">{r + 1}</td>
                  {#each row as cell, c}
                    <td class="excel-cell">
                      <input
                        class="cell-input"
                        value={cell.v}
                        on:input={e => onCellInput(e, r, c)}
                        on:keydown={e => e.ctrlKey && e.key === 's' && (e.preventDefault(), saveExcel())}
                      />
                    </td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>

  {:else if data.file_type === "powerpoint"}
    <div class="pptx-wrapper">
      {#if pptxLoading}
        <div class="doc-loading">Cargando presentación…</div>
      {:else if pptxSlides.length > 0}
        <div class="pptx-layout">
          <!-- Sidebar con miniaturas reales -->
          <div class="pptx-sidebar">
            {#each pptxSlides as slide, i}
              <!-- svelte-ignore a11y-no-static-element-interactions -->
              <div
                class="pptx-thumb"
                class:active={pptxActiveSlide === i}
                on:click={() => pptxActiveSlide = i}
                on:keydown={e => e.key === "Enter" && (pptxActiveSlide = i)}
                role="button"
                tabindex="0"
              >
                <div class="thumb-slide" style="background:{slide.bgColor}">
                  {#each slide.elements as el, eli}
                    {#if el.kind === "shape" && el.fillColor}
                      <div class="thumb-abs"
                        style="left:{el.x}%;top:{el.y}%;width:{el.w}%;height:{el.h}%;background:{el.fillColor};border-radius:{el.geom==='ellipse'?'50%':el.geom==='roundRect'?'3px':'0'}" />
                    {:else if el.kind === "image" && el.src}
                      <img class="thumb-abs" src={el.src} alt=""
                        style="left:{el.x}%;top:{el.y}%;width:{el.w}%;height:{el.h}%" />
                    {:else if el.kind === "video" && el.src}
                      <div class="thumb-abs thumb-video-icon"
                        style="left:{el.x}%;top:{el.y}%;width:{el.w}%;height:{el.h}%">▶</div>
                    {:else if el.kind === "text" && el.paras}
                      <div class="thumb-abs thumb-text-el"
                        style="left:{el.x}%;top:{el.y}%;width:{el.w}%;height:{el.h}%">
                        {#each el.paras as p}
                          <div style="text-align:{p.align}">
                            {#each p.runs as r}
                              <span style="color:{r.color};font-weight:{r.bold?'bold':'normal'}">{r.text}</span>
                            {/each}
                          </div>
                        {/each}
                      </div>
                    {/if}
                  {/each}
                  <span class="thumb-num">{slide.index}</span>
                </div>
              </div>
            {/each}
          </div>

          <!-- Vista principal de la diapositiva -->
          <div class="pptx-main">
            {#if pptxSlides[pptxActiveSlide]}
              {@const slide = pptxSlides[pptxActiveSlide]}
              <div class="slide-stage-wrap">
                <div class="slide-canvas" style="background:{slide.bgColor}">
                  {#each slide.elements as el, eli}
                    {#if el.kind === "image" && el.src}
                      <img class="slide-abs" src={el.src} alt=""
                        style="left:{el.x}%;top:{el.y}%;width:{el.w}%;height:{el.h}%;object-fit:cover" />
                    {:else if el.kind === "video" && el.src}
                      {@const vk = `s${pptxActiveSlide}e${eli}`}
                      <div class="slide-abs slide-video-wrap"
                        style="left:{el.x}%;top:{el.y}%;width:{el.w}%;height:{el.h}%">
                        <!-- svelte-ignore a11y-media-has-caption -->
                        <video class="slide-video-inner" controls preload="metadata"
                          on:loadedmetadata={e => onVideoRef(e, vk)}
                          on:pause={() => { videoPaused = { ...videoPaused, [vk]: true }; }}
                          on:play={() =>  { videoPaused = { ...videoPaused, [vk]: false }; }}>
                          <source src={el.src} type={el.mime ?? "video/mp4"} />
                        </video>
                        {#if videoPaused[vk] !== false}
                          <!-- svelte-ignore a11y-no-static-element-interactions -->
                          <div class="video-play-overlay" role="button" tabindex="0"
                            on:click={() => videoEls[vk]?.play()}
                            on:keydown={e => e.key === "Enter" && videoEls[vk]?.play()}>
                            <span class="video-play-icon">▶</span>
                          </div>
                        {/if}
                      </div>
                    {:else if el.kind === "shape" && el.fillColor}
                      <div class="slide-abs slide-shape"
                        style="left:{el.x}%;top:{el.y}%;width:{el.w}%;height:{el.h}%;background:{el.fillColor};{el.strokeColor?`border:${(el.strokeW??1).toFixed(1)}px solid ${el.strokeColor};`:''}border-radius:{el.geom==='ellipse'?'50%':el.geom==='roundRect'?'8px':'0'};{el.rot?`transform:rotate(${el.rot}deg);`:''}" />
                    {:else if el.kind === "text" && el.paras}
                      <div class="slide-abs slide-text-box"
                        style="left:{el.x}%;top:{el.y}%;width:{el.w}%;height:{el.h}%">
                        {#each el.paras as para}
                          <p class="slide-para" style="text-align:{para.align}">
                            {#each para.runs as run}
                              <span style="color:{run.color};font-size:{run.size * 0.9}pt;font-weight:{run.bold?'bold':'normal'};font-style:{run.italic?'italic':'normal'}">{run.text}</span>
                            {/each}
                          </p>
                        {/each}
                      </div>
                    {/if}
                  {/each}
                </div>
              </div>
              <div class="slide-controls">
                <button class="slide-nav-btn" disabled={pptxActiveSlide === 0}
                  on:click={() => pptxActiveSlide--}>‹</button>
                <span class="slide-counter">{slide.index} / {pptxSlides.length}</span>
                <button class="slide-nav-btn" disabled={pptxActiveSlide === pptxSlides.length - 1}
                  on:click={() => pptxActiveSlide++}>›</button>
                <button class="slide-open-btn" on:click={() => invoke('open_file', { path: data.stored_path })} title="Abrir con PowerPoint">↗</button>
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <div class="doc-loading">Sin diapositivas</div>
      {/if}
    </div>

  {:else}
    <div class="empty-file">
      <span>📄</span>
      <p>{data.name}</p>
      <p class="size">{formatSize(data.size)}</p>
    </div>
  {/if}
</div>

<style>
  .file-block { display: flex; flex-direction: column; flex: 1; overflow: hidden; }

  .empty-file {
    flex: 1; display: flex; flex-direction: column;
    align-items: center; justify-content: center; gap: 8px;
    text-align: center; padding: 20px;
    background: linear-gradient(160deg, var(--bg-surface) 60%, var(--bg-overlay));
  }
  .empty-file span { font-size: 52px; filter: drop-shadow(0 3px 8px rgba(0,0,0,0.12)); line-height: 1; }
  .empty-file p { color: var(--text-secondary); font-size: 13px; font-weight: 600; max-width: 150px; word-break: break-word; line-height: 1.4; }
  .size { font-size: 11px !important; color: var(--text-muted) !important; font-weight: 400 !important; }

  .file-img  { flex: 1; width: 100%; height: 100%; object-fit: cover; object-position: top; min-height: 0; display: block; }
  .file-video { flex: 1; width: 100%; height: 100%; min-height: 0; display: block; }

  .pdf-wrapper {
    flex: 1; display: flex; flex-direction: column;
    overflow: hidden; min-height: 0; background: #525659; position: relative;
  }
  .open-btn {
    position: absolute; top: 8px; right: 8px; z-index: 10;
    background: rgba(0,0,0,0.55); color: #fff;
    border: 1px solid rgba(255,255,255,0.2); border-radius: 6px;
    padding: 4px 8px; font-size: 13px; line-height: 1;
    opacity: 0; transition: opacity 0.2s; backdrop-filter: blur(4px);
  }
  .pdf-wrapper:hover .open-btn { opacity: 1; }
  .file-pdf { flex: 1; width: 100%; height: 100%; border: none; min-height: 0; display: block; }

  .file-footer {
    padding: 5px 12px; font-size: 11px; color: var(--text-secondary);
    border-top: 1px solid var(--border); flex-shrink: 0;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    background: var(--bg-overlay); font-weight: 500; letter-spacing: 0.2px;
  }

  /* ── Word ── */
  .doc-wrapper { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0; }

  .doc-toolbar {
    display: flex; align-items: center; justify-content: space-between;
    padding: 4px 10px; background: var(--bg-surface);
    border-bottom: 1px solid var(--border); flex-shrink: 0; gap: 6px;
  }
  .doc-actions { display: flex; gap: 4px; flex-shrink: 0; }
  .doc-btn {
    padding: 3px 8px; font-size: 11px; border-radius: 4px;
    background: var(--bg-overlay); color: var(--text-secondary);
    border: 1px solid var(--border); cursor: pointer;
    transition: all 0.15s; white-space: nowrap;
  }
  .doc-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
  .doc-btn.accent { background: var(--accent); color: #fff; border-color: var(--accent); }
  .doc-btn.accent:hover { opacity: 0.85; }

  .doc-loading {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: var(--text-muted); font-size: 13px;
  }

  .word-view {
    flex: 1; overflow-y: auto; padding: 20px 24px;
    background: #fff; color: #1a1a1a;
    font-family: Georgia, 'Times New Roman', serif; font-size: 14px; line-height: 1.7;
  }
  /* Styles for mammoth HTML output */
  .word-view :global(h1) { font-size: 22px; font-weight: 700; margin: 16px 0 8px; }
  .word-view :global(h2) { font-size: 18px; font-weight: 700; margin: 14px 0 6px; }
  .word-view :global(h3) { font-size: 15px; font-weight: 600; margin: 12px 0 4px; }
  .word-view :global(p)  { margin: 0 0 8px; }
  .word-view :global(ul), .word-view :global(ol) { padding-left: 24px; margin: 8px 0; }
  .word-view :global(table) { border-collapse: collapse; margin: 12px 0; width: 100%; }
  .word-view :global(td), .word-view :global(th) { border: 1px solid #ccc; padding: 4px 8px; font-size: 13px; }

  .word-editor {
    flex: 1; overflow-y: auto; padding: 20px 24px;
    background: #fff; color: #1a1a1a;
    font-family: Georgia, 'Times New Roman', serif; font-size: 14px; line-height: 1.7;
    outline: none; border: 2px solid var(--accent);
    min-height: 0;
  }

  /* ── Excel ── */
  .excel-wrapper { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0; }

  .sheet-tabs-inline {
    display: flex; gap: 2px; overflow-x: auto; scrollbar-width: none; flex: 1;
  }
  .sheet-tab {
    padding: 3px 12px; font-size: 11px; border-radius: 4px;
    background: var(--bg-overlay); color: var(--text-muted);
    border: 1px solid var(--border); cursor: pointer;
    white-space: nowrap; transition: all 0.15s;
  }
  .sheet-tab.active { background: var(--accent-dim); color: var(--accent); border-color: var(--accent); }
  .sheet-tab:hover:not(.active) { color: var(--text-primary); background: var(--bg-hover); }

  .excel-scroll { flex: 1; overflow: auto; background: #fff; min-height: 0; }

  .excel-table {
    border-collapse: collapse; font-size: 12px; font-family: monospace;
    color: #111; white-space: nowrap; width: max-content;
  }
  .excel-row-header {
    background: #f3f4f6; font-weight: 600; font-size: 11px; color: #666;
    border: 1px solid #d0d0d0; padding: 2px 6px; text-align: center;
    min-width: 32px; position: sticky; left: 0; z-index: 2;
    user-select: none;
  }
  thead .excel-row-header { position: sticky; top: 0; left: 0; z-index: 3; }
  .excel-col-header {
    background: #f3f4f6; font-weight: 600; font-size: 11px; color: #666;
    border: 1px solid #d0d0d0; padding: 2px 8px; text-align: center;
    min-width: 80px; position: sticky; top: 0; z-index: 1;
    user-select: none;
  }
  .excel-cell {
    border: 1px solid #e0e0e0; padding: 0; min-width: 80px;
  }
  .cell-input {
    width: 100%; min-width: 80px; padding: 2px 6px;
    border: none; outline: none; background: transparent;
    font-size: 12px; font-family: monospace; color: #111;
    height: 22px; box-sizing: border-box;
  }
  .cell-input:focus { background: #fffbea; outline: 2px solid #4a90e2; outline-offset: -1px; }

  /* ── PowerPoint ── */
  .pptx-wrapper { flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0; }
  .pptx-layout  { flex: 1; display: flex; overflow: hidden; min-height: 0; }

  /* Sidebar thumbnails */
  .pptx-sidebar {
    width: 76px; flex-shrink: 0; overflow-y: auto; overflow-x: hidden;
    background: var(--bg-surface); border-right: 1px solid var(--border);
    display: flex; flex-direction: column; gap: 5px; padding: 6px;
    scrollbar-width: thin;
  }
  .pptx-thumb {
    width: 64px; border: 2px solid transparent; border-radius: 3px;
    overflow: hidden; cursor: pointer; padding: 0; background: none;
    transition: border-color 0.15s; flex-shrink: 0;
  }
  .pptx-thumb:hover   { border-color: var(--border); }
  .pptx-thumb.active  { border-color: var(--accent); }
  .thumb-slide {
    width: 64px; height: 36px; position: relative; overflow: hidden;
  }
  .thumb-abs {
    position: absolute; overflow: hidden;
  }
  .thumb-text-el {
    font-size: 3px; line-height: 1.2; display: flex; flex-direction: column;
  }
  .thumb-num {
    position: absolute; bottom: 1px; right: 2px;
    font-size: 6px; color: rgba(255,255,255,0.55);
    text-shadow: 0 0 2px rgba(0,0,0,0.8);
  }

  /* Main slide view */
  .pptx-main {
    flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0;
    background: #1a1a1a; padding: 10px 10px 6px; gap: 6px;
    align-items: center; justify-content: center;
  }
  .slide-stage-wrap {
    flex: 1; width: 100%; display: flex; align-items: center;
    justify-content: center; min-height: 0; overflow: hidden;
  }
  .slide-canvas {
    position: relative; overflow: visible;
    /* Mantener aspect ratio 16:9 */
    aspect-ratio: 16 / 9;
    width: 100%; max-height: 100%;
    max-width: min(100%, calc((100% - 0px) * 9 / 5));
    box-shadow: 0 4px 24px rgba(0,0,0,0.5);
  }
  .slide-abs {
    position: absolute;
  }
  .slide-video-wrap { overflow: visible; }
  .slide-video-inner { width: 100%; height: 100%; object-fit: contain; background: #000; display: block; }
  .video-play-overlay {
    position: absolute; inset: 0;
    display: flex; align-items: center; justify-content: center;
    background: rgba(0,0,0,0.35); cursor: pointer; border-radius: 2px;
    transition: background 0.15s;
  }
  .video-play-overlay:hover { background: rgba(0,0,0,0.5); }
  .video-play-icon {
    font-size: 32px; color: #fff; line-height: 1;
    filter: drop-shadow(0 2px 6px rgba(0,0,0,0.6));
  }
  .thumb-video-icon {
    display: flex; align-items: center; justify-content: center;
    font-size: 7px; color: rgba(255,255,255,0.8);
    background: rgba(0,0,0,0.45); border-radius: 2px;
  }
  .slide-text-box {
    display: flex; flex-direction: column; justify-content: flex-start;
    overflow: hidden; box-sizing: border-box;
  }
  .slide-para { margin: 0; line-height: 1.25; }
  .slide-shape { pointer-events: none; box-sizing: border-box; }

  /* Slide navigation controls */
  .slide-controls {
    display: flex; gap: 10px; align-items: center; flex-shrink: 0;
  }
  .slide-nav-btn {
    padding: 4px 14px; font-size: 15px; border-radius: 5px;
    background: rgba(255,255,255,0.1); color: rgba(255,255,255,0.75);
    border: 1px solid rgba(255,255,255,0.15); cursor: pointer; transition: all 0.15s;
  }
  .slide-nav-btn:hover:not(:disabled) { background: rgba(255,255,255,0.2); color: #fff; }
  .slide-nav-btn:disabled { opacity: 0.3; cursor: not-allowed; }
  .slide-counter { font-size: 11px; color: rgba(255,255,255,0.45); }
  .slide-open-btn {
    padding: 3px 8px; font-size: 11px; border-radius: 4px;
    background: rgba(255,255,255,0.08); color: rgba(255,255,255,0.55);
    border: 1px solid rgba(255,255,255,0.12); cursor: pointer; transition: all 0.15s;
  }
  .slide-open-btn:hover { background: rgba(255,255,255,0.18); color: #fff; }
</style>
