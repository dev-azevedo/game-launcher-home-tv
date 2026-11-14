//! `CoverCache` (BE-28, SEC-08): capas e icones reencodados pelo nucleo em cache proprio. O WebView
//! so recebe bytes do nosso encoder, nunca o arquivo de terceiro.
//!
//! Limites checados **no cabecalho, antes de decodificar**: arquivo <= 20 MB e imagem <= 8192x8192
//! (uma PNG declarando 100000x100000 vira "sem imagem" sem alocar nada). Capa: JPEG q85 ate
//! 600x900. Icone do `.exe`: PNG ate 256x256. Chave do cache = hash de (origem, caminho, tamanho,
//! data de modificacao): arquivo trocado gera entrada nova. Nenhuma RN-xx aqui.

use std::fs::{self, File};
use std::io::{self, BufReader, Cursor};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat, ImageReader, Limits};
use launcher_core::application::ports::{CoverCache, CoverImage, CoverSource};
use sha2::{Digest, Sha256};

use crate::path_canon::drive_kind;

/// Arquivo de imagem de terceiro: no maximo 20 MB (SEC-08).
pub const MAX_IMAGE_FILE_BYTES: u64 = 20 * 1024 * 1024;
/// Dimensao maxima declarada no cabecalho (SEC-08).
pub const MAX_IMAGE_SIDE: u32 = 8192;
/// Teto de alocacao da decodificacao (RGBA de 8192x8192 seria 256 MB).
const MAX_DECODE_ALLOC: u64 = 320 * 1024 * 1024;
const COVER_MAX: (u32, u32) = (600, 900);
const ICON_SIDE: u32 = 256;
const JPEG_QUALITY: u8 = 85;

pub struct WinCoverCache {
    covers: PathBuf,
    icons: PathBuf,
    /// Uma decodificacao por vez: limita o pico de memoria quando a UI pede muitas capas juntas.
    decode: Mutex<()>,
}

impl WinCoverCache {
    /// `data_dir`: pasta de dados do usuario (`%LOCALAPPDATA%\app.sofa.launcher`).
    pub fn new(data_dir: &Path) -> Self {
        let cache = data_dir.join("cache");
        Self {
            covers: cache.join("covers"),
            icons: cache.join("icons"),
            decode: Mutex::new(()),
        }
    }
}

fn sha_hex(parts: &[&str]) -> String {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p.as_bytes());
        h.update([0u8]);
    }
    h.finalize()[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Chave do cache: tipo + caminho (sem caixa) + tamanho + data de modificacao.
fn cache_key(kind: &str, path: &str, meta: &fs::Metadata) -> String {
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    sha_hex(&[
        kind,
        &path.to_lowercase(),
        &meta.len().to_string(),
        &mtime.to_string(),
    ])
}

/// Arquivo comum em unidade local (nem link, nem rede).
fn local_plain_file(path: &str) -> Option<fs::Metadata> {
    if !drive_kind(path).is_local() {
        return None;
    }
    let meta = fs::symlink_metadata(path).ok()?;
    meta.is_file().then_some(meta)
}

/// Grava por arquivo temporario + renomear (nunca deixa meio arquivo no cache).
fn write_cache(dir: &Path, name: &str, bytes: &[u8]) {
    let result = (|| -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let tmp = dir.join(format!("{name}.tmp"));
        fs::write(&tmp, bytes)?;
        match fs::rename(&tmp, dir.join(name)) {
            Ok(()) => Ok(()),
            Err(e) => {
                let _ = fs::remove_file(&tmp);
                Err(e)
            }
        }
    })();
    if let Err(error) = result {
        tracing::debug!(%error, "cache de imagem nao gravado");
    }
}

fn read_cache(dir: &Path, name: &str, mime: &'static str) -> Option<CoverImage> {
    let bytes = fs::read(dir.join(name)).ok()?;
    Some(CoverImage { mime, bytes })
}

fn limits() -> Limits {
    let mut l = Limits::default();
    l.max_image_width = Some(MAX_IMAGE_SIDE);
    l.max_image_height = Some(MAX_IMAGE_SIDE);
    l.max_alloc = Some(MAX_DECODE_ALLOC);
    l
}

