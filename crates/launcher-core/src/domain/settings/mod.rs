//! `settings.json` (ADR-0004): monitor, marca de `InstallDefaults` aplicado e a secao `advanced`.
//!
//! Schema fechado no nivel de cima (`deny_unknown_fields`); a secao `advanced` e preservada crua
//! em toda gravacao (SEC-28 rev. 5: o app nunca altera `advanced` por conta propria) e lida com
//! tolerancia por valor (`AdvancedSettings::from_value`).

pub mod advanced;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use advanced::{
    AdvancedParse, AdvancedSettings, ForegroundSettings, LogLevel, SessionSettings, Step2Key,
};

/// Versao do formato gravado. Mudanca de formato passa por `migrate` (puro e testado).
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

const MAX_MONITOR_ID_CHARS: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub monitor_id: Option<String>,
    /// `apply_install_default_once` ja rodou neste perfil (BE-36).
    pub install_defaults_applied: bool,
    /// Secao `advanced` como esta no arquivo; devolvida intacta ao gravar.
    pub advanced_raw: Value,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            monitor_id: None,
            install_defaults_applied: false,
            advanced_raw: Value::Object(serde_json::Map::new()),
        }
    }
}

impl Settings {
    /// Valores efetivos de `advanced` (padrao onde o arquivo estiver invalido) e avisos para o log.
    pub fn advanced(&self) -> AdvancedParse {
        AdvancedSettings::from_value(Some(&self.advanced_raw))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingsFileError {
    #[error("JSON invalido ou fora do schema")]
    Schema,
    #[error("schemaVersion ausente ou desconhecido ({0})")]
    Version(u64),
    #[error("valor fora dos limites")]
    Limits,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettingsFile {
    schema_version: u32,
    #[serde(default)]
    monitor_id: Option<String>,
    #[serde(default)]
    install_defaults_applied: bool,
    #[serde(default = "empty_object")]
    advanced: Value,
}

fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}

/// Leva o JSON de qualquer versao conhecida ate a atual. Hoje so existe a versao 1.
fn migrate(mut value: Value) -> Result<Value, SettingsFileError> {
    let version = value
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .ok_or(SettingsFileError::Version(0))?;
    match version {
        1 => Ok(value),
        other => {
            // Futuro: `0 -> 1`, `1 -> 2` ... em sequencia, com fixture por passo.
            let _ = &mut value;
            Err(SettingsFileError::Version(other))
        }
    }
}

pub fn parse_settings(bytes: &[u8]) -> Result<Settings, SettingsFileError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| SettingsFileError::Schema)?;
    let value = migrate(value)?;
    let file: SettingsFile =
        serde_json::from_value(value).map_err(|_| SettingsFileError::Schema)?;
    if let Some(id) = &file.monitor_id {
        let n = id.chars().count();
        if !(1..=MAX_MONITOR_ID_CHARS).contains(&n) {
            return Err(SettingsFileError::Limits);
        }
    }
    Ok(Settings {
        monitor_id: file.monitor_id,
        install_defaults_applied: file.install_defaults_applied,
        advanced_raw: file.advanced,
    })
}

