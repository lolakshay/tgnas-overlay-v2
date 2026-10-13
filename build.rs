fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set("FileDescription", "Host Process Utility");
        res.set("ProductName", "Windows Desktop Host");
        res.set("CompanyName", "Windows Desktop Components");
        res.set("OriginalFilename", "ScreenClippingHost.exe");
        res.set("LegalCopyright", "Copyright (C) 2026");
        res.set("ProductVersion", "1.0.0.0");
        res.set("FileVersion", "1.0.0.0");
        let _ = res.compile();
    }
}
