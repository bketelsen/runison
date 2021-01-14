fn main() {
    let mut config = prost_build::Config::new();
    config.btree_map(&["."]);
    tonic_build::configure()
        .type_attribute("runison.Point", "#[derive(Hash)]")
        .compile(&["proto/runison/runison.proto"], &["proto"])
        .unwrap()
}
