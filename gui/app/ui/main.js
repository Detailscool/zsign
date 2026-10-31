import { mount } from './workflow.js';
const { invoke } = window.__TAURI__.core;
const auth = {
  subscribe: handler => window.__TAURI__.event.listen('authentication-challenge', event => handler(event.payload)),
  login: (email, password, remoteUrl) => invoke('login', { email, password, remoteUrl }),
  listTeams: () => invoke('list_teams'),
  submitCode: code => invoke('submit_verification', { code }),
  sendSms: phoneId => invoke('send_sms', { phoneId }),
  cancel: () => invoke('cancel_verification'),
};
mount(document, () => invoke('select_ipa'), () => invoke('list_devices'), auth);
