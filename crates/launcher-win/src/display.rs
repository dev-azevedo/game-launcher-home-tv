//! `DisplayTopology` no Win32 (BE-34, RN-20, CA-66, CA-67): monitores ligados, com id estavel.
//!
//! - Posicao e "principal": `EnumDisplayMonitors` + `GetMonitorInfoW` (nome GDI `\\.\DISPLAY1`).
//! - Id estavel entre sessoes: `monitorDevicePath` do `DisplayConfigGetDeviceInfo` (alvo do caminho
//!   ativo do `QueryDisplayConfig`), ligado ao monitor GDI pelo nome da fonte. O nome GDI muda de
//!   numero quando um monitor e ligado ou desligado; o caminho do dispositivo nao.
//! - Se o `DisplayConfig` falhar (sessao remota, driver), o id cai para o nome GDI: ainda e unico
//!   na sessao, so nao sobrevive a reordenacao.
//! - O nome vem do driver (`monitorFriendlyDeviceName`) e e texto nao confiavel: sai cru daqui e o
//!   DTO normaliza (SEC-07, CT-03).

use std::mem::size_of;

use launcher_core::application::ports::DisplayTopology;
use launcher_core::domain::display::{is_valid_monitor_id, Monitor, Rect};
use windows::Win32::Devices::Display::{
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QueryDisplayConfig,
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, QDC_ONLY_ACTIVE_PATHS,
};
use windows::Win32::Foundation::{BOOL, ERROR_SUCCESS, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};

const MONITORINFOF_PRIMARY: u32 = 1;

/// Monitor como o GDI enxerga.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GdiMonitor {
    pub device: String,
    pub primary: bool,
    pub rect: Rect,
}

/// Alvo (monitor fisico) de um caminho ativo, ligado a uma fonte GDI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetName {
    pub gdi_device: String,
    pub device_path: String,
    pub friendly_name: String,
}

/// Junta GDI e DisplayConfig (puro e testavel). Ordem: principal primeiro, depois por posicao.
pub fn assemble(gdi: &[GdiMonitor], targets: &[TargetName]) -> Vec<Monitor> {
    let mut out: Vec<Monitor> = Vec::new();
    for g in gdi {
        let target = targets.iter().find(|t| t.gdi_device == g.device);
        let id = target
            .map(|t| t.device_path.as_str())
            .filter(|p| is_valid_monitor_id(p))
            .unwrap_or(g.device.as_str());
        if !is_valid_monitor_id(id) || out.iter().any(|m| m.id == id) {
            continue; // sem id usavel ou repetido (tela clonada): fica so o primeiro
        }
        let name = target
            .map(|t| t.friendly_name.trim())
            .filter(|n| !n.is_empty())
            .map_or_else(|| format!("Monitor {}", out.len() + 1), str::to_owned);
        out.push(Monitor {
            id: id.to_owned(),
            name,
            primary: g.primary,
            rect: g.rect,
        });
    }
    out.sort_by_key(|m| (!m.primary, m.rect.x, m.rect.y));
    out
}

fn wide_to_string(buf: &[u16]) -> String {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n])
}

unsafe extern "system" fn collect(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    // SAFETY: `data` e o `&mut Vec<GdiMonitor>` passado por `gdi_monitors`, vivo durante a
    // enumeracao (callback sincrono na mesma thread).
    let list = unsafe { &mut *(data.0 as *mut Vec<GdiMonitor>) };
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: `MONITORINFOEXW` comeca com `MONITORINFO`; `cbSize` diz o tamanho estendido.
    let ok =
        unsafe { GetMonitorInfoW(monitor, std::ptr::from_mut(&mut info).cast::<MONITORINFO>()) };
    if ok.as_bool() {
        let r = info.monitorInfo.rcMonitor;
        list.push(GdiMonitor {
            device: wide_to_string(&info.szDevice),
            primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
            rect: Rect {
                x: r.left,
                y: r.top,
                width: u32::try_from(r.right - r.left).unwrap_or(0),
                height: u32::try_from(r.bottom - r.top).unwrap_or(0),
            },
        });
    }
    BOOL(1)
}

fn gdi_monitors() -> Vec<GdiMonitor> {
    let mut list: Vec<GdiMonitor> = Vec::new();
    // SAFETY: o ponteiro para `list` so e usado dentro desta chamada sincrona.
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(std::ptr::from_mut(&mut list) as isize),
        );
    }
    list
}

