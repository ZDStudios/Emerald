/*
 * Chrome entry point.
 *
 * Fonts are bundled rather than fetched: Emerald makes no network request the
 * user did not ask for, and that includes its own UI. `400` and `500` of
 * JetBrains Mono are the only weights the chrome uses; the reading faces are
 * loaded here too so the settings panel's live preview is accurate the moment
 * it opens.
 */

import { render } from 'solid-js/web';

import '@fontsource/jetbrains-mono/400.css';
import '@fontsource/jetbrains-mono/500.css';
import '@fontsource/jetbrains-mono/700.css';
import '@fontsource/atkinson-hyperlegible/400.css';
import '@fontsource/atkinson-hyperlegible/700.css';
import '@fontsource/opendyslexic/400.css';
import '@fontsource/opendyslexic/700.css';

import './styles/tokens.css';
import './styles/chrome.css';
import { App } from './App';

const root = document.getElementById('root');
if (!root) throw new Error('missing #root');
render(() => <App />, root);
