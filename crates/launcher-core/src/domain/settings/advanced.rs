//! `settings.json -> advanced` (arquitetura 9.3, SEC-18 a, SEC-28 rev. 5).
//!
//! Cada valor tem faixa validada. Valor fora da faixa, do tipo ou chave desconhecida cai no
//! padrao da spec, com um aviso para o log; **nunca** rejeita o arquivo inteiro por um valor
//! avancado. Nenhuma chave aqui desliga confirmacoes (RN-14), travas de SEC-15 a SEC-17 ou o
//! ticket de SEC-27: essas regras nao tem parametro.

use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step2Key {
    F24,
    Alt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundSettings {
    /// Permutacao de 1..4 (ordem dos degraus da escada, ADR-0003).
    pub order: [u8; 4],
    pub step1: bool,
    pub step2: bool,
    pub step3: bool,
    pub step4: bool,
    pub step2_key: Step2Key,
    pub verify_delay_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSettings {
    pub start_timeout_seconds: u32,
    pub end_grace_seconds: u32,
    pub close_timeout_seconds: u32,
    pub probe_interval_ms: u32,
    pub running_probe_interval_ms: u32,
    pub use_steam_running_app_id: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedSettings {
    pub foreground: ForegroundSettings,
    pub session: SessionSettings,
    pub deadzone_percent: u32,
    pub log_level: LogLevel,
}

impl Default for AdvancedSettings {
    /// Padroes da spec e dos spikes (9.3). Degraus 2 e 3 ligados: cenario A, SG-02.
    fn default() -> Self {
        Self {
            foreground: ForegroundSettings {
                order: [1, 3, 2, 4],
                step1: true,
                step2: true,
                step3: true,
                step4: true,
                step2_key: Step2Key::F24,
                verify_delay_ms: 150,
            },
            session: SessionSettings {
                start_timeout_seconds: 60,
                end_grace_seconds: 5,
                close_timeout_seconds: 10,
                probe_interval_ms: 1000,
                running_probe_interval_ms: 2000,
                use_steam_running_app_id: true,
            },
            deadzone_percent: 25,
            log_level: LogLevel::Info,
        }
    }
}

/// Resultado do parse: valores efetivos + avisos (um por valor trocado pelo padrao).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvancedParse {
    pub settings: AdvancedSettings,
    pub warnings: Vec<String>,
}

struct Reader<'a> {
    obj: Option<&'a Map<String, Value>>,
    scope: &'static str,
    warnings: &'a mut Vec<String>,
}

impl Reader<'_> {
    fn raw(&self, key: &str) -> Option<&Value> {
        self.obj.and_then(|o| o.get(key))
    }

    fn warn(&mut self, key: &str, why: &str) {
        self.warnings.push(format!(
            "advanced.{}.{key}: {why}; usando o padrao",
            self.scope
        ));
    }

    fn bool(&mut self, key: &str, default: bool) -> bool {
        match self.raw(key) {
            None => default,
            Some(Value::Bool(b)) => *b,
            Some(_) => {
                self.warn(key, "esperado booleano");
                default
            }
        }
    }

    fn uint(&mut self, key: &str, default: u32, min: u32, max: u32) -> u32 {
        match self.raw(key) {
            None => default,
            Some(v) => match v
                .as_u64()
                .filter(|n| (u64::from(min)..=u64::from(max)).contains(n))
            {
                Some(n) => n as u32,
                None => {
                    self.warn(key, &format!("esperado inteiro entre {min} e {max}"));
                    default
                }
            },
        }
    }

    fn choice<T: Copy>(&mut self, key: &str, default: T, options: &[(&str, T)]) -> T {
        match self.raw(key) {
            None => default,
            Some(Value::String(s)) => match options.iter().find(|(name, _)| name == s) {
                Some((_, v)) => *v,
                None => {
                    self.warn(key, "valor fora do conjunto permitido");
                    default
                }
            },
            Some(_) => {
                self.warn(key, "esperado texto");
                default
            }
        }
    }
}

fn section<'a>(
    root: Option<&'a Map<String, Value>>,
    name: &'static str,
    warnings: &mut Vec<String>,
) -> Option<&'a Map<String, Value>> {
    match root.and_then(|r| r.get(name)) {
        None | Some(Value::Null) => None,
        Some(Value::Object(o)) => Some(o),
        Some(_) => {
            warnings.push(format!("advanced.{name}: esperado objeto; usando o padrao"));
            None
        }
    }
}

