// Sem console no release. SetDefaultDllDirectories (SEC-23) e o desvio de
// --purge-user-data entram aqui na BE-09 e BE-41.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sofa_lib::run();
}
