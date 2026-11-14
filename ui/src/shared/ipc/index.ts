/** Fachada do IPC: unico ponto de entrada da UI para o nucleo (arch 2a). */
export { ipc, type Ipc } from './commands';
export { events, createCoreIntentSource } from './events';
export { IpcError, errorOf, toErrorDto } from './errors';
export * from './types';
