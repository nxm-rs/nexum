(async () => {
    const wasm = await import(chrome.runtime.getURL('./injector/injector.js'));
    await wasm.default();
})();