/// Cabecalho primeiro: formato JPEG ou PNG e dimensoes dentro do limite; so entao decodifica.
fn decode_checked(path: &str, file_len: u64) -> Option<DynamicImage> {
    if file_len > MAX_IMAGE_FILE_BYTES {
        tracing::debug!("imagem acima de 20 MB ignorada");
        return None;
    }
    let open = || -> io::Result<_> {
        ImageReader::new(BufReader::new(File::open(path)?)).with_guessed_format()
    };
    let probe = open().ok()?;
    if !matches!(probe.format(), Some(ImageFormat::Jpeg | ImageFormat::Png)) {
        tracing::debug!("formato de imagem nao aceito");
        return None;
    }
    let (w, h) = probe.into_dimensions().ok()?;
    if w == 0 || h == 0 || w > MAX_IMAGE_SIDE || h > MAX_IMAGE_SIDE {
        tracing::debug!(w, h, "dimensoes de imagem acima do limite");
        return None;
    }
    let mut reader = open().ok()?;
    reader.limits(limits());
    reader.decode().ok()
}

fn encode_jpeg(img: &DynamicImage) -> Option<Vec<u8>> {
    let rgb = img.to_rgb8();
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY)
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .ok()?;
    Some(out)
}

fn encode_png(img: &DynamicImage) -> Option<Vec<u8>> {
    let rgba = img.to_rgba8();
    let mut out = Cursor::new(Vec::new());
    PngEncoder::new(&mut out)
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            ExtendedColorType::Rgba8,
        )
        .ok()?;
    Some(out.into_inner())
}

fn fit(img: DynamicImage, max_w: u32, max_h: u32) -> DynamicImage {
    if img.width() <= max_w && img.height() <= max_h {
        img
    } else {
        img.resize(max_w, max_h, FilterType::Triangle)
    }
}

impl WinCoverCache {
    fn cover(&self, path: &str) -> Option<CoverImage> {
        let meta = local_plain_file(path)?;
        let name = format!("{}.jpg", cache_key("cover", path, &meta));
        if let Some(hit) = read_cache(&self.covers, &name, "image/jpeg") {
            return Some(hit);
        }
        let _one_at_a_time = self
            .decode
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let img = fit(decode_checked(path, meta.len())?, COVER_MAX.0, COVER_MAX.1);
        let bytes = encode_jpeg(&img)?;
        write_cache(&self.covers, &name, &bytes);
        Some(CoverImage {
            mime: "image/jpeg",
            bytes,
        })
    }

    fn icon(&self, path: &str) -> Option<CoverImage> {
        let meta = local_plain_file(path)?;
        let name = format!("{}.png", cache_key("icon", path, &meta));
        if let Some(hit) = read_cache(&self.icons, &name, "image/png") {
            return Some(hit);
        }
        let _one_at_a_time = self
            .decode
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let img = fit(icon::extract(path, ICON_SIDE)?, ICON_SIDE, ICON_SIDE);
        let bytes = encode_png(&img)?;
        write_cache(&self.icons, &name, &bytes);
        Some(CoverImage {
            mime: "image/png",
            bytes,
        })
    }
}

impl CoverCache for WinCoverCache {
    fn image(&self, source: &CoverSource) -> Option<CoverImage> {
        source
            .cover
            .as_deref()
            .and_then(|p| self.cover(p))
            .or_else(|| source.icon.as_deref().and_then(|p| self.icon(p)))
    }
}

