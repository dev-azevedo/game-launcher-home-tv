// Sem console no release. A 1a instrucao e a trava de busca de DLL (SEC-23); o desvio de
// --purge-user-data entra logo depois com o BE-41.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    launcher_win::hardening::set_default_dll_directories();
    sofa_lib::run();
}
