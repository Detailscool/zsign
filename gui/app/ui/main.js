import { mount } from './workflow.js';
mount(document, () => window.__TAURI__.core.invoke('select_ipa'));
