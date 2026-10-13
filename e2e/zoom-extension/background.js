// Test-only extension; Chrome's native tab zoom rather than CSS magnification.
// https://developer.chrome.com/docs/extensions/reference/api/tabs#method-setZoom
chrome.tabs.onUpdated.addListener((id, change, tab) => {
  if (change.status === "complete" && tab.url?.startsWith("http://127.0.0.1:1420/")) {
    chrome.tabs.setZoom(id, 2);
  }
});
