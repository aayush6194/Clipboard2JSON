fn main() {
    if !cfg!(target_os = "linux") { return; }

    pkg_config::Config::new().atleast_version("1.4.99.1").probe("xfixes").unwrap();
}