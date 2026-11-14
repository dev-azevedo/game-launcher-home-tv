//! Itens da Steam que aparecem como app instalado mas nao sao jogos (P-05 a, RN-28, CA-69).
//! Lista fixa mantida no launcher; o que ela nao cobrir o usuario oculta (CA-21).
//!
//! Confirmado: 228980. Os demais sao candidatos a conferir com loja instalada (QA-17, QF-04).

const NON_GAMES: &[u32] = &[
    228980,  // Steamworks Common Redistributables (confirmado)
    250820,  // SteamVR
    1070560, // Steam Linux Runtime 1.0 (scout)
    1391110, // Steam Linux Runtime 2.0 (soldier)
    1628350, // Steam Linux Runtime 3.0 (sniper)
    1493710, // Proton Experimental
    2180100, // Proton Hotfix
];

pub fn is_steam_non_game(app_id: u32) -> bool {
    NON_GAMES.contains(&app_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lista_fixa_filtra_so_o_que_nao_e_jogo() {
        assert!(is_steam_non_game(228980));
        assert!(is_steam_non_game(1493710));
        assert!(!is_steam_non_game(620));
        assert!(!is_steam_non_game(440));
    }
}
