//! Nhúng icon, manifest (DPI, giao diện Windows mới) và thông tin phiên bản vào GoViet.exe.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=res");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let res = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("res");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let path = |name: &str| res.join(name).display().to_string().replace('\\', "/");

    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut parts: Vec<u32> = version
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|p| p.parse().ok())
        .collect();
    parts.resize(4, 0);
    let commas = parts
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let dotted = parts
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(".");

    let manifest = include_str!("res/goviet.manifest").replace("{VERSION}", &dotted);
    let manifest_path = out.join("goviet.manifest");
    std::fs::write(&manifest_path, manifest).unwrap();

    let rc = format!(
        r#"#pragma code_page(65001)
1 ICON "{icon}"
2 ICON "{icon_off}"
1 24 "{manifest}"
1 VERSIONINFO
FILEVERSION {commas}
PRODUCTVERSION {commas}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "Zenix Labs"
      VALUE "FileDescription", "GoViet - Bộ gõ tiếng Việt"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "GoViet"
      VALUE "LegalCopyright", "© 2026 Zenix Labs - https://zenix.vn"
      VALUE "OriginalFilename", "GoViet.exe"
      VALUE "ProductName", "GoViet"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        icon = path("goviet.ico"),
        icon_off = path("goviet-off.ico"),
        manifest = manifest_path.display().to_string().replace('\\', "/"),
    );
    let dialog = include_str!("res/dialog.rc").replace("{VERSION}", &version);
    let rc_path = out.join("goviet.rc");
    std::fs::write(&rc_path, rc + &dialog).unwrap();
    embed_resource::compile(&rc_path, embed_resource::NONE)
        .manifest_required()
        .unwrap();
}