fn parse_order(raw: Option<&Value>, warnings: &mut Vec<String>) -> [u8; 4] {
    let default = AdvancedSettings::default().foreground.order;
    let Some(raw) = raw else { return default };
    let parsed: Option<Vec<u8>> = raw.as_array().and_then(|a| {
        a.iter()
            .map(|v| v.as_u64().and_then(|n| u8::try_from(n).ok()))
            .collect()
    });
    match parsed {
        Some(v) if v.len() == 4 => {
            let mut sorted = v.clone();
            sorted.sort_unstable();
            if sorted == [1, 2, 3, 4] {
                return [v[0], v[1], v[2], v[3]];
            }
            warnings.push(
                "advanced.foreground.order: nao e permutacao de 1..4; usando o padrao".into(),
            );
            default
        }
        _ => {
            warnings.push(
                "advanced.foreground.order: esperado lista de 4 inteiros; usando o padrao".into(),
            );
            default
        }
    }
}

const KNOWN: &[(&str, &[&str])] = &[
    (
        "foreground",
        &[
            "order",
            "step1",
            "step2",
            "step3",
            "step4",
            "step2Key",
            "verifyDelayMs",
        ],
    ),
    (
        "session",
        &[
            "startTimeoutSeconds",
            "endGraceSeconds",
            "closeTimeoutSeconds",
            "probeIntervalMs",
            "runningProbeIntervalMs",
            "useSteamRunningAppId",
        ],
    ),
    ("input", &["deadzonePercent"]),
    ("diagnostics", &["level"]),
];

fn warn_unknown(root: &Map<String, Value>, warnings: &mut Vec<String>) {
    for (key, value) in root {
        match KNOWN.iter().find(|(name, _)| name == key) {
            None => warnings.push(format!("advanced.{key}: chave desconhecida ignorada")),
            Some((name, keys)) => {
                if let Value::Object(inner) = value {
                    for k in inner.keys().filter(|k| !keys.contains(&k.as_str())) {
                        warnings.push(format!("advanced.{name}.{k}: chave desconhecida ignorada"));
                    }
                }
            }
        }
    }
}

