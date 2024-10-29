// chrome_bindings.js

// --- Action

// Wrapper for chrome.action.setIcon
export function setIcon(details) {
    chrome.action.setIcon(details);
}

// Wrapper for chrome.action.setPopup
export function setPopup(details) {
    chrome.action.setPopup(details);
}

// --- Runtime

// Wrapper for chrome.runtime.onMessage listener
export function addMessageListener(callback) {
    chrome.runtime.onMessage.addListener(callback);
}

// Wrapper for chrome.runtime.onConnect listener
export function addConnectListener(callback) {
    chrome.runtime.onConnect.addListener(callback);
}

// --- Tabs

// Wrapper for chrome.tabs.sendMessage, returning a Promise
export function sendMessageToTab(tabId, message) {
    return new Promise((resolve, reject) => {
        chrome.tabs.sendMessage(tabId, message, (response) => {
            if (chrome.runtime.lastError) {
                reject(chrome.runtime.lastError.message);
            } else {
                resolve(response);
            }
        });
    });
}

// Wrapper for chrome.tabs.query, returning a Promise
export function queryTabs(queryInfo) {
    return new Promise((resolve, reject) => {
        chrome.tabs.query(queryInfo, (tabs) => {
            if (chrome.runtime.lastError) {
                reject(chrome.runtime.lastError.message);
            } else {
                resolve(tabs);
            }
        });
    });
}

// Wrapper for chrome.tabs.get, returning a Promise that resolves to a Tab
export function getTab(tabId) {
    return new Promise((resolve, reject) => {
        chrome.tabs.get(tabId, (tab) => {
            if (chrome.runtime.lastError) {
                reject(chrome.runtime.lastError.message);
            } else {
                resolve(tab);
            }
        });
    });
}

// Wrapper for chrome.alarms.get, returning a Promise that resolves to an Alarm
export function getAlarm(name) {
    return new Promise((resolve, reject) => {
        chrome.alarms.get(name, (alarm) => {
            if (chrome.runtime.lastError) {
                reject(chrome.runtime.lastError.message);
            } else {
                resolve(alarm);
            }
        });
    });
}

export function addTabRemovedListener(callback) {
    chrome.tabs.onRemoved.addListener(callback);
}

export function addTabUpdatedListener(callback) {
    chrome.tabs.onUpdated.addListener(callback);
}

export function addTabActivatedListener(callback) {
    chrome.tabs.onActivated.addListener(callback);
}

// --- Alarms

// Wrapper for chrome.alarms.create
export function createAlarm(name, info) {
    chrome.alarms.create(name, info);
}

// Wrapper for chrome.alarms.onAlarm listener
export function addAlarmListener(callback) {
    chrome.alarms.onAlarm.addListener(callback);
}

// --- Ports

export function portAddOnDisconnectListener(port, callback) {
    port.onDisconnect.addListener(callback);
}

export function portRemoveOnDisconnectListener(port, callback) {
    port.onDisconnect.removeListener(callback);
}

export function portPostMessage(port, message) {
    port.postMessage(message);
}