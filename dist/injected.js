import init, { Eip1193Provider } from './pkg/ferris_wallet.js';

(async () => {
    // Initialize the WASM module
    await init();

    // Create the provider
    const provider = new Eip1193Provider();

    // Assign the provider to the window.ethereum object
    window.ethereum = {
        request: async (request) => {
            return provider.request(request);
        },
        on: (event, callback) => {
            provider.on(event, callback);
        },
        removeListener: (event, callback) => {
            provider.remove_listener(event, callback);
        }
    };

    // Example: Simulate a connect event
    provider.trigger_event('connect');
})();
