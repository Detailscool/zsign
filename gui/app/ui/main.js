import { mount } from './workflow.js';
const { invoke } = window.__TAURI__.core;
const auth = {
  subscribe: handler => window.__TAURI__.event.listen('authentication-challenge', event => handler(event.payload)),
  login: (email, password, remoteUrl, remember) => invoke('login', { email, password, remoteUrl, remember }),
  restore: (email, remoteUrl) => invoke('restore_session', { email, remoteUrl }),
  signOut: () => invoke('sign_out'),
  listTeams: () => invoke('list_teams'),
  submitCode: code => invoke('submit_verification', { code }),
  sendSms: phoneId => invoke('send_sms', { phoneId }),
  cancel: () => invoke('cancel_verification'),
};
const signing = {
  subscribe: handler => window.__TAURI__.event.listen('signing-progress', event => handler(event.payload.message)),
  execute: token => invoke('execute_signing', { token }),
};
mount(document, () => invoke('select_ipa'), () => invoke('list_devices'), auth,
  (ipaPath, teamId, deviceId, bundleId) => invoke('prepare_signing', { ipaPath, teamId, deviceId, bundleId }), signing);
