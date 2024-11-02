import __wbg_init, { initialize_extension } from "./pkg/worker.js";

async function init() {
  await __wbg_init();
  await initialize_extension();
}

init();