impl AdvancedSettings {
    /// Le a secao `advanced` crua. `None` ou tipo diferente de objeto = tudo no padrao.
    pub fn from_value(raw: Option<&Value>) -> AdvancedParse {
        let mut warnings = Vec::new();
        let root = match raw {
            None | Some(Value::Null) => None,
            Some(Value::Object(o)) => Some(o),
            Some(_) => {
                warnings.push("advanced: esperado objeto; usando os padroes".to_owned());
                None
            }
        };
        if let Some(r) = root {
            warn_unknown(r, &mut warnings);
        }
        let d = AdvancedSettings::default();

        let fg_obj = section(root, "foreground", &mut warnings);
        let order = parse_order(fg_obj.and_then(|o| o.get("order")), &mut warnings);
        let mut fg = Reader {
            obj: fg_obj,
            scope: "foreground",
            warnings: &mut warnings,
        };
        let foreground = ForegroundSettings {
            order,
            step1: fg.bool("step1", d.foreground.step1),
            step2: fg.bool("step2", d.foreground.step2),
            step3: fg.bool("step3", d.foreground.step3),
            step4: fg.bool("step4", d.foreground.step4),
            step2_key: fg.choice(
                "step2Key",
                d.foreground.step2_key,
                &[("F24", Step2Key::F24), ("Alt", Step2Key::Alt)],
            ),
            verify_delay_ms: fg.uint("verifyDelayMs", d.foreground.verify_delay_ms, 50, 500),
        };

        let session_obj = section(root, "session", &mut warnings);
        let mut se = Reader {
            obj: session_obj,
            scope: "session",
            warnings: &mut warnings,
        };
        let session = SessionSettings {
            start_timeout_seconds: se.uint(
                "startTimeoutSeconds",
                d.session.start_timeout_seconds,
                30,
                300,
            ),
            end_grace_seconds: se.uint("endGraceSeconds", d.session.end_grace_seconds, 2, 30),
            close_timeout_seconds: se.uint(
                "closeTimeoutSeconds",
                d.session.close_timeout_seconds,
                5,
                60,
            ),
            probe_interval_ms: se.uint("probeIntervalMs", d.session.probe_interval_ms, 500, 5000),
            running_probe_interval_ms: se.uint(
                "runningProbeIntervalMs",
                d.session.running_probe_interval_ms,
                1000,
                10_000,
            ),
            use_steam_running_app_id: se
                .bool("useSteamRunningAppId", d.session.use_steam_running_app_id),
        };

        let input_obj = section(root, "input", &mut warnings);
        let mut inp = Reader {
            obj: input_obj,
            scope: "input",
            warnings: &mut warnings,
        };
        let deadzone_percent = inp.uint("deadzonePercent", d.deadzone_percent, 10, 50);

        let diag_obj = section(root, "diagnostics", &mut warnings);
        let mut dg = Reader {
            obj: diag_obj,
            scope: "diagnostics",
            warnings: &mut warnings,
        };
        let log_level = dg.choice(
            "level",
            d.log_level,
            &[("info", LogLevel::Info), ("debug", LogLevel::Debug)],
        );

        AdvancedParse {
            settings: AdvancedSettings {
                foreground,
                session,
                deadzone_percent,
                log_level,
            },
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(v: Value) -> AdvancedParse {
        AdvancedSettings::from_value(Some(&v))
    }

    #[test]
    fn ausente_nulo_ou_vazio_e_o_padrao_da_spec_sem_aviso() {
        for raw in [None, Some(json!(null)), Some(json!({}))] {
            let p = AdvancedSettings::from_value(raw.as_ref());
            assert_eq!(p.settings, AdvancedSettings::default());
            assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        }
    }

    #[test]
    fn padroes_batem_com_a_tabela_9_3() {
        let d = AdvancedSettings::default();
        assert_eq!(d.foreground.order, [1, 3, 2, 4]);
        assert!(
            d.foreground.step1 && d.foreground.step2 && d.foreground.step3 && d.foreground.step4
        );
        assert_eq!(d.foreground.step2_key, Step2Key::F24);
        assert_eq!(d.foreground.verify_delay_ms, 150);
        assert_eq!(d.session.start_timeout_seconds, 60);
        assert_eq!(d.session.end_grace_seconds, 5);
        assert_eq!(d.session.close_timeout_seconds, 10);
        assert_eq!(d.session.probe_interval_ms, 1000);
        assert_eq!(d.session.running_probe_interval_ms, 2000);
        assert!(d.session.use_steam_running_app_id);
        assert_eq!(d.deadzone_percent, 25);
        assert_eq!(d.log_level, LogLevel::Info);
    }

    #[test]
    fn valores_validos_sao_aplicados() {
        let p = parse(json!({
            "foreground": { "order": [4, 3, 2, 1], "step2": false, "step2Key": "Alt", "verifyDelayMs": 500 },
            "session": { "startTimeoutSeconds": 300, "endGraceSeconds": 2, "closeTimeoutSeconds": 5,
                         "probeIntervalMs": 500, "runningProbeIntervalMs": 10000, "useSteamRunningAppId": false },
            "input": { "deadzonePercent": 10 },
            "diagnostics": { "level": "debug" }
        }));
        assert!(p.warnings.is_empty(), "{:?}", p.warnings);
        let s = p.settings;
        assert_eq!(s.foreground.order, [4, 3, 2, 1]);
        assert!(!s.foreground.step2 && s.foreground.step3);
        assert_eq!(s.foreground.step2_key, Step2Key::Alt);
        assert_eq!(s.foreground.verify_delay_ms, 500);
        assert_eq!(s.session.start_timeout_seconds, 300);
        assert_eq!(s.session.end_grace_seconds, 2);
        assert_eq!(s.session.close_timeout_seconds, 5);
        assert_eq!(s.session.probe_interval_ms, 500);
        assert_eq!(s.session.running_probe_interval_ms, 10_000);
        assert!(!s.session.use_steam_running_app_id);
        assert_eq!(s.deadzone_percent, 10);
        assert_eq!(s.log_level, LogLevel::Debug);
    }

    /// Fora da faixa, do tipo ou do conjunto: padrao so naquele valor, com aviso, sem afetar os vizinhos.
    #[test]
    fn invalido_vira_padrao_so_naquele_valor() {
        let casos: Vec<(Value, &str)> = vec![
            (
                json!({ "foreground": { "verifyDelayMs": 49 } }),
                "foreground.verifyDelayMs",
            ),
            (
                json!({ "foreground": { "verifyDelayMs": 501 } }),
                "foreground.verifyDelayMs",
            ),
            (
                json!({ "foreground": { "verifyDelayMs": "150" } }),
                "foreground.verifyDelayMs",
            ),
            (
                json!({ "foreground": { "verifyDelayMs": -1 } }),
                "foreground.verifyDelayMs",
            ),
            (
                json!({ "foreground": { "verifyDelayMs": 150.5 } }),
                "foreground.verifyDelayMs",
            ),
            (
                json!({ "foreground": { "step3": "true" } }),
                "foreground.step3",
            ),
            (
                json!({ "foreground": { "step2Key": "Ctrl" } }),
                "foreground.step2Key",
            ),
            (
                json!({ "foreground": { "step2Key": 24 } }),
                "foreground.step2Key",
            ),
            (
                json!({ "foreground": { "order": [1, 1, 2, 3] } }),
                "foreground.order",
            ),
            (
                json!({ "foreground": { "order": [1, 2, 3] } }),
                "foreground.order",
            ),
            (
                json!({ "foreground": { "order": [0, 1, 2, 3] } }),
                "foreground.order",
            ),
            (
                json!({ "foreground": { "order": "1324" } }),
                "foreground.order",
            ),
            (
                json!({ "session": { "startTimeoutSeconds": 29 } }),
                "session.startTimeoutSeconds",
            ),
            (
                json!({ "session": { "endGraceSeconds": 1 } }),
                "session.endGraceSeconds",
            ),
            (
                json!({ "session": { "closeTimeoutSeconds": 4 } }),
                "session.closeTimeoutSeconds",
            ),
            (
                json!({ "session": { "probeIntervalMs": 499 } }),
                "session.probeIntervalMs",
            ),
            (
                json!({ "session": { "runningProbeIntervalMs": 10001 } }),
                "session.runningProbeIntervalMs",
            ),
            (
                json!({ "session": { "useSteamRunningAppId": 1 } }),
                "session.useSteamRunningAppId",
            ),
            (
                json!({ "input": { "deadzonePercent": 9 } }),
                "input.deadzonePercent",
            ),
            (
                json!({ "input": { "deadzonePercent": 51 } }),
                "input.deadzonePercent",
            ),
            (
                json!({ "diagnostics": { "level": "trace" } }),
                "diagnostics.level",
            ),
        ];
        for (raw, key) in casos {
            let p = parse(raw.clone());
            assert_eq!(p.settings, AdvancedSettings::default(), "{raw}");
            assert!(
                p.warnings.iter().any(|w| w.contains(key)),
                "sem aviso para {key}: {:?}",
                p.warnings
            );
        }
    }

    #[test]
    fn valor_ruim_nao_contamina_os_valores_bons_vizinhos() {
        let p = parse(json!({
            "foreground": { "verifyDelayMs": 9999, "step1": false },
            "session": { "closeTimeoutSeconds": 20 }
        }));
        assert_eq!(p.settings.foreground.verify_delay_ms, 150);
        assert!(!p.settings.foreground.step1);
        assert_eq!(p.settings.session.close_timeout_seconds, 20);
        assert_eq!(p.warnings.len(), 1);
    }

    #[test]
    fn secao_ou_raiz_com_tipo_errado_usa_padrao_com_aviso() {
        for raw in [json!([]), json!("x"), json!(7), json!(true)] {
            let p = parse(raw);
            assert_eq!(p.settings, AdvancedSettings::default());
            assert_eq!(p.warnings.len(), 1);
        }
        let p = parse(json!({ "foreground": [], "session": 3, "input": "x", "diagnostics": null }));
        assert_eq!(p.settings, AdvancedSettings::default());
        assert_eq!(p.warnings.len(), 3); // `diagnostics: null` conta como ausente
    }

    #[test]
    fn chaves_desconhecidas_sao_ignoradas_com_aviso() {
        let p = parse(json!({
            "foreground": { "step9": true, "step1": false },
            "extra": 1
        }));
        assert!(!p.settings.foreground.step1);
        assert_eq!(p.warnings.len(), 2);
        assert!(p.warnings.iter().any(|w| w.contains("advanced.extra")));
        assert!(p.warnings.iter().any(|w| w.contains("foreground.step9")));
    }

    #[test]
    fn nenhuma_chave_avancada_toca_confirmacao_ticket_ou_travas() {
        // SEC-18 (a): a unica fonte de chaves e a tabela 9.3; nenhuma menciona essas regras.
        for (_, keys) in KNOWN {
            for k in *keys {
                let k = k.to_lowercase();
                for banned in ["confirm", "ticket", "force", "lock", "elev", "admin"] {
                    assert!(!k.contains(banned), "chave avancada suspeita: {k}");
                }
            }
        }
    }
}
