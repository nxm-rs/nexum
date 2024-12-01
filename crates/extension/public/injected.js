import __wbg_init, { initialize_provider } from "./injected/injected.js";

async function init() {
    await __wbg_init();
    initialize_provider();
}

init();