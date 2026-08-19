fn main() {
    slint_build::compile("ui/main.slint").unwrap();

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icons/app.ico");
        res.compile().expect("échec de l'intégration de l'icône Windows");
    }
}