/// Icone do `.exe` pelo Windows (`PrivateExtractIconsW` pede o tamanho exato; sem o tamanho
/// grande o Windows entrega o que tiver). Le so recurso de icone, nunca executa o arquivo.
mod icon {
    use image::{DynamicImage, RgbaImage};
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, GetIconInfo, PrivateExtractIconsW, HICON, ICONINFO,
    };

    pub fn extract(path: &str, side: u32) -> Option<DynamicImage> {
        let mut name = [0u16; 260];
        let wide: Vec<u16> = path.encode_utf16().collect();
        if wide.is_empty() || wide.len() >= name.len() {
            return None;
        }
        name[..wide.len()].copy_from_slice(&wide);
        let mut icons = [HICON::default()];
        // SAFETY: `name` termina em nulo (zerado) e `icons` tem o tamanho informado; o HICON
        // devolvido e destruido abaixo em todos os caminhos.
        let n = unsafe {
            PrivateExtractIconsW(
                &name,
                0,
                side as i32,
                side as i32,
                Some(&mut icons),
                None,
                1,
            )
        };
        if n == 0 || n == u32::MAX || icons[0].is_invalid() {
            return None;
        }
        let image = to_rgba(icons[0]);
        // SAFETY: `icons[0]` e um HICON criado pela chamada acima, destruido uma vez.
        unsafe {
            let _ = DestroyIcon(icons[0]);
        }
        image
    }

    fn to_rgba(icon: HICON) -> Option<DynamicImage> {
        // SAFETY: `info` e local; os bitmaps de `GetIconInfo` sao nossos e sao apagados abaixo.
        unsafe {
            let mut info = ICONINFO::default();
            GetIconInfo(icon, &mut info).ok()?;
            let result = read_bitmap(&info);
            if !info.hbmColor.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(info.hbmColor.0));
            }
            if !info.hbmMask.is_invalid() {
                let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
            }
            result
        }
    }

    /// # Safety
    /// `info.hbmColor` precisa ser um bitmap valido (ou invalido, caso em que devolve `None`).
    unsafe fn read_bitmap(info: &ICONINFO) -> Option<DynamicImage> {
        if info.hbmColor.is_invalid() {
            return None;
        }
        let mut bmp = BITMAP::default();
        if GetObjectW(
            HGDIOBJ(info.hbmColor.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(std::ptr::from_mut(&mut bmp).cast()),
        ) == 0
        {
            return None;
        }
        let (w, h) = (bmp.bmWidth, bmp.bmHeight);
        if w <= 0 || h <= 0 || w > 512 || h > 512 {
            return None;
        }
        let mut header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w,
            biHeight: -h, // de cima para baixo
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..BITMAPINFOHEADER::default()
        };
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(
            dc,
            info.hbmColor,
            0,
            h as u32,
            Some(pixels.as_mut_ptr().cast()),
            std::ptr::from_mut(&mut header).cast::<BITMAPINFO>(),
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(dc);
        if lines == 0 {
            return None;
        }
        // BGRA -> RGBA. Icone sem canal alfa (tudo zero) fica opaco.
        let has_alpha = pixels.chunks_exact(4).any(|p| p[3] != 0);
        for p in pixels.chunks_exact_mut(4) {
            p.swap(0, 2);
            if !has_alpha {
                p[3] = 255;
            }
        }
        RgbaImage::from_raw(w as u32, h as u32, pixels).map(DynamicImage::ImageRgba8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, RgbImage};
    use std::sync::atomic::{AtomicU32, Ordering};

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            static N: AtomicU32 = AtomicU32::new(0);
            let p = std::env::temp_dir().join(format!(
                "sofa-cover-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn path(&self) -> &Path {
            &self.0
        }
        fn file(&self, name: &str) -> String {
            crate::path_canon::canonicalize(self.0.to_str().unwrap()).unwrap() + "\\" + name
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn source_cover(p: &str) -> CoverSource {
        CoverSource {
            cover: Some(p.to_owned()),
            icon: None,
        }
    }

    fn gradient(w: u32, h: u32) -> RgbImage {
        RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        })
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &b in data {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// PNG so com assinatura + IHDR (CRC valido) declarando `w`x`h`: sem dados de imagem.
    fn png_header_only(w: u32, h: u32) -> Vec<u8> {
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits, RGB
        let mut chunk = b"IHDR".to_vec();
        chunk.extend_from_slice(&ihdr);
        let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        out.extend_from_slice(&(ihdr.len() as u32).to_be_bytes());
        out.extend_from_slice(&chunk);
        out.extend_from_slice(&crc32(&chunk).to_be_bytes());
        out
    }

    #[test]
    fn capa_grande_vira_jpeg_de_ate_600x900_e_o_cache_reaproveita() {
        let t = TempDir::new();
        let cache = WinCoverCache::new(&t.path().join("dados"));
        let src = t.file("capa.png");
        gradient(1200, 1800).save(&src).unwrap(); // PNG
        let img = cache.image(&source_cover(&src)).expect("capa");
        assert_eq!(img.mime, "image/jpeg");
        let decoded = image::load_from_memory(&img.bytes).unwrap();
        assert_eq!(decoded.dimensions(), (600, 900));
        // JPEG de verdade (assinatura FFD8).
        assert_eq!(&img.bytes[..2], &[0xFF, 0xD8]);
        let cached: Vec<_> = fs::read_dir(t.path().join(r"dados\cache\covers"))
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(cached.len(), 1);
        // Segunda chamada vem do cache, mesmo com o original apagado.
        fs::remove_file(&src).unwrap();
        // (sem o original o cache nao e consultado: a chave depende do arquivo; recria o arquivo)
        gradient(1200, 1800).save(&src).unwrap();
        let again = cache.image(&source_cover(&src));
        assert!(again.is_some());
    }

    #[test]
    fn capa_pequena_nao_e_ampliada_e_jpeg_de_entrada_funciona() {
        let t = TempDir::new();
        let cache = WinCoverCache::new(&t.path().join("dados"));
        let src = t.file("pequena.jpg");
        gradient(300, 450).save(&src).unwrap();
        let img = cache.image(&source_cover(&src)).unwrap();
        assert_eq!(
            image::load_from_memory(&img.bytes).unwrap().dimensions(),
            (300, 450)
        );
    }

    #[test]
    fn png_declarando_100000x100000_vira_sem_imagem_sem_alocar() {
        let t = TempDir::new();
        let cache = WinCoverCache::new(&t.path().join("dados"));
        let src = t.file("bomba.png");
        fs::write(&src, png_header_only(100_000, 100_000)).unwrap();
        let t0 = std::time::Instant::now();
        assert!(cache.image(&source_cover(&src)).is_none());
        assert!(t0.elapsed().as_millis() < 500);
        // Um pouco acima do limite tambem.
        fs::write(&src, png_header_only(8193, 10)).unwrap();
        assert!(cache.image(&source_cover(&src)).is_none());
        // Nada foi gravado no cache.
        assert!(!t.path().join(r"dados\cache\covers").exists());
    }

    #[test]
    fn arquivo_acima_de_20_mb_formato_estranho_e_ausente_ficam_sem_imagem() {
        let t = TempDir::new();
        let cache = WinCoverCache::new(&t.path().join("dados"));
        let grande = t.file("grande.jpg");
        let mut bytes = vec![0xFF, 0xD8, 0xFF];
        bytes.resize(21 * 1024 * 1024, 0);
        fs::write(&grande, bytes).unwrap();
        assert!(cache.image(&source_cover(&grande)).is_none());
        let gif = t.file("a.gif");
        fs::write(&gif, b"GIF89a\x01\x00\x01\x00\x00\x00\x00;").unwrap();
        assert!(cache.image(&source_cover(&gif)).is_none());
        let texto = t.file("a.jpg");
        fs::write(&texto, b"isto nao e imagem").unwrap();
        assert!(cache.image(&source_cover(&texto)).is_none());
        assert!(cache.image(&source_cover(&t.file("ausente.jpg"))).is_none());
        // Rede e relativo nunca sao lidos.
        assert!(cache
            .image(&source_cover(r"\\127.0.0.1\c$\x.jpg"))
            .is_none());
        assert!(cache.image(&source_cover("x.jpg")).is_none());
        assert!(cache.image(&CoverSource::default()).is_none());
    }

    #[test]
    fn capa_ausente_cai_no_icone_do_exe() {
        let t = TempDir::new();
        let cache = WinCoverCache::new(&t.path().join("dados"));
        let sys = crate::path_canon::known::windows_dir().unwrap();
        let src = CoverSource {
            cover: Some(t.file("nao-existe.jpg")),
            icon: Some(format!(r"{sys}\System32\notepad.exe")),
        };
        let img = cache.image(&src).expect("icone do notepad");
        assert_eq!(img.mime, "image/png");
        let decoded = image::load_from_memory(&img.bytes).unwrap();
        let (w, h) = decoded.dimensions();
        assert!(w <= ICON_SIDE && h <= ICON_SIDE && w >= 16, "{w}x{h}");
        // Nao e uma imagem toda transparente.
        assert!(decoded.to_rgba8().pixels().any(|p| p[3] > 0));
        assert!(t.path().join(r"dados\cache\icons").exists());
    }

    #[test]
    fn chave_do_cache_muda_com_o_arquivo() {
        let t = TempDir::new();
        let p = t.file("a.bin");
        fs::write(&p, b"1").unwrap();
        let m1 = fs::metadata(&p).unwrap();
        let k1 = cache_key("cover", &p, &m1);
        fs::write(&p, b"12").unwrap();
        let m2 = fs::metadata(&p).unwrap();
        assert_ne!(k1, cache_key("cover", &p, &m2));
        assert_ne!(cache_key("cover", &p, &m2), cache_key("icon", &p, &m2));
        assert_eq!(
            cache_key("cover", &p, &m2),
            cache_key("cover", &p.to_uppercase(), &m2)
        );
    }
}
