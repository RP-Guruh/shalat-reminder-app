fn main() {
    #[cfg(windows)]
    {
        embed_resource::compile("build/windows.rc", embed_resource::NONE)
            .manifest_optional()
            .expect("Failed to compile resources");
    }

    slint_build::compile("ui/app-window.slint").expect("Slint build failed");
}

