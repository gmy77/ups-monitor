// Incorpora l'icona nell'eseguibile Windows
fn main() {
    println!("cargo:rerun-if-changed=assets/ups-monitor.ico");
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/ups-monitor.ico");
    res.set("FileDescription", "UPS Monitor");
    res.set("ProductName", "UPS Monitor");
    if let Err(e) = res.compile() {
        println!("cargo:warning=icona non incorporata: {e}");
    }
}
