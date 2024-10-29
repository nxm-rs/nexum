const script = document.createElement('script');
script.src = chrome.runtime.getURL('injected.js');
script.type = 'module';
document.documentElement.appendChild(script);