fn target_names() -> Vec<TargetName> {
    let mut out = Vec::new();
    // SAFETY: buffers de caminhos e modos dimensionados por `GetDisplayConfigBufferSizes`; os
    // pacotes de `DisplayConfigGetDeviceInfo` sao locais com `size` e `type` preenchidos.
    unsafe {
        let (mut paths_n, mut modes_n) = (0u32, 0u32);
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut paths_n, &mut modes_n)
            != ERROR_SUCCESS
        {
            return out;
        }
        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); paths_n as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); modes_n as usize];
        if QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut paths_n,
            paths.as_mut_ptr(),
            &mut modes_n,
            modes.as_mut_ptr(),
            None,
        ) != ERROR_SUCCESS
        {
            return out;
        }
        for path in paths.iter().take(paths_n as usize) {
            let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
            source.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
            source.header.size = size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
            source.header.adapterId = path.sourceInfo.adapterId;
            source.header.id = path.sourceInfo.id;
            if DisplayConfigGetDeviceInfo(&mut source.header) != 0 {
                continue;
            }
            let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target.header.size = size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            target.header.adapterId = path.targetInfo.adapterId;
            target.header.id = path.targetInfo.id;
            if DisplayConfigGetDeviceInfo(&mut target.header) != 0 {
                continue;
            }
            out.push(TargetName {
                gdi_device: wide_to_string(&source.viewGdiDeviceName),
                device_path: wide_to_string(&target.monitorDevicePath),
                friendly_name: wide_to_string(&target.monitorFriendlyDeviceName),
            });
        }
    }
    out
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WinDisplayTopology;

impl DisplayTopology for WinDisplayTopology {
    fn monitors(&self) -> Vec<Monitor> {
        assemble(&gdi_monitors(), &target_names())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gdi(device: &str, primary: bool, x: i32) -> GdiMonitor {
        GdiMonitor {
            device: device.into(),
            primary,
            rect: Rect {
                x,
                y: 0,
                width: 1920,
                height: 1080,
            },
        }
    }

    fn tgt(device: &str, path: &str, name: &str) -> TargetName {
        TargetName {
            gdi_device: device.into(),
            device_path: path.into(),
            friendly_name: name.into(),
        }
    }

    #[test]
    fn id_estavel_vem_do_caminho_do_dispositivo_e_o_principal_vai_primeiro() {
        let list = assemble(
            &[
                gdi(r"\\.\DISPLAY2", false, 1920),
                gdi(r"\\.\DISPLAY1", true, 0),
            ],
            &[
                tgt(r"\\.\DISPLAY1", r"\\?\DISPLAY#AAA#1", "Monitor do PC"),
                tgt(r"\\.\DISPLAY2", r"\\?\DISPLAY#SAM#2", "SAMSUNG TV"),
            ],
        );
        let ids: Vec<_> = list.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, [r"\\?\DISPLAY#AAA#1", r"\\?\DISPLAY#SAM#2"]);
        assert!(list[0].primary && !list[1].primary);
        assert_eq!(list[1].name, "SAMSUNG TV");
    }

    #[test]
    fn sem_displayconfig_o_id_cai_para_o_nome_gdi_e_o_nome_e_generico() {
        let list = assemble(&[gdi(r"\\.\DISPLAY1", true, 0)], &[]);
        assert_eq!(list[0].id, r"\\.\DISPLAY1");
        assert_eq!(list[0].name, "Monitor 1");
    }

    #[test]
    fn nome_vazio_ou_so_espacos_vira_generico_e_id_invalido_cai_no_gdi() {
        let list = assemble(
            &[gdi("A", true, 0), gdi("B", false, 10)],
            &[
                tgt("A", "caminho-ok", "   "),
                tgt("B", &"x".repeat(300), "Tela B"),
            ],
        );
        assert_eq!(list[0].name, "Monitor 1");
        assert_eq!(list[1].id, "B"); // caminho de 300 caracteres nao serve de id
        assert_eq!(list[1].name, "Tela B");
    }

    #[test]
    fn tela_clonada_com_mesmo_caminho_aparece_uma_vez() {
        let list = assemble(
            &[gdi("A", true, 0), gdi("B", false, 0)],
            &[tgt("A", "mesmo", "X"), tgt("B", "mesmo", "X")],
        );
        assert_eq!(list.len(), 1);
    }

    /// So leitura do que esta ligado nesta maquina.
    #[test]
    fn monitores_reais_tem_um_principal_ids_unicos_e_validos() {
        let list = WinDisplayTopology.monitors();
        assert!(!list.is_empty(), "esta maquina tem ao menos uma tela");
        assert_eq!(list.iter().filter(|m| m.primary).count(), 1, "{list:?}");
        let mut ids: Vec<_> = list.iter().map(|m| m.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), list.len());
        for m in &list {
            assert!(is_valid_monitor_id(&m.id), "{m:?}");
            assert!(m.rect.width > 0 && m.rect.height > 0);
        }
        eprintln!("monitores: {list:?}");
    }
}
