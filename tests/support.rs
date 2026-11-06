pub fn fixture(name: &str) -> &'static [u8] {
    match name {
        "localconfig.vdf" => include_bytes!("data/localconfig.vdf"),
        "ProjectZomboid64.bat" => include_bytes!("data/ProjectZomboid64.bat"),
        "ProjectZomboid64.json" => include_bytes!("data/ProjectZomboid64.json"),
        "appworkshop_108600.acf" => include_bytes!("data/appworkshop_108600.acf"),
        _ => panic!("Unknown test fixture: {name}"),
    }
}
