fn export_pages_by_parity_rendered(
    path: &Path,
    output_dir: &Path,
    odd: bool,
    ext: &str,
    render: ParityPageRenderFn,
) -> Result<Vec<String>, String> {
    let kind = match ext {
        "png" => ExportImageKind::Png,
        "jpg" | "jpeg" => ExportImageKind::Jpeg,
        "webp" => ExportImageKind::Webp,
        "bmp" => ExportImageKind::Bmp,
        "tiff" => ExportImageKind::Tiff,
        "gif" => ExportImageKind::Gif,
        "ppm" => ExportImageKind::Ppm,
        "tga" => ExportImageKind::Tga,
        "ico" => ExportImageKind::Ico,
        other => return Err(format!("Unsupported export extension: {other}")),
    };
    pdf::export::export_pages_by_parity_rendered(
        path,
        output_dir,
        odd,
        kind,
        render,
        pdf::export::EXPORT_RENDER_W,
        pdf::export::EXPORT_RENDER_H,
    )
}

macro_rules! impl_export_page_cmd {
    ($cmd:ident, $render:expr) => {
        #[tauri::command]
        fn $cmd(path: String, page_index: u32, output_path: String) -> Result<String, String> {
            pdf::export::export_pdf_page(
                &PathBuf::from(&path),
                page_index,
                &PathBuf::from(&output_path),
                $render,
                pdf::export::EXPORT_RENDER_W,
                pdf::export::EXPORT_RENDER_H,
            )
        }
    };
}

macro_rules! impl_export_pages_cmd {
    ($cmd:ident, $kind:expr, $render:expr) => {
        #[tauri::command]
        fn $cmd(path: String, start_page: u32, end_page: u32, output_dir: String) -> Result<Vec<String>, String> {
            pdf::export::export_pdf_pages(
                &PathBuf::from(&path),
                start_page,
                end_page,
                &PathBuf::from(&output_dir),
                $kind,
                $render,
                pdf::export::EXPORT_RENDER_W,
                pdf::export::EXPORT_RENDER_H,
            )
        }
    };
}

macro_rules! impl_parity_image_export_cmds {
    ($odd:ident, $even:ident, $kind:expr, $render:expr) => {
        #[tauri::command]
        fn $odd(path: String, output_dir: String) -> Result<Vec<String>, String> {
            pdf::export::export_pages_by_parity_rendered(
                &PathBuf::from(&path),
                &PathBuf::from(&output_dir),
                true,
                $kind,
                $render,
                pdf::export::EXPORT_RENDER_W,
                pdf::export::EXPORT_RENDER_H,
            )
        }
        #[tauri::command]
        fn $even(path: String, output_dir: String) -> Result<Vec<String>, String> {
            pdf::export::export_pages_by_parity_rendered(
                &PathBuf::from(&path),
                &PathBuf::from(&output_dir),
                false,
                $kind,
                $render,
                pdf::export::EXPORT_RENDER_W,
                pdf::export::EXPORT_RENDER_H,
            )
        }
    };
}

impl_export_page_cmd!(export_pdf_page_png, render_page_png);
impl_export_page_cmd!(export_pdf_page_jpeg, render_page_jpeg);
impl_export_page_cmd!(export_pdf_page_webp, render_page_webp);
impl_export_page_cmd!(export_pdf_page_bmp, render_page_bmp);
impl_export_page_cmd!(export_pdf_page_tiff, render_page_tiff);
impl_export_page_cmd!(export_pdf_page_gif, render_page_gif);
impl_export_page_cmd!(export_pdf_page_ppm, render_page_ppm);
impl_export_page_cmd!(export_pdf_page_tga, render_page_tga);
impl_export_page_cmd!(export_pdf_page_ico, render_page_ico);

impl_export_pages_cmd!(export_pdf_pages_png, ExportImageKind::Png, render_page_png);
impl_export_pages_cmd!(export_pdf_pages_jpeg, ExportImageKind::Jpeg, render_page_jpeg);
impl_export_pages_cmd!(export_pdf_pages_webp, ExportImageKind::Webp, render_page_webp);
impl_export_pages_cmd!(export_pdf_pages_bmp, ExportImageKind::Bmp, render_page_bmp);
impl_export_pages_cmd!(export_pdf_pages_tiff, ExportImageKind::Tiff, render_page_tiff);
impl_export_pages_cmd!(export_pdf_pages_gif, ExportImageKind::Gif, render_page_gif);
impl_export_pages_cmd!(export_pdf_pages_ppm, ExportImageKind::Ppm, render_page_ppm);
impl_export_pages_cmd!(export_pdf_pages_tga, ExportImageKind::Tga, render_page_tga);
impl_export_pages_cmd!(export_pdf_pages_ico, ExportImageKind::Ico, render_page_ico);

impl_parity_image_export_cmds!(export_odd_pages_png, export_even_pages_png, ExportImageKind::Png, render_page_png);
impl_parity_image_export_cmds!(export_odd_pages_jpeg, export_even_pages_jpeg, ExportImageKind::Jpeg, render_page_jpeg);
impl_parity_image_export_cmds!(export_odd_pages_webp, export_even_pages_webp, ExportImageKind::Webp, render_page_webp);
impl_parity_image_export_cmds!(export_odd_pages_bmp, export_even_pages_bmp, ExportImageKind::Bmp, render_page_bmp);
impl_parity_image_export_cmds!(export_odd_pages_tiff, export_even_pages_tiff, ExportImageKind::Tiff, render_page_tiff);
impl_parity_image_export_cmds!(export_odd_pages_gif, export_even_pages_gif, ExportImageKind::Gif, render_page_gif);
impl_parity_image_export_cmds!(export_odd_pages_ppm, export_even_pages_ppm, ExportImageKind::Ppm, render_page_ppm);
impl_parity_image_export_cmds!(export_odd_pages_tga, export_even_pages_tga, ExportImageKind::Tga, render_page_tga);