pub fn serialize_settings(settings: &Settings) -> Vec<u8> {
    let file = SettingsFile {
        schema_version: SETTINGS_SCHEMA_VERSION,
        monitor_id: settings.monitor_id.clone(),
        install_defaults_applied: settings.install_defaults_applied,
        advanced: settings.advanced_raw.clone(),
    };
    // Serializar estruturas proprias nao falha.
    serde_json::to_vec_pretty(&file).expect("settings serializaveis")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(v: Value) -> Result<Settings, SettingsFileError> {
        parse_settings(v.to_string().as_bytes())
    }

    #[test]
    fn arquivo_minimo_usa_padroes() {
        let s = parse(json!({ "schemaVersion": 1 })).unwrap();
        assert_eq!(s, Settings::default());
        assert!(s.advanced().warnings.is_empty());
    }

    #[test]
    fn campos_validos_sao_lidos() {
        let s = parse(json!({
            "schemaVersion": 1, "monitorId": "DISPLAY1", "installDefaultsApplied": true,
            "advanced": { "input": { "deadzonePercent": 30 } }
        }))
        .unwrap();
        assert_eq!(s.monitor_id.as_deref(), Some("DISPLAY1"));
        assert!(s.install_defaults_applied);
        assert_eq!(s.advanced().settings.deadzone_percent, 30);
    }

    #[test]
    fn schema_fechado_no_nivel_de_cima() {
        for raw in [
            json!({ "schemaVersion": 1, "autostart": true }),
            json!({ "schemaVersion": 1, "launch": { "args": "-x" } }),
            json!({ "schemaVersion": 1, "monitorId": 5 }),
            json!({ "schemaVersion": 1, "installDefaultsApplied": "yes" }),
        ] {
            assert_eq!(parse(raw.clone()), Err(SettingsFileError::Schema), "{raw}");
        }
        assert_eq!(
            parse_settings(b"{ nao e json"),
            Err(SettingsFileError::Schema)
        );
        assert_eq!(parse_settings(b"[]"), Err(SettingsFileError::Version(0)));
    }

    #[test]
    fn versao_ausente_ou_desconhecida_e_recusada() {
        assert_eq!(parse(json!({})), Err(SettingsFileError::Version(0)));
        assert_eq!(
            parse(json!({ "schemaVersion": "1" })),
            Err(SettingsFileError::Version(0))
        );
        assert_eq!(
            parse(json!({ "schemaVersion": 2 })),
            Err(SettingsFileError::Version(2))
        );
        assert_eq!(
            parse(json!({ "schemaVersion": 0 })),
            Err(SettingsFileError::Version(0))
        );
    }

    #[test]
    fn monitor_id_respeita_o_limite() {
        let longo = "m".repeat(257);
        assert_eq!(
            parse(json!({ "schemaVersion": 1, "monitorId": longo })),
            Err(SettingsFileError::Limits)
        );
        assert_eq!(
            parse(json!({ "schemaVersion": 1, "monitorId": "" })),
            Err(SettingsFileError::Limits)
        );
        assert!(parse(json!({ "schemaVersion": 1, "monitorId": "m".repeat(256) })).is_ok());
    }

    /// SEC-18 (a): valor avancado invalido nunca derruba o arquivo; so vira padrao.
    #[test]
    fn advanced_invalido_nao_rejeita_o_arquivo() {
        for adv in [
            json!({ "foreground": { "verifyDelayMs": "muito" } }),
            json!({ "foreground": 7 }),
            json!([1, 2, 3]),
            json!("texto"),
            json!({ "chave": { "desconhecida": true } }),
        ] {
            let s =
                parse(json!({ "schemaVersion": 1, "monitorId": "M", "advanced": adv })).unwrap();
            assert_eq!(s.monitor_id.as_deref(), Some("M"));
            assert_eq!(s.advanced().settings, AdvancedSettings::default(), "{adv}");
            assert!(!s.advanced().warnings.is_empty(), "{adv}");
        }
    }

    /// SEC-28 rev. 5: gravar nao altera `advanced`, nem o que esta invalido.
    #[test]
    fn gravar_preserva_advanced_cru() {
        let advanced = json!({
            "foreground": { "step2": false, "verifyDelayMs": 9999 },
            "chave-do-usuario": [1, 2, 3],
            "diagnostics": { "level": "debug" }
        });
        let mut s = parse(json!({ "schemaVersion": 1, "advanced": advanced })).unwrap();
        s.monitor_id = Some("DISPLAY2".into()); // o que o app muda (settings_set_monitor)
        let bytes = serialize_settings(&s);
        let again = parse_settings(&bytes).unwrap();
        assert_eq!(again.advanced_raw, advanced);
        assert_eq!(again.monitor_id.as_deref(), Some("DISPLAY2"));
        let doc: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(doc["schemaVersion"], 1);
    }

    #[test]
    fn ida_e_volta_do_padrao() {
        let s = Settings::default();
        assert_eq!(parse_settings(&serialize_settings(&s)).unwrap(), s);
    }
}
