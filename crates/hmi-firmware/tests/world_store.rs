#[path = "../src/world_store.rs"]
mod world_store;
use hmi_core::living::{Automaton, Rule, GRID_HEIGHT, GRID_WIDTH};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};
use world_store::WorldStore;

#[test]
fn sd_roundtrip_corrupt_primary_and_interrupted_rename_recover_backup() {
    let root = std::env::temp_dir().join(format!(
        "living-store-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let store = WorldStore::new(&root);
    let mut world = Automaton::empty(GRID_WIDTH, GRID_HEIGHT, Rule::Life, 7);
    world.reseed();
    world.step();
    let first = world.snapshot();
    store.save(Rule::Life, &first).unwrap();
    world.step();
    let second = world.snapshot();
    store.save(Rule::Life, &second).unwrap();
    assert_eq!(store.load(Rule::Life).unwrap().snapshot(), second);
    fs::write(root.join("world-00.bin"), b"interrupted data").unwrap();
    assert_eq!(store.load(Rule::Life).unwrap().snapshot(), first);
    fs::remove_file(root.join("world-00.bin")).unwrap();
    assert_eq!(store.load(Rule::Life).unwrap().snapshot(), first);
    fs::write(root.join("world-00.tmp"), b"partial new snapshot").unwrap();
    assert_eq!(store.load(Rule::Life).unwrap().snapshot(), first);
    assert!(store.load(Rule::Brain).is_none());
    assert!(store.save(Rule::Brain, &first).is_err());
    store.save(Rule::Life, &second).unwrap();
    assert_eq!(store.load(Rule::Life).unwrap().snapshot(), second);
    fs::remove_dir_all(root).unwrap();
}
