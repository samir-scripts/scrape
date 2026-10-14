// Serialized into the isolated WKWebView by test-native-startup.mjs.
export async function checkNativeAppearance() {
  const assert = (condition, message) => { if (!condition) throw new Error(message); };
  const wait = async (read, label) => {
    const deadline = Date.now() + 15000;
    while (Date.now() < deadline) {
      const value = read(); if (value) return value;
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    throw new Error(`Native UI deadline: ${label}`);
  };
  const button = label => [...document.querySelectorAll('button')].find(el => el.textContent.trim() === label);
  (await wait(() => button('SETTINGS'), 'settings navigation')).click();
  const picker = await wait(() => document.querySelector('[aria-label="Interface font"]'), 'font picker');
  for (const [id, family] of Object.entries({ inter: 'Inter Variable', jetbrains: 'JetBrains Mono Variable', geist: 'Geist Mono Variable', fira_code: 'Fira Code Variable', ibm_plex: 'IBM Plex Mono' })) {
    const faces = await document.fonts.load(`14px "${family}"`, 'Software Engineer 45.25');
    assert(faces.length && faces.every(face => face.status === 'loaded'), `Font did not load: ${family}`);
    picker.value = id; picker.dispatchEvent(new Event('change', { bubbles: true }));
    await wait(() => document.documentElement.dataset.font === id, `apply ${id}`);
    assert(getComputedStyle(picker).fontFamily.includes(family), `Picker font mismatch: ${id}`);
    assert(getComputedStyle(button('SETTINGS')).fontFamily.includes(family), `Navigation font mismatch: ${id}`);
  }
  const samples = [];
  for (const [theme, scheme] of [['paper','light'],['obsidian','dark']]) {
    const card = [...document.querySelectorAll('[role="button"]')].find(el => el.textContent.trim().startsWith(theme.toUpperCase()));
    assert(card, `Theme card missing: ${theme}`); card.click();
    await wait(() => document.documentElement.dataset.theme === theme, `theme ${theme}`);
    assert(getComputedStyle(document.documentElement).colorScheme === scheme, `Native color-scheme mismatch: ${theme}`);
    assert(getComputedStyle(picker).colorScheme === scheme, `Native select scheme mismatch: ${theme}`);
    const scroll = [...document.querySelectorAll('.overflow-y-auto')].find(el => el.scrollHeight > el.clientHeight);
    assert(scroll, 'Expected a real native scrolling surface');
    const standard = getComputedStyle(scroll).scrollbarColor;
    const fallback = getComputedStyle(scroll, '::-webkit-scrollbar-thumb').backgroundColor;
    assert((standard && standard !== 'auto') || (fallback && fallback !== 'rgba(0, 0, 0, 0)'), `Scrollbar unthemed: ${theme}`);
    for (const action of document.querySelectorAll('.ui-icon-button')) {
      const bounds = action.getBoundingClientRect(); if (!bounds.width || !bounds.height) continue;
      assert(bounds.width >= 32 && bounds.height >= 32, `Small native target: ${action.getAttribute('aria-label')}`);
      assert(action.getAttribute('aria-label'), 'Unnamed icon control');
      assert(action.querySelector('svg[aria-hidden="true"]'), 'Missing decorative SVG');
    }
    samples.push({ theme, scheme, scrollbarColor: standard, webkitThumb: fallback, userAgent: navigator.userAgent });
  }
  (await wait(() => button('SCRAPER'), 'scraper navigation')).click();
  (await wait(() => [...document.querySelectorAll('button')].find(el => el.textContent.includes('ADD WORKFLOW')), 'add workflow')).click();
  const dialog = await wait(() => document.querySelector('dialog[open]'), 'workflow dialog');
  assert(getComputedStyle(dialog).fontFamily.includes('IBM Plex Mono'), 'Dialog lost selected font');
  const input = dialog.querySelector('input');
  assert(input && getComputedStyle(input).fontFamily.includes('IBM Plex Mono'), 'Dialog field lost selected font');
  const close = dialog.querySelector('[aria-label="Close dialog"]');
  assert(close?.querySelector('svg[aria-hidden="true"]'), 'Dialog close not SVG');
  close.click();
  return samples;
}
