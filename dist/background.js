chrome.runtime.onInstalled.addListener(() => {
    console.log('Background service worker installed');
});

// Example: You could handle long-running Ethereum-related tasks here
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
    if (message.type === 'eth_request') {
        // Handle Ethereum requests here if needed
    }
});
