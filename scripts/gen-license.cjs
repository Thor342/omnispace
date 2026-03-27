/**
 * Inyecta la versión actual de tauri.conf.json en el license.rtf antes del build.
 */
const fs = require("fs");
const path = require("path");

const conf = JSON.parse(fs.readFileSync(path.join(__dirname, "../src-tauri/tauri.conf.json"), "utf8"));
const version = conf.version;

const rtfPath = path.join(__dirname, "../src-tauri/resources/license.rtf");
let rtf = fs.readFileSync(rtfPath, "utf8");

// Reemplaza cualquier "Versión X.X.X" en el encabezado del RTF
// En RTF la ó se codifica como \'f3, el \ es literal en el archivo
rtf = rtf.replace(
  /Versi\\'f3n \d+\.\d+\.\d+/,
  `Versi\\'f3n ${version}`
);

fs.writeFileSync(rtfPath, rtf, "utf8");
console.log(`license.rtf actualizado a v${version}`);
